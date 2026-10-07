import type { StateCreator } from 'zustand';

import type { AnalysisOptions } from '@/api/patients';
import { DEFAULT_WINDOW_DAYS } from '@/shared/domain';

import type { Store } from '../store';

export interface ViewSlice {
  /** analyse as of this date (`YYYY-MM-DD`); null = latest result */
  asOf: string | null;
  /** trend lookback in days */
  windowDays: number;

  setAsOf(asOf: string | null): Promise<void>;
  setWindowDays(days: number): Promise<void>;
  /** Re-run every visible analysis with the current options. */
  refresh(): Promise<void>;
}

export function analysisOptions(s: Pick<ViewSlice, 'asOf' | 'windowDays'>): AnalysisOptions {
  return { asOf: s.asOf, windowDays: s.windowDays };
}

export const createViewSlice: StateCreator<Store, [], [], ViewSlice> = (set, get) => ({
  asOf: null,
  windowDays: DEFAULT_WINDOW_DAYS,

  setAsOf: async (asOf) => {
    if (asOf === get().asOf) return;
    set({ asOf });
    await get().refresh();
  },

  setWindowDays: async (windowDays) => {
    if (windowDays === get().windowDays) return;
    set({ windowDays });
    await get().refresh();
  },

  refresh: async () => {
    const { selectedPatient, selectedCode } = get();
    await get().loadPatients();
    if (selectedPatient) await get().loadSummary(selectedPatient, selectedCode);
  },
});
