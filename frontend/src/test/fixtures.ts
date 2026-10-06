import type { BiomarkerSeries, DriftReport, PatientSummary, PatientsResponse } from '@/api/schemas';

/** Synthetic API payloads (same shape as crates/api/src/routes/views.rs). */
export function report(code: string, overrides: Partial<DriftReport> = {}): DriftReport {
  return {
    code,
    unit: '%',
    baseline: { median: 5.6, robust_std: 0.1, n: 12 },
    latest: 5.7,
    latest_z: 1,
    ewma: 5.65,
    ewma_z: 0.5,
    slope_per_day: 0.001,
    trend: 'flat',
    anomalies: [],
    status: 'normal',
    window_days: 90,
    series_len: 3,
    ...overrides,
  };
}

export const patients: PatientsResponse = {
  patients: [
    { patient_id: 'alice', biomarkers: 2, observations: 6 },
    { patient_id: 'bob', biomarkers: 2, observations: 6 },
  ],
};

export function summary(patientId: string): PatientSummary {
  return { patient_id: patientId, window_days: 90, reports: [report('HBA1C'), report('LDL', { unit: 'mg/dL' })] };
}

export function series(patientId: string, code: string): BiomarkerSeries {
  return {
    patient_id: patientId,
    code,
    observations: [
      { taken_at: '2026-01-01 00:00:00', value: 5.5, unit: '%', source: 'test' },
      { taken_at: '2026-02-01 00:00:00', value: 5.6, unit: '%', source: 'test' },
      { taken_at: '2026-03-01 00:00:00', value: 7.0, unit: '%', source: 'test' },
    ],
    report: report(code, {
      status: 'alert',
      anomalies: [{ t: Date.UTC(2026, 2, 1) / 1000, v: 7.0 }],
    }),
  };
}

/** A promise you resolve from the test (to interleave responses). */
export function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}
