# Architecture

## Crates

```
biomarker-trend-analyzer/
├── crates/drift/      # biomarker-drift — pure math, no IO, serde only (tests/purity.rs enforces it)
├── crates/ingest/    # biomarker-ingest — polars CSV pipeline
├── crates/api/       # biomarker-api — axum + stores + routes
│   ├── src/routes/   #   one module per resource (patients, biomarkers, observations, health)
│   │                 #   + views.rs: every JSON response shape, typed
│   ├── src/analysis.rs  # Observation → SeriesPoint → drift::analyze (the only call site)
│   ├── src/store/    #   Memory | Postgres (query! macros, .sqlx/ committed)
│   └── migrations/   #   embedded by sqlx::migrate!
└── frontend/src/
    ├── app/          # shell only: App (layout + initial load), AppHeader
    ├── features/     # patients · biomarker · chart · upload — each exposes index.ts
    ├── shared/       # domain (status, tokens), components (ErrorBanner)
    ├── state/        # one zustand store from slices (patients, biomarker, upload) + selectors
    └── api/          # http (zod/mini-validated) + per-resource clients + schemas.ts
```

Frontend rules are enforced by `src/test/architecture.test.ts`: ≤ ~300
lines per file, hex colours only in `shared/domain/tokens.ts`, features
import each other only through `index.ts`, and `api/`, `state/`, `shared/`
never import `features/` or `app/`.

## Drift math (crates/drift)

The core IP — deliberately dependency-free so it can run anywhere
observations come from:

- **Baseline** — median + MAD over the lookback window; robust σ =
  `1.4826 · MAD`, floored at 1e-9 so flat series stay usable. Falls back
  to the whole series when the window is empty.
- **z-score** of the latest reading against the personal baseline —
  catches **step changes**.
- **EWMA** (α = 0.3) control value + its own z — smoothed confirmation.
- **Theil–Sen slope** — median of pairwise slopes, expressed per day —
  catches **gradual drift**. Robust to outliers by construction.
- **Status** — `alert` (|z| ≥ 3), `watch` (|z| ≥ 2 or any window anomaly),
  `normal`.

Detector semantics (a deliberate design, covered by tests):

| Drift shape | Detector | Why |
|------------|----------|-----|
| Sudden step | z-score / EWMA | Theil–Sen is robust to steps by design — the median pairwise slope stays flat |
| Gradual trend | Theil–Sen | z-scores normalize away slow consistent change once the baseline absorbs it |

`window_days` is the sensitivity knob: with a 365-day window alice's
injected HBA1C step-up alerts immediately; with 90 days the step
partially becomes the new normal (both behaviors asserted in tests).

## Ingestion (crates/ingest)

polars with an **enforced schema** — but polars 0.55 applies schemas
**positionally** when headers mismatch, so `validate_header` checks the
header row explicitly first and returns a schema-mismatch error naming
expected vs. got columns. Rows are then validated in order with the
offending column named in every error. `biomarker_stats` uses the lazy
engine (scan → group_by → one collect) for batch files.

## Stores (crates/api/src/store)

Enum dispatch (no dyn-async plumbing), the ZAP Runtime `store/mod.rs`
pattern:

- `MemoryStore` — Mutex-guarded BTreeMaps; tests and the offline demo
- `PgStore` — sqlx pool, migrations run at startup
  (`sqlx::migrate!`); compile-time-checked `query!` macros against the
  committed `.sqlx/` metadata (`SQLX_OFFLINE=true`), so builds and CI
  need no database; a CI job re-checks the macros against a live schema

Inserts are idempotent in both backends: `(patient_id, code, taken_at)`
identifies a result (Postgres: unique constraint + `ON CONFLICT DO
NOTHING`, one `UNNEST` statement per batch), so re-seeding on restart or
re-uploading a file reports duplicates instead of doubling the series.
Store errors are logged and returned as a generic `internal` error — SQL
text never reaches clients.

## Seeded demo

Synthetic generator (deterministic): 3 patients × 4 biomarkers (HBA1C,
LDL, TSH, CREAT) × 48 weekly readings — alice's HBA1C steps up at week
36, bob's LDL rises +0.38/week, carol is the healthy control. The demo
proves both detector paths on first load.

## Design decisions

| Decision | Why |
|----------|-----|
| Drift crate has zero deps beyond serde | Portable; no IO, no clock — callers pass timestamps and "now" |
| polars here, plain csv+regex in app #1 | Data-shaped workload (scan→filter→group_by) vs one-shot row-wise ETL — the documented decision table in BEST_PRACTICES/RUST_DATA_STACK_2026.md |
| Enum store dispatch | Same API, swappable backends, object-safe without async-trait |
