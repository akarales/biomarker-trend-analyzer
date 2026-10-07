import { useState } from 'react';
import { CircleStop, MessageSquareText, RotateCcw } from 'lucide-react';

import type { ExplainTarget, ModelChoice } from '@/api/llm';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { analysisOptions, useAnalyzer } from '@/state';

import { DraftView } from './DraftView';
import { ModelChooser } from './ModelChooser';
import { runDraft, useExplainStream } from './useExplainStream';

/**
 * "Explain this drift" for the selected biomarker, streamed. Runs are kept
 * per patient, biomarker, as-of, window and model; only the current key is
 * shown, so switching never displays another series' text.
 */
export function ExplainPanel() {
  const patientId = useAnalyzer((s) => s.selectedPatient);
  const code = useAnalyzer((s) => s.series?.code ?? null);
  const asOf = useAnalyzer((s) => s.asOf);
  const windowDays = useAnalyzer((s) => s.windowDays);
  const [choice, setChoice] = useState<ModelChoice | null>(null);
  const { runs, start, stop, fallback } = useExplainStream();
  if (!patientId || !code) return null;

  const target: ExplainTarget = { patientId, code, options: analysisOptions({ asOf, windowDays }) };
  const key = [patientId, code, asOf ?? 'latest', windowDays, choice ? `${choice.provider}:${choice.model}` : 'default'].join('|');
  const run = runs[key];
  const streaming = run?.status === 'streaming';
  const draft = run ? runDraft(run) : {};
  const hasText = Object.values(draft).some(Boolean);

  return (
    <Card role="region" aria-label="Explain this drift">
      <CardHeader>
        <CardTitle>
          <h3 className="text-base">Explain this drift</h3>
        </CardTitle>
        <p className="text-xs text-muted-foreground">
          A draft for the reviewing clinician, grounded in the computed signals above. The signals stay authoritative.
        </p>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex flex-wrap items-end gap-2">
          <div className="min-w-56 flex-1">
            <ModelChooser choice={choice} onChange={setChoice} />
          </div>
          {streaming ? (
            <Button type="button" size="sm" variant="secondary" onClick={stop}>
              <CircleStop aria-hidden="true" /> Stop
            </Button>
          ) : (
            <Button type="button" size="sm" onClick={() => start(key, target, choice)}>
              <MessageSquareText aria-hidden="true" /> {run ? 'Explain again' : `Explain ${code}`}
            </Button>
          )}
        </div>
        {streaming && !hasText && <p className="text-xs text-muted-foreground">Waiting for the model…</p>}
        {run?.status === 'error' && (
          <div role="alert" className="flex flex-wrap items-center gap-2 text-xs text-destructive">
            <span>{run.error}</span>
            <Button type="button" size="xs" variant="ghost" onClick={() => fallback(key, target, choice)}>
              <RotateCcw aria-hidden="true" /> Try without streaming
            </Button>
          </div>
        )}
        {run?.status === 'stopped' && (
          <p role="status" className="text-xs text-muted-foreground">
            Stopped — the model request was cancelled.
          </p>
        )}
        {run && run.status !== 'error' && (run.meta || hasText) && (
          <DraftView draft={draft} meta={run.meta} result={run.result} streaming={streaming} />
        )}
      </CardContent>
    </Card>
  );
}
