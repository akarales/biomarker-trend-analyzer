import type { BiomarkerSeries } from '@/api/schemas';

export const WIDTH = 880;
export const HEIGHT = 320;
export const PAD = 40;

export interface ChartPoint {
  /** screen coordinates */
  cx: number;
  cy: number;
  anomaly: boolean;
}

export interface ChartGeometry {
  points: ChartPoint[];
  /** SVG path through every observation */
  path: string;
  bandTop: number;
  bandBottom: number;
  medianLine: number;
}

/**
 * Pure layout for the trend chart: x = observation time, y = value, padded
 * so the personal-baseline band (median ± robust σ) is always in view.
 */
export function chartGeometry(series: BiomarkerSeries): ChartGeometry {
  const data = series.observations.map((o) => ({
    x: new Date(o.taken_at).getTime(),
    y: o.value,
  }));
  // Anomaly timestamps arrive as epoch seconds; chart x is epoch ms.
  const anomalyTimes = new Set(series.report.anomalies.map((a) => a.t * 1000));
  const { median, robust_std: sigma } = series.report.baseline;

  const xs = data.map((p) => p.x);
  const ys = data.map((p) => p.y);
  const xMin = Math.min(...xs);
  const xMax = Math.max(...xs, xMin + 1);
  const yMin = Math.min(...ys, median - sigma);
  const yMax = Math.max(...ys, median + sigma);
  const yPad = (yMax - yMin) * 0.1 || 1;
  const yLo = yMin - yPad;
  const yHi = yMax + yPad;

  const sx = (x: number) => PAD + ((x - xMin) / (xMax - xMin)) * (WIDTH - 2 * PAD);
  const sy = (y: number) => HEIGHT - PAD - ((y - yLo) / (yHi - yLo)) * (HEIGHT - 2 * PAD);

  return {
    points: data.map((p) => ({ cx: sx(p.x), cy: sy(p.y), anomaly: anomalyTimes.has(p.x) })),
    path: data.map((p, i) => `${i === 0 ? 'M' : 'L'}${sx(p.x).toFixed(1)},${sy(p.y).toFixed(1)}`).join(' '),
    bandTop: sy(median + sigma),
    bandBottom: sy(median - sigma),
    medianLine: sy(median),
  };
}
