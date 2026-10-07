import { BiomarkerCards } from '@/features/biomarker';
import { TrendPanel } from '@/features/chart';
import { ExplainPanel } from '@/features/explain';
import { PatientList } from '@/features/patients';
import { UploadPanel } from '@/features/upload';
import { ErrorBanner } from '@/shared/components/ErrorBanner';
import { useAnalyzer } from '@/state';

import { AppHeader } from './AppHeader';
import { useUrlSync } from './useUrlSync';

/**
 * Shell only. ≥ lg: triage rail (patients, upload) | workspace. Below lg
 * the DOM order applies: patients (capped height), workspace, upload.
 */
export default function App() {
  useUrlSync();
  const error = useAnalyzer((s) => s.error);

  return (
    <>
      <div className="flex min-h-full flex-col">
        <AppHeader />
        <div className="grid flex-1 grid-cols-1 content-start gap-4 p-4 lg:grid-cols-[20rem_minmax(0,1fr)]">
          <div className="max-h-[45vh] overflow-y-auto pr-1 lg:col-start-1 lg:row-start-1 lg:max-h-none">
            <PatientList />
          </div>
          <main className="flex min-w-0 flex-col gap-4 lg:col-start-2 lg:row-span-2 lg:row-start-1">
            <ErrorBanner message={error} />
            <BiomarkerCards />
            <TrendPanel />
            <ExplainPanel />
          </main>
          <div className="lg:col-start-1 lg:row-start-2 lg:self-start">
            <UploadPanel />
          </div>
        </div>
      </div>
    </>
  );
}
