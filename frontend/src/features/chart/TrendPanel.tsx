import { showTrendPrompt, useAnalyzer } from '@/state';

import { TrendChart } from './TrendChart';

/** The selected biomarker's chart, or a prompt to pick one. */
export function TrendPanel() {
  const series = useAnalyzer((s) => s.series);
  const prompt = useAnalyzer(showTrendPrompt);
  return (
    <>
      {series && <TrendChart series={series} />}
      {prompt && <p className="text-xs text-muted">Select a biomarker card to see its trend.</p>}
    </>
  );
}
