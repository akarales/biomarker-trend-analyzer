<p align="center">
  <h1>📈 Biomarker Trend Analyzer</h1>
  <p><b>Personal-baseline drift detection — personalised reference intervals, RCV, CUSUM/EWMA, Mann–Kendall</b></p>
  <p>
    <a href="https://github.com/akarales/biomarker-trend-analyzer/actions/workflows/ci.yml"><img src="https://github.com/akarales/biomarker-trend-analyzer/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
    <img src="https://img.shields.io/badge/Rust-1.96-orange?logo=rust" alt="Rust 1.96">
    <img src="https://img.shields.io/badge/tests-71%20rust%20%2B%2040%20web-success" alt="tests">
    <img src="https://img.shields.io/badge/port-8003-blue" alt="port 8003">
  </p>
</p>

The core concept behind longitudinal patient monitoring: judge each new lab
result against the patient's **own** steady state — using published
biological variation, the way laboratory medicine does — next to the
population limits, and say *why* something was flagged.
Rust-native — a pure drift-math crate, polars ingestion, an axum API with
pluggable stores (in-memory or PostgreSQL), and a React client with
hand-rolled SVG trend charts.

**Jump to:** [Features](#-features) · [Architecture](#-architecture) · [Quickstart](#-quickstart) · [Configuration](#️-configuration) · [API](#-api) · [Docs](#-documentation) · [Roadmap](#️-roadmap)

> [!WARNING]
> Demo application with synthetic data. Drift reports are informational
> only — not clinical decision support, not medical advice.

## ⚡ Features

- **Personalised reference interval** (Coşkun et al. 2021) — a set point
  from the earliest steady-state results and a 95 % prediction interval
  from cited within-subject variation (CVI) and analytical imprecision (CVA)
- **One detector per drift shape** — reference change value for jumps,
  CUSUM (with change point) and EWMA for sustained shifts, Mann–Kendall +
  Sen slope (with CI) for trends, cited clinical thresholds (ADA, ATP III,
  ATA) and population intervals for context
- **Explainable** — every flag is a signal with the rule, the limit, a
  plain-language explanation and its source; "not assessed" says what
  could not be checked; analysis `as_of` an explicit date, units
  normalised (UCUM, LOINC)
- **polars ingestion** — schema-enforced CSV parsing with explicit
  header validation and row-level, column-named errors
- **Store abstraction** (the ZAP Runtime `store/mod.rs` pattern) —
  MemoryStore for tests/demos, PgStore (sqlx + startup migrations) for
  runtime; idempotent inserts (re-seeding or re-uploading never
  duplicates results), real-Postgres tests in CI
- **Zero-dependency charts** — SVG trend chart with the personal
  reference interval and out-of-range markers, plus a "why this status"
  panel listing every signal and what was not assessed

## 📐 Architecture

```mermaid
flowchart TD
    FE["React 19 + SVG<br/>upload · patients · trends"] -->|"/api/v1"| API["axum 0.8<br/>routes · config"]
    API --> ING["biomarker-ingest<br/>polars 0.55<br/>schema-enforced CSV"]
    API --> ST["Store<br/>Memory | Pg (sqlx + migrations)"]
    API --> DRIFT["biomarker-drift<br/>prRI · RCV · CUSUM/EWMA · Mann–Kendall · thresholds<br/>(pure, no IO, no clock)"]
```

Full breakdown in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## 🚀 Quickstart

```bash
cargo run -p biomarker-api    # :8003 — memory store, seeded with synthetic
                              # demo data (3 patients, 576 readings, one
                              # injected HBA1C step-up, one gradual LDL rise)
cd frontend && pnpm install && pnpm dev   # → http://localhost:5174
```

Postgres mode (migrations run automatically at startup):

```bash
docker compose up -d db       # postgres:17-alpine on 127.0.0.1:5435
APP_STORE=postgres APP_DATABASE_URL=postgresql://app:app@127.0.0.1:5435/biomarkers \
  cargo run -p biomarker-api
```

## ⚙️ Configuration

| Variable | Default | Notes |
|----------|---------|-------|
| `APP_STORE` | `memory` | `memory` or `postgres` |
| `APP_DATABASE_URL` | — | required for postgres |
| `APP_SEED_DEMO` | `true` | seed synthetic demo data at startup |
| `APP_DEMO_CSV` | `crates/api/tests/fixtures/demo_labs.csv` | seed source |
| `APP_WINDOW_DAYS` | `365` | default trend lookback (per request: `?window_days=`) |
| `APP_PORT` | `8003` | 8000–8002 taken on this machine |

## 📡 API

| Endpoint | Purpose |
|----------|---------|
| `GET /health` | liveness + active store |
| `POST /api/v1/observations` | upload CSV (text/csv body) → validated, stored |
| `GET /api/v1/patients` | listing with counts |
| `GET /api/v1/patients/{id}/summary` | per-biomarker drift reports |
| `GET /api/v1/patients/{id}/biomarkers/{code}` | full series + report + band |

curl + JSON examples: [docs/API.md](docs/API.md).

## 📚 Documentation

| Page | What's inside |
|------|---------------|
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Drift math, detector semantics, ingestion pipeline, store pattern |
| [docs/API.md](docs/API.md) | Endpoint reference with payloads + error taxonomy |
| [docs/DEVELOPMENT.md](docs/DEVELOPMENT.md) | Setup, Postgres mode, testing, gotchas |

## 🗺️ Roadmap

<details>
<summary>Phased plan</summary>

- [x] Phase 0 — scaffold: drift math, ingestion, stores, API, charts, CI
- [ ] Phase 1 — FHIR R4 Observation ingestion (reuses app #3's models)
- [ ] Phase 2 — similar-patient trajectories (embeddings; deferred)
- [ ] Phase 3 — LLM narrative summaries of drift (Ollama)

</details>

## 🤝 Contributing

PRs welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). Gates: `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo test
--workspace -q`, `pnpm build`.

## 📄 License

MIT — see [LICENSE](LICENSE).
