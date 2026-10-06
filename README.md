<p align="center">
  <h1>📈 Biomarker Trend Analyzer</h1>
  <p><b>Personal-baseline drift detection — robust z-scores, EWMA, Theil–Sen</b></p>
  <p>
    <a href="https://github.com/akarales/biomarker-trend-analyzer/actions/workflows/ci.yml"><img src="https://github.com/akarales/biomarker-trend-analyzer/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
    <img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="License: MIT">
    <img src="https://img.shields.io/badge/Rust-1.96-orange?logo=rust" alt="Rust 1.96">
    <img src="https://img.shields.io/badge/tests-24-success" alt="tests">
    <img src="https://img.shields.io/badge/port-8003-blue" alt="port 8003">
  </p>
</p>

The core concept behind longitudinal patient monitoring: compute a
**personal** baseline for each biomarker series (not population reference
ranges), flag drift with robust statistics, and visualize trends over time.
Rust-native — a pure drift-math crate, polars ingestion, an axum API with
pluggable stores (in-memory or PostgreSQL), and a React client with
hand-rolled SVG trend charts.

**Jump to:** [Features](#-features) · [Architecture](#-architecture) · [Quickstart](#-quickstart) · [Configuration](#️-configuration) · [API](#-api) · [Docs](#-documentation) · [Roadmap](#️-roadmap)

> [!WARNING]
> Demo application with synthetic data. Drift reports are informational
> only — not clinical decision support, not medical advice.

## ⚡ Features

- **Personal baselines** — median + MAD over a configurable lookback
  window (robust σ = 1.4826·MAD); outlier labs don't skew the baseline
- **Two drift shapes, two detectors** — steps caught by z-scores/EWMA,
  gradual drift caught by Theil–Sen slope; Normal / Watch / Alert status
- **polars ingestion** — schema-enforced CSV parsing with explicit
  header validation and row-level, column-named errors
- **Store abstraction** (the ZAP Runtime `store/mod.rs` pattern) —
  MemoryStore for tests/demos, PgStore (sqlx + startup migrations) for
  runtime; idempotent inserts (re-seeding or re-uploading never
  duplicates results), real-Postgres tests in CI
- **Zero-dependency charts** — SVG trend chart with baseline band and
  anomaly markers, written by hand

## 📐 Architecture

```mermaid
flowchart TD
    FE["React 19 + SVG<br/>upload · patients · trends"] -->|"/api/v1"| API["axum 0.8<br/>routes · config"]
    API --> ING["biomarker-ingest<br/>polars 0.55<br/>schema-enforced CSV"]
    API --> ST["Store<br/>Memory | Pg (sqlx + migrations)"]
    API --> DRIFT["biomarker-drift<br/>median/MAD · z · EWMA · Theil–Sen<br/>(pure, no IO)"]
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
| `APP_WINDOW_DAYS` | `90` | personal-baseline lookback window |
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
