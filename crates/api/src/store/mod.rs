//! Store abstraction (the ZAP_RUNTIME `store/mod.rs` pattern):
//! `Memory` for tests and quick demos, `Postgres` (sqlx) for runtime.
//! Enum dispatch — no dyn-async plumbing. Holds lab results and the
//! append-only review audit trail.

pub mod memory;
pub mod postgres;

use std::sync::Arc;

use biomarker_ingest::Observation;

use crate::config::StoreBackend;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(String),
}

/// Enum dispatch — no dyn-async plumbing. AppState holds `Arc<Store>`.
pub enum Store {
    Memory(memory::MemoryStore),
    Postgres(postgres::PgStore),
}

impl Store {
    pub async fn connect(
        backend: StoreBackend,
        database_url: Option<&str>,
    ) -> Result<Self, StoreError> {
        match backend {
            StoreBackend::Memory => Ok(Self::Memory(memory::MemoryStore::default())),
            StoreBackend::Postgres => {
                let url = database_url.ok_or_else(|| {
                    StoreError::Database("APP_DATABASE_URL required for postgres store".into())
                })?;
                Ok(Self::Postgres(postgres::PgStore::connect(url).await?))
            }
        }
    }

    pub fn backend_name(&self) -> &'static str {
        match self {
            Store::Memory(_) => "memory",
            Store::Postgres(_) => "postgres",
        }
    }

    /// Idempotent insert: results already stored are reported as duplicates.
    pub async fn insert_observations(
        &self,
        observations: &[Observation],
    ) -> Result<InsertReport, StoreError> {
        match self {
            Store::Memory(store) => store.insert_observations(observations),
            Store::Postgres(store) => store.insert_observations(observations).await,
        }
    }

    /// Observations for one patient+biomarker, ordered by taken_at.
    pub async fn series(
        &self,
        patient_id: &str,
        code: &str,
    ) -> Result<Vec<Observation>, StoreError> {
        match self {
            Store::Memory(store) => store.series(patient_id, code),
            Store::Postgres(store) => store.series(patient_id, code).await,
        }
    }

    /// All (patient_id, code) pairs with counts, for listings.
    pub async fn listing(&self) -> Result<Vec<PatientSummary>, StoreError> {
        match self {
            Store::Memory(store) => store.listing(),
            Store::Postgres(store) => store.listing().await,
        }
    }

    /// Append one review event (never updated or deleted).
    pub async fn append_review(&self, event: NewReviewEvent) -> Result<ReviewEvent, StoreError> {
        match self {
            Store::Memory(store) => store.append_review(event),
            Store::Postgres(store) => store.append_review(event).await,
        }
    }

    /// Review events of a patient (optionally one biomarker), oldest first.
    pub async fn reviews(
        &self,
        patient_id: &str,
        code: Option<&str>,
    ) -> Result<Vec<ReviewEvent>, StoreError> {
        match self {
            Store::Memory(store) => store.reviews(patient_id, code),
            Store::Postgres(store) => store.reviews(patient_id, code).await,
        }
    }

    /// Insert or update demographics (re-importing a Patient corrects them).
    pub async fn upsert_demographics(&self, rows: &[Demographics]) -> Result<(), StoreError> {
        match self {
            Store::Memory(store) => store.upsert_demographics(rows),
            Store::Postgres(store) => store.upsert_demographics(rows).await,
        }
    }

    pub async fn demographics(&self, patient_id: &str) -> Result<Option<Demographics>, StoreError> {
        match self {
            Store::Memory(store) => store.demographics(patient_id),
            Store::Postgres(store) => store.demographics(patient_id).await,
        }
    }

    /// Distinct biomarker codes observed for a patient.
    pub async fn patient_codes(&self, patient_id: &str) -> Result<Vec<String>, StoreError> {
        match self {
            Store::Memory(store) => store.patient_codes(patient_id),
            Store::Postgres(store) => store.patient_codes(patient_id).await,
        }
    }
}

/// Outcome of an idempotent insert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct InsertReport {
    pub inserted: usize,
    /// Rows whose (patient_id, code, taken_at) was already stored (or
    /// repeated within the same batch).
    pub duplicates: usize,
}

impl InsertReport {
    pub fn new(inserted: usize, submitted: usize) -> Self {
        Self {
            inserted,
            duplicates: submitted.saturating_sub(inserted),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PatientSummary {
    pub patient_id: String,
    pub biomarkers: usize,
    pub observations: usize,
}

/// Gender + birth year of a patient (from FHIR Patient; CSV has none).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Demographics {
    pub patient_id: String,
    /// administrative gender as recorded: female, male, other, unknown
    pub gender: Option<String>,
    pub birth_year: Option<i32>,
}

impl Demographics {
    /// The sex the eGFR equation needs (female / male only).
    pub fn sex(&self) -> Option<biomarker_drift::egfr::Sex> {
        match self.gender.as_deref() {
            Some("female") => Some(biomarker_drift::egfr::Sex::Female),
            Some("male") => Some(biomarker_drift::egfr::Sex::Male),
            _ => None,
        }
    }
}

/// What a clinician did with a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReviewAction {
    /// seen and accepted as a real finding (reason optional)
    Acknowledge,
    /// a note; does not change the review state (reason = the note)
    Annotate,
    /// not clinically relevant / not actionable (reason required)
    Dismiss,
    /// back to unreviewed (reason required)
    Reopen,
}

impl ReviewAction {
    pub fn as_str(self) -> &'static str {
        match self {
            ReviewAction::Acknowledge => "acknowledge",
            ReviewAction::Annotate => "annotate",
            ReviewAction::Dismiss => "dismiss",
            ReviewAction::Reopen => "reopen",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        [
            Self::Acknowledge,
            Self::Annotate,
            Self::Dismiss,
            Self::Reopen,
        ]
        .into_iter()
        .find(|a| a.as_str() == s)
    }
}

/// The pseudonymous actor of every review (no login in this demo).
pub const DEMO_ACTOR: &str = "demo-clinician";

#[derive(Debug, Clone, PartialEq)]
pub struct NewReviewEvent {
    pub patient_id: String,
    pub code: String,
    /// drift rule (`prri`, `rcv`, …)
    pub rule: String,
    /// epoch seconds of the result the signal fired on
    pub signal_t: i64,
    pub action: ReviewAction,
    pub reason: Option<String>,
    /// the signal + context as computed server-side at decision time
    pub snapshot: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ReviewEvent {
    pub id: i64,
    pub patient_id: String,
    pub code: String,
    pub rule: String,
    pub signal_t: i64,
    pub action: ReviewAction,
    pub reason: Option<String>,
    pub actor: String,
    pub snapshot: serde_json::Value,
    /// epoch seconds
    pub created_at: i64,
}

/// Shared handle for AppState.
pub type SharedStore = Arc<Store>;
