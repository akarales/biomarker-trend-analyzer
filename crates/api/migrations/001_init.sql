-- Biomarker Trend Analyzer schema.
-- One row per lab result. (patient_id, code, taken_at) identifies a result,
-- so re-seeding or re-uploading the same file is a no-op (ON CONFLICT DO
-- NOTHING) instead of duplicating the series.

CREATE TABLE IF NOT EXISTS observations (
    id BIGSERIAL PRIMARY KEY,
    patient_id TEXT NOT NULL,
    code TEXT NOT NULL,
    value DOUBLE PRECISION NOT NULL,
    unit TEXT NOT NULL,
    taken_at TIMESTAMPTZ NOT NULL,
    source TEXT NOT NULL DEFAULT 'csv',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT observations_result_key UNIQUE (patient_id, code, taken_at)
);
