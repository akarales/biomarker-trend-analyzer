import { useEffect } from 'react';

import { BiomarkerCards } from '@/features/biomarker';
import { TrendPanel } from '@/features/chart';
import { PatientList } from '@/features/patients';
import { UploadPanel } from '@/features/upload';
import { ErrorBanner } from '@/shared/components/ErrorBanner';
import { useAnalyzer } from '@/state';

import { AppHeader } from './AppHeader';

/** Shell only: layout + initial load; features own their content. */
export default function App() {
  const loadPatients = useAnalyzer((s) => s.loadPatients);
  const error = useAnalyzer((s) => s.error);

  useEffect(() => {
    void loadPatients();
  }, [loadPatients]);

  return (
    <div className="flex h-full flex-col">
      <AppHeader />

      <main className="grid flex-1 grid-cols-1 gap-4 overflow-y-auto p-4 lg:grid-cols-[1fr_3fr]">
        <aside className="flex flex-col gap-4">
          <PatientList />
          <UploadPanel />
        </aside>

        <section className="flex flex-col gap-4">
          <ErrorBanner message={error} />
          <BiomarkerCards />
          <TrendPanel />
        </section>
      </main>
    </div>
  );
}
