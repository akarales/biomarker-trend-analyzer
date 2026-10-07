// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

const { postReview } = vi.hoisted(() => ({ postReview: vi.fn() }));
vi.mock('@/api/reviews', () => ({ postReview }));

import type { ReviewEvent } from '@/api/schemas';
import { unreviewedCount, useAnalyzer } from '@/state';
import { report, series, signal, unreviewed } from '@/test/fixtures';

import { ReviewControls } from './ReviewControls';
import { ReviewHistory } from './ReviewHistory';

const initial = useAnalyzer.getState();
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  useAnalyzer.setState(initial, true);
});

const s = signal();

describe('ReviewControls', () => {
  it('acknowledges in one click', async () => {
    const onSubmit = vi.fn(async () => null);
    render(<ReviewControls signal={s} review={unreviewed('prri', s.t)} onSubmit={onSubmit} />);
    expect(screen.getByText('Unreviewed')).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Acknowledge' }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith({ rule: 'prri', t: s.t, action: 'acknowledge', reason: undefined }));
  });

  it('dismissing needs a reason and shows server errors inline', async () => {
    const onSubmit = vi.fn(async () => 'reason looks like it contains an ID number');
    render(<ReviewControls signal={s} review={undefined} onSubmit={onSubmit} />);
    fireEvent.click(screen.getByRole('button', { name: 'Dismiss…' }));
    const box = screen.getByLabelText(/Why is this signal not clinically relevant\?/);
    const submit = screen.getByRole('button', { name: 'Dismiss signal' }) as HTMLButtonElement;
    expect(submit.disabled).toBe(true);
    fireEvent.change(box, { target: { value: 'MRN 12345678' } });
    expect(submit.disabled).toBe(false);
    fireEvent.click(submit);
    await waitFor(() => expect(screen.getByRole('alert').textContent).toContain('ID number'));
    expect(onSubmit).toHaveBeenCalledWith({ rule: 'prri', t: s.t, action: 'dismiss', reason: 'MRN 12345678' });
    expect(screen.getByRole('button', { name: 'Dismiss signal' })).toBeTruthy();
  });

  it('a decided signal shows who, when and why, and can be reopened', async () => {
    const onSubmit = vi.fn(async () => null);
    const review = {
      ...unreviewed('prri', s.t),
      state: 'dismissed' as const,
      decided_by: 'demo-clinician',
      decided_at: 1_790_000_000,
      reason: 'haemolysed sample',
      notes: 2,
    };
    render(<ReviewControls signal={s} review={review} onSubmit={onSubmit} />);
    const text = document.body.textContent ?? '';
    expect(text).toContain('Dismissed');
    expect(text).toContain('by demo-clinician, 2026-09-21 14:13 UTC');
    expect(text).toContain('— haemolysed sample');
    expect(text).toContain('· 2 notes');
    expect(screen.queryByRole('button', { name: 'Acknowledge' })).toBeNull();
    fireEvent.click(screen.getByRole('button', { name: 'Reopen…' }));
    fireEvent.change(screen.getByLabelText(/Why reopen it\?/), { target: { value: 'still rising' } });
    fireEvent.click(screen.getByRole('button', { name: 'Reopen signal' }));
    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith({ rule: 'prri', t: s.t, action: 'reopen', reason: 'still rising' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Reopen…' })).toBeTruthy());
  });
});

describe('ReviewHistory', () => {
  it('lists events newest first with the status at decision time', () => {
    const event: ReviewEvent = {
      id: 7,
      patient_id: 'alice',
      code: 'HBA1C',
      rule: 'prri',
      signal_t: s.t,
      action: 'dismiss',
      reason: 'haemolysed sample',
      actor: 'demo-clinician',
      snapshot: { status: 'alert', signal: s },
      created_at: 1_790_000_000,
    };
    render(<ReviewHistory history={[event]} />);
    const region = screen.getByRole('region', { name: 'Review history' }).textContent ?? '';
    expect(region).toContain('Dismissed · Personal reference interval (result 2026-03-01) · demo-clinician · 2026-09-21 14:13 UTC');
    expect(region).toContain('“haemolysed sample”');
    expect(region).toContain('At decision: status alert —');
    expect(region).toContain('never edited or deleted');
  });

  it('says when there is nothing yet', () => {
    render(<ReviewHistory history={[]} />);
    expect(screen.getByText('No reviews yet.')).toBeTruthy();
  });
});

describe('review slice', () => {
  it('posts with the view options, then refreshes everything', async () => {
    postReview.mockResolvedValue({});
    const refresh = vi.fn(async () => {});
    useAnalyzer.setState({ selectedPatient: 'alice', series: series('alice', 'HBA1C'), asOf: '2026-08-31', windowDays: 365, refresh });
    const err = await useAnalyzer.getState().submitReview({ rule: 'prri', t: s.t, action: 'acknowledge' });
    expect(err).toBeNull();
    expect(postReview).toHaveBeenCalledWith('alice', 'HBA1C', { rule: 'prri', t: s.t, action: 'acknowledge' }, { asOf: '2026-08-31', windowDays: 365 });
    expect(refresh).toHaveBeenCalled();
  });

  it('returns the error message (stale signal) and does not refresh', async () => {
    postReview.mockRejectedValue(new Error('no prri signal at 1 in the current analysis'));
    const refresh = vi.fn(async () => {});
    useAnalyzer.setState({ selectedPatient: 'alice', series: series('alice', 'HBA1C'), refresh });
    expect(await useAnalyzer.getState().submitReview({ rule: 'prri', t: 1, action: 'acknowledge' })).toMatch(/current analysis/);
    expect(refresh).not.toHaveBeenCalled();
  });

  it('counts unreviewed watch/alert signals only', () => {
    const r = report('HBA1C', { signals: [signal(), signal({ rule: 'population', severity: 'info' })] });
    expect(unreviewedCount(r, undefined)).toBe(1);
    expect(unreviewedCount(r, [{ ...unreviewed('prri', s.t), state: 'acknowledged' }, unreviewed('population', s.t)])).toBe(0);
  });
});
