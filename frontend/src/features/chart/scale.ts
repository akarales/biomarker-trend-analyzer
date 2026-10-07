/** Axis maths for the hand-rolled SVG chart: "nice" value ticks and calendar time ticks (UTC). */

export interface ValueTicks {
  lo: number;
  hi: number;
  ticks: number[];
}

/** Round-number ticks covering [min, max] with about `target` intervals. */
export function niceTicks(min: number, max: number, target = 5): ValueTicks {
  if (!Number.isFinite(min) || !Number.isFinite(max)) return { lo: 0, hi: 1, ticks: [0, 1] };
  if (max - min < 1e-9) {
    const pad = Math.abs(max) * 0.05 || 1;
    return niceTicks(min - pad, max + pad, target);
  }
  const raw = (max - min) / target;
  const magnitude = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 2.5, 5, 10].map((m) => m * magnitude).find((s) => s >= raw) ?? 10 * magnitude;
  const lo = Math.floor(min / step) * step;
  const hi = Math.ceil(max / step) * step;
  const ticks: number[] = [];
  for (let v = lo; v <= hi + step / 2; v += step) ticks.push(Number(v.toFixed(10)));
  return { lo, hi, ticks };
}

export interface TimeTick {
  /** epoch seconds */
  t: number;
  label: string;
}

const MONTH = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/**
 * Calendar ticks between two instants: years for long spans, quarters or
 * months for shorter ones, thinned to at most `max` labels.
 */
export function timeTicks(from: number, to: number, max = 7): TimeTick[] {
  const start = new Date(from * 1000);
  const end = new Date(to * 1000);
  const months = (end.getUTCFullYear() - start.getUTCFullYear()) * 12 + end.getUTCMonth() - start.getUTCMonth();
  const stepMonths = months > 36 ? 12 : months > 12 ? 3 : 1;
  const ticks: TimeTick[] = [];
  let y = start.getUTCFullYear();
  let m = Math.ceil((start.getUTCMonth() + (start.getUTCDate() > 1 ? 1 : 0)) / stepMonths) * stepMonths;
  for (;;) {
    y += Math.floor(m / 12);
    m %= 12;
    const t = Date.UTC(y, m, 1) / 1000;
    if (t > to) break;
    if (t >= from) ticks.push({ t, label: stepMonths === 12 ? String(y) : `${MONTH[m]} ${y}` });
    m += stepMonths;
  }
  const every = Math.ceil(ticks.length / max);
  return every > 1 ? ticks.filter((_, i) => i % every === 0) : ticks;
}
