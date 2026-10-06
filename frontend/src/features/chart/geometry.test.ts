import { describe, expect, it } from 'vitest';

import { series } from '@/test/fixtures';

import { chartGeometry, HEIGHT, PAD, WIDTH } from './geometry';

describe('chartGeometry', () => {
  it('spans the plot area left to right in time order', () => {
    const g = chartGeometry(series('alice', 'HBA1C'));
    expect(g.points).toHaveLength(3);
    expect(g.points[0].cx).toBeCloseTo(PAD);
    expect(g.points[2].cx).toBeCloseTo(WIDTH - PAD);
    expect(g.path.startsWith('M')).toBe(true);
    expect(g.path.split(' L')).toHaveLength(3);
  });

  it('keeps every point and the baseline band inside the plot', () => {
    const g = chartGeometry(series('alice', 'HBA1C'));
    for (const y of [...g.points.map((p) => p.cy), g.bandTop, g.bandBottom, g.medianLine]) {
      expect(y).toBeGreaterThanOrEqual(PAD);
      expect(y).toBeLessThanOrEqual(HEIGHT - PAD);
    }
    expect(g.bandTop).toBeLessThan(g.medianLine);
    expect(g.medianLine).toBeLessThan(g.bandBottom);
  });

  it('marks the anomalous reading (higher value = smaller y)', () => {
    const g = chartGeometry(series('alice', 'HBA1C'));
    expect(g.points.map((p) => p.anomaly)).toEqual([false, false, true]);
    expect(g.points[2].cy).toBeLessThan(g.points[0].cy);
  });

  it('handles a single flat reading without dividing by zero', () => {
    const s = series('alice', 'HBA1C');
    s.observations = s.observations.slice(0, 1);
    s.report = { ...s.report, baseline: { median: 5.5, robust_std: 0, n: 1 } };
    const g = chartGeometry(s);
    expect(Number.isFinite(g.points[0].cx)).toBe(true);
    expect(Number.isFinite(g.points[0].cy)).toBe(true);
  });
});
