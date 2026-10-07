import { cn } from 'cn';

import { SEVERITY_COLOR, SEVERITY_LABEL, STATUS_COLOR, STATUS_LABEL, tint, type Severity, type Status } from '@/shared/domain';

type Props = { className?: string } & ({ status: Status; severity?: never } | { severity: Severity; status?: never });

/** Coloured dot + word on an opaque tint: colour is never the only channel. */
export function StatusChip({ status, severity, className }: Props) {
  const color = status ? STATUS_COLOR[status] : SEVERITY_COLOR[severity!];
  const label = status ? STATUS_LABEL[status] : SEVERITY_LABEL[severity!];
  return (
    <span
      className={cn('inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 text-xs font-semibold', className)}
      style={{ color, backgroundColor: tint(color) }}
    >
      <span aria-hidden="true" className="size-1.5 rounded-full" style={{ backgroundColor: color }} />
      {label}
    </span>
  );
}
