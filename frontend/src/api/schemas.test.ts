import { describe, expect, it } from 'vitest';

import { patients, series, summary } from '@/test/fixtures';

import {
  BiomarkerSeriesSchema,
  PatientSummarySchema,
  PatientsResponseSchema,
  UploadResultSchema,
} from './schemas';

describe('response schemas', () => {
  it('accept the API shapes', () => {
    expect(PatientsResponseSchema.safeParse(patients).success).toBe(true);
    expect(PatientSummarySchema.safeParse(summary('alice')).success).toBe(true);
    expect(BiomarkerSeriesSchema.safeParse(series('alice', 'HBA1C')).success).toBe(true);
    expect(
      UploadResultSchema.safeParse({
        format: 'fhir', inserted: 1, duplicates: 0, skipped: 1,
        skipped_reasons: [{ reason: 'no LOINC coding', count: 1 }], patients: 1, biomarkers: ['HBA1C'], demographics: 1,
      }).success,
    ).toBe(true);
  });

  it('accept null detector outputs (short series, unknown analyte)', () => {
    const s = summary('alice');
    s.reports[0] = { ...s.reports[0], analyte: null, baseline: null, trend: null, rcv: null, latest: null, population: null };
    expect(PatientSummarySchema.safeParse(s).success).toBe(true);
  });

  it('reject an unknown rule or severity in a signal', () => {
    const s = series('alice', 'HBA1C');
    const bad = { ...s, report: { ...s.report, signals: [{ ...s.report.signals[0], rule: 'zscore' }] } };
    expect(BiomarkerSeriesSchema.safeParse(bad).success).toBe(false);
  });

  it('reject an unknown status or a missing field', () => {
    const s = summary('alice');
    expect(
      PatientSummarySchema.safeParse({ ...s, reports: [{ ...s.reports[0], status: 'critical' }] }).success,
    ).toBe(false);
    expect(UploadResultSchema.safeParse({ inserted: 1, patients: 1, biomarkers: [] }).success).toBe(false);
  });
});
