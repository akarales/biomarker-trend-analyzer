import { displayUnit, formatDate, formatPct, formatValue } from '@/shared/domain';

import type { ChartPoint } from './geometry';

/** What a point means, in words (hover box, keyboard announcement, table). */
export function pointFlags(p: ChartPoint): string[] {
  const flags: string[] = [];
  if (p.outside) flags.push('outside personal range');
  if (p.jump !== null) flags.push(`change beyond RCV (${formatPct(p.jump)})`);
  if (p.threshold) flags.push(p.threshold.label);
  return flags;
}

/** "2026-03-06: 6.90 % — outside personal range; diabetes range (ADA ≥ 6.5 %)" */
export function readoutText(p: ChartPoint, unit: string, index: number, count: number): string {
  const flags = pointFlags(p);
  return (
    `Result ${index + 1} of ${count}, ${formatDate(p.t)}: ${formatValue(p.v)} ${displayUnit(unit)}` +
    (flags.length ? ` — ${flags.join('; ')}` : '')
  );
}
