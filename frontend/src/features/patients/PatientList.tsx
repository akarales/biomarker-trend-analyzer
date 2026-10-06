import { useAnalyzer } from '@/state';

/** Patient listing with counts; clicking selects and loads the summary. */
export function PatientList() {
  const patients = useAnalyzer((s) => s.patients);
  const selectedPatient = useAnalyzer((s) => s.selectedPatient);
  const selectPatient = useAnalyzer((s) => s.selectPatient);

  return (
    <section className="rounded-lg border border-line bg-panel p-3">
      <h2 className="mb-2 text-sm font-semibold">Patients</h2>
      <ul className="flex flex-col gap-1">
        {patients.map((patient) => (
          <li key={patient.patient_id}>
            <button
              type="button"
              onClick={() => void selectPatient(patient.patient_id)}
              className={`w-full rounded px-2 py-1 text-left text-xs hover:bg-surface ${
                selectedPatient === patient.patient_id ? 'bg-surface font-semibold' : ''
              }`}
            >
              {patient.patient_id}
              <span className="ml-2 text-muted">
                {patient.biomarkers} biomarkers · {patient.observations} readings
              </span>
            </button>
          </li>
        ))}
      </ul>
      {patients.length === 0 && (
        <p className="text-xs text-muted">No data yet — upload a CSV.</p>
      )}
    </section>
  );
}
