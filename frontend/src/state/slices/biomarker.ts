import type { StateCreator } from 'zustand';

import { fetchSeries } from '@/api/patients';
import type { BiomarkerSeries, DriftReport } from '@/api/schemas';

import type { Store } from '../store';

export interface BiomarkerSlice {
  /** drift reports for the selected patient, one per biomarker */
  reports: DriftReport[];
  selectedCode: string | null;
  /** full series of the selected biomarker (chart data) */
  series: BiomarkerSeries | null;

  /** Select a biomarker of the selected patient and load its series. */
  selectCode(code: string): Promise<void>;
}

export const createBiomarkerSlice: StateCreator<Store, [], [], BiomarkerSlice> = (set, get) => ({
  reports: [],
  selectedCode: null,
  series: null,

  selectCode: async (code) => {
    const patientId = get().selectedPatient;
    if (get().selectedCode === code || !patientId) return;
    set({ selectedCode: code });
    try {
      const series = await fetchSeries(patientId, code);
      // only show the series that is still selected (patient AND code)
      if (get().selectedPatient === patientId && get().selectedCode === code) set({ series });
    } catch (err) {
      set({ error: String(err) });
    }
  },
});
