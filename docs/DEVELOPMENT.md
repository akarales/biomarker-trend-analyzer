# Development Guide

Machine-facing commands live in [AGENTS.md](../AGENTS.md).

## Prerequisites

- Rust 1.96, cargo · pnpm 11 / Node 24 (frontend)
- Docker (only for Postgres mode)

## Daily loop

```bash
cargo run -p biomarker-api      # :8003, demo-seeded memory store
cd frontend && pnpm dev         # :5173 → /api proxied
cargo test --workspace -q      # 24 tests — no Postgres, no network
cargo clippy --workspace --all-targets -- -D warnings
```

## Postgres mode

```bash
docker compose up -d db         # pgvector/pgvector:pg17 on :5433
APP_STORE=postgres APP_DATABASE_URL=postgresql://app:app@localhost:5433/biomarkers \
  cargo run -p biomarker-api    # migrations run at startup
```

CI never runs a database — the PgStore code compiles, MemoryStore serves
tests. Live Postgres tests are a roadmap item (skip-if-no-db pattern).

## Testing notes

- **Drift tests** assert detector semantics, not just math: a step-up
  must alert via z but NOT report a rising Theil–Sen slope (robust by
  design); a gradual rise must be caught by the slope instead; a control
  series must stay normal
- **Ingestion tests** cover header mismatch, bad values, bad dates, and
  lazy group_by stats over a temp file
- **API tests** upload, list, and read through the real router in-process
  against the seeded fixture

## Gotchas learned here

- **polars 0.55**: schemas apply positionally on header mismatch —
  always validate the header yourself; `LazyCsvReader::new` takes a
  `PlRefPath`, not a `&str`
- **sqlx 0.9**: `query_as` turbofish is `<DB, O>` order; the `macros`
  feature is required for `sqlx::migrate!`
- Port 8002 is taken on this machine — this service runs on 8003

## Conventions

Conventional commits; hygiene hook strips AI attribution. Store changes
must implement both backends behind the enum; drift changes must come
with updated semantics tests. Deps pinned, ≥7 days old when added.
