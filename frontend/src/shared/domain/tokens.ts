import type { Severity, Status } from './status';

/**
 * The ONE colour source for colours set from TypeScript (SVG chart, status
 * chips). Catppuccin Mocha accents on the dark "Vitals" theme in index.css;
 * every text colour here is ≥ 7:1 on the card surface (CARD_BG).
 * `applyTokenVars()` mirrors them as CSS custom properties (`--status-*`,
 * `--severity-*`, `--chart-*`). No hex literal may appear in .ts/.tsx
 * outside this file (src/test/architecture.test.ts).
 */

/** approximates the theme's dark --card for opaque tints */
export const CARD_BG = '#1e1e2e';

export const STATUS_COLOR: Record<Status, string> = {
  alert: '#f38ba8',
  watch: '#fab387',
  normal: '#a6e3a1',
};

export const SEVERITY_COLOR: Record<Severity, string> = {
  alert: '#f38ba8',
  watch: '#fab387',
  info: '#89dceb',
};

export const CHART_COLOR = {
  /** observed values (line + points) */
  series: '#89b4fa',
  /** results outside the personal reference interval */
  flagged: '#f38ba8',
  /** personalised reference interval band + set point */
  prri: '#94e2d5',
  /** population reference interval band */
  population: '#7f849c',
  /** clinical decision thresholds */
  threshold: '#f9e2af',
  /** consecutive change beyond the reference change value */
  rcv: '#fab387',
  /** CUSUM change point */
  changePoint: '#cba6f7',
  /** grid lines */
  grid: '#45475a',
  /** axis labels */
  axis: '#a6adc8',
  /** keyboard / hover cursor */
  cursor: '#cdd6f4',
} as const;

/** Blend a hex colour toward a background hex (t = share of the colour). */
export function mixHex(color: string, background: string, t: number): string {
  const parse = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16));
  const [c, b] = [parse(color), parse(background)];
  return `#${c
    .map((v, i) => Math.round(v * t + b[i] * (1 - t)).toString(16).padStart(2, '0'))
    .join('')}`;
}

/** Opaque chip background for a status/severity colour (text stays ≥ 6:1). */
export function tint(color: string): string {
  return mixHex(color, CARD_BG, 0.16);
}

/** Every token as a CSS custom property. */
export function tokenVars(): Record<string, string> {
  const vars: Record<string, string> = {};
  for (const [k, v] of Object.entries(STATUS_COLOR)) vars[`--status-${k}`] = v;
  for (const [k, v] of Object.entries(SEVERITY_COLOR)) vars[`--severity-${k}`] = v;
  for (const [k, v] of Object.entries(CHART_COLOR)) vars[`--chart-${k}`] = v;
  return vars;
}

/** Publish the tokens on `:root` once at startup. */
export function applyTokenVars(el: HTMLElement = document.documentElement): void {
  for (const [k, v] of Object.entries(tokenVars())) el.style.setProperty(k, v);
}
