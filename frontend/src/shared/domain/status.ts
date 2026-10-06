/** Drift status as computed by the API (worst rule wins). */
export const STATUSES = ['normal', 'watch', 'alert'] as const;
export type Status = (typeof STATUSES)[number];

export const TRENDS = ['rising', 'falling', 'flat'] as const;
export type TrendDirection = (typeof TRENDS)[number];

export const SEVERITIES = ['info', 'watch', 'alert'] as const;
export type Severity = (typeof SEVERITIES)[number];

/** Detector that produced a signal (crates/drift/src/model.rs `Rule`). */
export const RULES = ['prri', 'rcv', 'shift', 'ewma', 'trend', 'threshold', 'population'] as const;
export type Rule = (typeof RULES)[number];

export const RULE_LABEL: Record<Rule, string> = {
  prri: 'Personal reference interval',
  rcv: 'Reference change value',
  shift: 'Sustained shift (CUSUM)',
  ewma: 'Smoothed level (EWMA)',
  trend: 'Trend (Mann–Kendall)',
  threshold: 'Clinical threshold',
  population: 'Population interval',
};

/** Card surface per status (Tailwind palette classes, light theme). */
export const STATUS_CARD: Record<string, string> = {
  alert: 'border-red-300 bg-red-50',
  watch: 'border-amber-300 bg-amber-50',
  normal: 'border-emerald-300 bg-emerald-50',
};

export const STATUS_CARD_FALLBACK = 'border-line bg-panel';
