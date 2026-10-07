import type { ReviewInput } from '@/api/reviews';
import type { DriftReport, SignalReview } from '@/api/schemas';
import { ReviewControls } from '@/features/review';
import { StatusChip } from '@/shared/components/StatusChip';
import { RULE_LABEL, SEVERITY_COLOR, formatDate } from '@/shared/domain';

interface Props {
  report: DriftReport;
  /** review status per signal (same order); review controls show when given with `onReview` */
  reviews?: SignalReview[];
  onReview?(input: ReviewInput): Promise<string | null>;
}

/**
 * Why the status is what it is: every signal with its explanation and
 * source, then what could not be assessed ("no signal" ≠ "healthy").
 */
export function SignalList({ report, reviews, onReview }: Props) {
  return (
    <section aria-label="Signals" className="flex flex-col gap-3">
      <h3 className="text-sm font-semibold">Why this status</h3>
      {report.signals.length === 0 ? (
        <p className="text-sm text-muted-foreground">
          No rule fired for the checks below — this is not a statement that the result is healthy.
        </p>
      ) : (
        <ul className="flex flex-col gap-2">
          {report.signals.map((s, i) => (
            <li
              key={`${s.rule}-${i}`}
              className="rounded-md border border-border bg-background/40 p-3 text-sm"
              style={{ borderLeft: `3px solid ${SEVERITY_COLOR[s.severity]}` }}
            >
              <p className="flex flex-wrap items-center gap-2 font-semibold">
                <StatusChip severity={s.severity} />
                {RULE_LABEL[s.rule]}
                <span className="text-xs font-normal text-muted-foreground">{formatDate(s.t)}</span>
              </p>
              <p className="mt-1.5">{s.explanation}</p>
              <p className="mt-1 text-xs text-muted-foreground">Source: {s.source}</p>
              {onReview && s.severity !== 'info' && <ReviewControls signal={s} review={reviews?.[i]} onSubmit={onReview} />}
            </li>
          ))}
        </ul>
      )}
      {report.not_assessed.length > 0 && (
        <div>
          <h4 className="text-xs font-semibold">Not assessed</h4>
          <ul className="mt-1 list-disc pl-4 text-xs text-muted-foreground">
            {report.not_assessed.map((n) => (
              <li key={n.rule}>
                {RULE_LABEL[n.rule]}: {n.reason}
              </li>
            ))}
          </ul>
        </div>
      )}
      {report.analyte && (
        <p className="text-xs text-muted-foreground">
          {report.analyte.display} (LOINC {report.analyte.loinc}) · CVI {(report.analyte.cvi * 100).toFixed(1)} % ·
          CVA {(report.analyte.cva * 100).toFixed(1)} % (assumed) · profile reviewed {report.analyte.reviewed} · demo,
          not medical advice; clinician review required
        </p>
      )}
    </section>
  );
}
