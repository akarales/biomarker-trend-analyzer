//! In-memory store — tests and quick demos. No infra required.

use std::collections::BTreeMap;
use std::sync::Mutex;

use biomarker_ingest::Observation;

use super::{
    DEMO_ACTOR, Demographics, InsertReport, NewReviewEvent, PatientSummary, ReviewEvent, StoreError,
};

#[derive(Default)]
struct Inner {
    // (patient, code) -> observations sorted by taken_at
    series: BTreeMap<(String, String), Vec<Observation>>,
    // patient -> biomarker codes seen (dedup on insert)
    patients: BTreeMap<String, Vec<String>>,
    // append-only, in insertion (id) order — no method mutates or removes
    reviews: Vec<ReviewEvent>,
    demographics: BTreeMap<String, Demographics>,
}

#[derive(Default)]
pub struct MemoryStore {
    inner: Mutex<Inner>,
}

impl MemoryStore {
    /// Append-only like the Postgres trigger: there is no update or delete.
    pub fn append_review(&self, event: NewReviewEvent) -> Result<ReviewEvent, StoreError> {
        let mut guard = self.lock();
        let stored = ReviewEvent {
            id: guard.reviews.len() as i64 + 1,
            patient_id: event.patient_id,
            code: event.code,
            rule: event.rule,
            signal_t: event.signal_t,
            action: event.action,
            reason: event.reason,
            actor: DEMO_ACTOR.to_string(),
            snapshot: event.snapshot,
            created_at: chrono::Utc::now().timestamp(),
        };
        guard.reviews.push(stored.clone());
        Ok(stored)
    }

    pub fn reviews(
        &self,
        patient_id: &str,
        code: Option<&str>,
    ) -> Result<Vec<ReviewEvent>, StoreError> {
        Ok(self
            .lock()
            .reviews
            .iter()
            .filter(|e| e.patient_id == patient_id && code.is_none_or(|c| e.code == c))
            .cloned()
            .collect())
    }

    pub fn upsert_demographics(&self, rows: &[Demographics]) -> Result<(), StoreError> {
        let mut guard = self.lock();
        for row in rows {
            guard
                .demographics
                .insert(row.patient_id.clone(), row.clone());
        }
        Ok(())
    }

    pub fn demographics(&self, patient_id: &str) -> Result<Option<Demographics>, StoreError> {
        Ok(self.lock().demographics.get(patient_id).cloned())
    }

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
