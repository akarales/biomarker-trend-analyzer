import type { StateCreator } from 'zustand';

import { postReview, type ReviewInput } from '@/api/reviews';

import type { Store } from '../store';
import { analysisOptions } from './view';

export interface ReviewSlice {
  /**
   * Review a signal of the open biomarker, then re-run the visible
   * analyses (triage, cards, series). Resolves to an error message for the
   * form, or null on success.
   */
  submitReview(input: ReviewInput): Promise<string | null>;
}

export const createReviewSlice: StateCreator<Store, [], [], ReviewSlice> = (_set, get) => ({
  submitReview: async (input) => {
    const { selectedPatient, series } = get();
    if (!selectedPatient || !series) return 'No biomarker is open.';
    try {
      await postReview(selectedPatient, series.code, input, analysisOptions(get()));
      await get().refresh();
      return null;
    } catch (err) {
      return err instanceof Error ? err.message : String(err);
    }
  },
});
