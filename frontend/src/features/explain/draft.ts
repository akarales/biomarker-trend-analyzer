import { EXPLANATION_FIELDS, type ExplainResponse, type ExplanationField } from '@/api/schemas';

export const FIELD_LABEL: Record<ExplanationField, string> = {
  summary: 'What changed',
  interpretation: 'Possible explanations to consider',
  follow_up: 'Follow-up to consider',
  limitations: 'Limits of this analysis',
};

/** Plain-text copy of a validated draft, disclaimer first. */
export function draftText(result: ExplainResponse): string {
  const body = EXPLANATION_FIELDS.map((f) => `${FIELD_LABEL[f]}\n${result.explanation[f]}`).join('\n\n');
  return `${result.disclaimer}\nModel: ${result.provider} / ${result.model} · computed status: ${result.computed_status}\n\n${body}\n`;
}
