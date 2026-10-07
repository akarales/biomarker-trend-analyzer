<p align="center">
  <h1>📈 Biomarker Trend Analyzer</h1>
  <p><b>Clinician workspace for lab-result drift — personalised reference intervals, RCV, CUSUM/EWMA, Mann–Kendall, FHIR R4, review audit, grounded streamed AI</b></p>
  <p>
    <a href="https://github.com/akarales/biomarker-trend-analyzer/actions/workflows/ci.yml"><img src="https://github.com/akarales/biomarker-trend-analyzer/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
    <img src="https://img.shields.io/badge/Rust-1.96-orange?logo=rust" alt="Rust 1.96">
    <img src="https://img.shields.io/badge/tests-127_rust_·_80_vitest_·_12_e2e-success" alt="tests: 127 Rust, 80 vitest, 12 Playwright">
    <img src="https://img.shields.io/badge/data-synthetic_(Synthea)-blue" alt="synthetic data (Synthea)">
  </p>
</p>

<p align="center">
  <img src="docs/demo.gif" width="720" alt="Demo: the triage list puts SYN-01 first; opening the HbA1c card shows the trend chart with the personal reference interval, diabetes threshold and a hover readout; the signals explain why the status is Alert, the personal-reference-interval signal is acknowledged and appears in the append-only review history, and an explanation draft grounded in the computed signals streams in">
</p>

Longitudinal lab monitoring the way laboratory medicine does it: judge each
new result against the patient's **own** steady state using published
biological variation, keep the population limits and clinical thresholds
alongside, and say **why** something was flagged. Clinicians triage patients
by what still needs review, read the chart (or its table), acknowledge or
dismiss each signal into an append-only audit trail, and can ask for a
streamed draft explanation that stays grounded in the computed signals.
Rust-native: a pure drift-math crate, polars and FHIR R4 ingestion, an axum
API over an in-memory or PostgreSQL store, and a React 19 + shadcn client
with hand-rolled SVG charts.

