import { beforeEach, describe, expect, it, vi } from 'vitest';

import * as patientsApi from '@/api/patients';
import * as observationsApi from '@/api/observations';
import { ApiError } from '@/api/http';
import type { BiomarkerSeries, PatientSummary } from '@/api/schemas';
import { deferred, patients, series, summary } from '@/test/fixtures';

import { showTrendPrompt, uploadSummary, useAnalyzer } from './index';

vi.mock('@/api/patients');
vi.mock('@/api/observations');

const initial = useAnalyzer.getState();
const state = () => useAnalyzer.getState();

beforeEach(() => {
  vi.resetAllMocks();
  useAnalyzer.setState(initial, true);
  vi.mocked(patientsApi.fetchPatients).mockResolvedValue(patients);
  vi.mocked(patientsApi.fetchPatientSummary).mockImplementation(async (id) => summary(id));
  vi.mocked(patientsApi.fetchSeries).mockImplementation(async (id, code) => series(id, code));
});

describe('patients slice', () => {
  it('loads the listing and selects + summarises the first patient', async () => {
    await state().loadPatients();
    expect(state().patients).toHaveLength(2);
    expect(state().selectedPatient).toBe('alice');
    expect(state().reports.map((r) => r.code)).toEqual(['HBA1C', 'LDL']);
    expect(showTrendPrompt(state())).toBe(true);
  });

  it('keeps the selection on reload and does not refetch the summary', async () => {
    await state().loadPatients();
    await state().loadPatients();
    expect(patientsApi.fetchPatientSummary).toHaveBeenCalledTimes(1);
  });

  it('re-selecting the current patient is a no-op', async () => {
    await state().loadPatients();
    await state().selectCode('HBA1C');
    await state().selectPatient('alice');
    expect(state().selectedCode).toBe('HBA1C');
    expect(state().series).not.toBeNull();
  });

  it('drops a stale summary when a newer patient is selected', async () => {
    const slow = deferred<PatientSummary>();
    vi.mocked(patientsApi.fetchPatientSummary).mockImplementation((id) =>
      id === 'alice' ? slow.promise : Promise.resolve(summary(id)),
    );
    const first = state().selectPatient('alice');
    await state().selectPatient('bob');
    slow.resolve({ ...summary('alice'), reports: [] });
    await first;
    expect(state().selectedPatient).toBe('bob');
    expect(state().reports).toHaveLength(2);
  });

  it('shows load errors as the banner text', async () => {
    vi.mocked(patientsApi.fetchPatients).mockRejectedValue(new ApiError(500, 'internal error', 'internal', 'r-1'));
    await state().loadPatients();
    expect(state().error).toBe('Error: internal error');
  });
});

describe('biomarker slice', () => {
  it('loads the selected series', async () => {
    await state().loadPatients();
    await state().selectCode('HBA1C');
    expect(state().series?.code).toBe('HBA1C');
    expect(showTrendPrompt(state())).toBe(false);
  });

  it('switching patient never fetches the old code for the new patient (v1 race)', async () => {
    await state().loadPatients();
    await state().selectCode('HBA1C');
    await state().selectPatient('bob');
    expect(patientsApi.fetchSeries).toHaveBeenCalledTimes(1);
    expect(patientsApi.fetchSeries).toHaveBeenCalledWith('alice', 'HBA1C');
    expect(state().selectedCode).toBeNull();
    expect(state().series).toBeNull();
  });

  it('ignores a series that arrives after the patient changed', async () => {
    await state().loadPatients();
    const slow = deferred<BiomarkerSeries>();
    vi.mocked(patientsApi.fetchSeries).mockReturnValue(slow.promise);
    const pending = state().selectCode('LDL');
    await state().selectPatient('bob');
    slow.resolve(series('alice', 'LDL'));
    await pending;
    expect(state().series).toBeNull();
  });
});

describe('upload slice', () => {
  it('summarises inserted and duplicate rows', () => {
    expect(uploadSummary({ inserted: 2, duplicates: 0, patients: 1, biomarkers: ['HBA1C', 'LDL'] })).toBe(
      'Inserted 2 observations across 1 patient(s): HBA1C, LDL',
    );
    expect(uploadSummary({ inserted: 0, duplicates: 3, patients: 1, biomarkers: ['LDL'] })).toBe(
      'Inserted 0 observations across 1 patient(s): LDL (3 already stored, skipped)',
    );
  });

  it('uploads, clears the box and refreshes the listing', async () => {
    vi.mocked(observationsApi.uploadCsv).mockResolvedValue({
      inserted: 1,
      duplicates: 0,
      patients: 1,
      biomarkers: ['HBA1C'],
    });
    useAnalyzer.setState({ uploadText: 'csv', error: 'old' });
    await state().upload();
    expect(observationsApi.uploadCsv).toHaveBeenCalledWith('csv');
    expect(state().uploadText).toBe('');
    expect(state().error).toBeNull();
    expect(state().uploadMessage).toMatch(/^Inserted 1 observations/);
    expect(patientsApi.fetchPatients).toHaveBeenCalledTimes(1);
  });

  it('keeps the text and shows the error on failure; ignores blank input', async () => {
    await state().upload();
    expect(observationsApi.uploadCsv).not.toHaveBeenCalled();
    vi.mocked(observationsApi.uploadCsv).mockRejectedValue(new ApiError(422, 'csv error: schema mismatch', 'invalid_csv', null));
    useAnalyzer.setState({ uploadText: 'garbage' });
    await state().upload();
    expect(state().uploadText).toBe('garbage');
    expect(state().uploadMessage).toBe('Error: csv error: schema mismatch');
  });
});
