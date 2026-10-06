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

export const PatientSummaryEntrySchema = z.object({
  patient_id: z.string(),
  biomarkers: z.number(),
  observations: z.number(),
});
export type PatientSummaryEntry = z.infer<typeof PatientSummaryEntrySchema>;

export const PatientsResponseSchema = z.object({ patients: z.array(PatientSummaryEntrySchema) });
export type PatientsResponse = z.infer<typeof PatientsResponseSchema>;

export const PatientSummarySchema = z.object({
  patient_id: z.string(),
  /** null = each series' latest result */
  as_of: z.nullable(z.number()),
  window_days: z.number(),
  reports: z.array(DriftReportSchema),
});
export type PatientSummary = z.infer<typeof PatientSummarySchema>;

export const BiomarkerSeriesSchema = z.object({
  patient_id: z.string(),
  code: z.string(),
  observations: z.array(ObservationSchema),
  report: DriftReportSchema,
});
export type BiomarkerSeries = z.infer<typeof BiomarkerSeriesSchema>;

export const UploadResultSchema = z.object({
  inserted: z.number(),
  /** rows already stored (same patient, code and timestamp) — skipped */
  duplicates: z.number(),
  patients: z.number(),
  biomarkers: z.array(z.string()),
});
export type UploadResult = z.infer<typeof UploadResultSchema>;
