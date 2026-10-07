import type { z } from 'zod/mini';

export class ApiError extends Error {
  status: number;
  /** stable machine-readable code from the `{error, code}` body */
  code: string | null;
  requestId: string | null;
  constructor(status: number, message: string, code: string | null, requestId: string | null) {
    super(message);
    this.status = status;
    this.code = code;
    this.requestId = requestId;
  }
}

async function errorFrom(res: Response): Promise<ApiError> {
  const text = await res.text();
  let message = text || res.statusText;
  let code: string | null = null;
  try {
    const body = JSON.parse(text) as { error?: string; code?: string };
    message = body.error ?? message;
    code = body.code ?? null;
  } catch {
    // non-JSON body (proxy error page): keep the raw text
  }
  const requestId = res.headers.get('x-request-id');
  return new ApiError(
    res.status,
    requestId ? `${message} (request ${requestId})` : message,
    code,
    requestId,
  );
}

/** Upload bodies are CSV or a FHIR R4 JSON document (starts with `{`). */
export function bodyContentType(body: string): string {
  return body.trimStart().startsWith('{') ? 'application/fhir+json' : 'text/csv';
}

/** Fetch `/api/v1{path}` and validate the JSON body against `schema`. */
export async function api<S extends z.ZodMiniType>(
  path: string,
  schema: S,
  init?: RequestInit,
): Promise<z.infer<S>> {
  const body = typeof init?.body === 'string' ? init.body : null;
  const res = await fetch(`/api/v1${path}`, {
    ...init,
    headers: body !== null ? { 'Content-Type': bodyContentType(body) } : undefined,
  });
  if (!res.ok) throw await errorFrom(res);
  const parsed = schema.safeParse(await res.json());
  if (!parsed.success) {
    const issue = parsed.error.issues[0];
    throw new ApiError(
      res.status,
      `Unexpected response from ${path}: ${issue?.path.join('.') || '(root)'} ${issue?.message ?? ''}`.trim(),
      null,
      res.headers.get('x-request-id'),
    );
  }
  return parsed.data;
}
