-- Biomarker Trend Analyzer schema.
-- pgvector is enabled here (compose runs the pgvector image) for the
-- roadmap feature: embedding patient biomarker trajectories to find
-- similar patients. No vector columns yet — introduced with that feature.

CREATE EXTENSION IF NOT EXISTS vector;

CREATE TABLE IF NOT EXISTS observations (
    id BIGSERIAL PRIMARY KEY,
    patient_id TEXT NOT NULL,
    code TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    unit TEXT NOT NULL,
    taken_at TIMESTAMPTZ NOT NULL,
    source TEXT NOT NULL DEFAULT 'csv',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_obs_patient_code_time
    ON observations (patient_id, code, taken_at);
