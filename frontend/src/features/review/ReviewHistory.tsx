import type { ReviewEvent } from '@/api/schemas';
import { RULE_LABEL, STATUS_LABEL, formatDate } from '@/shared/domain';

import { ACTION_LABEL, formatTimestamp } from './labels';

interface Props {
  history: ReviewEvent[];
}

/** The biomarker's append-only review trail, newest first, with the status at decision time. */
export function ReviewHistory({ history }: Props) {
  return (
    <section aria-label="Review history" className="flex flex-col gap-2">
      <h3 className="text-sm font-semibold">Review history</h3>
      {history.length === 0 ? (
        <p className="text-xs text-muted-foreground">No reviews yet.</p>
      ) : (
        <ol className="flex flex-col gap-1.5 text-xs">
          {history.map((e) => (
            <li key={e.id} className="rounded-md border border-border bg-background/40 px-2 py-1.5">
              <span className="font-semibold">{ACTION_LABEL[e.action]}</span> · {RULE_LABEL[e.rule]} (result{' '}
              {formatDate(e.signal_t)}) · {e.actor} · {formatTimestamp(e.created_at)}
              {e.reason && <span className="block text-muted-foreground">“{e.reason}”</span>}
              <span className="block text-muted-foreground">
                At decision: status {STATUS_LABEL[e.snapshot.status].toLowerCase()} — {e.snapshot.signal.explanation}
              </span>
            </li>
          ))}
        </ol>
      )}
      <p className="text-xs text-muted-foreground">
        Append-only audit trail: events are never edited or deleted. Actor is a pseudonym (no login in this demo).
      </p>
    </section>
  );
}
