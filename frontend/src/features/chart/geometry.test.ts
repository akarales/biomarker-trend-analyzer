import { describe, expect, it } from 'vitest';

import { report, series } from '@/test/fixtures';

import { chartGeometry, HEIGHT, PAD, WIDTH } from './geometry';

describe('chartGeometry', () => {
  it('spans the plot area left to right in time order', () => {
    const g = chartGeometry(series('alice', 'HBA1C').report);
    expect(g.points).toHaveLength(3);
    expect(g.points[0].cx).toBeCloseTo(PAD);
    expect(g.points[2].cx).toBeCloseTo(WIDTH - PAD);
    expect(g.path.startsWith('M')).toBe(true);
    expect(g.path.split(' L')).toHaveLength(3);
  });

  it('keeps every point and the personal reference interval inside the plot', () => {
    const g = chartGeometry(series('alice', 'HBA1C').report);
    const band = g.band!;
    for (const y of [...g.points.map((p) => p.cy), band.top, band.bottom, band.middle]) {
      expect(y).toBeGreaterThanOrEqual(PAD);
      expect(y).toBeLessThanOrEqual(HEIGHT - PAD);
    }
    expect(band.top).toBeLessThan(band.middle);
    expect(band.middle).toBeLessThan(band.bottom);
  });

  it('flags results outside the prRI (higher value = smaller y)', () => {
    const g = chartGeometry(series('alice', 'HBA1C').report);
    expect(g.points.map((p) => p.flagged)).toEqual([false, false, true]);
    expect(g.points[2].cy).toBeLessThan(g.points[0].cy);
  });

  it('has no band and flags nothing without a baseline', () => {
    const g = chartGeometry(report('HBA1C'));
    expect(g.band).toBeNull();
    expect(g.points.every((p) => !p.flagged)).toBe(true);
  });

  it('handles a single flat reading without dividing by zero', () => {
    const r = report('HBA1C', { points: [{ t: 0, v: 5.5 }], latest: { t: 0, v: 5.5 } });
    const g = chartGeometry(r);
    expect(Number.isFinite(g.points[0].cx)).toBe(true);
    expect(Number.isFinite(g.points[0].cy)).toBe(true);
  });
});
