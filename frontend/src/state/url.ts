import { DEFAULT_WINDOW_DAYS, WINDOW_CHOICES } from '@/shared/domain';

/** What the address bar carries, so a view can be shared or reloaded. */
export interface UrlState {
  patient: string | null;
  code: string | null;
  asOf: string | null;
  windowDays: number;
}

const DATE = /^\d{4}-\d{2}-\d{2}$/;
const TOKEN = /^[A-Za-z0-9._-]{1,64}$/;

/** Parse `?patient=&code=&as_of=&window=`, ignoring anything malformed. */
export function readUrlState(search: string): UrlState {
  const p = new URLSearchParams(search);
  const token = (k: string) => {
    const v = p.get(k);
    return v && TOKEN.test(v) ? v : null;
  };
  const asOf = p.get('as_of');
  const window = Number(p.get('window'));
  return {
    patient: token('patient'),
    code: token('code'),
    asOf: asOf && DATE.test(asOf) ? asOf : null,
    windowDays: WINDOW_CHOICES.some((c) => c.days === window) ? window : DEFAULT_WINDOW_DAYS,
  };
}

/** Inverse of `readUrlState`; defaults are left out to keep links short. */
export function writeUrlState(s: UrlState): string {
  const p = new URLSearchParams();
  if (s.patient) p.set('patient', s.patient);
  if (s.patient && s.code) p.set('code', s.code);
  if (s.asOf) p.set('as_of', s.asOf);
  if (s.windowDays !== DEFAULT_WINDOW_DAYS) p.set('window', String(s.windowDays));
  const q = p.toString();
  return q ? `?${q}` : '';
}
