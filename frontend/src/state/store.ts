import { create } from 'zustand';

import { createBiomarkerSlice, type BiomarkerSlice } from './slices/biomarker';
import { createPatientsSlice, type PatientsSlice } from './slices/patients';
import { createUploadSlice, type UploadSlice } from './slices/upload';

export type Store = PatientsSlice & BiomarkerSlice & UploadSlice;

/**
 * App state composed from slices (patients · biomarker · upload). Network
 * calls live in slice actions; components subscribe with narrow selectors.
 */
export const useAnalyzer = create<Store>()((...a) => ({
  ...createPatientsSlice(...a),
  ...createBiomarkerSlice(...a),
  ...createUploadSlice(...a),
}));
