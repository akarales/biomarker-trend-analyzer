# Development Guide

Machine-facing commands live in [AGENTS.md](../AGENTS.md).

## Prerequisites

- Rust 1.96, cargo · pnpm 11 / Node 24 (frontend)
- Docker (only for Postgres mode)

## Daily loop

```bash
cargo run -p biomarker-api      # :8003, demo-seeded memory store
cd frontend && pnpm dev         # :5174 (strict) → /api proxied
cargo test --workspace -q      # 71 tests — no Postgres, no network
cargo clippy --workspace --all-targets -- -D warnings
cd frontend && pnpm test && pnpm e2e   # vitest (40) + Playwright smoke
```

## Postgres mode

```bash
docker compose up -d db         # postgres:17-alpine on 127.0.0.1:5435
APP_STORE=postgres APP_DATABASE_URL=postgresql://app:app@127.0.0.1:5435/biomarkers \
  cargo run -p biomarker-api    # migrations run at startup; re-seeding is a no-op
```

Real-database tests sit behind the `pg-tests` feature (`#[sqlx::test]`
creates a fresh database per test); CI runs them in a `postgres` job:

```bash
DATABASE_URL=postgresql://app:app@127.0.0.1:5435/biomarkers \
  cargo test -p biomarker-api --features pg-tests --test pg_store
```

## Testing notes

- **Drift tests** assert clinical semantics, not just math:
  `tests/scenarios.rs` has one test per v1 audit finding (D1–D9) plus
  outlier-that-reverts, statin start, unknown analyte and empty series;
  `tests/properties.rs` checks order, time-shift and scale invariance over
  200 seeded series (deterministic noise, no proptest dependency); unit
  tests pin the RCV worked example (creatinine +13.4 % / −11.8 %), the
  EWMA limit at i = 1 (0.6σ) and Mann–Kendall on a monotone series
- **Ingestion tests** cover header mismatch, bad values, bad dates, and
  lazy group_by stats over a temp file
- **API tests** upload, list, and read through the real router in-process
  against the seeded fixture
- **Architecture tests**: `crates/drift/tests/purity.rs` (serde-only, no
  clock/IO) and `frontend/src/test/architecture.test.ts` (file size, tokens,
  feature boundaries, layering) — both mutation-checked
- **Frontend**: store tests mock `@/api/*` and interleave responses with
  deferred promises (stale summaries/series are dropped); components run
  in jsdom; e2e runs the real API + production build

## Gotchas learned here

- **polars 0.55**: schemas apply positionally on header mismatch —
  always validate the header yourself; `LazyCsvReader::new` takes a
  `PlRefPath`, not a `&str`
- **sqlx 0.9**: `query_as` turbofish is `<DB, O>` order; the `macros`
  feature is required for `sqlx::migrate!`
- **sqlx offline**: after any query/migration change run
  `cargo sqlx prepare --workspace -- --all-targets --features biomarker-api/pg-tests`
  against the compose db and commit `.sqlx/`; `COUNT(*)` needs
  `AS "name!"` to come back non-nullable
- **Chart time zone (fixed in M2)**: the chart now draws `report.points`
  (epoch seconds) and `taken_at` is RFC 3339 with `Z`; never parse naive
  date-time text in the browser (it is read as local time)
- **sqlx + TIMESTAMPTZ**: decode into `DateTime<Utc>`, never
  `NaiveDateTime` (v1 shipped that mismatch untested)
- Port 8002 is taken on this machine — this service runs on 8003;
  Postgres 5433/5434 belong to other projects — compose uses 5435

## Conventions

Conventional commits; hygiene hook strips AI attribution. Store changes
must implement both backends behind the enum; drift changes must come
with updated semantics tests. Deps pinned, ≥7 days old when added.
