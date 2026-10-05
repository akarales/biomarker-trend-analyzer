//! Store abstraction (the ZAP_RUNTIME `store/mod.rs` pattern):
//! `Memory` for tests and quick demos, `Postgres` (sqlx) for runtime.
//! Enum dispatch — no dyn-async plumbing.

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

    pub async fn insert_observations(
        &self,
        observations: &[Observation],
    ) -> Result<usize, StoreError> {
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

    /// Distinct biomarker codes observed for a patient.
    pub async fn patient_codes(&self, patient_id: &str) -> Result<Vec<String>, StoreError> {
        match self {
            Store::Memory(store) => store.patient_codes(patient_id),
            Store::Postgres(store) => store.patient_codes(patient_id).await,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PatientSummary {
    pub patient_id: String,
    pub biomarkers: usize,
    pub observations: usize,
}

/// Shared handle for AppState.
pub type SharedStore = Arc<Store>;
