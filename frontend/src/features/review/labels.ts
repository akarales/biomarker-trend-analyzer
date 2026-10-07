import type { ReviewAction, ReviewState } from '@/api/schemas';

export const STATE_LABEL: Record<ReviewState, string> = {
  unreviewed: 'Unreviewed',
  acknowledged: 'Acknowledged',
  dismissed: 'Dismissed',
};

export const ACTION_LABEL: Record<ReviewAction, string> = {
  acknowledge: 'Acknowledged',
  annotate: 'Note',
  dismiss: 'Dismissed',
  reopen: 'Reopened',
};

/** What the reason field asks for, per action (acknowledge needs none). */
export const REASON_PROMPT: Record<Exclude<ReviewAction, 'acknowledge'>, string> = {
  dismiss: 'Why is this signal not clinically relevant?',
  annotate: 'Note',
  reopen: 'Why reopen it?',
};

/** `YYYY-MM-DD HH:MM UTC` for epoch seconds. */
export function formatTimestamp(epochSeconds: number): string {
  return `${new Date(epochSeconds * 1000).toISOString().slice(0, 16).replace('T', ' ')} UTC`;
}
