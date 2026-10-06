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
      UploadResultSchema.safeParse({ inserted: 1, duplicates: 0, patients: 1, biomarkers: ['HBA1C'] }).success,
    ).toBe(true);
  });

  it('accept null detector outputs (short series)', () => {
    const s = summary('alice');
    s.reports[0] = { ...s.reports[0], latest_z: null, trend: null, slope_per_day: null };
    expect(PatientSummarySchema.safeParse(s).success).toBe(true);
  });

  it('reject an unknown status or a missing field', () => {
    const s = summary('alice');
    expect(
      PatientSummarySchema.safeParse({ ...s, reports: [{ ...s.reports[0], status: 'critical' }] }).success,
    ).toBe(false);
    expect(UploadResultSchema.safeParse({ inserted: 1, patients: 1, biomarkers: [] }).success).toBe(false);
  });
});
