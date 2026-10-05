//! Postgres store (sqlx) — the runtime backend. Migrations ship in
//! `migrations/postgres/` and run at startup (ZAP_RUNTIME's
//! `sqlx::migrate!` pattern). Non-macro queries keep CI DB-free; switch to
//! `query!` macros with a committed `.sqlx/` offline cache once the schema
//! stabilizes.

use biomarker_ingest::Observation;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use super::{PatientSummary, StoreError};

pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(|e| StoreError::Database(format!("connect: {e}")))?;
        sqlx::migrate!("../../migrations/postgres")
            .run(&pool)
            .await
            .map_err(|e| StoreError::Database(format!("migrate: {e}")))?;
        Ok(Self { pool })
    }

    pub async fn insert_observations(
        &self,
        observations: &[Observation],
    ) -> Result<usize, StoreError> {
        let mut inserted = 0usize;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StoreError::Database(format!("begin: {e}")))?;
        for observation in observations {
            sqlx::query(
                "INSERT INTO observations (patient_id, code, value, unit, taken_at, source) \
                 VALUES ($1, $2, $3, $4, $5, $6)",
            )
            .bind(&observation.patient_id)
            .bind(&observation.code)
            .bind(observation.value)
            .bind(&observation.unit)
            .bind(observation.taken_at.and_utc())
            .bind(&observation.source)
            .execute(&mut *tx)
            .await
            .map_err(|e| StoreError::Database(format!("insert: {e}")))?;
            inserted += 1;
        }
        tx.commit()
            .await
            .map_err(|e| StoreError::Database(format!("commit: {e}")))?;
        Ok(inserted)
    }

    pub async fn series(
        &self,
        patient_id: &str,
        code: &str,
    ) -> Result<Vec<Observation>, StoreError> {
        let rows: Vec<(String, String, f64, String, chrono::NaiveDateTime, String)> =
            sqlx::query_as::<sqlx::Postgres, _>(
                "SELECT patient_id, code, value, unit, taken_at, source \
                 FROM observations WHERE patient_id = $1 AND code = $2 \
                 ORDER BY taken_at ASC",
            )
            .bind(patient_id)
            .bind(code)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StoreError::Database(format!("select: {e}")))?;

        Ok(rows
            .into_iter()
            .map(
                |(patient_id, code, value, unit, taken_at, source)| Observation {
                    patient_id,
                    code,
                    value,
                    unit,
                    taken_at,
                    source,
                },
            )
            .collect())
    }

    pub async fn listing(&self) -> Result<Vec<PatientSummary>, StoreError> {
        let rows: Vec<(String, i64, i64)> = sqlx::query_as::<sqlx::Postgres, _>(
            "SELECT patient_id, COUNT(DISTINCT code) AS biomarkers, COUNT(*) AS observations \
                 FROM observations GROUP BY patient_id ORDER BY patient_id",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StoreError::Database(format!("listing: {e}")))?;

        Ok(rows
            .into_iter()
            .map(|(patient_id, biomarkers, observations)| PatientSummary {
                patient_id,
                biomarkers: biomarkers.max(0) as usize,
                observations: observations.max(0) as usize,
            })
            .collect())
    }

    pub async fn patient_codes(&self, patient_id: &str) -> Result<Vec<String>, StoreError> {
        let rows: Vec<(String,)> = sqlx::query_as::<sqlx::Postgres, _>(
            "SELECT DISTINCT code FROM observations WHERE patient_id = $1 ORDER BY code",
        )
        .bind(patient_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StoreError::Database(format!("patient_codes: {e}")))?;
        Ok(rows.into_iter().map(|(code,)| code).collect())
    }
}
