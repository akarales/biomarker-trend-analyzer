import { api } from './http';
import {
  BiomarkerSeriesSchema,
  PatientSummarySchema,
  PatientsResponseSchema,
  type BiomarkerSeries,
  type PatientSummary,
  type PatientsResponse,
} from './schemas';

/** Analysis options sent with every analysis request. */
export interface AnalysisOptions {
  /** `YYYY-MM-DD`; null = each series' latest result */
  asOf: string | null;
  windowDays: number;
}

export function analysisQuery({ asOf, windowDays }: AnalysisOptions): string {
  const params = new URLSearchParams({ window_days: String(windowDays) });
  if (asOf) params.set('as_of', asOf);
  return `?${params.toString()}`;
}

export function fetchPatients(options: AnalysisOptions): Promise<PatientsResponse> {
  return api(`/patients${analysisQuery(options)}`, PatientsResponseSchema);
}

export function fetchPatientSummary(patientId: string, options: AnalysisOptions): Promise<PatientSummary> {
  return api(`/patients/${encodeURIComponent(patientId)}/summary${analysisQuery(options)}`, PatientSummarySchema);
}

export function fetchSeries(patientId: string, code: string, options: AnalysisOptions): Promise<BiomarkerSeries> {
  return api(
    `/patients/${encodeURIComponent(patientId)}/biomarkers/${encodeURIComponent(code)}${analysisQuery(options)}`,
    BiomarkerSeriesSchema,
  );
}
