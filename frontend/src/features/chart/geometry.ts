import type { DriftReport } from '@/api/schemas';
import type { Severity } from '@/shared/domain';

import { niceTicks, timeTicks, type TimeTick } from './scale';

export const HEIGHT = 300;
export const MARGIN = { top: 16, right: 16, bottom: 32, left: 52 };

export interface ChartPoint {
  t: number;
  v: number;
  cx: number;
  cy: number;
  /** outside the personal reference interval */
  outside: boolean;
  /** relative change from the previous result when it exceeded the RCV */
  jump: number | null;
  /** most severe clinical threshold this value meets */
  threshold: { label: string; severity: Severity } | null;
}

export interface Band {
  top: number;
  bottom: number;
}

export interface ChartLayout {
  width: number;
  height: number;
  points: ChartPoint[];
  path: string;
  prri: (Band & { middle: number }) | null;
  population: Band | null;
  thresholds: { y: number; value: number; label: string; severity: Severity }[];
  jumps: { x1: number; y1: number; x2: number; y2: number }[];
  changePoint: { x: number; t: number } | null;
  xTicks: (TimeTick & { x: number })[];
  yTicks: { v: number; y: number }[];
}

/**
 * The most severe threshold a value meets (≥ above / < below). Profiles
 * list thresholds mildest first, so on a tie the LAST one met — the
 * deepest category (e.g. KDIGO G3b over G3a) — wins, as in the engine.
 */
export function thresholdFor(report: DriftReport, v: number): ChartPoint['threshold'] {
  const order: Record<Severity, number> = { info: 0, watch: 1, alert: 2 };
  let worst: DriftReport['thresholds'][number] | undefined;
  for (const t of report.thresholds) {
    const met = t.direction === 'above' ? v >= t.value : v < t.value;
    if (met && (!worst || order[t.severity] >= order[worst.severity])) worst = t;
  }
  return worst ? { label: worst.label, severity: worst.severity } : null;
}

/**
 * Pure layout for one biomarker at a given pixel width: x = time (epoch
 * seconds, no browser time-zone parsing), y = value. The y domain covers
 * the results and the personal interval, plus the population interval and
 * thresholds when they are near the data (far-away limits would flatten
 * the series).
 */
export function chartLayout(report: DriftReport, width: number): ChartLayout {
  const { points: data, baseline, population } = report;
  const values = data.map((p) => p.v);
  const core = [...values, ...(baseline ? [baseline.prri_low, baseline.prri_high] : [])];
  const cMin = Math.min(...core);
  const cMax = Math.max(...core);
  const span = Math.max(cMax - cMin, Math.abs(cMax) * 0.05, 1e-6);
  const near = (v: number) => v >= cMin - span * 0.75 && v <= cMax + span * 0.75;
  const extra = [
    ...(population ? [population.low, population.high].filter(near) : []),
    ...report.thresholds.map((t) => t.value).filter(near),
  ];
  const yt = niceTicks(Math.min(cMin, ...extra), Math.max(cMax, ...extra));

  const ts = data.map((p) => p.t);
  const tMin = Math.min(...ts);
  const tMax = Math.max(...ts, tMin + 86_400);
  const plotW = Math.max(width - MARGIN.left - MARGIN.right, 10);
  const plotH = HEIGHT - MARGIN.top - MARGIN.bottom;
  const x = (t: number) => MARGIN.left + ((t - tMin) / (tMax - tMin)) * plotW;
  const y = (v: number) => MARGIN.top + plotH - ((v - yt.lo) / (yt.hi - yt.lo)) * plotH;
  const clampY = (v: number) => y(Math.min(Math.max(v, yt.lo), yt.hi));

  const jumpAt = new Map(report.rcv_jumps.map((j) => [j.to_t, j]));
  const points: ChartPoint[] = data.map((p) => ({
    ...p,
    cx: x(p.t),
    cy: y(p.v),
    outside: baseline !== null && (p.v < baseline.prri_low || p.v > baseline.prri_high),
    jump: jumpAt.get(p.t)?.change ?? null,
    threshold: thresholdFor(report, p.v),
  }));
  const byTime = new Map(points.map((p) => [p.t, p]));

  return {
    width,
    height: HEIGHT,
    points,
    path: points.map((p, i) => `${i === 0 ? 'M' : 'L'}${p.cx.toFixed(1)},${p.cy.toFixed(1)}`).join(' '),
    prri: baseline && {
      top: clampY(baseline.prri_high),
      bottom: clampY(baseline.prri_low),
      middle: clampY(baseline.set_point),
    },
    population:
      population && population.high >= yt.lo && population.low <= yt.hi
        ? { top: clampY(population.high), bottom: clampY(population.low) }
        : null,
    thresholds: report.thresholds
      .filter((t) => t.value >= yt.lo && t.value <= yt.hi)
      .map((t) => ({ y: y(t.value), value: t.value, label: t.label, severity: t.severity })),
    jumps: report.rcv_jumps.flatMap((j) => {
      const a = byTime.get(j.from_t);
      const b = byTime.get(j.to_t);
      return a && b ? [{ x1: a.cx, y1: a.cy, x2: b.cx, y2: b.cy }] : [];
    }),
    changePoint: report.change_point ? { x: x(report.change_point.t), t: report.change_point.t } : null,
    // ~80 px per label ("Jan 2025") so narrow screens never overlap
    xTicks: timeTicks(tMin, tMax, Math.max(2, Math.floor(plotW / 80))).map((tick) => ({ ...tick, x: x(tick.t) })),
    yTicks: yt.ticks.map((v) => ({ v, y: y(v) })),
  };
}

/** Index of the point nearest to a pixel x (points are in time order). */
export function nearestIndex(points: ChartPoint[], px: number): number {
  let best = 0;
  for (let i = 1; i < points.length; i += 1) {
    if (Math.abs(points[i].cx - px) < Math.abs(points[best].cx - px)) best = i;
  }
  return best;
}
