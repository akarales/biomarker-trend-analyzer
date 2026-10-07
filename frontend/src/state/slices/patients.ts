import type { StateCreator } from 'zustand';

import { fetchPatients, fetchPatientSummary } from '@/api/patients';
import type { PatientSummaryEntry } from '@/api/schemas';

import type { Store } from '../store';
import { analysisOptions } from './view';

export interface PatientsSlice {
  /** triage listing, worst first */
  patients: PatientSummaryEntry[];
  selectedPatient: string | null;
  /** last load error, shown as a banner (cleared when an upload starts) */
  error: string | null;

  /**
   * Refresh the listing. Opens `preferred` (and its `code`) when it exists,
   * else the worst patient when none is selected yet.
   */
  loadPatients(preferred?: string | null, code?: string | null): Promise<void>;
  /** Select a patient and load its drift summary (no-op if already selected). */
  selectPatient(patientId: string, code?: string | null): Promise<void>;
  /** (Re)load the summary of the selected patient, keeping `keepCode` open if it still exists. */
  loadSummary(patientId: string, keepCode: string | null): Promise<void>;
}

// the newest summary request wins; older responses are dropped
let summaryRequest = 0;

export const createPatientsSlice: StateCreator<Store, [], [], PatientsSlice> = (set, get) => ({
  patients: [],
  selectedPatient: null,
  error: null,

  loadPatients: async (preferred = null, code = null) => {
    try {
      const body = await fetchPatients(analysisOptions(get()));
      set({ patients: body.patients });
      const ids = body.patients.map((p) => p.patient_id);
      if (preferred && ids.includes(preferred)) {
        await get().selectPatient(preferred, code);
      } else if (!get().selectedPatient && ids[0]) {
        // a link's code belongs to the linked patient only
        await get().selectPatient(ids[0]);
      }
    } catch (err) {
      set({ error: String(err) });
    }
  },

  selectPatient: async (patientId, code = null) => {
    if (get().selectedPatient === patientId && !code) return;
    set({ selectedPatient: patientId });
    await get().loadSummary(patientId, code);
  },

  loadSummary: async (patientId, keepCode) => {
    const request = ++summaryRequest;
    try {
      const summary = await fetchPatientSummary(patientId, analysisOptions(get()));
      if (request !== summaryRequest || get().selectedPatient !== patientId) return;
      const code = keepCode && summary.reports.some((r) => r.code === keepCode) ? keepCode : null;
      set({ reports: summary.reports, ...(code ? {} : { selectedCode: null, series: null }) });
      if (code) await get().selectCode(code, true);
    } catch (err) {
      set({ error: String(err) });
    }
  },
});
