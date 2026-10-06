/** Title bar with the always-visible demo / not-medical-advice notice. */
export function AppHeader() {
  return (
    <header className="flex flex-wrap items-center justify-between gap-3 border-b border-line bg-panel px-4 py-3">
      <div>
        <h1 className="text-base font-semibold">Biomarker Trend Analyzer</h1>
        <p className="text-xs text-muted">
          Personal baselines · robust z-scores · Theil–Sen drift · synthetic
          demo data · not medical advice
        </p>
      </div>
    </header>
  );
}
