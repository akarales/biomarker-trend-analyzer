import { z } from 'zod/mini';

import { RULES, STATUSES } from '@/shared/domain';

import { SignalSchema } from './schemas';

/**
 * "Explain this drift" response schemas (crates/api/src/routes/explain*.rs):
 * model chooser, stream events, final body (computed status + signals
 * re-asserted). Split from schemas.ts to keep files small.
 */

export const LLM_PROVIDERS = ['stub', 'ollama', 'anthropic'] as const;
export type LlmProvider = (typeof LLM_PROVIDERS)[number];

export const ModelOptionSchema = z.object({
  provider: z.enum(LLM_PROVIDERS),
  id: z.string(),
  label: z.string(),
  /** Ollama only: already resident in the shared instance */
  loaded: z.optional(z.boolean()),
  size_gb: z.optional(z.number()),
});
export type ModelOption = z.infer<typeof ModelOptionSchema>;

export const ModelsResponseSchema = z.object({
  default: z.object({ provider: z.enum(LLM_PROVIDERS), model: z.string() }),
  providers: z.array(
    z.object({
      provider: z.enum(LLM_PROVIDERS),
      available: z.boolean(),
      note: z.optional(z.string()),
      models: z.array(ModelOptionSchema),
    }),
  ),
});
export type ModelsResponse = z.infer<typeof ModelsResponseSchema>;

/** The computed facts + model identity (first stream line). */
export const ExplainMetaSchema = z.object({
  patient_id: z.string(),
  code: z.string(),
  as_of: z.nullable(z.number()),
  window_days: z.number(),
  computed_status: z.enum(STATUSES),
  provider: z.enum(LLM_PROVIDERS),
  model: z.string(),
  stub: z.boolean(),
  disclaimer: z.string(),
});
export type ExplainMeta = z.infer<typeof ExplainMetaSchema>;

export const EXPLANATION_FIELDS = ['summary', 'interpretation', 'follow_up', 'limitations'] as const;
export type ExplanationField = (typeof EXPLANATION_FIELDS)[number];

export const ExplanationSchema = z.object({
  summary: z.string(),
  interpretation: z.string(),
  follow_up: z.string(),
  limitations: z.string(),
  status: z.enum(STATUSES),
});
export type Explanation = z.infer<typeof ExplanationSchema>;

/** Final body (also the non-streaming response): computed status + signals re-asserted. */
export const ExplainResponseSchema = z.extend(ExplainMetaSchema, {
  explanation: ExplanationSchema,
  status_overridden: z.boolean(),
  signals: z.array(SignalSchema),
  not_assessed: z.array(z.object({ rule: z.enum(RULES), reason: z.string() })),
});
export type ExplainResponse = z.infer<typeof ExplainResponseSchema>;

/** One NDJSON line of POST /explain/stream. */
export const StreamEventSchema = z.discriminatedUnion('type', [
  z.extend(ExplainMetaSchema, { type: z.literal('start') }),
  z.object({ type: z.literal('delta'), field: z.string(), text: z.string() }),
  z.extend(ExplainResponseSchema, { type: z.literal('done') }),
  z.object({ type: z.literal('error'), code: z.string(), error: z.string() }),
]);
export type StreamEvent = z.infer<typeof StreamEventSchema>;
