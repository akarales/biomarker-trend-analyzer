import { api } from './http';
import { UploadResultSchema, type UploadResult } from './schemas';

/** POST a CSV body; idempotent server-side (duplicates are reported, not stored). */
export function uploadCsv(csv: string): Promise<UploadResult> {
  return api('/observations', UploadResultSchema, { method: 'POST', body: csv });
}
