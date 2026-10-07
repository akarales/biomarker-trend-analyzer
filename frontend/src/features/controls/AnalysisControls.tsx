import { useState } from 'react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { NativeSelect, NativeSelectOption } from '@/components/ui/native-select';
import { WINDOW_CHOICES } from '@/shared/domain';
import { useAnalyzer } from '@/state';

/** Date field; remounted (via `key`) when the committed value changes. */
function AsOfField({ asOf, onCommit }: { asOf: string | null; onCommit(v: string | null): void }) {
  const [draft, setDraft] = useState(asOf ?? '');
  const commit = () => {
    const value = /^\d{4}-\d{2}-\d{2}$/.test(draft) ? draft : null;
    if (value !== asOf) onCommit(value);
  };
  return (
    <Input
      id="as-of"
      type="date"
      value={draft}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={commit}
      onKeyDown={(e) => e.key === 'Enter' && commit()}
      className="h-8 w-40"
      aria-describedby="as-of-hint"
    />
  );
}

/** As-of date (default: each series' latest result) and trend window. */
export function AnalysisControls() {
  const asOf = useAnalyzer((s) => s.asOf);
  const windowDays = useAnalyzer((s) => s.windowDays);
  const setAsOf = useAnalyzer((s) => s.setAsOf);
  const setWindowDays = useAnalyzer((s) => s.setWindowDays);

  return (
    <div role="group" aria-label="Analysis options" className="flex flex-wrap items-end gap-3">
      <div className="flex flex-col gap-1">
        <Label htmlFor="as-of" className="text-xs">
          As of
        </Label>
        <div className="flex gap-1">
          <AsOfField key={asOf ?? 'latest'} asOf={asOf} onCommit={(v) => void setAsOf(v)} />
          <Button type="button" variant="outline" size="sm" onClick={() => void setAsOf(null)} disabled={asOf === null}>
            Latest
          </Button>
        </div>
        <span id="as-of-hint" className="sr-only">
          Results after this date are ignored. Empty means each series' latest result.
        </span>
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor="window" className="text-xs">
          Trend window
        </Label>
        <NativeSelect
          id="window"
          size="sm"
          value={String(windowDays)}
          onChange={(e) => void setWindowDays(Number(e.target.value))}
          className="w-32"
        >
          {WINDOW_CHOICES.map((c) => (
            <NativeSelectOption key={c.days} value={String(c.days)}>
              {c.label}
            </NativeSelectOption>
          ))}
        </NativeSelect>
      </div>
    </div>
  );
}
