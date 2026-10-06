import type { BiomarkerSeries, DriftReport, PatientSummary, PatientsResponse, Signal } from '@/api/schemas';

/** Synthetic API payloads (same shape as crates/drift/src/model.rs). */
const T0 = Date.UTC(2026, 0, 1) / 1000;
const DAY = 86_400;

export function signal(overrides: Partial<Signal> = {}): Signal {
  return {
    rule: 'prri',
    severity: 'alert',
    t: T0 + 59 * DAY,
    value: 7.0,
    threshold: 5.82,
    explanation: 'Hemoglobin A1c 7.00 % is above this patient’s personal reference interval 5.38–5.82 %.',
    source: 'Coşkun A et al., Clin Chem 2021',
    ...overrides,
  };
}

export function report(code: string, overrides: Partial<DriftReport> = {}): DriftReport {
  return {
    code,
    analyte: {
      code,
      loinc: '4548-4',
      display: 'Hemoglobin A1c',
      cvi: 0.012,
      cva: 0.015,
      cvi_source: 'meta-analysis',
      cva_source: 'assumed',
      reviewed: '2026-10-06',
    },
    unit: '%',
    as_of: null,
    window_days: 365,
    points: [
      { t: T0, v: 5.5 },
      { t: T0 + 31 * DAY, v: 5.6 },
      { t: T0 + 59 * DAY, v: 5.7 },
    ],
    excluded: { after_as_of: 0, unit_unknown: 0 },
    latest: { t: T0 + 59 * DAY, v: 5.7 },
    baseline: null,
    population: { low: 4.0, high: 5.6, source: 'ADA' },
    thresholds: [],
    rcv: { up: 0.055, down: -0.052 },
    rcv_jumps: [],
    ewma: null,
    change_point: null,
    trend: null,
    signals: [],
    not_assessed: [{ rule: 'trend', reason: 'needs ≥ 4 results in the last 365 days (has 3)' }],
    status: 'normal',
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
  return {
    patient_id: patientId,
    as_of: null,
    window_days: 365,
    reports: [report('HBA1C'), report('LDL', { unit: 'mg/dL' })],
  };
}

export function series(patientId: string, code: string): BiomarkerSeries {
  const points = [
    { t: T0, v: 5.5 },
    { t: T0 + 31 * DAY, v: 5.6 },
    { t: T0 + 59 * DAY, v: 7.0 },
  ];
  return {
    patient_id: patientId,
    code,
    observations: points.map((p) => ({
      taken_at: new Date(p.t * 1000).toISOString().replace('.000', ''),
      value: p.v,
      unit: '%',
      source: 'test',
    })),
    report: report(code, {
      points,
      latest: points[2],
      status: 'alert',
      baseline: { n: 3, from: T0, to: T0 + 31 * DAY, set_point: 5.55, prri_low: 5.38, prri_high: 5.82, level: 0.95 },
      signals: [signal(), signal({ rule: 'threshold', explanation: 'Hemoglobin A1c 7.00 %: diabetes range (ADA ≥ 6.5 %).' })],
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
