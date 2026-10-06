import { afterEach, describe, expect, it, vi } from 'vitest';
import { z } from 'zod/mini';

import { api, ApiError } from './http';

function respond(status: number, body: string, headers: Record<string, string> = {}) {
  vi.stubGlobal('fetch', vi.fn(async () => new Response(body, { status, headers })));
}

afterEach(() => vi.unstubAllGlobals());

describe('api()', () => {
  const Schema = z.object({ ok: z.boolean() });

  it('returns the validated body', async () => {
    respond(200, '{"ok":true,"extra":1}');
    await expect(api('/x', Schema)).resolves.toEqual({ ok: true });
    expect(fetch).toHaveBeenCalledWith('/api/v1/x', expect.objectContaining({ headers: undefined }));
  });

  it('sends CSV bodies as text/csv', async () => {
    respond(201, '{"ok":true}');
    await api('/x', Schema, { method: 'POST', body: 'a,b' });
    expect(fetch).toHaveBeenCalledWith(
      '/api/v1/x',
      expect.objectContaining({ method: 'POST', headers: { 'Content-Type': 'text/csv' } }),
    );
  });

  it('parses {error, code} and keeps the request id', async () => {
    respond(404, '{"error":"unknown patient or biomarker: ghost","code":"not_found"}', {
      'x-request-id': 'abc-1',
    });
    const err = await api('/x', Schema).catch((e: unknown) => e);
    expect(err).toBeInstanceOf(ApiError);
    expect(err).toMatchObject({ status: 404, code: 'not_found', requestId: 'abc-1' });
    expect(String(err)).toBe('Error: unknown patient or biomarker: ghost (request abc-1)');
  });

  it('keeps a non-JSON error body as the message', async () => {
    respond(502, 'Bad Gateway');
    await expect(api('/x', Schema)).rejects.toMatchObject({ status: 502, code: null, message: 'Bad Gateway' });
  });

  it('fails loudly when the response does not match the schema', async () => {
    respond(200, '{"ok":"yes"}');
    await expect(api('/x', Schema)).rejects.toThrow(/Unexpected response from \/x: ok/);
  });
});
