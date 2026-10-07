import { useMemo, useState, type KeyboardEvent, type PointerEvent } from 'react';

import type { DriftReport } from '@/api/schemas';
import { useElementWidth } from '@/shared/hooks/useElementWidth';
import { displayUnit } from '@/shared/domain';

import { ChartLayers } from './ChartLayers';
import { chartLayout, nearestIndex } from './geometry';
import { readoutText } from './readout';

interface Props {
  report: DriftReport;
  /** accessible name, e.g. "Hemoglobin A1c trend" */
  label: string;
}

/**
 * Interactive trend chart. One focusable region (not one tab stop per
 * result): ←/→ move between results, Home/End jump, PageUp/PageDown move
 * 10; hover does the same with the pointer. The readout is announced
 * politely; the table view is the full non-visual alternative.
 */
export function TrendChart({ report, label }: Props) {
  const { ref, width } = useElementWidth<HTMLDivElement>();
  const layout = useMemo(() => chartLayout(report, width), [report, width]);
  const [active, setActive] = useState<number | null>(null);
  const count = layout.points.length;
  const point = active !== null ? layout.points[active] : null;
  const text = point && active !== null ? readoutText(point, report.unit, active, count) : '';

  const move = (to: number) => setActive(Math.min(Math.max(to, 0), count - 1));
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const from = active ?? count - 1;
    const keys: Record<string, number> = {
      ArrowLeft: from - 1,
      ArrowRight: from + 1,
      Home: 0,
      End: count - 1,
      PageUp: from - 10,
      PageDown: from + 10,
    };
    if (e.key in keys) {
      e.preventDefault();
      move(active === null ? count - 1 : keys[e.key]);
    } else if (e.key === 'Escape') {
      setActive(null);
    }
  };
  const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
    const box = e.currentTarget.getBoundingClientRect();
    if (count) setActive(nearestIndex(layout.points, e.clientX - box.left));
  };

  return (
    <div ref={ref} className="relative w-full">
      <div
        role="group"
        aria-roledescription="interactive chart"
        aria-label={`${label}, ${count} results in ${displayUnit(report.unit)}. Use the arrow keys to read each result.`}
        tabIndex={0}
        onKeyDown={onKeyDown}
        onFocus={() => active === null && count && setActive(count - 1)}
        onPointerMove={onPointerMove}
        onPointerLeave={() => setActive(null)}
        className="rounded-md outline-none focus-visible:ring-2 focus-visible:ring-ring"
      >
        <ChartLayers layout={layout} active={active} />
      </div>
      {point && (
        <div
          aria-hidden="true"
          className="pointer-events-none absolute z-10 max-w-64 -translate-x-1/2 rounded-md border border-border bg-popover px-2 py-1 text-xs text-popover-foreground shadow-md"
          style={{ left: Math.min(Math.max(point.cx, 110), width - 110), top: Math.max(point.cy - 70, 0) }}
        >
          {text.replace(/^Result \d+ of \d+, /, '')}
        </div>
      )}
      <p aria-live="polite" className="sr-only">
        {text}
      </p>
    </div>
  );
}
