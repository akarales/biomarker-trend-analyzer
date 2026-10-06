import type { DriftReport } from '@/api/schemas';
import { STATUS_CARD, STATUS_CARD_FALLBACK } from '@/shared/domain';

interface Props {
  report: DriftReport;
  selected: boolean;
  onSelect(code: string): void;
}

/** One biomarker's drift status: latest z, trend and status word. */
export function BiomarkerCard({ report, selected, onSelect }: Props) {
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
        {report.unit} · {report.series_len} readings
      </p>
      <p className="mt-1 text-xs">
        z={report.latest_z?.toFixed(1) ?? '—'} ·{' '}
        {report.trend ?? '—'}
      </p>
      <p className="mt-1 text-[10px] uppercase tracking-wide">
        {report.status}
      </p>
    </button>
  );
}
