-- Clinician review of drift signals: an append-only audit trail.
-- A signal is identified by (patient_id, code, rule, signal_t) — signal_t is
-- the result the signal fired on, so a new result yields a new, unreviewed
-- signal. Synthetic demo data only; the actor is a pseudonym (no login).

CREATE TABLE review_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    patient_id TEXT NOT NULL,
    code TEXT NOT NULL,
    rule TEXT NOT NULL
        CHECK (rule IN ('prri', 'rcv', 'shift', 'ewma', 'trend', 'threshold', 'population')),
    signal_t TIMESTAMPTZ NOT NULL,
    action TEXT NOT NULL CHECK (action IN ('acknowledge', 'annotate', 'dismiss', 'reopen')),
    -- required for every action except acknowledge
    reason TEXT CHECK (reason IS NULL OR char_length(reason) BETWEEN 3 AND 500),
    actor TEXT NOT NULL DEFAULT 'demo-clinician',
    -- the signal and its context as computed by the server at decision time
    snapshot JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT review_events_reason_required CHECK (action = 'acknowledge' OR reason IS NOT NULL)
);

CREATE INDEX review_events_by_patient ON review_events (patient_id, code, id);

-- Append-only: an audit trail that can be edited is not an audit trail.
CREATE FUNCTION review_events_append_only() RETURNS trigger
LANGUAGE plpgsql AS $$
BEGIN
    RAISE EXCEPTION 'review_events is append-only (% rejected)', TG_OP
        USING ERRCODE = 'insufficient_privilege';
END
$$;

CREATE TRIGGER review_events_no_update_delete
    BEFORE UPDATE OR DELETE ON review_events
    FOR EACH ROW EXECUTE FUNCTION review_events_append_only();

CREATE TRIGGER review_events_no_truncate
    BEFORE TRUNCATE ON review_events
    FOR EACH STATEMENT EXECUTE FUNCTION review_events_append_only();
