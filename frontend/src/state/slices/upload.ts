import type { StateCreator } from 'zustand';

import { uploadCsv } from '@/api/observations';
import type { UploadResult } from '@/api/schemas';

import type { Store } from '../store';

export interface UploadSlice {
  uploadText: string;
  /** outcome of the last upload (success summary or error text) */
  uploadMessage: string | null;

  setUploadText(text: string): void;
  /** Upload the pasted CSV/FHIR, then re-run the visible analyses. */
  upload(): Promise<void>;
}

/** Human summary of an idempotent upload. */
export function uploadSummary(result: UploadResult): string {
  const skipped = result.skipped_reasons
    .slice(0, 3)
    .map((r) => `${r.count}× ${r.reason}`)
    .join('; ');
  return (
    `${result.format === 'fhir' ? 'FHIR: i' : 'I'}nserted ${result.inserted} observations across ${result.patients} patient(s): ${result.biomarkers.join(', ')}` +
    (result.duplicates > 0 ? ` (${result.duplicates} already stored, skipped)` : '') +
    (result.skipped > 0 ? `. Not imported: ${skipped}${result.skipped_reasons.length > 3 ? '; …' : ''}` : '')
  );
}

export const createUploadSlice: StateCreator<Store, [], [], UploadSlice> = (set, get) => ({
  uploadText: '',
  uploadMessage: null,

  setUploadText: (uploadText) => set({ uploadText }),

  upload: async () => {
    const text = get().uploadText;
    if (!text.trim()) return;
    set({ uploadMessage: null, error: null });
    try {
      const result = await uploadCsv(text);
      set({ uploadMessage: uploadSummary(result), uploadText: '' });
      await get().refresh();
    } catch (err) {
      set({ uploadMessage: String(err) });
    }
  },
});
