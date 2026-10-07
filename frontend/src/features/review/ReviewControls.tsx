import { useId, useState } from 'react';

import type { ReviewInput } from '@/api/reviews';
import type { ReviewAction, Signal, SignalReview } from '@/api/schemas';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';

import { REASON_PROMPT, STATE_LABEL, formatTimestamp } from './labels';

type ReasonAction = Exclude<ReviewAction, 'acknowledge'>;

interface Props {
  signal: Signal;
  review: SignalReview | undefined;
  onSubmit(input: ReviewInput): Promise<string | null>;
}

/**
 * Review one watch/alert signal: acknowledge, dismiss (reason), add a
 * note, or reopen a decided signal (reason). Every action appends an
 * audit event; nothing is ever edited or deleted.
 */
export function ReviewControls({ signal, review, onSubmit }: Props) {
  const [form, setForm] = useState<ReasonAction | null>(null);
  const [reason, setReason] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const fieldId = useId();
  const state = review?.state ?? 'unreviewed';

  const submit = async (action: ReviewAction, text?: string) => {
    setBusy(true);
    const err = await onSubmit({ rule: signal.rule, t: signal.t, action, reason: text });
    setBusy(false);
    setError(err);
    if (!err) {
      setForm(null);
      setReason('');
    }
  };

  return (
    <div className="mt-2 flex flex-col gap-2 border-t border-border pt-2 text-xs">
      <p className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <span className={state === 'unreviewed' ? 'font-semibold text-foreground' : 'font-semibold text-muted-foreground'}>
          {STATE_LABEL[state]}
        </span>
        {review?.decided_by && review.decided_at !== null && (
          <span className="text-muted-foreground">
            by {review.decided_by}, {formatTimestamp(review.decided_at)}
          </span>
        )}
        {review?.reason && <span className="text-muted-foreground">— {review.reason}</span>}
        {review && review.notes > 0 && (
          <span className="text-muted-foreground">
            · {review.notes} note{review.notes > 1 ? 's' : ''}
          </span>
        )}
      </p>
      {form === null ? (
        <div className="flex flex-wrap gap-1.5" role="group" aria-label="Review actions">
          {state === 'unreviewed' ? (
            <>
              <Button type="button" size="xs" onClick={() => void submit('acknowledge')} disabled={busy}>
                Acknowledge
              </Button>
              <Button type="button" size="xs" variant="outline" onClick={() => setForm('dismiss')} disabled={busy}>
                Dismiss…
              </Button>
            </>
          ) : (
            <Button type="button" size="xs" variant="outline" onClick={() => setForm('reopen')} disabled={busy}>
              Reopen…
            </Button>
          )}
          <Button type="button" size="xs" variant="ghost" onClick={() => setForm('annotate')} disabled={busy}>
            Add note…
          </Button>
        </div>
      ) : (
        <form
          className="flex flex-col gap-1.5"
          onSubmit={(e) => {
            e.preventDefault();
            void submit(form, reason);
          }}
        >
          <Label htmlFor={fieldId} className="text-xs">
            {REASON_PROMPT[form]} (required, no patient identifiers)
          </Label>
          <Textarea id={fieldId} value={reason} onChange={(e) => setReason(e.target.value)} rows={2} maxLength={500} className="text-xs" />
          <div className="flex gap-1.5">
            <Button type="submit" size="xs" disabled={busy || reason.trim().length < 3}>
              {form === 'dismiss' ? 'Dismiss signal' : form === 'reopen' ? 'Reopen signal' : 'Save note'}
            </Button>
            <Button
              type="button"
              size="xs"
              variant="ghost"
              onClick={() => {
                setForm(null);
                setError(null);
              }}
            >
              Cancel
            </Button>
          </div>
        </form>
      )}
      {error && (
        <p role="alert" className="text-destructive">
          {error}
        </p>
      )}
    </div>
  );
}
