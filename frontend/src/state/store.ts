import { create } from 'zustand';

import { createBiomarkerSlice, type BiomarkerSlice } from './slices/biomarker';
import { createPatientsSlice, type PatientsSlice } from './slices/patients';
import { createReviewSlice, type ReviewSlice } from './slices/review';
import { createUploadSlice, type UploadSlice } from './slices/upload';
import { createViewSlice, type ViewSlice } from './slices/view';

export type Store = PatientsSlice & BiomarkerSlice & UploadSlice & ViewSlice & ReviewSlice;

/**
 * App state composed from slices (patients · biomarker · upload · view · review).
 * Network calls live in slice actions and carry the view's analysis
 * options; components subscribe with narrow selectors.
 */
export const useAnalyzer = create<Store>()((...a) => ({
  ...createPatientsSlice(...a),
  ...createBiomarkerSlice(...a),
  ...createUploadSlice(...a),
  ...createViewSlice(...a),
  ...createReviewSlice(...a),
}));
