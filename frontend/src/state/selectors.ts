import type { DriftReport, PatientSummaryEntry } from '@/api/schemas';

import type { Store } from './store';

/** The trend prompt shows once reports exist but no series is loaded. */
export const showTrendPrompt = (s: Store): boolean => s.series === null && s.reports.length > 0;

/** Triage row of the selected patient. */
export const selectedEntry = (s: Store): PatientSummaryEntry | undefined =>
  s.patients.find((p) => p.patient_id === s.selectedPatient);

/** Report of the selected biomarker. */
export const selectedReport = (s: Store): DriftReport | undefined =>
  s.reports.find((r) => r.code === s.selectedCode);
