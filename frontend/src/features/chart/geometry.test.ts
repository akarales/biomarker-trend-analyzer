import { describe, expect, it } from 'vitest';

import { report, series } from '@/test/fixtures';

import { chartLayout, HEIGHT, MARGIN, nearestIndex, thresholdFor } from './geometry';
import { pointFlags, readoutText } from './readout';
import { niceTicks, timeTicks } from './scale';

const DAY = 86_400;

describe('niceTicks', () => {
  it('covers the range with round steps', () => {
    expect(niceTicks(5.38, 7.02)).toEqual({ lo: 5, hi: 7.5, ticks: [5, 5.5, 6, 6.5, 7, 7.5] });
    expect(niceTicks(103, 167).ticks).toEqual([100, 120, 140, 160, 180]);
  });

  it('survives a flat or invalid range', () => {
    const flat = niceTicks(2, 2);
    expect(flat.lo).toBeLessThan(2);
    expect(flat.hi).toBeGreaterThan(2);
    expect(niceTicks(Number.NaN, 1)).toEqual({ lo: 0, hi: 1, ticks: [0, 1] });
  });
});

describe('timeTicks', () => {
  const at = (y: number, m: number, d = 1) => Date.UTC(y, m, d) / 1000;

  it('uses years for long spans and quarters/months for shorter ones', () => {
    expect(timeTicks(at(2016, 11, 16), at(2026, 2, 6), 20).map((t) => t.label)).toEqual(
      ['2017', '2018', '2019', '2020', '2021', '2022', '2023', '2024', '2025', '2026'],
    );
    expect(timeTicks(at(2024, 9, 14), at(2026, 8, 14), 20)[0].label).toBe('Jan 2025');
    expect(timeTicks(at(2026, 0, 6), at(2026, 6, 6), 20).map((t) => t.label)).toEqual(
      ['Feb 2026', 'Mar 2026', 'Apr 2026', 'May 2026', 'Jun 2026', 'Jul 2026'],
    );
  });

  it('thins labels to the maximum', () => {
    expect(timeTicks(at(2000, 0), at(2026, 0), 6).length).toBeLessThanOrEqual(6);
  });
});

describe('chartLayout', () => {
  const layout = chartLayout(series('alice', 'HBA1C').report, 600);

  it('spans the plot area in time order', () => {
    expect(layout.points).toHaveLength(3);
    expect(layout.points[0].cx).toBeCloseTo(MARGIN.left);
    expect(layout.points[2].cx).toBeCloseTo(600 - MARGIN.right);
    for (const p of layout.points) {
      expect(p.cy).toBeGreaterThanOrEqual(MARGIN.top);
      expect(p.cy).toBeLessThanOrEqual(HEIGHT - MARGIN.bottom);
    }
  });

  it('draws the prRI band and flags the result outside it', () => {
    expect(layout.prri!.top).toBeLessThan(layout.prri!.middle);
    expect(layout.prri!.middle).toBeLessThan(layout.prri!.bottom);
    expect(layout.points.map((p) => p.outside)).toEqual([false, false, true]);
  });

  it('includes nearby thresholds and the population band, places jumps and the change point', () => {
    const base = series('alice', 'HBA1C').report;
    const r = {
      ...base,
      thresholds: [
        { value: 6.5, direction: 'above' as const, severity: 'alert' as const, label: 'diabetes range', source: 'ADA' },
        { value: 60, direction: 'above' as const, severity: 'alert' as const, label: 'far away', source: 'x' },
      ],
      rcv_jumps: [{ from_t: base.points[1].t, to_t: base.points[2].t, change: 0.25 }],
      change_point: { t: base.points[2].t, detected_t: base.points[2].t, before: 5.55, after: 7.0, change: 0.26 },
    };
    const l = chartLayout(r, 600);
    expect(l.thresholds.map((t) => t.value)).toEqual([6.5]);
    expect(l.population).not.toBeNull();
    expect(l.jumps).toHaveLength(1);
    expect(l.changePoint!.x).toBeCloseTo(l.points[2].cx);
    expect(l.points[2].jump).toBe(0.25);
    expect(l.points[2].threshold?.label).toBe('diabetes range');
  });

  it('works without a baseline or with a single result', () => {
    const one = report('HBA1C', { points: [{ t: 0, v: 5.5 }], latest: { t: 0, v: 5.5 }, population: null });
    const l = chartLayout(one, 300);
    expect(l.prri).toBeNull();
    expect(Number.isFinite(l.points[0].cx) && Number.isFinite(l.points[0].cy)).toBe(true);
    expect(l.yTicks.length).toBeGreaterThan(1);
  });

  it('finds the nearest point to a pixel', () => {
    expect(nearestIndex(layout.points, 0)).toBe(0);
    expect(nearestIndex(layout.points, 10_000)).toBe(2);
  });
});

describe('thresholdFor', () => {
  it('reports the deepest category on a severity tie (as the engine does)', () => {
    const kdigo = (value: number, severity: 'watch' | 'alert', label: string) =>
      ({ value, direction: 'below' as const, severity, label, source: 'KDIGO' });
    const r = report('EGFR', {
      thresholds: [kdigo(60, 'watch', 'G3a'), kdigo(45, 'watch', 'G3b'), kdigo(30, 'alert', 'G4'), kdigo(15, 'alert', 'G5')],
    });
    expect(thresholdFor(r, 50)?.label).toBe('G3a');
    expect(thresholdFor(r, 40)?.label).toBe('G3b');
    expect(thresholdFor(r, 20)?.label).toBe('G4');
    expect(thresholdFor(r, 10)?.label).toBe('G5');
    expect(thresholdFor(r, 75)).toBeNull();
  });
});

describe('readout', () => {
  it('describes a point in words', () => {
    const r = series('alice', 'HBA1C').report;
    const l = chartLayout({ ...r, rcv_jumps: [{ from_t: r.points[1].t, to_t: r.points[2].t, change: 0.25 }] }, 600);
    expect(pointFlags(l.points[0])).toEqual([]);
    expect(readoutText(l.points[2], '%', 2, 3)).toBe(
      'Result 3 of 3, 2026-03-01: 7.00 % — outside personal range; change beyond RCV (+25.0 %)',
    );
    expect(readoutText({ ...l.points[0], t: 59 * DAY }, 'm[IU]/L', 0, 3)).toContain('mIU/L');
  });
});
