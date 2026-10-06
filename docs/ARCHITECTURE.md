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

## Drift engine (crates/drift)

The core IP, built the way laboratory medicine reads serial results.
Pure (no IO, no clock), serde-only, deterministic: the same input gives
the same report whatever day it runs.

```
src/ engine.rs        analyze(): normalise → baseline → detectors → signals → status
     profiles.rs      analyte table: LOINC, UCUM, CVI, CVA, popRI, thresholds — cited, dated
     model.rs         Reading/AnalysisInput in, DriftReport/Signal out (the API wire format)
     detectors/       baseline (prRI) · rcv · control (CUSUM + EWMA) · trend · limits
     stats.rs fmt.rs  Mann–Kendall, Sen slope + CI, normal CDF; dates/values for explanations
```

**Pipeline for one patient × analyte**

1. **Normalise** to the analyte's canonical UCUM unit (HbA1c mmol/mol → %
   by the IFCC/NGSP master equation, creatinine µmol/L → mg/dL, LDL
   mmol/L → mg/dL). Unknown units are excluded and counted.
2. **As of** an explicit instant (`?as_of=`, default: the latest result —
   never the wall clock). Later results are excluded and counted.
3. **Personal baseline**: the *earliest steady-state run* of 3–10 results
   (each inside the prediction interval of the ones before it), never the
   result being judged. Set point = geometric mean; **prRI** = log-normal
   95 % prediction interval `exp(mean ± 1.96·σ·√(1+1/n))` with
   `σ² = ln(1+CVI²) + ln(1+CVA²)` (Coşkun et al., Clin Chem 2021). A
   sustained change stays visible until a clinician re-baselines it
   (review workflow, M6) instead of becoming "the new normal".
4. **Detectors**, each emitting explainable `signals`
   `{rule, severity, t, value, threshold, explanation, source}`:

| Rule | Question | Method | Severity |
|------|----------|--------|----------|
| `prri` | Is the latest result unexpected *for this person*? | outside the prRI | watch > 95 %, alert > 99 % |
| `rcv` | Is the change since the previous result real? | log-normal asymmetric RCV `exp(±z·√2·σ) − 1` (Fokkema 2006) | watch > 95 %, alert > 99 % |
| `shift` | Has the level moved and stayed moved? | tabular CUSUM after the baseline (k 0.5σ, h 5σ, z clipped at ±4 so one bad result cannot alarm); change point = start of the run; signalled when the post-change mean is outside the prRI | watch |
| `ewma` | Is a small persistent change accumulating? | EWMA λ 0.2, L 3, exact time-varying σ (v1's limits were ~2.4× too wide) | watch |
| `trend` | Is there a monotone trend? | Mann–Kendall (p < 0.05) + Sen slope with 95 % CI, over `window_days`; signalled when ≥ CVI per year | watch |
| `threshold` | Which clinical category? | cited decision limits (ADA HbA1c 5.7 / 6.5 %, ATP III / AHA LDL-C, ATA TSH) | per threshold |
| `population` | Outside the population interval? | popRI, only when no threshold covers that side | info (context) |

5. **Status** = worst signal (`info` → `normal`). `not_assessed` lists every
   rule that could not run and why (short history, no profile, no
   thresholds) — "normal" means *no rule fired*, not "healthy".

**Analyte profiles** (`profiles.rs`, reviewed 2026-10-06, not clinically
validated): CVI HbA1c 1.2 % (Adv Lab Med 2020 meta-analysis), LDL-C 7.8 %
(EFLM BV database), TSH 17.7 % (EuBIVAS, CCLM 2022), creatinine 4.4 %
(EuBIVAS, Clin Chem 2017). CVA values are **assumed** typical analyser
imprecision — a deployment must use its laboratory's CVA. Creatinine has
no thresholds yet: KDIGO staging needs eGFR (age/sex), planned with FHIR
Patient import. Codes are matched by short code or LOINC.

What v1 got wrong and the tests that pin it (`tests/scenarios.rs`):
D1 baseline judged the window it was in · D2 wall clock + future points ·
D3 EWMA σ · D4 σ ignoring CVI/CVA · D5 no minimum n · D6 trend without a
test · D7 unexplained status · D8 no units · D9 no population context.
`tests/properties.rs` checks order, time-shift and scale invariance, and
monotone-transform invariance of Mann–Kendall over 200 seeded series.

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
36, bob's LDL rises +0.38/week, carol is the healthy control. With the
v2 engine: alice HbA1c **alert** (prRI + ADA diabetes threshold, shift on
2026-09-15, EWMA, trend); bob LDL **watch** (trend +16 %/yr, EWMA); bob's
creatinine noise **normal**; carol's HbA1c 5.63 % shows the population
interval as *info* only. (Weekly HbA1c is unrealistic — M3 replaces the
fixture with Synthea cadences.)

## Design decisions

| Decision | Why |
|----------|-----|
| Drift crate has zero deps beyond serde | Portable; no IO, no clock — callers pass timestamps and `as_of` |
| Profiles as a typed Rust table, not a CSV | Compile-checked, reviewable in one diff with sources; a CSV parser would break the serde-only rule |
| Population CVI, not the patient's own SD | Works from 3 results and makes limits independent of how noisy a window happens to be (v1's D4); personal-SD models need ≥ 5 steady-state results |
| Earliest steady state as baseline | A change must stay flagged until a clinician accepts it; a sliding baseline silently normalises disease progression |
| Seeded property loops instead of proptest | Keeps the drift crate's lockfile surface at serde only |
| polars here, plain csv+regex in app #1 | Data-shaped workload (scan→filter→group_by) vs one-shot row-wise ETL — the documented decision table in BEST_PRACTICES/RUST_DATA_STACK_2026.md |
| Enum store dispatch | Same API, swappable backends, object-safe without async-trait |
