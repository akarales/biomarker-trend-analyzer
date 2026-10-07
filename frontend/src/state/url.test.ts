import { describe, expect, it } from 'vitest';

import { DEFAULT_WINDOW_DAYS } from '@/shared/domain';

import { readUrlState, writeUrlState } from './url';

describe('URL state', () => {
  it('round-trips a full view', () => {
    const s = { patient: 'SYN-01', code: 'HBA1C', asOf: '2026-08-31', windowDays: 365 };
    expect(writeUrlState(s)).toBe('?patient=SYN-01&code=HBA1C&as_of=2026-08-31&window=365');
    expect(readUrlState(writeUrlState(s))).toEqual(s);
  });

  it('leaves defaults out and ignores malformed values', () => {
    expect(writeUrlState({ patient: null, code: 'X', asOf: null, windowDays: DEFAULT_WINDOW_DAYS })).toBe('');
    expect(readUrlState('?patient=<script>&code=HBA1C&as_of=yesterday&window=12')).toEqual({
      patient: null,
      code: 'HBA1C',
      asOf: null,
      windowDays: DEFAULT_WINDOW_DAYS,
    });
  });
});
