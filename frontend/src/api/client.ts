import type {
  BiomarkerSeries,
  PatientSummary,
  PatientSummaryEntry,
  UploadResult,
} from './types';

export class ApiError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api/v1${path}`, {
    ...init,
    headers: init?.body ? { 'Content-Type': 'text/csv' } : undefined,
  });
  if (!res.ok) {
    const body = await res.text();
    throw new ApiError(res.status, body || res.statusText);
  }
  return (await res.json()) as T;
}

export function fetchPatients(): Promise<{ patients: PatientSummaryEntry[] }> {
  return api('/patients');
}

export function fetchPatientSummary(patientId: string): Promise<PatientSummary> {
  return api(`/patients/${encodeURIComponent(patientId)}/summary`);
}

export function fetchSeries(
  patientId: string,
  code: string,
): Promise<BiomarkerSeries> {
  return api(`/patients/${encodeURIComponent(patientId)}/biomarkers/${encodeURIComponent(code)}`);
}

export function uploadCsv(csv: string): Promise<UploadResult> {
  return api('/observations', { method: 'POST', body: csv });
}