**Jump to:** [Features](#-features) · [Architecture](#-architecture) · [Quickstart](#-quickstart) · [Safety & data](#️-safety--data) · [Configuration](#️-configuration) · [API](#-api) · [Docs](#-documentation)

> [!WARNING]
> Demo application on **synthetic** data (Synthea + hand-made cases). Drift
> signals and AI drafts are informational only — not clinical decision
> support, not medical advice; every output needs clinician review.

## ⚡ Features

- **Personalised reference interval** (Coşkun et al. 2021) — a set point
  from the earliest steady-state results and a 95 % prediction interval
  from cited within-subject variation (CVI) and analytical imprecision (CVA)
- **One detector per drift shape** — reference change value for jumps,
  CUSUM (with change point) and EWMA for sustained shifts, Mann–Kendall +
  Sen slope (with CI) for trends, cited clinical thresholds (ADA, ATP III,
  ATA) and population intervals for context
- **Kidney function staged properly** — eGFR derived per creatinine
  result with the race-free 2021 CKD-EPI equation from FHIR Patient sex
  and birth year, analysed like any series with KDIGO G3a–G5 categories
  (clearly labelled as derived; "not assessed" with the reason when
  demographics are missing)
- **Explainable** — every flag is a signal with the rule, the limit, a
  plain-language explanation and its source; "not assessed" says what
  could not be checked; analysis `as_of` an explicit date, units
  normalised (UCUM, LOINC)
- **FHIR R4 + CSV ingestion** — FHIR Bundles/Observations (LOINC, UCUM,
  status-aware, every skipped resource explained) and schema-enforced CSV
  via polars; uploads are idempotent
- **Realistic synthetic demo** — a curated, reproducible Synthea v4.0.0
  subset (yearly/quarterly labs, comorbid diabetes/CKD/hyperlipidaemia)
  plus hand-made edge cases (TSH dose drift, unit change, erroneous
  outlier, sparse history) — provenance in [demo/PROVENANCE.md](demo/PROVENANCE.md)
- **Store abstraction** (the ZAP Runtime `store/mod.rs` pattern) —
  MemoryStore for tests/demos, PgStore (sqlx + startup migrations) for
  runtime; idempotent inserts (re-seeding or re-uploading never
  duplicates results), real-Postgres tests in CI
- **Clinician workspace** — triage list (worst first, with the signal
  behind it), cards showing personal vs population range, a hand-rolled
  SVG trend chart (axes, personal/population bands, thresholds, RCV jumps,
  change point, hover + keyboard readout, table alternative), as-of date
  and trend window in a shareable URL — dark shadcn theme, WCAG 2.2 AA
  checked with axe in every state
- **Clinician review + audit** — acknowledge, dismiss (with a reason),
  annotate or reopen each signal; append-only events (Postgres trigger)
  with a server-computed snapshot of the signal at decision time; triage
  orders patients by unreviewed signals while keeping the computed status
- **Explain this drift** — a streamed AI draft for the reviewing clinician
  (offline stub, local Ollama or Claude), grounded in the computed signals:
  disclaimer from the first byte, Stop cancels the model call, and the
  computed status and signals are re-asserted over the model's text

## 📐 Architecture

```mermaid
flowchart TD
    FE["React 19 + shadcn + SVG<br/>triage · cards · chart · upload"] -->|"/api/v1"| API["axum 0.8<br/>routes · config"]
    API --> ING["biomarker-ingest<br/>FHIR R4 Bundles · polars CSV"]
    API --> ST["Store<br/>Memory | Pg (sqlx + migrations)"]
    API --> LLM["llm<br/>stub · Ollama · Claude<br/>NDJSON stream"]
    API --> DRIFT["biomarker-drift<br/>prRI · RCV · CUSUM/EWMA · Mann–Kendall · thresholds<br/>(pure, no IO, no clock)"]
```

Full breakdown in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## 🚀 Quickstart

```bash
cargo run -p biomarker-api    # :8003 — memory store, seeded from demo/
                              # (10 synthetic patients, 556 FHIR lab results)
cd frontend && pnpm install && pnpm dev   # → http://localhost:5174
```

Postgres mode (migrations run automatically at startup):

```bash
docker compose up -d db       # postgres:17-alpine on 127.0.0.1:5435
APP_STORE=postgres APP_DATABASE_URL=postgresql://app:app@127.0.0.1:5435/biomarkers \
  cargo run -p biomarker-api
```

Everything in containers (API on Postgres, web UI via nginx):

```bash
docker compose up --build     # → http://localhost:3002 (API on 127.0.0.1:8003)
```

Both images run as non-root users with read-only root filesystems.
nginx (the unprivileged image, port 8080) sends a strict CSP and security
headers, and streams the explanation endpoint unbuffered.

## 🛡️ Safety & data

- **Synthetic only.** The demo patients are a curated Synthea subset
  (pseudonyms, birth year only) plus hand-made edge cases
  ([demo/PROVENANCE.md](demo/PROVENANCE.md)). Free-text review reasons
  are screened for identifier patterns.
- **The computed signals are authoritative.** Every signal carries its
  rule, limit, explanation and source; "not assessed" lists what could not
  be checked; "no rule fired" is never presented as healthy.
- **AI drafts:**
  - the disclaimer arrives from the first byte;
  - the model never sees the patient identifier;
  - the server overwrites the model's status with the computed one and
    re-attaches the signals.
- **Audit.** Reviews are append-only events (a database trigger rejects
  edits) carrying a server-computed snapshot of the signal at decision
  time. A review never hides a finding: the status stays visible and the
  next result raises a new signal.
- **Not validated:** the analyte profiles are cited and dated, but CVA
  values are assumptions and nothing here is clinically validated.

## ⚙️ Configuration

| Variable | Default | Notes |
|----------|---------|-------|
| `APP_STORE` | `memory` | `memory` or `postgres` |
| `APP_DATABASE_URL` | — | required for postgres |
| `APP_SEED_DEMO` | `true` | seed synthetic demo data at startup |
| `APP_DEMO_DATA` | `demo` | file or directory of FHIR `*.json` / `*.csv` seeded at startup (see `demo/PROVENANCE.md`) |
| `APP_WINDOW_DAYS` | `1095` | default trend lookback (per request: `?window_days=`) |
| `APP_PORT` | `8003` | 8000–8002 taken on this machine |
| `APP_LLM_STUB` | `true` | offline stub drafts (no model call); `false` → `APP_LLM_PROVIDER` |
| `APP_LLM_PROVIDER` | `ollama` | `ollama` or `anthropic` default when not stubbed |
| `APP_OLLAMA_URL` / `APP_OLLAMA_MODEL` | `http://localhost:11434` / `qwen3:14b` | shared instance, `APP_OLLAMA_KEEP_ALIVE=2m` |
| `APP_ANTHROPIC_MODEL` / `ANTHROPIC_API_KEY` | `claude-sonnet-5-5` / — | key only in the gitignored `.env` |

## 📡 API

| Endpoint | Purpose |
|----------|---------|
| `GET /health` | liveness + active store |
| `POST /api/v1/observations` | upload CSV or FHIR R4 Bundle → validated, stored idempotently |
| `GET /api/v1/patients` | triage listing, worst first |
| `GET /api/v1/patients/{id}/summary` | per-biomarker drift reports |
| `GET /api/v1/patients/{id}/biomarkers/{code}` | full series + report |
| `POST /api/v1/explain/stream` · `POST /api/v1/explain` | grounded AI draft (NDJSON stream · JSON) |
| `GET /api/v1/llm/models` | model chooser (stub, Ollama, Claude) |
| `POST · GET /api/v1/patients/{id}/biomarkers/{code}/reviews` | review a signal · audit trail |

curl + JSON examples: [docs/API.md](docs/API.md).

## 📚 Documentation

| Page | What's inside |
|------|---------------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Drift method + sources, ingestion, stores, review audit, LLM pipeline, workspace |
| [docs/API.md](docs/API.md) | Endpoint reference with payloads + error taxonomy |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Setup, Postgres mode, testing, demo data + demo GIF, gotchas |
| [demo/PROVENANCE.md](demo/PROVENANCE.md) | Where every demo patient comes from (Synthea version, seed, rules) |

## 🗺️ Roadmap

<details>
<summary>Phased plan</summary>

- [x] v1 — scaffold: drift math, ingestion, stores, API, charts, CI
- [x] v2 — prRI/RCV drift engine, FHIR R4 + Synthea demo, clinician workspace, streamed explanations
- [x] v2 — clinician review workflow with an append-only audit
- [ ] later — similar-patient trajectories (embeddings; deferred)

</details>

## 🤝 Contributing

PRs welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). Gates: `cargo fmt
--check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo
test --workspace -q`, and in `frontend/`: `pnpm build`, `pnpm lint`,
`pnpm test`, `pnpm e2e`.

## 📄 License

MIT — see [LICENSE](LICENSE).
