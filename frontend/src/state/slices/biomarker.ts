import type { StateCreator } from 'zustand';

import { fetchSeries } from '@/api/patients';
import type { BiomarkerSeries, Demographics, Derived, DriftReport, SignalReview } from '@/api/schemas';

import type { Store } from '../store';
import { analysisOptions } from './view';

export interface BiomarkerSlice {
  /** drift reports for the selected patient, one per biomarker */
  reports: DriftReport[];
  /** review status per biomarker code (from the summary) */
  summaryReviews: Record<string, SignalReview[]>;
  /** derived series of the selected patient by code (EGFR) */
  summaryDerived: Record<string, Derived>;
  /** gender + birth year of the selected patient (null for CSV-only data) */
  demographics: Demographics | null;
  selectedCode: string | null;
  /** full series of the selected biomarker (chart data) */
  series: BiomarkerSeries | null;

  /** Select a biomarker of the selected patient and load its series (`force` reloads it). */
  selectCode(code: string, force?: boolean): Promise<void>;
}

// the newest series request wins; older responses are dropped
let seriesRequest = 0;

export const createBiomarkerSlice: StateCreator<Store, [], [], BiomarkerSlice> = (set, get) => ({
  reports: [],
  summaryReviews: {},
  summaryDerived: {},
  demographics: null,
  selectedCode: null,
  series: null,

  selectCode: async (code, force = false) => {
    const patientId = get().selectedPatient;
    if ((get().selectedCode === code && !force) || !patientId) return;
    set({ selectedCode: code });
    const request = ++seriesRequest;
    try {
      const series = await fetchSeries(patientId, code, analysisOptions(get()));
      // only show the series that is still selected (patient AND code)
      if (request === seriesRequest && get().selectedPatient === patientId && get().selectedCode === code) {
        set({ series });
      }
    } catch (err) {
      set({ error: String(err) });
    }
  },
});
