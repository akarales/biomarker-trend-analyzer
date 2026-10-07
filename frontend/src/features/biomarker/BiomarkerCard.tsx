import { cn } from 'cn';

import type { DriftReport } from '@/api/schemas';
import { StatusChip } from '@/shared/components/StatusChip';
import {
  RULE_LABEL,
  STATUS_COLOR,
  TREND_SYMBOL,
  displayUnit,
  formatDate,
  formatPct,
  formatRange,
  formatValue,
} from '@/shared/domain';

interface Props {
  report: DriftReport;
  selected: boolean;
  onSelect(code: string): void;
  /** watch/alert signals still awaiting review */
  unreviewed?: number;
  /** code of the measured series this one is derived from (EGFR ← CREAT) */
  derivedFrom?: string;
}

function Row({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex justify-between gap-2">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="text-right tabular-nums">{value}</dd>
    </div>
  );
}

/** One biomarker: latest result, personal vs population range, trend and the rule behind the status. */
export function BiomarkerCard({ report, selected, onSelect, unreviewed = 0, derivedFrom }: Props) {
  const top = report.signals.find((s) => s.severity !== 'info');
  const unit = report.unit;
  const trend = report.trend;
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={() => onSelect(report.code)}
      className={cn(
        'flex flex-col gap-2 rounded-lg border border-border bg-card p-3 text-left text-sm transition-colors hover:bg-muted/60 focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
        selected && 'ring-2 ring-primary',
      )}
      style={{ borderTop: `3px solid ${STATUS_COLOR[report.status]}` }}
    >
      <span className="flex items-start justify-between gap-2">
        <span>
          <span className="block font-semibold">{report.code}</span>
          <span className="block text-xs text-muted-foreground">{report.analyte?.display ?? 'no analyte profile'}</span>
          {derivedFrom && <span className="block text-xs text-muted-foreground">derived from {derivedFrom}</span>}
        </span>
        <StatusChip status={report.status} />
      </span>
      <span className="text-lg font-semibold tabular-nums">
        {report.latest ? `${formatValue(report.latest.v)} ${displayUnit(unit)}` : '—'}
        {report.latest && <span className="ml-2 text-xs font-normal text-muted-foreground">{formatDate(report.latest.t)}</span>}
      </span>
      <dl className="flex flex-col gap-0.5 text-xs">
        <Row label="Personal" value={report.baseline ? formatRange(report.baseline.prri_low, report.baseline.prri_high, unit) : 'not yet (needs 3 earlier results)'} />
        <Row label="Population" value={report.population ? formatRange(report.population.low, report.population.high, unit) : report.thresholds.length ? 'see clinical thresholds' : 'none in profile'} />
        <Row
          label="Trend"
          value={trend ? `${TREND_SYMBOL[trend.direction]} ${trend.direction}${trend.direction !== 'flat' ? ` ${formatPct(trend.change_per_year)}/yr` : ''}` : 'too few results'}
        />
      </dl>
      <span className="text-xs">{top ? `Why: ${RULE_LABEL[top.rule]}` : 'No rule fired'}</span>
      {top && (
        <span className={unreviewed ? 'text-xs font-semibold' : 'text-xs text-muted-foreground'}>
          {unreviewed ? `${unreviewed} unreviewed` : 'All signals reviewed'}
        </span>
      )}
    </button>
  );
}
