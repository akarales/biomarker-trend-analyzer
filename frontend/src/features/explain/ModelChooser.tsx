import { useEffect, useState } from 'react';
import { RefreshCw } from 'lucide-react';

import { fetchModels, type ModelChoice } from '@/api/llm';
import type { LlmProvider, ModelsResponse } from '@/api/schemas';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { NativeSelect, NativeSelectOptGroup, NativeSelectOption } from '@/components/ui/native-select';

const PROVIDER_LABEL: Record<LlmProvider, string> = {
  stub: 'Offline',
  ollama: 'Ollama (local, shared)',
  anthropic: 'Anthropic (cloud)',
};

const encode = (c: ModelChoice) => `${c.provider}::${c.model}`;

interface Props {
  choice: ModelChoice | null;
  onChange(choice: ModelChoice): void;
}

/**
 * Provider + model for drafts. Ollama models are discovered read-only from
 * the shared instance; a "loaded" model answers without loading anything
 * into its memory. Native select: fully keyboard/screen-reader accessible.
 */
export function ModelChooser({ choice, onChange }: Props) {
  const [data, setData] = useState<ModelsResponse | null>(null);
  const [failed, setFailed] = useState(false);
  const [tick, setTick] = useState(0);

  useEffect(() => {
    const controller = new AbortController();
    fetchModels(controller.signal)
      .then((body) => {
        setData(body);
        setFailed(false);
      })
      .catch(() => {
        if (!controller.signal.aborted) setFailed(true);
      });
    return () => controller.abort();
  }, [tick]);

  const effective = choice ?? data?.default ?? null;
  const selected = data?.providers
    .flatMap((p) => p.models)
    .find((m) => effective && m.provider === effective.provider && m.id === effective.model);

  return (
    <div className="flex flex-col gap-1">
      <Label htmlFor="explain-model" className="text-xs">
        Model
      </Label>
      <div className="flex items-center gap-1">
        <NativeSelect
          id="explain-model"
          size="sm"
          className="min-w-0 flex-1"
          disabled={!data}
          value={effective ? encode(effective) : ''}
          onChange={(e) => {
            const [provider, ...rest] = e.target.value.split('::');
            onChange({ provider: provider as LlmProvider, model: rest.join('::') });
          }}
        >
          {!data && <NativeSelectOption value="">{failed ? 'Models unavailable' : 'Loading models…'}</NativeSelectOption>}
          {data?.providers.map((p) => (
            <NativeSelectOptGroup key={p.provider} label={`${PROVIDER_LABEL[p.provider]}${p.available ? '' : ' — unavailable'}`}>
              {p.models.map((m) => (
                <NativeSelectOption key={m.id} value={encode({ provider: m.provider, model: m.id })} disabled={!p.available}>
                  {m.label}
                  {m.loaded ? ' (loaded)' : ''}
                  {m.size_gb !== undefined ? ` · ${m.size_gb} GB` : ''}
                </NativeSelectOption>
              ))}
            </NativeSelectOptGroup>
          ))}
        </NativeSelect>
        <Button type="button" size="icon-sm" variant="ghost" aria-label="Refresh model list" onClick={() => setTick((t) => t + 1)}>
          <RefreshCw aria-hidden="true" />
        </Button>
      </div>
      {selected?.provider === 'ollama' && (
        <p className="text-xs text-muted-foreground">
          {selected.loaded
            ? 'Already loaded in the shared Ollama — no load, no eviction.'
            : 'Not loaded: the first call loads it into the shared Ollama (released after a short keep-alive). Prefer a loaded model.'}
        </p>
      )}
      {selected?.provider === 'anthropic' && (
        <p className="text-xs text-muted-foreground">Sends the analyte data shown here (no patient identifier) to Anthropic.</p>
      )}
    </div>
  );
}
