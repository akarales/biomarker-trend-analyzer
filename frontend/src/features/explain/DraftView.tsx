import { useState } from 'react';
import { Copy } from 'lucide-react';

import { EXPLANATION_FIELDS, type ExplainMeta, type ExplainResponse } from '@/api/schemas';
import { Button } from '@/components/ui/button';
import { StatusChip } from '@/shared/components/StatusChip';
import { RULE_LABEL } from '@/shared/domain';

import { FIELD_LABEL, draftText } from './draft';
import type { Draft } from './useExplainStream';

interface Props {
  draft: Draft;
  meta?: ExplainMeta;
  result?: ExplainResponse;
  streaming: boolean;
}

/**
 * The draft: disclaimer from the first line, fields as they stream
 * (`aria-busy`, not a live region), then the validated text with the
 * computed status and signals restated as the authority.
 */
export function DraftView({ draft, meta, result, streaming }: Props) {
  const [copied, setCopied] = useState(false);
  const fields = EXPLANATION_FIELDS.filter((f) => draft[f]);

  return (
    <article aria-label="AI draft" aria-busy={streaming} className="flex flex-col gap-3 rounded-md border border-border bg-background/40 p-3 text-sm">
      {meta && <p className="rounded-md border border-border bg-muted/50 p-2 text-xs">{meta.disclaimer}</p>}
      {meta && (
        <p className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          Computed status (authoritative): <StatusChip status={meta.computed_status} />
          <span>
            · {meta.stub ? 'offline stub' : `${meta.provider} / ${meta.model}`}
          </span>
        </p>
      )}
      {fields.map((f) => (
        <section key={f} aria-label={FIELD_LABEL[f]}>
          <h4 className="text-xs font-semibold">{FIELD_LABEL[f]}</h4>
          <p className="mt-0.5 whitespace-pre-wrap">{draft[f]}</p>
        </section>
      ))}
      {result && (
        <footer className="flex flex-col gap-2 border-t border-border pt-2 text-xs text-muted-foreground">
          {result.status_overridden && (
            <p className="text-foreground">The model proposed a different status; the computed status replaced it.</p>
          )}
          <p>
            Grounded in {result.signals.length} computed signal{result.signals.length === 1 ? '' : 's'}
            {result.signals.length > 0 && `: ${[...new Set(result.signals.map((s) => RULE_LABEL[s.rule]))].join(', ')}`}. Review before use.
          </p>
          <div>
            <Button
              type="button"
              size="xs"
              variant="outline"
              onClick={() => void navigator.clipboard?.writeText(draftText(result)).then(() => setCopied(true))}
            >
              <Copy aria-hidden="true" /> {copied ? 'Copied' : 'Copy draft'}
            </Button>
          </div>
        </footer>
      )}
    </article>
  );
}
