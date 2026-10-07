//! Postgres store (sqlx) — the runtime backend. Migrations ship in
//! `crates/api/migrations/` and run at startup (`sqlx::migrate!`).
//! Queries are compile-time checked (`query!`) against the committed
//! `.sqlx/` metadata, so builds and CI need no database
//! (`SQLX_OFFLINE=true` in `.cargo/config.toml`).

use biomarker_ingest::Observation;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use super::{InsertReport, NewReviewEvent, PatientSummary, ReviewAction, ReviewEvent, StoreError};

pub struct PgStore {
    pool: PgPool,
}

fn db(context: &'static str) -> impl FnOnce(sqlx::Error) -> StoreError {
    move |e| StoreError::Database(format!("{context}: {e}"))
}

impl PgStore {
    pub async fn connect(url: &str) -> Result<Self, StoreError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .map_err(db("connect"))?;
        sqlx::migrate!("./migrations")
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
        let mut taken_ats: Vec<DateTime<Utc>> = Vec::with_capacity(observations.len());
        let mut sources = Vec::with_capacity(observations.len());
        for o in observations {
            patient_ids.push(o.patient_id.clone());
            codes.push(o.code.clone());
            values.push(o.value);
            units.push(o.unit.clone());
            taken_ats.push(o.taken_at.and_utc());
            sources.push(o.source.clone());
        }
        let result = sqlx::query!(
            "INSERT INTO observations (patient_id, code, value, unit, taken_at, source) \
             SELECT * FROM UNNEST($1::text[], $2::text[], $3::float8[], $4::text[], \
                                  $5::timestamptz[], $6::text[]) \
             ON CONFLICT ON CONSTRAINT observations_result_key DO NOTHING",
            &patient_ids,
            &codes,
            &values,
            &units,
            &taken_ats,
            &sources,
        )
        .execute(&self.pool)
        .await
        .map_err(db("insert"))?;
        let inserted = usize::try_from(result.rows_affected()).unwrap_or(usize::MAX);
        Ok(InsertReport::new(inserted, observations.len()))
    }

    pub async fn series(
        &self,
        patient_id: &str,
        code: &str,
    ) -> Result<Vec<Observation>, StoreError> {
        let rows = sqlx::query!(
            "SELECT patient_id, code, value, unit, taken_at, source \
             FROM observations WHERE patient_id = $1 AND code = $2 \
             ORDER BY taken_at ASC",
            patient_id,
            code,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db("select"))?;

        Ok(rows
            .into_iter()
            .map(|r| Observation {
                patient_id: r.patient_id,
                code: r.code,
                value: r.value,
                unit: r.unit,
                taken_at: r.taken_at.naive_utc(),
                source: r.source,
            })
            .collect())
    }

    pub async fn listing(&self) -> Result<Vec<PatientSummary>, StoreError> {
        let rows = sqlx::query!(
            r#"SELECT patient_id,
                      COUNT(DISTINCT code) AS "biomarkers!",
                      COUNT(*) AS "observations!"
               FROM observations GROUP BY patient_id ORDER BY patient_id"#
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db("listing"))?;

        Ok(rows
            .into_iter()
            .map(|r| PatientSummary {
                patient_id: r.patient_id,
                biomarkers: usize::try_from(r.biomarkers).unwrap_or(0),
                observations: usize::try_from(r.observations).unwrap_or(0),
            })
            .collect())
    }

    pub async fn patient_codes(&self, patient_id: &str) -> Result<Vec<String>, StoreError> {
        let rows = sqlx::query!(
            "SELECT DISTINCT code FROM observations WHERE patient_id = $1 ORDER BY code",
            patient_id,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db("patient_codes"))?;
        Ok(rows.into_iter().map(|r| r.code).collect())
    }

    /// INSERT only — the table's trigger rejects UPDATE/DELETE/TRUNCATE.
    pub async fn append_review(&self, e: NewReviewEvent) -> Result<ReviewEvent, StoreError> {
        let signal_t = DateTime::<Utc>::from_timestamp(e.signal_t, 0)
            .ok_or_else(|| StoreError::Database("append_review: signal_t out of range".into()))?;
        let row = sqlx::query!(
            r#"INSERT INTO review_events (patient_id, code, rule, signal_t, action, reason, snapshot)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id, actor, created_at"#,
            e.patient_id,
            e.code,
            e.rule,
            signal_t,
            e.action.as_str(),
            e.reason,
            e.snapshot,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(db("append_review"))?;
        Ok(ReviewEvent {
            id: row.id,
            patient_id: e.patient_id,
            code: e.code,
            rule: e.rule,
            signal_t: e.signal_t,
            action: e.action,
            reason: e.reason,
            actor: row.actor,
            snapshot: e.snapshot,
            created_at: row.created_at.timestamp(),
        })
    }

    pub async fn reviews(
        &self,
        patient_id: &str,
        code: Option<&str>,
    ) -> Result<Vec<ReviewEvent>, StoreError> {
        let rows = sqlx::query!(
            r#"SELECT id, patient_id, code, rule, signal_t, action, reason, actor, snapshot, created_at
               FROM review_events
               WHERE patient_id = $1 AND ($2::text IS NULL OR code = $2)
               ORDER BY id"#,
            patient_id,
            code,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(db("reviews"))?;
        rows.into_iter()
            .map(|r| {
                let action = ReviewAction::parse(&r.action).ok_or_else(|| {
                    StoreError::Database(format!("reviews: unknown action {}", r.action))
                })?;
                Ok(ReviewEvent {
                    id: r.id,
                    patient_id: r.patient_id,
                    code: r.code,
                    rule: r.rule,
                    signal_t: r.signal_t.timestamp(),
                    action,
                    reason: r.reason,
                    actor: r.actor,
                    snapshot: r.snapshot,
                    created_at: r.created_at.timestamp(),
                })
            })
            .collect()
    }
}
