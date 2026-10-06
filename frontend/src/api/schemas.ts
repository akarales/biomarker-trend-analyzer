import { z } from 'zod/mini';

import { STATUSES, TRENDS } from '@/shared/domain';

/**
 * Response schemas for every endpoint the UI calls (mirrors
 * crates/api/src/routes/views.rs). Responses are parsed at the boundary
 * (api/http.ts), so server/client drift fails loudly with a readable error
 * instead of leaking `undefined` into the clinical UI. Types are inferred.
 */

export const ObservationSchema = z.object({
  /** `YYYY-MM-DD HH:MM:SS`, naive UTC */
  taken_at: z.string(),
  value: z.number(),
  unit: z.string(),
  source: z.string(),
});
export type Observation = z.infer<typeof ObservationSchema>;

export const BaselineSchema = z.object({
  median: z.number(),
  robust_std: z.number(),
  n: z.number(),
});
export type Baseline = z.infer<typeof BaselineSchema>;

export const DriftReportSchema = z.object({
  code: z.string(),
  unit: z.string(),
  baseline: BaselineSchema,
  latest: z.nullable(z.number()),
  latest_z: z.nullable(z.number()),
  ewma: z.nullable(z.number()),
  ewma_z: z.nullable(z.number()),
  slope_per_day: z.nullable(z.number()),
  trend: z.nullable(z.enum(TRENDS)),
  /** epoch seconds */
  anomalies: z.array(z.object({ t: z.number(), v: z.number() })),
  status: z.enum(STATUSES),
  window_days: z.number(),
  series_len: z.number(),
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
