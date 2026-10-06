import type { StateCreator } from 'zustand';

import { fetchPatients, fetchPatientSummary } from '@/api/patients';
import type { PatientSummaryEntry } from '@/api/schemas';

import type { Store } from '../store';

export interface PatientsSlice {
  patients: PatientSummaryEntry[];
  selectedPatient: string | null;
  /** last load error, shown as a banner (cleared when an upload starts) */
  error: string | null;

  /** Refresh the listing; selects the first patient when none is selected. */
  loadPatients(): Promise<void>;
  /** Select a patient and load its drift summary (no-op if already selected). */
  selectPatient(patientId: string): Promise<void>;
}

export const createPatientsSlice: StateCreator<Store, [], [], PatientsSlice> = (set, get) => ({
  patients: [],
  selectedPatient: null,
  error: null,

  loadPatients: async () => {
    try {
      const body = await fetchPatients();
      set({ patients: body.patients });
      const first = body.patients[0]?.patient_id;
      if (!get().selectedPatient && first) await get().selectPatient(first);
    } catch (err) {
      set({ error: String(err) });
    }
  },

  selectPatient: async (patientId) => {
    if (get().selectedPatient === patientId) return;
    set({ selectedPatient: patientId });
    try {
      const summary = await fetchPatientSummary(patientId);
      // a newer selection wins; stale responses are dropped
      if (get().selectedPatient !== patientId) return;
      set({ reports: summary.reports, selectedCode: null, series: null });
    } catch (err) {
      set({ error: String(err) });
    }
  },
});
