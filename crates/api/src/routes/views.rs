//! JSON response shapes. Handlers build these typed views; nothing else in
//! the routes decides field names, so the wire contract lives in one file
//! (mirrored by `frontend/src/api/schemas.ts`).

use std::collections::{BTreeMap, BTreeSet};

use biomarker_drift::DriftReport;
use biomarker_ingest::Observation;
use serde::Serialize;

use crate::import::{Format, Parsed, SkipReason};
use crate::reports::Derived;
use crate::review::SignalReview;
use crate::store::{Demographics, InsertReport, PatientSummary, ReviewEvent};
use crate::triage::Triage;

#[derive(Serialize)]
pub struct HealthView {
    pub status: &'static str,
    pub version: &'static str,
    pub store: &'static str,
}

/// A listing row: counts + triage, flattened into one JSON object.
#[derive(Serialize)]
pub struct PatientTriageView {
    #[serde(flatten)]
    pub entry: PatientSummary,
    #[serde(flatten)]
    pub triage: Triage,
}

#[derive(Serialize)]
pub struct PatientsView {
    pub as_of: Option<i64>,
    pub window_days: i64,
    /// worst first
    pub patients: Vec<PatientTriageView>,
}

#[derive(Serialize)]
pub struct SummaryView {
    pub patient_id: String,
    /// requested as-of (epoch seconds); null = each series' latest result
    pub as_of: Option<i64>,
    pub window_days: i64,
    pub reports: Vec<DriftReport>,
    /// review status per biomarker code, aligned with each report's `signals`
    pub reviews: BTreeMap<String, Vec<SignalReview>>,
    /// how each derived series (e.g. EGFR) was computed, by code
    pub derived: BTreeMap<String, Derived>,
    /// gender + birth year from FHIR Patient (null for CSV-only patients)
    pub demographics: Option<Demographics>,
}

/// A stored observation as recorded (original unit; the report's `points`
/// hold the normalised values).
#[derive(Serialize)]
pub struct ObservationView {
    /// RFC 3339, UTC (`2026-01-06T00:00:00Z`) — unambiguous in any browser
    pub taken_at: String,
    pub value: f64,
    pub unit: String,
    pub source: String,
}

impl From<&Observation> for ObservationView {
    fn from(o: &Observation) -> Self {
        Self {
            taken_at: o
                .taken_at
                .and_utc()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
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
    /// set for a derived series; `observations` are then its inputs
    pub derived: Option<Derived>,
    /// review status of each signal (same order as `report.signals`)
    pub reviews: Vec<SignalReview>,
    /// the biomarker's review audit trail, newest first
    pub history: Vec<ReviewEvent>,
}

/// Response of `POST …/reviews`.
#[derive(Serialize)]
pub struct ReviewCreatedView {
    pub event: ReviewEvent,
    pub review: SignalReview,
}

#[derive(Serialize)]
pub struct HistoryView {
    pub patient_id: String,
    pub code: String,
    /// newest first
    pub history: Vec<ReviewEvent>,
}

#[derive(Serialize)]
pub struct UploadView {
    pub format: Format,
    pub inserted: usize,
    pub duplicates: usize,
    /// resources/rows not imported (FHIR: not final, no LOINC, no profile, …)
    pub skipped: usize,
    /// grouped reasons, most frequent first
    pub skipped_reasons: Vec<SkipReason>,
    pub patients: usize,
    pub biomarkers: BTreeSet<String>,
    /// FHIR Patient demographics stored (gender + birth year, for eGFR)
    pub demographics: usize,
}

impl UploadView {
    pub fn new(report: InsertReport, parsed: &Parsed) -> Self {
        let observations = &parsed.observations;
        let patients: BTreeSet<&str> = observations.iter().map(|o| o.patient_id.as_str()).collect();
        Self {
            format: parsed.format,
            inserted: report.inserted,
            duplicates: report.duplicates,
            skipped: parsed.skipped_total(),
            skipped_reasons: parsed.skipped.clone(),
            patients: patients.len(),
            biomarkers: observations.iter().map(|o| o.code.clone()).collect(),
            demographics: parsed.patients.len(),
        }
    }
}
