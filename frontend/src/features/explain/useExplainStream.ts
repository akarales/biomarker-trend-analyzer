import { useEffect, useRef, useState } from 'react';

import { postExplain, streamExplain, type ExplainTarget, type ModelChoice } from '@/api/llm';
import type { ExplainMeta, ExplainResponse, ExplanationField } from '@/api/llmSchemas';

export type RunStatus = 'streaming' | 'done' | 'stopped' | 'error';

export interface Run {
  status: RunStatus;
  /** computed facts + disclaimer, from the first line of the stream */
  meta?: ExplainMeta;
  /** text streamed so far, by field */
  partial: Record<string, string>;
  /** the validated final body — replaces the streamed text */
  result?: ExplainResponse;
  error?: string;
}

export type Draft = Partial<Record<ExplanationField, string>>;

/** Fields to render: the validated result once done, else the stream so far. */
export function runDraft(run: Run): Draft {
  if (run.result) return run.result.explanation;
  const { summary, interpretation, follow_up, limitations } = run.partial;
  return { summary, interpretation, follow_up, limitations };
}

const message = (err: unknown) => (err instanceof Error ? err.message : String(err));

/**
 * Streamed drafts, one run per key (patient, biomarker, as-of, window,
 * model). Starting a run cancels the previous one; Stop aborts the request
 * (the server then drops the model call); unmount aborts too.
 */
export function useExplainStream() {
  const [runs, setRuns] = useState<Record<string, Run>>({});
  const controller = useRef<AbortController | null>(null);
  useEffect(() => () => controller.current?.abort(), []);

  const update = (key: string, fn: (run: Run) => Run) =>
    setRuns((r) => ({ ...r, [key]: fn(r[key] ?? { status: 'streaming', partial: {} }) }));

  const start = (key: string, target: ExplainTarget, choice: ModelChoice | null) => {
    controller.current?.abort();
    const ctrl = new AbortController();
    controller.current = ctrl;
    setRuns((r) => ({ ...r, [key]: { status: 'streaming', partial: {} } }));
    streamExplain(
      target,
      choice,
      (event) => {
        if (event.type === 'start') update(key, (run) => ({ ...run, meta: event }));
        else if (event.type === 'delta')
          update(key, (run) => ({ ...run, partial: { ...run.partial, [event.field]: (run.partial[event.field] ?? '') + event.text } }));
        else if (event.type === 'done') update(key, (run) => ({ ...run, status: 'done', result: event, meta: event }));
        else update(key, (run) => ({ ...run, status: 'error', error: event.error }));
      },
      ctrl.signal,
    ).catch((err: unknown) => {
      if (ctrl.signal.aborted) update(key, (run) => (run.status === 'streaming' ? { ...run, status: 'stopped' } : run));
      else update(key, (run) => ({ ...run, status: 'error', error: message(err) }));
    });
  };

  const stop = () => controller.current?.abort();

  /** The non-streaming endpoint, for when streaming fails (e.g. a buffering proxy). */
  const fallback = (key: string, target: ExplainTarget, choice: ModelChoice | null) => {
    setRuns((r) => ({ ...r, [key]: { status: 'streaming', partial: {} } }));
    postExplain(target, choice)
      .then((result) => update(key, (run) => ({ ...run, status: 'done', result, meta: result })))
      .catch((err: unknown) => update(key, (run) => ({ ...run, status: 'error', error: message(err) })));
  };

  return { runs, start, stop, fallback };
}
