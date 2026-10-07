import { selectedEntry, useAnalyzer } from '@/state';

import { BiomarkerCard } from './BiomarkerCard';

/** The selected patient's biomarkers. */
export function BiomarkerCards() {
  const reports = useAnalyzer((s) => s.reports);
  const selectedCode = useAnalyzer((s) => s.selectedCode);
  const selectCode = useAnalyzer((s) => s.selectCode);
  const entry = useAnalyzer(selectedEntry);
  if (reports.length === 0) return null;

  return (
    <section aria-label="Biomarkers" className="flex flex-col gap-2">
      <h2 className="text-sm font-semibold">
        {entry?.patient_id ?? 'Patient'} <span className="font-normal text-muted-foreground">· {reports.length} biomarkers</span>
      </h2>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 2xl:grid-cols-3">
        {reports.map((report) => (
          <BiomarkerCard
            key={report.code}
            report={report}
            selected={selectedCode === report.code}
            onSelect={(code) => void selectCode(code)}
          />
        ))}
      </div>
    </section>
  );
}
