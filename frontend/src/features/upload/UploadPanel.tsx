import { useAnalyzer } from '@/state';

/** Paste or load lab results: CSV (enforced header) or a FHIR R4 Bundle. */
export function UploadPanel() {
  const uploadText = useAnalyzer((s) => s.uploadText);
  const uploadMessage = useAnalyzer((s) => s.uploadMessage);
  const setUploadText = useAnalyzer((s) => s.setUploadText);
  const upload = useAnalyzer((s) => s.upload);

  const loadFile = (file: File | undefined) => {
    if (file) void file.text().then(setUploadText);
  };

  return (
    <section className="rounded-lg border border-line bg-panel p-3">
      <h2 className="mb-2 text-sm font-semibold">Upload lab results</h2>
      <p className="mb-2 text-[10px] text-muted">
        CSV: patient_id,code,value,unit,taken_at,source — or a FHIR R4 Bundle (JSON)
      </p>
      <textarea
        aria-label="Lab results (CSV or FHIR JSON)"
        value={uploadText}
        onChange={(e) => setUploadText(e.target.value)}
        rows={5}
        placeholder={'EDGE-04,HBA1C,6.1,%,2026-10-01,labs'}
        className="w-full rounded border border-line p-2 font-mono text-[11px]"
      />
      <div className="mt-2 flex items-center gap-2">
        <button
          type="button"
          onClick={() => void upload()}
          disabled={!uploadText.trim()}
          className="rounded bg-primary px-3 py-1 text-xs font-medium text-white disabled:opacity-50"
        >
          Upload
        </button>
        <label className="cursor-pointer text-xs text-muted underline">
          Load file…
          <input
            type="file"
            accept=".csv,.json,text/csv,application/json,application/fhir+json"
            className="sr-only"
            onChange={(e) => loadFile(e.target.files?.[0])}
          />
        </label>
      </div>
      {uploadMessage && <p className="mt-2 text-xs text-muted">{uploadMessage}</p>}
    </section>
  );
}
