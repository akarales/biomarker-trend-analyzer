import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
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
    <Card role="region" aria-label="Upload lab results" className="gap-3 py-4">
      <CardHeader className="px-4">
        <CardTitle>
          <h2 className="text-sm">Upload lab results</h2>
        </CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-2 px-4">
        <Label htmlFor="upload-text" className="text-xs font-normal text-muted-foreground">
          CSV: patient_id,code,value,unit,taken_at,source — or a FHIR R4 Bundle (JSON)
        </Label>
        <Textarea
          id="upload-text"
          aria-label="Lab results (CSV or FHIR JSON)"
          value={uploadText}
          onChange={(e) => setUploadText(e.target.value)}
          rows={4}
          placeholder={'EDGE-04,HBA1C,6.1,%,2026-10-01,labs'}
          className="font-mono text-xs"
        />
        <div className="flex items-center gap-3">
          <Button type="button" size="sm" onClick={() => void upload()} disabled={!uploadText.trim()}>
            Upload
          </Button>
          <label className="cursor-pointer text-xs text-muted-foreground underline underline-offset-2 focus-within:ring-2 focus-within:ring-ring">
            Load file…
            <input
              type="file"
              accept=".csv,.json,text/csv,application/json,application/fhir+json"
              className="sr-only"
              onChange={(e) => loadFile(e.target.files?.[0])}
            />
          </label>
        </div>
        {uploadMessage && (
          <p role="status" className="text-xs text-muted-foreground">
            {uploadMessage}
          </p>
        )}
      </CardContent>
    </Card>
  );
}
