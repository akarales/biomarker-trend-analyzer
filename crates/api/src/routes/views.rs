//! JSON response shapes. Handlers build these typed views; nothing else in
//! the routes decides field names, so the wire contract lives in one file
//! (mirrored by `frontend/src/api/schemas.ts`).

use std::collections::BTreeSet;

use biomarker_drift::DriftReport;
use biomarker_ingest::Observation;
use serde::Serialize;

use crate::store::{InsertReport, PatientSummary};

#[derive(Serialize)]
pub struct HealthView {
    pub status: &'static str,
    pub version: &'static str,
    pub store: &'static str,
}

#[derive(Serialize)]
pub struct PatientsView {
    pub patients: Vec<PatientSummary>,
}

#[derive(Serialize)]
pub struct SummaryView {
    pub patient_id: String,
    pub window_days: i64,
    pub reports: Vec<DriftReport>,
}

#[derive(Serialize)]
pub struct ObservationView {
    /// `YYYY-MM-DD HH:MM:SS` (naive UTC), as in v1.
    pub taken_at: String,
    pub value: f64,
    pub unit: String,
    pub source: String,
}

impl From<&Observation> for ObservationView {
    fn from(o: &Observation) -> Self {
        Self {
            taken_at: o.taken_at.to_string(),
            value: o.value,
            unit: o.unit.clone(),
            source: o.source.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct SeriesView {
    pub patient_id: String,
    pub code: String,
    pub observations: Vec<ObservationView>,
    pub report: DriftReport,
}

#[derive(Serialize)]
pub struct UploadView {
    pub inserted: usize,
    pub duplicates: usize,
    pub patients: usize,
    pub biomarkers: BTreeSet<String>,
}

impl UploadView {
    pub fn new(report: InsertReport, observations: &[Observation]) -> Self {
        let patients: BTreeSet<&str> = observations.iter().map(|o| o.patient_id.as_str()).collect();
        Self {
            inserted: report.inserted,
            duplicates: report.duplicates,
            patients: patients.len(),
            biomarkers: observations.iter().map(|o| o.code.clone()).collect(),
        }
    }
}
