import { AnalysisControls } from '@/features/controls';

/** Title bar: product, the always-visible notice, analysis options. */
export function AppHeader() {
  return (
    <header className="flex flex-wrap items-end justify-between gap-4 border-b border-border bg-card px-4 py-3">
      <div>
        <h1 className="text-lg font-semibold">Biomarker Trend Analyzer</h1>
        <p className="text-xs text-muted-foreground">
          Personal reference intervals · reference change values · CUSUM/EWMA · Mann–Kendall trends · synthetic demo
          data · not medical advice (clinician review required)
        </p>
      </div>
      <AnalysisControls />
    </header>
  );
}
