import { api, errorFrom } from './http';
import type { AnalysisOptions } from './patients';
import {
  ExplainResponseSchema,
  ModelsResponseSchema,
  StreamEventSchema,
  type ExplainResponse,
  type LlmProvider,
  type ModelsResponse,
  type StreamEvent,
} from './schemas';

export interface ModelChoice {
  provider: LlmProvider;
  model: string;
}

/** What to explain: one biomarker of one patient, with the chart's options. */
export interface ExplainTarget {
  patientId: string;
  code: string;
  options: AnalysisOptions;
}

function requestBody({ patientId, code, options }: ExplainTarget, choice: ModelChoice | null): string {
  return JSON.stringify({
    patient_id: patientId,
    code,
    as_of: options.asOf,
    window_days: options.windowDays,
    ...choice,
  });
}

export function fetchModels(signal?: AbortSignal): Promise<ModelsResponse> {
  return api('/llm/models', ModelsResponseSchema, { signal });
}

/** Non-streaming fallback (same final body as the stream's `done`). */
export function postExplain(target: ExplainTarget, choice: ModelChoice | null): Promise<ExplainResponse> {
  return api('/explain', ExplainResponseSchema, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: requestBody(target, choice),
  });
}

/**
 * POST /explain/stream: calls `onEvent` for each NDJSON line (start →
 * delta… → done | error). Abort with `signal` (Stop) — the server then
 * drops the upstream model request. Validation errors before streaming
 * throw ApiError (with code + request id).
 */
export async function streamExplain(
  target: ExplainTarget,
  choice: ModelChoice | null,
  onEvent: (event: StreamEvent) => void,
  signal?: AbortSignal,
): Promise<void> {
  const res = await fetch('/api/v1/explain/stream', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: requestBody(target, choice),
    signal,
  });
  if (!res.ok || !res.body) throw await errorFrom(res);
  const reader = res.body.pipeThrough(new TextDecoderStream()).getReader();
  let buffer = '';
  const emit = (line: string) => {
    if (!line.trim()) return;
    const parsed = StreamEventSchema.safeParse(JSON.parse(line));
    if (!parsed.success) throw new Error('Unexpected stream event from the server');
    onEvent(parsed.data);
  };
  for (;;) {
    const { value, done } = await reader.read();
    if (done) break;
    buffer += value;
    let nl = buffer.indexOf('\n');
    while (nl >= 0) {
      emit(buffer.slice(0, nl));
      buffer = buffer.slice(nl + 1);
      nl = buffer.indexOf('\n');
    }
  }
  emit(buffer);
}
