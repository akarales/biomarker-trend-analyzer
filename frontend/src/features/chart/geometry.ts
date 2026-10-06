import type { DriftReport } from '@/api/schemas';

export const WIDTH = 880;
export const HEIGHT = 320;
export const PAD = 40;

export interface ChartPoint {
  /** screen coordinates */
  cx: number;
  cy: number;
  /** outside the personal reference interval */
  flagged: boolean;
}

export interface ChartBand {
  top: number;
  bottom: number;
  /** set point line */
  middle: number;
}

export interface ChartGeometry {
  points: ChartPoint[];
  /** SVG path through every analysed point */
  path: string;
  /** personal reference interval, when a baseline exists */
  band: ChartBand | null;
}

/**
 * Pure layout for the trend chart from the analysed (normalised) points:
 * x = time (epoch seconds, so no browser time-zone parsing), y = value,
 * padded so the personal reference interval is always in view.
 */
export function chartGeometry(report: DriftReport): ChartGeometry {
  const { points: data, baseline } = report;
  const xs = data.map((p) => p.t);
  const ys = [...data.map((p) => p.v), ...(baseline ? [baseline.prri_low, baseline.prri_high] : [])];
  const xMin = Math.min(...xs);
  const xMax = Math.max(...xs, xMin + 1);
  const yMin = Math.min(...ys);
  const yMax = Math.max(...ys);
  const yPad = (yMax - yMin) * 0.1 || 1;
  const yLo = yMin - yPad;
  const yHi = yMax + yPad;

  const sx = (x: number) => PAD + ((x - xMin) / (xMax - xMin)) * (WIDTH - 2 * PAD);
  const sy = (y: number) => HEIGHT - PAD - ((y - yLo) / (yHi - yLo)) * (HEIGHT - 2 * PAD);
  const outside = (v: number) => baseline !== null && (v < baseline.prri_low || v > baseline.prri_high);

  return {
    points: data.map((p) => ({ cx: sx(p.t), cy: sy(p.v), flagged: outside(p.v) })),
    path: data.map((p, i) => `${i === 0 ? 'M' : 'L'}${sx(p.t).toFixed(1)},${sy(p.v).toFixed(1)}`).join(' '),
    band: baseline && {
      top: sy(baseline.prri_high),
      bottom: sy(baseline.prri_low),
      middle: sy(baseline.set_point),
    },
  };
}
