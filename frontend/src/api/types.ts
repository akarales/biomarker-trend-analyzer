export interface Observation {
  taken_at: string;
  value: number;
  unit: string;
  source: string;
}

export interface Baseline {
  median: number;
  robust_std: number;
  n: number;
}

export type TrendDirection = 'rising' | 'falling' | 'flat';
export type Status = 'normal' | 'watch' | 'alert';

export interface DriftReport {
  code: string;
  unit: string;
  baseline: Baseline;
  latest: number | null;
  latest_z: number | null;
  ewma: number | null;
  ewma_z: number | null;
  slope_per_day: number | null;
  trend: TrendDirection | null;
  anomalies: { t: number; v: number }[];
  status: Status;
  window_days: number;
  series_len: number;
}

export interface PatientSummaryEntry {
  patient_id: string;
  biomarkers: number;
  observations: number;
}

export interface PatientSummary {
  patient_id: string;
  window_days: number;
  reports: DriftReport[];
}

export interface BiomarkerSeries {
  patient_id: string;
  code: string;
  observations: Observation[];
  report: DriftReport;
}

export interface UploadResult {
  inserted: number;
  patients: number;
  biomarkers: string[];
}
