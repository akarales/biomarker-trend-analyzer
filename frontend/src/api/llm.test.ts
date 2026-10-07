import { afterEach, describe, expect, it, vi } from 'vitest';

import { postExplain, streamExplain } from './llm';
import type { StreamEvent } from './llmSchemas';

const meta = {
  patient_id: 'alice',
  code: 'HBA1C',
  as_of: null,
  window_days: 1095,
  computed_status: 'alert',
  provider: 'stub',
  model: 'stub',
  stub: true,
  disclaimer: 'Not medical advice.',
};
const explanation = { summary: 'Rose ✓ "quoted"', interpretation: 'i', follow_up: 'f', limitations: 'l', status: 'alert' };
const lines = [
  { type: 'start', ...meta },
  { type: 'delta', field: 'summary', text: 'Rose ✓ ' },
  { type: 'delta', field: 'summary', text: '"quoted"' },
  { type: 'done', ...meta, explanation, status_overridden: false, signals: [], not_assessed: [] },
].map((l) => JSON.stringify(l) + '\n');

const target = { patientId: 'alice', code: 'HBA1C', options: { asOf: '2026-08-31', windowDays: 365 } };

function streamOf(text: string, chunk: number): ReadableStream<Uint8Array> {
  const bytes = new TextEncoder().encode(text);
  let i = 0;
  return new ReadableStream({
    pull(controller) {
      if (i >= bytes.length) return controller.close();
      controller.enqueue(bytes.slice(i, i + chunk));
      i += chunk;
    },
  });
}

afterEach(() => vi.unstubAllGlobals());

describe('streamExplain', () => {
  it.each([1, 7, 64, 4096])('parses NDJSON split into %i-byte chunks (multi-byte chars split too)', async (chunk) => {
    const fetchMock = vi.fn(async () => new Response(streamOf(lines.join(''), chunk), { status: 200 }));
    vi.stubGlobal('fetch', fetchMock);
    const events: StreamEvent[] = [];
    await streamExplain(target, { provider: 'stub', model: 'stub' }, (e) => events.push(e));
    expect(events.map((e) => e.type)).toEqual(['start', 'delta', 'delta', 'done']);
    const text = events.flatMap((e) => (e.type === 'delta' ? [e.text] : [])).join('');
    expect(text).toBe('Rose ✓ "quoted"');
    const [, init] = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
    expect(JSON.parse(String(init.body))).toEqual({
      patient_id: 'alice',
      code: 'HBA1C',
      as_of: '2026-08-31',
      window_days: 365,
      provider: 'stub',
      model: 'stub',
    });
  });

  it('throws the coded server error when validation fails before streaming', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('{"error":"unknown patient or biomarker: x","code":"not_found"}', { status: 404, headers: { 'x-request-id': 'r1' } })),
    );
    await expect(streamExplain(target, null, () => {})).rejects.toMatchObject({ code: 'not_found', requestId: 'r1' });
  });

  it('rejects an unexpected event shape', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('{"type":"surprise"}\n', { status: 200 })));
    await expect(streamExplain(target, null, () => {})).rejects.toThrow(/Unexpected stream event/);
  });
});

describe('postExplain', () => {
  it('sends JSON (not the upload content-type sniffing)', async () => {
    const fetchMock = vi.fn(
      async () => new Response(JSON.stringify({ ...meta, explanation, status_overridden: false, signals: [], not_assessed: [] }), { status: 200 }),
    );
    vi.stubGlobal('fetch', fetchMock);
    const result = await postExplain(target, null);
    expect(result.explanation.summary).toBe('Rose ✓ "quoted"');
    const [, init] = fetchMock.mock.calls[0] as unknown as [string, RequestInit];
    expect(init.headers).toEqual({ 'content-type': 'application/json' });
  });
});
