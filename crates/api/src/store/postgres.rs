//! Postgres store (sqlx) — the runtime backend. Migrations ship in
//! `migrations/postgres/` and run at startup (ZAP_RUNTIME's
//! `sqlx::migrate!` pattern). Non-macro queries keep CI DB-free; switch to
//! `query!` macros with a committed `.sqlx/` offline cache once the schema
//! stabilizes.

use biomarker_ingest::Observation;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use super::{InsertReport, PatientSummary, StoreError};

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
        Ok(Self::from_pool(pool))
    }

    /// Wrap an already-migrated pool (pg-tests: `#[sqlx::test]`).
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// One statement for the whole batch; rows whose
    /// (patient_id, code, taken_at) already exist are skipped.
    pub async fn insert_observations(
        &self,
        observations: &[Observation],
    ) -> Result<InsertReport, StoreError> {
        let mut patient_ids = Vec::with_capacity(observations.len());
        let mut codes = Vec::with_capacity(observations.len());
        let mut values = Vec::with_capacity(observations.len());
        let mut units = Vec::with_capacity(observations.len());
        let mut taken_ats = Vec::with_capacity(observations.len());
        let mut sources = Vec::with_capacity(observations.len());
        for o in observations {
            patient_ids.push(o.patient_id.clone());
            codes.push(o.code.clone());
            values.push(o.value);
            units.push(o.unit.clone());
            taken_ats.push(o.taken_at.and_utc());
            sources.push(o.source.clone());
        }
        let result = sqlx::query(
            "INSERT INTO observations (patient_id, code, value, unit, taken_at, source) \
             SELECT * FROM UNNEST($1::text[], $2::text[], $3::float8[], $4::text[], \
                                  $5::timestamptz[], $6::text[]) \
             ON CONFLICT ON CONSTRAINT observations_result_key DO NOTHING",
        )
        .bind(&patient_ids)
        .bind(&codes)
        .bind(&values)
        .bind(&units)
        .bind(&taken_ats)
        .bind(&sources)
        .execute(&self.pool)
        .await
        .map_err(|e| StoreError::Database(format!("insert: {e}")))?;
        let inserted = usize::try_from(result.rows_affected()).unwrap_or(usize::MAX);
        Ok(InsertReport::new(inserted, observations.len()))
    }

    pub async fn series(
        &self,
        patient_id: &str,
        code: &str,
    ) -> Result<Vec<Observation>, StoreError> {
        let rows: Vec<(String, String, f64, String, DateTime<Utc>, String)> =
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
                    taken_at: taken_at.naive_utc(),
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
