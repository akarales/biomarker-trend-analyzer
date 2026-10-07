# AGENTS.md

Build, test, and verification commands for the Biomarker Trend Analyzer.

## Tooling

- **Rust: cargo** — run from the repo root (cargo workspace)
- **JS/TS: pnpm only** — `frontend/` as the working directory

## Rust

```bash
cargo run -p biomarker-api          # serve :8003 (memory store, demo-seeded)
cargo test --workspace -q           # 118 tests (drift, ingest incl. FHIR, api incl. demo data), no infra
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

- Queries use sqlx `query!` macros with committed `.sqlx/` metadata;
  `.cargo/config.toml` sets `SQLX_OFFLINE=true`, so builds/CI need no DB.
  After changing a query or migration (`crates/api/migrations/`):
  `sqlx migrate run --source crates/api/migrations` then
  `cargo sqlx prepare --workspace -- --all-targets --features biomarker-api/pg-tests`
  (sqlx-cli 0.9.0) and commit `.sqlx/`
- Port 5433 (and 5434) belong to other projects on this machine — never use them
- Inserts are idempotent: `(patient_id, code, taken_at)` is unique; both
  stores return `InsertReport { inserted, duplicates }`
- Errors: body `{error, code}`; store errors map to `internal` (logged,
  never sent); every response has `x-request-id`

## Frontend

```bash
pnpm install
pnpm dev          # :5174 strictPort (5173 is app #1's), proxies /api -> :8003
pnpm build        # tsc -b + vite build (the type check gate)
pnpm lint         # oxlint
pnpm test         # vitest: architecture rules, schemas, http, store, URL state, chart scales/layout/readout, components + chart keyboard (jsdom); network mocked
pnpm e2e          # Playwright: smoke + a11y (axe WCAG 2.2 AA in every state, keyboard chart, mobile) against the real API (:8093, demo seed) + production build (:4184)
                  # (locally: PW_CHROMIUM_PATH=/usr/bin/google-chrome pnpm e2e)
```

Refactor without behaviour change — local screenshot comparison
(baselines are machine-specific, gitignored in `e2e/__visual__/`). Run the
visual spec ON ITS OWN (other specs upload data into the shared test API):

```bash
PW_VISUAL=1 PW_CHROMIUM_PATH=/usr/bin/google-chrome pnpm e2e visual --update-snapshots   # on the old code
PW_VISUAL=1 PW_CHROMIUM_PATH=/usr/bin/google-chrome pnpm e2e visual                      # on the new code: 0 px diff
```

UI: shadcn (style `radix-nova`, app #1's "Vitals" Catppuccin dark theme in
`src/index.css`, `<html class="dark">`). `components/ui/` is registry-owned:
add with `pnpm exec shadcn add <name>` (shadcn 4.21.0 is a pinned devDependency),
never hand-edit; delete unused ones (no `remove` command). Prefer
`native-select` over the Radix select (its `aria-hidden` page overlay fails
axe `aria-hidden-focus`).

## Code structure (enforced)

```
frontend/src/
  app/        shell only: App (layout), AppHeader, useUrlSync (?patient=&code=&as_of=&window=)
  features/   patients (triage) · biomarker (cards) · chart · controls (as-of, window) · upload · explain · review
              — each exposes index.ts; other features import ONLY that
  shared/     domain (status, tokens, format), components (StatusChip, ErrorBanner), hooks
  state/      zustand store composed from slices/{patients,biomarker,upload,view,review}.ts
              + selectors.ts + url.ts; import from @/state
  api/        http (zod/mini-validated) + patients.ts / observations.ts + schemas.ts
  components/ui/  shadcn registry (CLI-owned)
crates/api/src/  routes/{patients,biomarkers,observations,health,explain,explain_stream,reviews}.rs + views.rs, llm/, explain.rs, review.rs, validate.rs,
                 analysis.rs (only caller of drift::analyze), triage.rs, import.rs, store/, error.rs
