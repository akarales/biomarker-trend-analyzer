import { z } from 'zod/mini';

import { RULES, SEVERITIES, STATUSES, TRENDS } from '@/shared/domain';

/**
 * Response schemas for every endpoint the UI calls (mirrors
 * crates/drift/src/model.rs and crates/api/src/routes/views.rs). Responses
 * are parsed at the boundary (api/http.ts), so server/client drift fails
 * loudly with a readable error instead of leaking `undefined` into the
 * clinical UI. Types are inferred. Times are epoch seconds unless noted.
 */

export const ObservationSchema = z.object({
  /** RFC 3339, UTC */
  taken_at: z.string(),
  value: z.number(),
  unit: z.string(),
  source: z.string(),
});
export type Observation = z.infer<typeof ObservationSchema>;

const PointSchema = z.object({ t: z.number(), v: z.number() });
export type Point = z.infer<typeof PointSchema>;

export const SignalSchema = z.object({
  rule: z.enum(RULES),
  severity: z.enum(SEVERITIES),
  t: z.number(),
  value: z.number(),
  threshold: z.nullable(z.number()),
  explanation: z.string(),
  source: z.string(),
});
export type Signal = z.infer<typeof SignalSchema>;

export const BaselineSchema = z.object({
  n: z.number(),
  from: z.number(),
  to: z.number(),
  set_point: z.number(),
  prri_low: z.number(),
  prri_high: z.number(),
  level: z.number(),
});
export type Baseline = z.infer<typeof BaselineSchema>;

export const TrendSchema = z.object({
  n: z.number(),
  from: z.number(),
  slope_per_day: z.number(),
  ci_low_per_day: z.number(),
  ci_high_per_day: z.number(),
  change_per_year: z.number(),
  tau: z.number(),
  p_value: z.number(),
  direction: z.enum(TRENDS),
});
export type Trend = z.infer<typeof TrendSchema>;

export const DriftReportSchema = z.object({
  code: z.string(),
  analyte: z.nullable(
    z.object({
      code: z.string(),
      loinc: z.string(),
      display: z.string(),
      cvi: z.number(),
      cva: z.number(),
      cvi_source: z.string(),
      cva_source: z.string(),
      reviewed: z.string(),
    }),
  ),
  /** canonical unit of `points` and every value in the report */
  unit: z.string(),
  as_of: z.nullable(z.number()),
  window_days: z.number(),
  points: z.array(PointSchema),
  excluded: z.object({ after_as_of: z.number(), unit_unknown: z.number() }),
  latest: z.nullable(PointSchema),
  baseline: z.nullable(BaselineSchema),
  population: z.nullable(z.object({ low: z.number(), high: z.number(), source: z.string() })),
  thresholds: z.array(
    z.object({
      value: z.number(),
      direction: z.enum(['above', 'below']),
      severity: z.enum(SEVERITIES),
      label: z.string(),
      source: z.string(),
    }),
  ),
  rcv: z.nullable(z.object({ up: z.number(), down: z.number() })),
  rcv_jumps: z.array(z.object({ from_t: z.number(), to_t: z.number(), change: z.number() })),
  ewma: z.nullable(z.object({ value: z.number(), low: z.number(), high: z.number() })),
  change_point: z.nullable(
    z.object({ t: z.number(), detected_t: z.number(), before: z.number(), after: z.number(), change: z.number() }),
  ),
  trend: z.nullable(TrendSchema),
  /** worst first */
  signals: z.array(SignalSchema),
  not_assessed: z.array(z.object({ rule: z.enum(RULES), reason: z.string() })),
  status: z.enum(STATUSES),
});
export type DriftReport = z.infer<typeof DriftReportSchema>;

export const REVIEW_STATES = ['unreviewed', 'acknowledged', 'dismissed'] as const;
export type ReviewState = (typeof REVIEW_STATES)[number];
export const REVIEW_ACTIONS = ['acknowledge', 'annotate', 'dismiss', 'reopen'] as const;
export type ReviewAction = (typeof REVIEW_ACTIONS)[number];

