//! In-memory store — tests and quick demos. No infra required.

use std::collections::BTreeMap;
use std::sync::Mutex;

use biomarker_ingest::Observation;

use super::{InsertReport, PatientSummary, StoreError};

#[derive(Default)]
struct Inner {
    // (patient, code) -> observations sorted by taken_at
    series: BTreeMap<(String, String), Vec<Observation>>,
    // patient -> biomarker codes seen (dedup on insert)
    patients: BTreeMap<String, Vec<String>>,
}

#[derive(Default)]
pub struct MemoryStore {
    inner: Mutex<Inner>,
}

impl MemoryStore {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner.lock().expect("store lock poisoned")
    }

    /// Same contract as Postgres: (patient_id, code, taken_at) is unique;
    /// the first row wins and later ones count as duplicates.
    pub fn insert_observations(
        &self,
        observations: &[Observation],
    ) -> Result<InsertReport, StoreError> {
        let mut guard = self.lock();
        let mut inserted = 0usize;
        for observation in observations {
            let key = (observation.patient_id.clone(), observation.code.clone());
            let series = guard.series.entry(key).or_default();
            if let Err(at) = series.binary_search_by_key(&observation.taken_at, |o| o.taken_at) {
                series.insert(at, observation.clone());
                inserted += 1;
            }
        }
        for observation in observations {
            let codes = guard
                .patients
                .entry(observation.patient_id.clone())
                .or_default();
            if !codes.contains(&observation.code) {
                codes.push(observation.code.clone());
            }
        }
        Ok(InsertReport::new(inserted, observations.len()))
    }

    pub fn series(&self, patient_id: &str, code: &str) -> Result<Vec<Observation>, StoreError> {
        let guard = self.lock();
        Ok(guard
            .series
            .get(&(patient_id.to_string(), code.to_string()))
            .cloned()
            .unwrap_or_default())
    }

    pub fn listing(&self) -> Result<Vec<PatientSummary>, StoreError> {
        let guard = self.lock();
        let mut out: Vec<PatientSummary> = guard
            .patients
            .iter()
            .map(|(patient, codes)| {
                let observations: usize = codes
                    .iter()
                    .map(|code| {
                        guard
                            .series
                            .get(&(patient.clone(), code.clone()))
                            .map(Vec::len)
                            .unwrap_or(0)
                    })
                    .sum();
                PatientSummary {
                    patient_id: patient.clone(),
                    biomarkers: codes.len(),
                    observations,
                }
            })
            .collect();
        out.sort_by(|a, b| a.patient_id.cmp(&b.patient_id));
        Ok(out)
    }

    pub fn patient_codes(&self, patient_id: &str) -> Result<Vec<String>, StoreError> {
        let guard = self.lock();
        let mut codes = guard.patients.get(patient_id).cloned().unwrap_or_default();
        codes.sort();
        Ok(codes)
    }
}
