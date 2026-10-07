import { useMemo } from 'react';

import type { BiomarkerSeries } from '@/api/schemas';
import { Table, TableBody, TableCaption, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table';
import { displayUnit, formatDate, formatValue } from '@/shared/domain';

import { chartLayout } from './geometry';
import { pointFlags } from './readout';

interface Props {
  series: BiomarkerSeries;
  label: string;
}

/** Every analysed result as a table (newest first) — the chart's text alternative. */
export function DataTable({ series, label }: Props) {
  const { report } = series;
  const rows = useMemo(() => {
    const recorded = new Map(series.observations.map((o) => [Date.parse(o.taken_at) / 1000, o]));
    return chartLayout(report, 880)
      .points.map((p) => ({ p, recorded: recorded.get(p.t) }))
      .reverse();
  }, [series, report]);
  const unit = displayUnit(report.unit);

  return (
    // scrollable, so focusable and named (WCAG 2.1.1 / axe scrollable-region-focusable)
    <div
      role="region"
      aria-label={`${label} table`}
      tabIndex={0}
      className="max-h-96 overflow-auto rounded-md border border-border focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
    >
      <Table>
        <TableCaption className="mb-2">
          {label}: {rows.length} results, newest first, values normalised to {unit}
        </TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead scope="col">Date</TableHead>
            <TableHead scope="col" className="text-right">
              Value ({unit})
            </TableHead>
            <TableHead scope="col">{series.derived ? `${series.derived.from} (input)` : 'As recorded'}</TableHead>
            <TableHead scope="col">Flags</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map(({ p, recorded }) => (
            <TableRow key={p.t}>
              <TableCell>{formatDate(p.t)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatValue(p.v)}</TableCell>
              <TableCell className="text-muted-foreground">
                {recorded ? `${recorded.value} ${displayUnit(recorded.unit)}` : '—'}
              </TableCell>
              <TableCell className="whitespace-normal">{pointFlags(p).join('; ') || '—'}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  );
}
