<p align="center">
  <h1>📈 Biomarker Trend Analyzer</h1>
  <p><b>Personal-baseline drift detection — personalised reference intervals, RCV, CUSUM/EWMA, Mann–Kendall</b></p>
  <p>
    <a href="https://github.com/akarales/biomarker-trend-analyzer/actions/workflows/ci.yml"><img src="https://github.com/akarales/biomarker-trend-analyzer/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
    <img src="https://img.shields.io/badge/Rust-1.96-orange?logo=rust" alt="Rust 1.96">
    <img src="https://img.shields.io/badge/tests-107%20rust%20%2B%2069%20web-success" alt="tests">
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

- [x] v1 — scaffold: drift math, ingestion, stores, API, charts, CI
- [x] v2 — prRI/RCV drift engine, FHIR R4 + Synthea demo, clinician workspace, streamed explanations
- [ ] v2 — clinician review workflow with an append-only audit (M6)
- [ ] later — similar-patient trajectories (embeddings; deferred)

</details>

## 🤝 Contributing

PRs welcome — see [CONTRIBUTING.md](CONTRIBUTING.md). Gates: `cargo
clippy --workspace --all-targets -- -D warnings`, `cargo test
--workspace -q`, `pnpm build`.

## 📄 License

MIT — see [LICENSE](LICENSE).
