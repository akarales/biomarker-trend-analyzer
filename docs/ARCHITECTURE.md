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
    ├── features/     # patients (triage) · biomarker · chart · controls · upload — each exposes index.ts
    ├── shared/       # domain (status, tokens), components (ErrorBanner)
    ├── state/        # one zustand store from slices (patients, biomarker, upload) + selectors
    └── api/          # http (zod/mini-validated) + per-resource clients + schemas.ts
```

**Clinician workspace.** Triage rail (patients worst first, with the
signal that put them there) → biomarker cards (latest value, personal vs
population range, trend, deciding rule) → trend panel. The trend panel
has chart and table tabs: axes, personal and population bands,
thresholds, RCV jump segments, change point, hover readout, and keyboard
navigation with a live readout. Below it the signals are listed with
their sources and the not-assessed reasons. As-of date and trend window
apply to every view and live in the URL (`?patient=&code=&as_of=&window=`).
The house style is app #1's dark "Vitals" shadcn theme. axe reports no
serious or critical WCAG 2.2 AA violation in any tested state, mobile
included (`e2e/a11y.spec.ts`).

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
imprecision — a deployment must use its laboratory's CVA. Codes are
matched by short code or LOINC. Creatinine itself has no thresholds;
kidney function is staged on the derived eGFR (below).

**Derived eGFR** (`egfr.rs` in the drift crate, `crates/api/src/reports.rs`).
When a patient has creatinine results and a FHIR Patient with sex
(female/male) and birth year, every creatinine result also yields an eGFR
through the race-free 2021 CKD-EPI creatinine equation (Inker et al.,
NEJM 2021; LOINC 98979-8). The `EGFR` series is analysed like any other
analyte:

- **Variation:** CVI 5.3 % and CVA 2.4 %, both derived as 1.2 × the
  creatinine values from the equation's −1.200 exponent.
- **Thresholds:** KDIGO 2024 categories, G3a/G3b `watch` and G4/G5
  `alert`. On a tie the deepest category crossed is reported.

Limits: age is result year − birth year (only the birth year is stored,
±1 year ≈ ≤ 0.6 % on eGFR). Adults ≥ 18 only. No eGFR is estimated
without demographics (CSV-only data) or for a recorded sex other than
female/male; the creatinine report then says why under "not assessed".
The derived series is labelled as derived everywhere: card, panel,
`derived` in the API, and its table shows the creatinine inputs.

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

**FHIR R4** (`ingest::fhir`): hand-defined serde structs for only the
fields we read (Bundle → Observation: status, LOINC coding, valueQuantity,
effectiveDateTime, subject). Each resource is converted on its own, so
one bad Observation is skipped with a reason instead of failing the
bundle. The API layer (`import.rs`) then keeps only analytes with a drift
profile (mapping LOINC → short code) and groups the skip reasons for the
upload response. The full FHIR models belong to app #3, fhir-r4-explorer.

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

## Review workflow (review.rs, routes/reviews.rs, migration 002)

The app follows app #1's override-audit pattern. A clinician
acknowledges, dismisses or reopens a watch/alert signal (dismiss and
reopen need a reason) or adds a note. Each action is one row in
`review_events`, which a trigger makes append-only.

- **Identity.** A signal is `(patient, code, rule, t)`, where `t` is the
  result it fired on. A review covers exactly that finding: the next
  result raises a new, unreviewed signal, so a review never silences
  future drift.
- **State.** It is folded from the events: the latest acknowledge,
  dismiss or reopen decides; notes are counted but don't change the
  state.
- **Integrity.** The client only names the signal. The server recomputes
  the report with the same options, refuses a signal it no longer
  computes (409), and stores its own snapshot: the signal, status,
  latest result, baseline, options and engine version. Each audit row
  therefore shows what the clinician saw. Reasons pass an identifier
  screen (synthetic data only), and the actor is a pseudonym.
- **Triage.** Patients are ordered by their worst *unreviewed* signal.
  The computed status stays visible, so a reviewed alert moves down but
  never disappears.

## Explain this drift (crates/api/src/llm, routes/explain*.rs)

The pipeline is ported from app #1: one schema-constrained model call,
streamed. Anthropic streams SSE and Ollama streams NDJSON; both feed a
partial-JSON field extractor that emits text per field as it arrives,
which the server re-emits to the browser as NDJSON
(`start → delta… → done | error`).

- **Grounding.** The model sees only `explain::context(report)`: analyte,
  computed status, baseline and prRI, CVI/CVA, population interval,
  thresholds, signals with sources, not-assessed reasons, trend and the
  last 12 results. It never sees the patient identifier. The prompt
  forbids re-grading, diagnoses and doses, and asks the model to say
  "not in this record" when information is missing.
- **Authority.** `done` is validated against the schema. The model's
  `status` is overwritten by the engine's (`status_overridden` records any
  disagreement), and the computed signals are attached. The UI shows
  "computed status (authoritative)" next to the draft.
- **Safety UX.** The disclaimer is on the first line. Stop aborts the
  fetch; the server then drops the upstream request, frees the slot and
  stops billing. Runs are keyed by patient, biomarker, as-of, window and
  model, so another series' text can never show. Copy is offered only
  for the validated draft.
- **Providers.** The offline stub is the default and restates the
  computed signals; it streams through the same extractor and validation
  as a real model. Ollama is a shared instance (read-only discovery,
  2-minute keep-alive). Claude needs `ANTHROPIC_API_KEY`. A semaphore
  allows 2 model calls in flight, and a stream holds its permit until it
  ends.

## Seeded demo (`demo/`, see `demo/PROVENANCE.md`)

- **`synthea-subset.json`**: 6 patients and 502 results, curated from a
  pinned Synthea v4.0.0 run (fixed seed, reproducible byte for byte).
  Realistic yearly/quarterly cadences for HbA1c, LDL-C and creatinine;
  pseudonyms, birth year only; physiologically impossible Synthea series
  are dropped whole by documented rules.
- **`edge-cases.json`**: 4 hand-made synthetic patients for what Synthea
  cannot give: a TSH rise on levothyroxine, CKD creatinine creep with a
  mid-series switch to µmol/L, an erroneous result with repeats plus an
  `entered-in-error` duplicate, and a sparse/gapped history.

Every detector fires for at least one demo patient, and all three
statuses occur (`crates/api/tests/demo_data.rs`). The v1 CSV
(alice/bob/carol, weekly) remains as the API test fixture only.

## Design decisions

| Decision | Why |
|----------|-----|
| Drift crate has zero deps beyond serde | Portable; no IO, no clock — callers pass timestamps and `as_of` |
| Profiles as a typed Rust table, not a CSV | Compile-checked, reviewable in one diff with sources; a CSV parser would break the serde-only rule |
| Population CVI, not the patient's own SD | Works from 3 results and makes limits independent of how noisy a window happens to be (v1's D4); personal-SD models need ≥ 5 steady-state results |
| Earliest steady state as baseline | A change must stay flagged until a clinician accepts it; a sliding baseline silently normalises disease progression |
| Seeded property loops instead of proptest | Keeps the drift crate's lockfile surface at serde only |
| Curated Synthea subset + hand-made edge cases | Synthea gives realistic cadences and comorbidity, but some lab values are impossible and TSH/CKD creatinine are unusable; committing a small reviewed subset (with its generator) beats both raw Synthea and invented-only data |
| FHIR import keeps profiled analytes only | Without biological variation and limits an analyte can only be trended; a full EHR export would bury the clinically tracked tests |
| polars here, plain csv+regex in app #1 | Data-shaped workload (scan→filter→group_by) vs one-shot row-wise ETL — the documented decision table in BEST_PRACTICES/RUST_DATA_STACK_2026.md |
| Enum store dispatch | Same API, swappable backends, object-safe without async-trait |
