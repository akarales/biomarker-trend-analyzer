import { useCallback, useEffect, useState } from 'react';

import { TrendChart } from '@/components/TrendChart';
import { fetchPatients, fetchPatientSummary, fetchSeries, uploadCsv } from '@/api/client';
import type {
  BiomarkerSeries,
  DriftReport,
  PatientSummaryEntry,
} from '@/api/types';

const STATUS_CARD: Record<string, string> = {
  alert: 'border-red-300 bg-red-50',
  watch: 'border-amber-300 bg-amber-50',
  normal: 'border-emerald-300 bg-emerald-50',
};

export default function App() {
  const [patients, setPatients] = useState<PatientSummaryEntry[]>([]);
  const [selectedPatient, setSelectedPatient] = useState<string | null>(null);
  const [reports, setReports] = useState<DriftReport[]>([]);
  const [selectedCode, setSelectedCode] = useState<string | null>(null);
  const [series, setSeries] = useState<BiomarkerSeries | null>(null);
  const [uploadText, setUploadText] = useState('');
  const [uploadMessage, setUploadMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reloadPatients = useCallback(() => {
    fetchPatients()
      .then((body) => {
        setPatients(body.patients);
        if (!selectedPatient && body.patients.length > 0) {
          setSelectedPatient(body.patients[0]?.patient_id ?? null);
        }
      })
      .catch((err) => setError(String(err)));
  }, [selectedPatient]);

  useEffect(() => {
    reloadPatients();
  }, [reloadPatients]);

  useEffect(() => {
    if (!selectedPatient) return;
    let cancelled = false;
    fetchPatientSummary(selectedPatient)
      .then((summary) => {
        if (cancelled) return;
        setReports(summary.reports);
        setSelectedCode(null);
        setSeries(null);
      })
      .catch((err) => setError(String(err)));
    return () => {
      cancelled = true;
    };
  }, [selectedPatient]);

  useEffect(() => {
    if (!selectedPatient || !selectedCode) return;
    let cancelled = false;
    fetchSeries(selectedPatient, selectedCode)
      .then((data) => {
        if (!cancelled) setSeries(data);
      })
      .catch((err) => setError(String(err)));
    return () => {
      cancelled = true;
    };
  }, [selectedPatient, selectedCode]);

  const doUpload = useCallback(() => {
    if (!uploadText.trim()) return;
    setUploadMessage(null);
    setError(null);
    uploadCsv(uploadText)
      .then((result) => {
        setUploadMessage(
          `Inserted ${result.inserted} observations across ${result.patients} patient(s): ${result.biomarkers.join(', ')}` +
            (result.duplicates > 0 ? ` (${result.duplicates} already stored, skipped)` : ''),
        );
        setUploadText('');
        reloadPatients();
      })
      .catch((err) => setUploadMessage(String(err)));
  }, [uploadText, reloadPatients]);

  return (
    <div className="flex h-full flex-col">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b border-line bg-panel px-4 py-3">
        <div>
          <h1 className="text-base font-semibold">Biomarker Trend Analyzer</h1>
          <p className="text-xs text-muted">
            Personal baselines · robust z-scores · Theil–Sen drift · synthetic
            demo data · not medical advice
          </p>
        </div>
      </header>

      <main className="grid flex-1 grid-cols-1 gap-4 overflow-y-auto p-4 lg:grid-cols-[1fr_3fr]">
        <aside className="flex flex-col gap-4">
          <section className="rounded-lg border border-line bg-panel p-3">
            <h2 className="mb-2 text-sm font-semibold">Patients</h2>
            <ul className="flex flex-col gap-1">
              {patients.map((patient) => (
                <li key={patient.patient_id}>
                  <button
                    type="button"
                    onClick={() => setSelectedPatient(patient.patient_id)}
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

          <section className="rounded-lg border border-line bg-panel p-3">
            <h2 className="mb-2 text-sm font-semibold">Upload lab CSV</h2>
            <p className="mb-2 text-[10px] text-muted">
              patient_id,code,value,unit,taken_at,source
            </p>
            <textarea
              value={uploadText}
              onChange={(e) => setUploadText(e.target.value)}
              rows={5}
              placeholder={'alice,HBA1C,5.7,%,2026-01-15,labs'}
              className="w-full rounded border border-line p-2 font-mono text-[11px]"
            />
            <button
              type="button"
              onClick={doUpload}
              disabled={!uploadText.trim()}
              className="mt-2 rounded bg-primary px-3 py-1 text-xs font-medium text-white disabled:opacity-50"
            >
              Upload
            </button>
            {uploadMessage && <p className="mt-2 text-xs text-muted">{uploadMessage}</p>}
          </section>
        </aside>

        <section className="flex flex-col gap-4">
          {error && (
            <p className="rounded border border-red-300 bg-red-50 p-2 text-xs text-red-800">
              {error}
            </p>
          )}

          <div className="grid grid-cols-2 gap-3 md:grid-cols-4">
            {reports.map((report) => (
              <button
                key={report.code}
                type="button"
                onClick={() => setSelectedCode(report.code)}
                className={`rounded-lg border p-3 text-left ${
                  STATUS_CARD[report.status] ?? 'border-line bg-panel'
                } ${selectedCode === report.code ? 'ring-2 ring-primary' : ''}`}
              >
                <p className="text-sm font-semibold">{report.code}</p>
                <p className="text-xs text-muted">
                  {report.unit} · {report.series_len} readings
                </p>
                <p className="mt-1 text-xs">
                  z={report.latest_z?.toFixed(1) ?? '—'} ·{' '}
                  {report.trend ?? '—'}
                </p>
                <p className="mt-1 text-[10px] uppercase tracking-wide">
                  {report.status}
                </p>
              </button>
            ))}
          </div>

          {series && <TrendChart series={series} />}
          {!series && reports.length > 0 && (
            <p className="text-xs text-muted">Select a biomarker card to see its trend.</p>
          )}
        </section>
      </main>
    </div>
  );
}
