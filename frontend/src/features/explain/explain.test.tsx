// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

const { streamExplain, postExplain, fetchModels } = vi.hoisted(() => ({
  streamExplain: vi.fn(),
  postExplain: vi.fn(),
  fetchModels: vi.fn(),
}));
vi.mock('@/api/llm', () => ({ streamExplain, postExplain, fetchModels }));

import type { ExplainResponse, StreamEvent } from '@/api/schemas';
import { useAnalyzer } from '@/state';
import { series } from '@/test/fixtures';

import { draftText } from './draft';
import { ExplainPanel } from './ExplainPanel';
import { runDraft } from './useExplainStream';

const meta = {
  patient_id: 'alice',
  code: 'HBA1C',
  as_of: null,
  window_days: 1095,
  computed_status: 'alert' as const,
  provider: 'stub' as const,
  model: 'stub',
  stub: true,
  disclaimer: 'AI-generated draft. clinician review required.',
};
const explanation = {
  summary: 'HbA1c rose to 7.0 %.',
  interpretation: 'Beyond biological variation.',
  follow_up: 'Consider a repeat.',
  limitations: 'Medications are not in this record.',
  status: 'alert' as const,
};
const done: ExplainResponse = {
  ...meta,
  explanation,
  status_overridden: false,
  signals: series('alice', 'HBA1C').report.signals,
  not_assessed: [],
};
const models = {
  default: { provider: 'stub', model: 'stub' },
  providers: [
    { provider: 'anthropic', available: false, note: 'set ANTHROPIC_API_KEY', models: [{ provider: 'anthropic', id: 'claude-haiku-4-5', label: 'Claude Haiku' }] },
    { provider: 'stub', available: true, models: [{ provider: 'stub', id: 'stub', label: 'Offline stub' }] },
  ],
};

const initial = useAnalyzer.getState();
afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  useAnalyzer.setState(initial, true);
});

describe('drafts', () => {
  it('render the stream until the validated result replaces it', () => {
    expect(runDraft({ status: 'streaming', partial: { summary: 'HbA1c ro', status: 'al' } })).toEqual({
      summary: 'HbA1c ro',
      interpretation: undefined,
      follow_up: undefined,
      limitations: undefined,
    });
    expect(runDraft({ status: 'done', partial: { summary: 'draft' }, result: done }).summary).toBe('HbA1c rose to 7.0 %.');
  });

  it('copy as text with the disclaimer first', () => {
    const text = draftText(done);
    expect(text.startsWith('AI-generated draft. clinician review required.')).toBe(true);
    expect(text).toContain('What changed\nHbA1c rose to 7.0 %.');
    expect(text).toContain('computed status: alert');
  });
});

describe('ExplainPanel', () => {
  function setup() {
    fetchModels.mockResolvedValue(models);
    useAnalyzer.setState({ selectedPatient: 'alice', series: series('alice', 'HBA1C') });
    let emit: (e: StreamEvent) => void = () => {};
    let finish: () => void = () => {};
    let signal: AbortSignal | undefined;
    streamExplain.mockImplementation((_t, _c, onEvent: (e: StreamEvent) => void, s?: AbortSignal) => {
      emit = onEvent;
      signal = s;
      return new Promise<void>((resolve, reject) => {
        finish = resolve;
        s?.addEventListener('abort', () => reject(new DOMException('aborted', 'AbortError')));
      });
    });
    render(<ExplainPanel />);
    return { emit: (e: StreamEvent) => act(() => emit(e)), finish: () => act(() => finish()), signal: () => signal };
  }

  it('streams, then shows the validated draft with the computed facts', async () => {
    const s = setup();
    await waitFor(() => expect((screen.getByLabelText('Model') as HTMLSelectElement).value).toBe('stub::stub'));
    expect((screen.getByRole('option', { name: 'Claude Haiku' }) as HTMLOptionElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole('button', { name: 'Explain HBA1C' }));
    expect(streamExplain).toHaveBeenCalledWith(
      { patientId: 'alice', code: 'HBA1C', options: { asOf: null, windowDays: 1095 } },
      null,
      expect.any(Function),
      expect.any(AbortSignal),
    );
    expect(screen.getByRole('button', { name: 'Stop' })).toBeTruthy();
    s.emit({ type: 'start', ...meta });
    s.emit({ type: 'delta', field: 'summary', text: 'HbA1c ' });
    s.emit({ type: 'delta', field: 'summary', text: 'rose' });
    const draft = screen.getByRole('article', { name: 'AI draft' });
    expect(draft.getAttribute('aria-busy')).toBe('true');
    expect(draft.textContent).toContain('AI-generated draft. clinician review required.');
    expect(screen.getByRole('region', { name: 'What changed' }).textContent).toContain('HbA1c rose');
    expect(screen.queryByRole('button', { name: /Copy draft/ })).toBeNull();

    s.emit({ type: 'done', ...done });
    await s.finish();
    expect(draft.getAttribute('aria-busy')).toBe('false');
    expect(screen.getByRole('region', { name: 'Follow-up to consider' }).textContent).toContain('Consider a repeat.');
    expect(draft.textContent).toContain('Grounded in 2 computed signals: Personal reference interval, Clinical threshold');
    expect(screen.getByRole('button', { name: /Copy draft/ })).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Explain again' })).toBeTruthy();
  });

  it('Stop aborts the request and says so', async () => {
    const s = setup();
    fireEvent.click(screen.getByRole('button', { name: 'Explain HBA1C' }));
    s.emit({ type: 'start', ...meta });
    fireEvent.click(screen.getByRole('button', { name: 'Stop' }));
    expect(s.signal()?.aborted).toBe(true);
    await waitFor(() => expect(screen.getByRole('status').textContent).toContain('model request was cancelled'));
  });

  it('a stream error offers the non-streaming fallback', async () => {
    const s = setup();
    postExplain.mockResolvedValue(done);
    fireEvent.click(screen.getByRole('button', { name: 'Explain HBA1C' }));
    s.emit({ type: 'error', code: 'llm_upstream', error: 'ollama request failed' });
    expect(screen.getByRole('alert').textContent).toContain('ollama request failed');
    fireEvent.click(screen.getByRole('button', { name: /Try without streaming/ }));
    await waitFor(() => expect(screen.getByRole('region', { name: 'What changed' }).textContent).toContain('HbA1c rose to 7.0 %.'));
  });

  it('renders nothing until a biomarker is open', () => {
    fetchModels.mockResolvedValue(models);
    const { container } = render(<ExplainPanel />);
    expect(container.textContent).toBe('');
  });
});
