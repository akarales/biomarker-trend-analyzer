import type { Store } from './store';

/** The trend prompt shows once reports exist but no series is loaded. */
export const showTrendPrompt = (s: Store): boolean => s.series === null && s.reports.length > 0;
