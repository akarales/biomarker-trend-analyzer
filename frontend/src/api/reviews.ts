import { api } from './http';
import type { AnalysisOptions } from './patients';
import type { Rule } from '@/shared/domain';

import { ReviewCreatedSchema, type ReviewAction, type ReviewCreated } from './schemas';

export interface ReviewInput {
  rule: Rule;
  /** epoch seconds of the result the signal fired on */
  t: number;
  action: ReviewAction;
  reason?: string;
}

/**
 * Append a review event. The server recomputes the analysis with the same
 * options and stores its own snapshot; a signal it no longer computes is
 * refused with 409 `stale_signal`.
 */
export function postReview(
  patientId: string,
  code: string,
  input: ReviewInput,
  { asOf, windowDays }: AnalysisOptions,
): Promise<ReviewCreated> {
  return api(`/patients/${encodeURIComponent(patientId)}/biomarkers/${encodeURIComponent(code)}/reviews`, ReviewCreatedSchema, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ ...input, as_of: asOf, window_days: windowDays }),
  });
}
