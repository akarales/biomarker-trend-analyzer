-- Demographics needed to estimate kidney function (2021 CKD-EPI eGFR):
-- administrative gender and birth YEAR only, from FHIR Patient resources.
-- No names, addresses, identifiers or full birth dates are stored.

CREATE TABLE patient_demographics (
    patient_id TEXT PRIMARY KEY,
    gender TEXT CHECK (gender IN ('female', 'male', 'other', 'unknown')),
    birth_year INT CHECK (birth_year BETWEEN 1900 AND 2100),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
