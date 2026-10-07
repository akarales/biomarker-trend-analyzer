import { useAnalyzer } from '@/state';

import { TriageRow } from './TriageRow';

/** Patients, worst first (status, alerting biomarkers, watch biomarkers). */
export function PatientList() {
  const patients = useAnalyzer((s) => s.patients);
  const selectedPatient = useAnalyzer((s) => s.selectedPatient);
  const selectPatient = useAnalyzer((s) => s.selectPatient);

  return (
    <nav aria-label="Patients" className="flex flex-col gap-2">
      <h2 className="px-1 text-sm font-semibold">
        Patients <span className="font-normal text-muted-foreground">· worst first</span>
      </h2>
      {patients.length === 0 ? (
        <p className="px-1 text-sm text-muted-foreground">No data yet — upload a CSV or FHIR bundle.</p>
      ) : (
        <ul className="flex flex-col gap-1">
          {patients.map((p) => (
            <TriageRow
              key={p.patient_id}
              patient={p}
              selected={p.patient_id === selectedPatient}
              onSelect={(id) => void selectPatient(id)}
            />
          ))}
        </ul>
      )}
    </nav>
  );
}
