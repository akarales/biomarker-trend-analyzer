import { useAnalyzer } from '@/state';

import { BiomarkerCard } from './BiomarkerCard';

/** Grid of drift cards for the selected patient. */
export function BiomarkerCards() {
  const reports = useAnalyzer((s) => s.reports);
  const selectedCode = useAnalyzer((s) => s.selectedCode);
  const selectCode = useAnalyzer((s) => s.selectCode);

  return (
    <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
      {reports.map((report) => (
        <BiomarkerCard
          key={report.code}
          report={report}
          selected={selectedCode === report.code}
          onSelect={(code) => void selectCode(code)}
        />
      ))}
    </div>
  );
}
