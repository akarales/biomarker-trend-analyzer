import type { Status } from './status';

/**
 * The ONE colour source for colours set from TypeScript (SVG fills, inline
 * styles). `applyTokenVars()` mirrors them as CSS custom properties
 * (`--status-*`, `--chart-*`) so CSS can use the same values. No hex
 * literal may appear in .ts/.tsx outside this file (enforced by
 * src/test/architecture.test.ts). The base theme lives in index.css.
 */

export const STATUS_COLOR: Record<Status, string> = {
  alert: '#b91c1c',
  watch: '#b45309',
  normal: '#15803d',
};

/** Badge colour for a status the client doesn't know. */
export const STATUS_COLOR_FALLBACK = '#0369a1';

export const CHART_COLOR = {
  /** personal-baseline band (median ± robust σ) and median line */
  baseline: '#0f766e',
  /** observed values */
  series: '#0369a1',
  /** readings flagged as anomalies */
  anomaly: '#b91c1c',
} as const;

/** Every token as a CSS custom property. */
export function tokenVars(): Record<string, string> {
  const vars: Record<string, string> = {};
  for (const [k, v] of Object.entries(STATUS_COLOR)) vars[`--status-${k}`] = v;
  for (const [k, v] of Object.entries(CHART_COLOR)) vars[`--chart-${k}`] = v;
  return vars;
}

/** Publish the tokens on `:root` once at startup. */
export function applyTokenVars(el: HTMLElement = document.documentElement): void {
  for (const [k, v] of Object.entries(tokenVars())) el.style.setProperty(k, v);
}
