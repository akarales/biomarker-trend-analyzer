/** Drift status as computed by the API (worst rule wins). */
export const STATUSES = ['normal', 'watch', 'alert'] as const;
export type Status = (typeof STATUSES)[number];

export const TRENDS = ['rising', 'falling', 'flat'] as const;
export type TrendDirection = (typeof TRENDS)[number];

/** Card surface per status (Tailwind palette classes, light theme). */
export const STATUS_CARD: Record<string, string> = {
  alert: 'border-red-300 bg-red-50',
  watch: 'border-amber-300 bg-amber-50',
  normal: 'border-emerald-300 bg-emerald-50',
};

export const STATUS_CARD_FALLBACK = 'border-line bg-panel';
