# AGENTS.md

Build, test, and verification commands for the Biomarker Trend Analyzer.

## Tooling

- **Rust: cargo** — run from the repo root (cargo workspace)
- **JS/TS: pnpm only** — `frontend/` as the working directory

## Rust

```bash
cargo run -p biomarker-api          # serve :8003 (memory store, demo-seeded)
cargo test --workspace -q           # 27 tests, no infra needed
cargo clippy --workspace --all-targets -- -D warnings   # must be clean
cargo fmt --check
cargo audit                         # accepted advisories + reasons: .cargo/audit.toml
```

Postgres mode:

```bash
docker compose up -d db             # postgres:17-alpine on 127.0.0.1:5435
APP_STORE=postgres APP_DATABASE_URL=postgresql://app:app@127.0.0.1:5435/biomarkers \
  cargo run -p biomarker-api         # migrations run at startup
DATABASE_URL=postgresql://app:app@127.0.0.1:5435/biomarkers \
  cargo test -p biomarker-api --features pg-tests --test pg_store   # real-DB tests (CI job)
```

- Port 5433 (and 5434) belong to other projects on this machine — never use them
- Inserts are idempotent: `(patient_id, code, taken_at)` is unique; both
  stores return `InsertReport { inserted, duplicates }`
- Errors: body `{error, code}`; store errors map to `internal` (logged,
  never sent); every response has `x-request-id`

## Frontend

```bash
pnpm install
pnpm dev          # :5174 strictPort (5173 is app #1's), proxies /api -> :8003
pnpm build        # tsc -b + vite build
pnpm lint         # oxlint
```

## Environment

| Variable | Default | Notes |
|----------|---------|-------|
| `APP_STORE` | `memory` | `memory` or `postgres` |
| `APP_DATABASE_URL` | — | required for postgres |
| `APP_SEED_DEMO` | `true` | seed synthetic demo data at startup |
| `APP_DEMO_CSV` | `crates/api/tests/fixtures/demo_labs.csv` | seed source |
| `APP_WINDOW_DAYS` | `90` | personal-baseline lookback window |
| `APP_PORT` | `8003` | 8000/8001 taken by ZAP_AGI / app #1 |

## Conventions

- Conventional commits: `<type>: <subject>` — lowercase, imperative, ≤72 chars
- Commit hygiene hook strips AI attribution — never add it, never `--no-verify`
- `crates/drift` stays pure: no IO, no clock access, no dependencies beyond
  serde — analyze() receives timestamps and "now"
- `crates/ingest`: schema is declared and the header is validated explicitly
  (polars maps the schema positionally otherwise)
- Store changes: implement both backends behind the enum in
  `crates/api/src/store/` + tests run against MemoryStore only
- Deps ≥7 days old (see BEST_PRACTICES/INDEX.md), exact via Cargo.lock;
  frontend deps exact, `frontend/pnpm-workspace.yaml` sets
  `minimumReleaseAge: 10080` (pnpm refuses anything younger, incl. transitive)
- CI actions are SHA-pinned (tag in a comment), `permissions: contents: read`
- `cargo test` never needs Postgres or network
