import { cn } from 'cn';

import type { PatientSummaryEntry } from '@/api/schemas';
import { StatusChip } from '@/shared/components/StatusChip';
import { RULE_LABEL, STATUS_COLOR } from '@/shared/domain';

interface Props {
  patient: PatientSummaryEntry;
  selected: boolean;
  onSelect(id: string): void;
}

function counts(p: PatientSummaryEntry): string {
  const parts = [];
  if (p.alerts) parts.push(`${p.alerts} alert`);
  if (p.watches) parts.push(`${p.watches} watch`);
  return parts.length ? parts.join(' · ') : 'no rule fired';
}

/** One triage row: id, worst status, counts and the signal that put the patient there. */
export function TriageRow({ patient, selected, onSelect }: Props) {
  const top = patient.top_signal;
  return (
    <li>
      <button
        type="button"
        aria-current={selected ? 'true' : undefined}
        onClick={() => onSelect(patient.patient_id)}
        className={cn(
          'flex w-full flex-col gap-1 rounded-md border border-transparent px-3 py-2 text-left transition-colors hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
          selected && 'border-border bg-muted',
        )}
        style={{ borderLeft: `3px solid ${STATUS_COLOR[patient.status]}` }}
      >
        <span className="flex items-center justify-between gap-2">
          <span className="font-semibold">{patient.patient_id}</span>
          <StatusChip status={patient.status} />
        </span>
        <span className="text-xs text-muted-foreground">
          {counts(patient)} · {patient.biomarkers} biomarkers · {patient.observations} results
        </span>
        {top && (
          <span className="truncate text-xs">
            {top.display} — {RULE_LABEL[top.rule]}
          </span>
        )}
      </button>
    </li>
  );
}
