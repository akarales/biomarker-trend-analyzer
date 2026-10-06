import { useAnalyzer } from '@/state';

/** Paste-a-CSV upload (enforced header shown above the box). */
export function UploadPanel() {
  const uploadText = useAnalyzer((s) => s.uploadText);
  const uploadMessage = useAnalyzer((s) => s.uploadMessage);
  const setUploadText = useAnalyzer((s) => s.setUploadText);
  const upload = useAnalyzer((s) => s.upload);

  return (
    <section className="rounded-lg border border-line bg-panel p-3">
      <h2 className="mb-2 text-sm font-semibold">Upload lab CSV</h2>
      <p className="mb-2 text-[10px] text-muted">
        patient_id,code,value,unit,taken_at,source
      </p>
      <textarea
        value={uploadText}
        onChange={(e) => setUploadText(e.target.value)}
        rows={5}
        placeholder={'alice,HBA1C,5.7,%,2026-01-15,labs'}
        className="w-full rounded border border-line p-2 font-mono text-[11px]"
      />
      <button
        type="button"
        onClick={() => void upload()}
        disabled={!uploadText.trim()}
        className="mt-2 rounded bg-primary px-3 py-1 text-xs font-medium text-white disabled:opacity-50"
      >
        Upload
      </button>
      {uploadMessage && <p className="mt-2 text-xs text-muted">{uploadMessage}</p>}
    </section>
  );
}
