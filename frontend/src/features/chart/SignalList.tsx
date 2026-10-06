import type { DriftReport } from '@/api/schemas';
import { RULE_LABEL, STATUS_CARD, formatDate } from '@/shared/domain';

interface Props {
  report: DriftReport;
}

/**
 * Why the status is what it is: every signal with its explanation and
 * source, then what could not be assessed ("no signal" ≠ "healthy").
 */
export function SignalList({ report }: Props) {
  return (
    <section aria-label="Signals" className="rounded-lg border border-line bg-panel p-3">
      <h3 className="mb-2 text-sm font-semibold">Why this status</h3>
      {report.signals.length === 0 ? (
        <p className="text-xs text-muted">
          No rule fired for the checks below — this is not a statement that the result is healthy.
        </p>
      ) : (
        <ul className="flex flex-col gap-2">
          {report.signals.map((s, i) => (
            <li key={`${s.rule}-${i}`} className={`rounded border p-2 text-xs ${STATUS_CARD[s.severity] ?? 'border-line'}`}>
              <p className="font-semibold">
                {RULE_LABEL[s.rule]} · <span className="uppercase">{s.severity}</span> · {formatDate(s.t)}
              </p>
              <p className="mt-1">{s.explanation}</p>
              <p className="mt-1 text-[10px] text-muted">Source: {s.source}</p>
            </li>
          ))}
        </ul>
      )}
      {report.not_assessed.length > 0 && (
        <>
          <h4 className="mt-3 text-xs font-semibold">Not assessed</h4>
          <ul className="mt-1 list-disc pl-4 text-xs text-muted">
            {report.not_assessed.map((n) => (
              <li key={n.rule}>
                {RULE_LABEL[n.rule]}: {n.reason}
              </li>
            ))}
          </ul>
        </>
      )}
      {report.analyte && (
        <p className="mt-3 text-[10px] text-muted">
          {report.analyte.display} (LOINC {report.analyte.loinc}) · CVI {(report.analyte.cvi * 100).toFixed(1)} % ·
          CVA {(report.analyte.cva * 100).toFixed(1)} % (assumed) · profile reviewed {report.analyte.reviewed} ·
          demo, not medical advice; clinician review required
        </p>
      )}
    </section>
  );
}