```

Chart (`features/chart/`): pure `scale.ts` (nice ticks, calendar ticks) and
`geometry.ts` (layout at a measured pixel width — text never scales down),
static `ChartLayers` (aria-hidden SVG), `TrendChart` = one focusable group:
←/→/Home/End/PageUp/PageDown + hover, polite live readout; `DataTable` is the
full text alternative (focusable scroll region). Slices drop stale responses
with request tokens — keep that when adding requests.

Rules (`src/test/architecture.test.ts` fails the build on them):
- source files ≤ ~300 lines (tests exempt) — split before adding to a long file
- colours set from TS only from `shared/domain/tokens.ts` (Catppuccin Mocha on
  the dark card, text ≥ 7:1; mirrored to CSS as `--status-*`, `--severity-*`,
  `--chart-*`); no hex literals elsewhere in .ts/.tsx; app chrome uses the
  theme's semantic classes (`bg-card`, `text-muted-foreground`, …)
- colour is never the only channel: statuses render through `StatusChip` (word + dot)
- `features/X` imports `@/features/Y` (its index), never `@/features/Y/...`
- `api/`, `state/`, `shared/` never import `features/` or `app/`
- every new module gets a test; API responses get a zod schema
- Rust: JSON shaping lives in `routes/views.rs`; handlers use `?`
  (`From<StoreError> for ApiError`)

## Review workflow (append-only audit)

- `review_events` (migration 002) is append-only: a trigger rejects
  UPDATE/DELETE/TRUNCATE. Never work around it. Withdrawals are new
  `reopen` events. MemoryStore has no mutate/remove method either.
- A signal = `(patient, code, rule, t)`. Snapshots are built server-side
  (`review::snapshot`) from the recomputed report; never accept one from
  the client. A missing signal is `409 stale_signal`.
- Reasons go through `validate::reason` (3–500 chars, identifier screen).
  The actor is the pseudonym `demo-clinician`.
- Triage = worst UNREVIEWED signal first. `status` stays the computed one;
  a review must never hide a finding.
- After changing a query: `sqlx migrate run --source crates/api/migrations`
  against the compose db, then `cargo sqlx prepare --workspace -- --all-targets
  --features biomarker-api/pg-tests`, and commit `.sqlx/`.

## LLM ("explain this drift")

- All model calls go through `crates/api/src/llm/` (ported from app #1:
  `extract`, `stream`, `ollama`, `anthropic`, `models` copied; `prompts`,
  `schema`, `stub` are this app's). Tests use the stub and never touch the
  network (test configs point Ollama at a closed port).
- The model input is `crates/api/src/explain.rs::context(report)`: computed
  facts only, **no patient identifier**. The computed status/signals are
  authoritative: `response_body` overwrites the model's `status` and
  attaches `signals` — keep that for any new LLM output.
- Disclaimer on the stream's first line; `done` == the `/explain` body.

## Demo data (`demo/`, provenance in `demo/PROVENANCE.md`)

```bash
scripts/synthea/generate_synthea.sh                                  # Java 17+, uv; pinned Synthea v4.0.0 (sha256-checked, cached in .cache/), seed 20261006 → demo/synthea-subset.json (byte-identical)
uv run --no-project scripts/demo/edge_cases.py demo/edge-cases.json  # hand-made edge cases (fixed seed; CI checks it regenerates identically)
```

- Synthetic only. The selector keeps pseudonyms, gender and birth YEAR
  only, and drops implausible Synthea series WHOLE by documented rules
  (never edits values). The patient selection is an explicit, reviewed
  list in `select_subset.py`.
- `crates/api/tests/demo_data.rs` pins the demo: every detector fires,
  edge cases behave as documented, re-seeding is a no-op. Update it with
  the data.
- `crates/api/tests/fixtures/demo_labs.csv` (v1 alice/bob/carol, weekly)
  is a test fixture only.

## Environment

| Variable | Default | Notes |
|----------|---------|-------|
| `APP_STORE` | `memory` | `memory` or `postgres` |
| `APP_DATABASE_URL` | — | required for postgres |
| `APP_SEED_DEMO` | `true` | seed synthetic demo data at startup |
| `APP_DEMO_DATA` | `demo` | file or directory of FHIR `*.json` / `*.csv` seeded at startup (see `demo/PROVENANCE.md`) |
| `APP_WINDOW_DAYS` | `1095` | default trend lookback (per request: `?window_days=`, 7–3650); 3 years suits annual/quarterly labs |
| `APP_PORT` | `8003` | 8000/8001 taken by ZAP_AGI / app #1 |
| `APP_LLM_STUB` | `true` | deterministic offline drafts built from the computed report |
| `APP_LLM_PROVIDER` | `ollama` | `ollama` or `anthropic` (used when stub=false) |
| `APP_OLLAMA_URL` / `APP_OLLAMA_MODEL` | `http://localhost:11434` / `qwen3:14b` | |
| `APP_OLLAMA_KEEP_ALIVE` | `2m` | the Ollama instance is SHARED — never pull/delete/configure models, never send reload-forcing options (`num_ctx`) |
| `APP_ANTHROPIC_MODEL` | `claude-sonnet-5-5` | Messages API + JSON-schema structured output |
| `ANTHROPIC_API_KEY` | — | only in the gitignored `.env` (loaded at startup via dotenvy) |

## Conventions

- Conventional commits: `<type>: <subject>` — lowercase, imperative, ≤72 chars
- Commit hygiene hook strips AI attribution — never add it, never `--no-verify`
- `crates/drift` stays pure: no IO, no clock access, no dependencies beyond
  serde (`tests/purity.rs`) — `analyze()` receives timestamps and an explicit
  `as_of` (default: latest result)
- Analyte data (CVI, CVA, popRI, thresholds) lives ONLY in
  `crates/drift/src/profiles.rs`, each value with its `source`; bump
  `REVIEWED` when changing any of them. CVA values are assumptions.
- Every detector emits `Signal`s with explanation + source; a rule that
  cannot run adds a `NotAssessed` reason — never return a silent "normal"
- Drift changes come with scenario tests (`tests/scenarios.rs`, one per
  D1–D9 finding) and must keep `tests/properties.rs` green
- `crates/ingest`: CSV schema is declared and the header is validated explicitly
  (polars maps the schema positionally otherwise); `fhir.rs` converts each
  Observation on its own and skips with a reason — never coerce a value
- Upload/seed parsing goes through `crates/api/src/import.rs` (format
  detection, LOINC → profile code, FHIR keeps profiled analytes only)
- Store changes: implement both backends behind the enum in
  `crates/api/src/store/` + tests run against MemoryStore only
- Deps ≥7 days old (see BEST_PRACTICES/INDEX.md), exact via Cargo.lock;
  frontend deps exact, `frontend/pnpm-workspace.yaml` sets
  `minimumReleaseAge: 10080` (pnpm refuses anything younger, incl. transitive)
- CI actions are SHA-pinned (tag in a comment), `permissions: contents: read`
- `cargo test` never needs Postgres or network
