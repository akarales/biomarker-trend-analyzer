import { useEffect } from 'react';

import { readUrlState, useAnalyzer, writeUrlState } from '@/state';

/**
 * Initial load from the address bar (`?patient=&code=&as_of=&window=`),
 * then keep the address bar in sync (replaceState — no history spam).
 */
export function useUrlSync() {
  useEffect(() => {
    const initial = readUrlState(window.location.search);
    useAnalyzer.setState({ asOf: initial.asOf, windowDays: initial.windowDays });
    void useAnalyzer.getState().loadPatients(initial.patient, initial.code);

    return useAnalyzer.subscribe((s) => {
      const search = writeUrlState({
        patient: s.selectedPatient,
        code: s.selectedCode,
        asOf: s.asOf,
        windowDays: s.windowDays,
      });
      if (search !== window.location.search) {
        window.history.replaceState(null, '', `${window.location.pathname}${search}`);
      }
    });
  }, []);
}
