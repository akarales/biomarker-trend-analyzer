/** Display helpers matching the engine's explanation text (crates/drift/src/fmt.rs). */

/** A lab value at a precision that matches its magnitude. */
export function formatValue(v: number): string {
  const a = Math.abs(v);
  return v.toFixed(a >= 100 ? 0 : a >= 10 ? 1 : 2);
}

/** A fraction as a signed percentage, e.g. `+12.3 %`. */
export function formatPct(fraction: number): string {
  const pct = fraction * 100;
  return `${pct >= 0 ? '+' : ''}${pct.toFixed(1)} %`;
}

/** `YYYY-MM-DD` (UTC) for epoch seconds. */
export function formatDate(epochSeconds: number): string {
  return new Date(epochSeconds * 1000).toISOString().slice(0, 10);
}

/** UCUM codes as clinicians read them. */
export function displayUnit(unit: string): string {
  return unit.replace('m[IU]/L', 'mIU/L').replace('umol/L', 'µmol/L').replace('mL/min/{1.73_m2}', 'mL/min/1.73 m²');
}

/** "5.94–6.43 %" */
export function formatRange(low: number, high: number, unit: string): string {
  return `${formatValue(low)}–${formatValue(high)} ${displayUnit(unit)}`;
}
