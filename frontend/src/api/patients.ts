import { api } from './http';
import {
  BiomarkerSeriesSchema,
  PatientSummarySchema,
  PatientsResponseSchema,
  type BiomarkerSeries,
  type PatientSummary,
  type PatientsResponse,
} from './schemas';

export function fetchPatients(): Promise<PatientsResponse> {
  return api('/patients', PatientsResponseSchema);
}

export function fetchPatientSummary(patientId: string): Promise<PatientSummary> {
  return api(`/patients/${encodeURIComponent(patientId)}/summary`, PatientSummarySchema);
}

export function fetchSeries(patientId: string, code: string): Promise<BiomarkerSeries> {
  return api(
    `/patients/${encodeURIComponent(patientId)}/biomarkers/${encodeURIComponent(code)}`,
    BiomarkerSeriesSchema,
  );
}
