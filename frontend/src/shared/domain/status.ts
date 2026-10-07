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

/** Words shown next to every colour (colour is never the only channel). */
export const STATUS_LABEL: Record<Status, string> = {
  alert: 'Alert',
  watch: 'Watch',
  normal: 'No rule fired',
};

export const SEVERITY_LABEL: Record<Severity, string> = {
  alert: 'Alert',
  watch: 'Watch',
  info: 'Info',
};

export const TREND_SYMBOL: Record<TrendDirection, string> = {
  rising: '↑',
  falling: '↓',
  flat: '→',
};

/** Trend lookback choices for the window control (days). */
export const WINDOW_CHOICES = [
  { days: 365, label: '1 year' },
  { days: 1095, label: '3 years' },
  { days: 1825, label: '5 years' },
  { days: 3650, label: '10 years' },
] as const;

export const DEFAULT_WINDOW_DAYS = 1095;
