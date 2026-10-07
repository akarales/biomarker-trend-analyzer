import type { ReactNode } from 'react';

import type { DriftReport } from '@/api/schemas';
import { CHART_COLOR, SEVERITY_COLOR, displayUnit, formatDate, formatRange } from '@/shared/domain';

interface Props {
  report: DriftReport;
}

function Swatch({ color, dashed = false, band = false }: { color: string; dashed?: boolean; band?: boolean }) {
  return (
    <span
      aria-hidden="true"
      className="inline-block h-2.5 w-5 shrink-0 rounded-sm"
      style={
        band
          ? { backgroundColor: color, opacity: 0.45 }
          : { borderTop: `2px ${dashed ? 'dashed' : 'solid'} ${color}`, height: 0 }
      }
    />
  );
}

/** Text legend for every layer actually drawn (with the values, not just colours). */
export function ChartLegend({ report }: Props) {
  const unit = report.unit;
  const items: { key: string; swatch: ReactNode; text: string }[] = [];
  if (report.baseline) {
    const b = report.baseline;
    items.push({
      key: 'prri',
      swatch: <Swatch color={CHART_COLOR.prri} band />,
      text: `Personal range ${formatRange(b.prri_low, b.prri_high, unit)} (from ${b.n} results ${formatDate(b.from)} → ${formatDate(b.to)})`,
    });
  }
  if (report.population) {
    items.push({
      key: 'pop',
      swatch: <Swatch color={CHART_COLOR.population} band />,
      text: `Population range ${formatRange(report.population.low, report.population.high, unit)}`,
    });
  }
  for (const t of report.thresholds) {
    items.push({ key: `t${t.value}`, swatch: <Swatch color={SEVERITY_COLOR[t.severity]} dashed />, text: t.label });
  }
  if (report.rcv_jumps.length) {
    items.push({
      key: 'rcv',
      swatch: <Swatch color={CHART_COLOR.rcv} />,
      text: `${report.rcv_jumps.length} change${report.rcv_jumps.length > 1 ? 's' : ''} beyond the reference change value`,
    });
  }
  if (report.change_point) {
    items.push({
      key: 'cp',
      swatch: <Swatch color={CHART_COLOR.changePoint} dashed />,
      text: `Shift starting ${formatDate(report.change_point.t)} (CUSUM)`,
    });
  }
  items.push({
    key: 'out',
    swatch: <span aria-hidden="true" className="size-2.5 rounded-full" style={{ backgroundColor: CHART_COLOR.flagged }} />,
    text: `Result outside the personal range · values in ${displayUnit(unit)}`,
  });

  return (
    <ul aria-label="Chart legend" className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
      {items.map((item) => (
        <li key={item.key} className="flex items-center gap-1.5">
          {item.swatch}
          {item.text}
        </li>
      ))}
    </ul>
  );
}
