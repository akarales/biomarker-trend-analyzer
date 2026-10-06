import type {
  BiomarkerSeries,
  PatientSummary,
  PatientSummaryEntry,
  UploadResult,
} from './types';

export class ApiError extends Error {
  status: number;
  /** stable machine-readable code from the `{error, code}` body */
  code: string | null;
  requestId: string | null;
  constructor(status: number, message: string, code: string | null, requestId: string | null) {
    super(message);
    this.status = status;
    this.code = code;
    this.requestId = requestId;
  }
}

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`/api/v1${path}`, {
    ...init,
    headers: init?.body ? { 'Content-Type': 'text/csv' } : undefined,
  });
  if (!res.ok) {
    const text = await res.text();
    let message = text || res.statusText;
    let code: string | null = null;
    try {
      const body = JSON.parse(text) as { error?: string; code?: string };
      message = body.error ?? message;
      code = body.code ?? null;
    } catch {
      // non-JSON body (proxy error page): keep the raw text
    }
    const requestId = res.headers.get('x-request-id');
    throw new ApiError(
      res.status,
      requestId ? `${message} (request ${requestId})` : message,
      code,
      requestId,
    );
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
