import type { DriftReport } from '@/api/schemas';
import { RULE_LABEL, STATUS_CARD, STATUS_CARD_FALLBACK, formatValue } from '@/shared/domain';

interface Props {
  report: DriftReport;
  selected: boolean;
  onSelect(code: string): void;
}

/** One biomarker's drift status: latest value, trend, status and the rule that set it. */
export function BiomarkerCard({ report, selected, onSelect }: Props) {
  const top = report.signals[0];
  return (
    <button
      type="button"
      onClick={() => onSelect(report.code)}
      className={`rounded-lg border p-3 text-left ${
        STATUS_CARD[report.status] ?? STATUS_CARD_FALLBACK
      } ${selected ? 'ring-2 ring-primary' : ''}`}
    >
      <p className="text-sm font-semibold">{report.code}</p>
      <p className="text-xs text-muted">
        {report.unit} · {report.points.length} readings
      </p>
      <p className="mt-1 text-xs">
        latest {report.latest ? formatValue(report.latest.v) : '—'} · {report.trend?.direction ?? '—'}
      </p>
      <p className="mt-1 text-[10px] uppercase tracking-wide">
        {report.status}
        {top && top.severity !== 'info' ? ` · ${RULE_LABEL[top.rule]}` : ''}
      </p>
    </button>
  );
}