/** Review status of one signal (crates/api/src/review.rs), aligned with `report.signals`. */
export const SignalReviewSchema = z.object({
  rule: z.enum(RULES),
  t: z.number(),
  state: z.enum(REVIEW_STATES),
  decided_by: z.nullable(z.string()),
  decided_at: z.nullable(z.number()),
  reason: z.nullable(z.string()),
  notes: z.number(),
});
export type SignalReview = z.infer<typeof SignalReviewSchema>;

/** One append-only audit event, with the server's snapshot at decision time. */
export const ReviewEventSchema = z.object({
  id: z.number(),
  patient_id: z.string(),
  code: z.string(),
  rule: z.enum(RULES),
  signal_t: z.number(),
  action: z.enum(REVIEW_ACTIONS),
  reason: z.nullable(z.string()),
  actor: z.string(),
  snapshot: z.object({ status: z.enum(STATUSES), signal: SignalSchema }),
  created_at: z.number(),
});
export type ReviewEvent = z.infer<typeof ReviewEventSchema>;

export const ReviewCreatedSchema = z.object({ event: ReviewEventSchema, review: SignalReviewSchema });
export type ReviewCreated = z.infer<typeof ReviewCreatedSchema>;

/** A triage listing row (crates/api/src/triage.rs), worst first. */
export const PatientSummaryEntrySchema = z.object({
  patient_id: z.string(),
  biomarkers: z.number(),
  observations: z.number(),
  status: z.enum(STATUSES),
  /** biomarkers whose status is alert / watch */
  alerts: z.number(),
  watches: z.number(),
  /** watch/alert signals not yet acknowledged or dismissed */
  unreviewed: z.number(),
  /** the most severe UNREVIEWED signal */
  top_signal: z.nullable(
    z.object({
      code: z.string(),
      display: z.string(),
      rule: z.enum(RULES),
      severity: z.enum(SEVERITIES),
      explanation: z.string(),
    }),
  ),
});
export type PatientSummaryEntry = z.infer<typeof PatientSummaryEntrySchema>;

export const PatientsResponseSchema = z.object({
  as_of: z.nullable(z.number()),
  window_days: z.number(),
  patients: z.array(PatientSummaryEntrySchema),
});
export type PatientsResponse = z.infer<typeof PatientsResponseSchema>;

export const PatientSummarySchema = z.object({
  patient_id: z.string(),
  /** null = each series' latest result */
  as_of: z.nullable(z.number()),
  window_days: z.number(),
  reports: z.array(DriftReportSchema),
  /** review status per biomarker code, aligned with each report's signals */
  reviews: z.record(z.string(), z.array(SignalReviewSchema)),
});
export type PatientSummary = z.infer<typeof PatientSummarySchema>;

export const BiomarkerSeriesSchema = z.object({
  patient_id: z.string(),
  code: z.string(),
  observations: z.array(ObservationSchema),
  report: DriftReportSchema,
  /** review status of each signal (same order as report.signals) */
  reviews: z.array(SignalReviewSchema),
  /** audit trail, newest first */
  history: z.array(ReviewEventSchema),
});
export type BiomarkerSeries = z.infer<typeof BiomarkerSeriesSchema>;

export const UploadResultSchema = z.object({
  format: z.enum(['csv', 'fhir']),
  inserted: z.number(),
  /** rows already stored (same patient, code and timestamp) — skipped */
  duplicates: z.number(),
  /** resources not imported (FHIR: not final, no LOINC, no analyte profile, …) */
  skipped: z.number(),
  skipped_reasons: z.array(z.object({ reason: z.string(), count: z.number() })),
  patients: z.number(),
  biomarkers: z.array(z.string()),
});
export type UploadResult = z.infer<typeof UploadResultSchema>;

// ── "Explain this drift" (crates/api/src/routes/explain*.rs) ─────────────

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
