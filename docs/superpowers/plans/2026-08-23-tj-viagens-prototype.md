# TJ-Viagens Prototype Implementation Plan (Rust + React)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the functional prototype of TJ-Viagens — a web platform where TJRR staff open air-ticket quotations that accredited suppliers bid on blindly within a server-controlled 1-hour window, with lowest-price ranking, service-order emission, 30-minute e-ticket return, and a hash-chained audit trail — plus the phase-2 pitch assets (canvas + 5-minute video script).

**Architecture:** Two top-level apps. `api/` is a Rust crate: Axum 0.8 + SQLx (PostgreSQL 16 in Docker), JWT auth with role extractors, SSE via tokio broadcast channels for the live countdown, multipart uploads for documents/e-tickets, and **printable HTML pages** (browser "Salvar como PDF") for the Ordem de Serviço and the per-quotation dossier. `web/` is a Vite + React + TypeScript SPA styled with Tailwind v4 + shadcn/ui + **Fluid Functionalism** components (spring motion, proximity hover) with two role areas (staff and supplier).

**Tech Stack:** Rust stable, Axum 0.8, SQLx 0.8, PostgreSQL 16 (Docker, host port 5433), jsonwebtoken, argon2, sha2, askama, tracing; **Bun** for all JS tooling; Vite 6, React 18, Tailwind CSS 4, shadcn/ui + Fluid Functionalism registry, TanStack Query, sonner, Inter variable font.

---

## Deadline and evaluation context (from `Blueprint_do_Pitch_Vencedor.pdf`)

**Submission: video link pasted at npi.tjrr.jus.br/si before 14h on 26/08/2026** — 3 days from plan date. The ≤5-minute YouTube video (public or unlisted; private = zero) is the primary evaluated artifact; the prototype exists to be *shown* in it ("Show, Don't Tell").

Phase-2 criteria and weights — every UI/demo decision below serves them:

| Criterion | Weight | What this plan does for it |
|---|---|---|
| E2C3 Evolução do Conceito | 3.0 | Working full flow (not mockups): credenciamento → disputa cega → OS → e-ticket → economia; audit chain and KPIs prove technical maturation of the phase-1 idea |
| E2C1 Usabilidade Preliminar | 2.5 | Fluid Functionalism motion/components, task-oriented screens, live countdown, responsive layout |
| E2C2 Viabilidade de Integração | 2.5 | PostgreSQL + containers (STI-compatible), open source, OpenAPI-style JSON API, printable dossiê ready for SEI attachment, LGPD redaction shown on camera |
| E2C4 Qualidade da Defesa | 2.0 | Task 16 writes the 5-act script with timed demo beats; recording checklist enforces blueprint rules |

**Execution priority if time runs short:** Tasks 0–11 (API with demoable flow) → Task 15 (seed + demo env) → Tasks 12–14 (UI) → Task 16 (pitch). The video can show Insomnia/terminal + UI; it cannot show missing flow.

## Source-of-truth requirements (from `tema.pdf`, Tema 1 / Desafio 1, Edital 13/2026)

| # | Rule (edital) | Where implemented |
|---|---|---|
| R1 | Permanent accreditation: company data, **CNPJ format validation**, mandatory fiscal/labor regularity documents attached | Tasks 2, 5 |
| R2 | Staff registers demand with reference flight data; **reference price absolutely secret** to suppliers during bidding | Task 6 |
| R3 | Simultaneous notification of all ACTIVE suppliers (e-mail + internal panel) on quotation open | Task 6 |
| R4 | **1-hour countdown** for proposals, visible to suppliers, **server clock authoritative** | Tasks 6, 7, 11 |
| R5 | Proposal = total price R$ + flight observations; suppliers never see competitors' bids | Tasks 6, 7 |
| R6 | After deadline: table auto-ordered lowest→highest; staff declares winner | Task 8 |
| R7 | Passenger form (Nome, CPF, Sexo, Nascimento) auto-filled into an emitted **Ordem de Serviço** | Tasks 6, 8, 10 |
| R8 | Winner has **30 minutes** to attach the e-ticket; divergence check vs demand/proposal | Task 9 |
| R9 | Automated per-quotation report: participants, values, user logs, timestamps | Tasks 4, 10 |
| R10 | Measurable economy: reference vs contracted price (KPIs) | Task 10 |
| R11 | Usable on desktop and mobile (responsive) | Tasks 12–14 |
| NS | **Non-scope:** no payment processing, no scraping/paid flight APIs, no cloud dependency, no diárias/hotel/ground transport, no AI dependency (deterministic rules only) | Whole plan — none of these are built |

## Design decisions locked in (do not re-litigate during execution)

- **Money is integer cents (`i64`/BIGINT)** end-to-end in the API. The UI converts with `parseBRL`/`formatBRL` in `web/src/lib/domain.ts`.
- **Server time is authoritative** (R4): windows are `opens_at`/`closes_at` timestamps checked against `Utc::now()`. No cron — expiry applies lazily on read (`load_quotation`) and is pushed over SSE. `PROPOSAL_WINDOW_MINUTES` (60) and `TICKET_WINDOW_MINUTES` (30) are env vars so the live demo can shrink them.
- **Late e-tickets are accepted but flagged** (`late: true`) — the 30-min rule is a KPI (% on time), not a hard rejection.
- **Audit trail**: append-only `audit_events` with SHA-256 chaining (`hash = sha256(prev_hash + "|" + canonical_json)`), serialized under `pg_advisory_xact_lock`. serde_json's default map is a BTreeMap, so `serde_json::to_string` of a built `Value` is already key-sorted/canonical. `at` is stored as the exact RFC3339 **TEXT** that was hashed (no timestamp round-trip drift). **Never put floats in audit payloads** — integers/strings/null only.
- **"AI-assistive" features are deterministic rules** (document checklist pre-triage, e-ticket divergence check). This IS the required fallback; no LLM anywhere.
- **Proposals may be replaced** while the window is open (one row per supplier per quotation, upsert; `submitted_at` keeps first submission time). Ranking tie-break: earlier `submitted_at` wins.
- **Redaction is centralized** in `views.rs` (`staff_view` / `supplier_view`) — the only functions that serialize quotations. Suppliers never receive `reference_price_cents`, rival proposals, or passenger PII; the **winner** receives passenger data after award (needed to emit the ticket). Reports mask CPF; only the OS carries it in full.
- **Printable HTML instead of PDF bytes**: `/quotations/:id/service-order` and `/quotations/:id/report` return styled HTML with print CSS; the browser produces the PDF. Faster in Rust, and looks better on camera.
- **Auth token in `Authorization: Bearer` OR `?token=` query param** — one extractor serves JSON routes, EventSource (SSE cannot set headers), and browser-opened HTML pages.
- **Status/role enums are real Rust enums** stored as TEXT, with exhaustive `match` (no wildcard arms except in `parse` of untrusted strings) — honoring the user's no-switch-defaults rule.
- **DB rows use `String` for status columns**, converted at boundaries via `Enum::parse`/`as_str`. No SQLx macros (`query!`) — runtime `query`/`query_as` only, so builds never require a live DB.

## File structure

```
hacka-roraima/                     # repo root (git initialized)
├── docker-compose.yml             # postgres:16 on host port 5433 (dev + test DBs)
├── docker/initdb/01-test-db.sql
├── README.md                      # quickstart + demo roteiro (Task 15)
├── docs/pitch/                    # canvas.md, roteiro-video.md, checklist-gravacao.md (Task 16)
├── api/
│   ├── Cargo.toml  .env  .env.example
│   ├── migrations/0001_init.sql
│   ├── src/
│   │   ├── main.rs                # serve
│   │   ├── lib.rs                 # app(state) -> Router; AppState
│   │   ├── config.rs  db.rs  error.rs  auth.rs  sse.rs  html.rs
│   │   ├── domain/                # pure logic, unit-tested inline
│   │   │   ├── mod.rs  types.rs  cnpj.rs  brl.rs  cpf.rs
│   │   │   ├── checklist.rs  divergence.rs  economy.rs
│   │   ├── audit.rs               # hash chain append/verify
│   │   └── routes/
│   │       ├── mod.rs  auth.rs  suppliers.rs  quotations.rs
│   │       ├── proposals.rs  award.rs  tickets.rs  reports.rs
│   │       └── views.rs           # staff_view / supplier_view redaction
│   ├── src/bin/seed.rs            # demo data (Task 15)
│   └── tests/
│       ├── common/mod.rs          # spawn_app, factories, truncate
│       └── integration.rs         # core rule tests + full flow
└── web/
    ├── package.json  vite.config.ts  tsconfig.json  index.html  components.json
    └── src/
        ├── main.tsx  App.tsx  index.css
        ├── components/ui/         # shadcn + Fluid Functionalism (CLI-generated)
        ├── components/{Countdown.tsx, StatusBadge.tsx, Layout.tsx}
        ├── lib/{api.ts, auth.tsx, domain.ts, utils.ts}
        └── pages/
            ├── Login.tsx  Register.tsx
            ├── supplier/{Home.tsx, QuotationBid.tsx}
            └── staff/{Dashboard.tsx, NewQuotation.tsx, QuotationDetail.tsx, Suppliers.tsx}
```

## Conventions for every task

- Commands run from repo root `/Users/davi/gits/hacka-roraima` unless `cd` shown. Requires Rust stable (rustup), **Bun ≥ 1.1** (all JS tooling — `bun install`, `bun add`, `bunx`, `bun run`), Docker.
- API tests share one test database: **always** `cargo test -- --test-threads=1` (run inside `api/`). Docker Postgres must be up.
- API port **3001**, web port **5173**, Postgres host port **5433**.
- Test-depth policy (user decision): pure domain logic gets inline unit tests; HTTP behavior gets **core integration tests** (window enforcement, redaction, ranking, audit chain, full flow) — not per-endpoint TDD ceremony.
- Commit after every green step; conventional commits.

---

### Task 0: Repo scaffold + Postgres

**Files:**
- Modify: `.gitignore`
- Create: `docker-compose.yml`, `docker/initdb/01-test-db.sql`

- [ ] **Step 1: Verify repo root is `hacka-roraima` itself (NOT `~/gits`)**

Run: `git rev-parse --show-toplevel`
Expected: `/Users/davi/gits/hacka-roraima`. If it prints `/Users/davi/gits`, STOP and run `git init -b main` inside `hacka-roraima` (the parent dir is a catch-all repo).

- [ ] **Step 2: Write infra files**

`.gitignore` (replace whole file):

```gitignore
.DS_Store
node_modules/
dist/
target/
.env
uploads/
```

`docker-compose.yml`:

```yaml
services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: tj
      POSTGRES_PASSWORD: tj
      POSTGRES_DB: tjviagens
    ports:
      - "5433:5432"
    volumes:
      - ./docker/initdb:/docker-entrypoint-initdb.d
      - pgdata:/var/lib/postgresql/data
volumes:
  pgdata:
```

`docker/initdb/01-test-db.sql`:

```sql
CREATE DATABASE tjviagens_test;
```

- [ ] **Step 3: Start Postgres, verify both DBs**

Run: `docker compose up -d --wait && docker compose exec db psql -U tj -d postgres -c '\l' | grep tjviagens`
Expected: rows for `tjviagens` and `tjviagens_test`. (`-d postgres` is required — with no database flag psql tries a DB named `tj`, which doesn't exist. `--wait` blocks on the healthcheck. Stale volume without the test DB: `docker compose down -v && docker compose up -d --wait`.)

- [ ] **Step 4: Commit**

```bash
git add .gitignore docker-compose.yml docker/ tema.pdf docs/
git commit -m "chore: repo scaffold, postgres compose, challenge pdf and plan"
```

---

### Task 1: Rust crate — config, DB pool, router, health, test harness

**Files:**
- Create: `api/Cargo.toml`, `api/.env.example`, `api/.env`, `api/src/main.rs`, `api/src/lib.rs`, `api/src/config.rs`, `api/src/db.rs`, `api/src/error.rs`, `api/migrations/0001_init.sql` (full schema — `sqlx::migrate!` embeds it from day one), plus the module stub files listed in Step 3
- Test: `api/tests/common/mod.rs`, `api/tests/integration.rs`

- [ ] **Step 1: Create the crate and Cargo.toml**

Run: `cargo new api --lib && cd api && mkdir -p migrations src/bin tests/common && cd ..`

`api/Cargo.toml`:

```toml
[package]
name = "tj-viagens-api"
version = "0.1.0"
edition = "2021"

[lib]
name = "tj_viagens_api"
path = "src/lib.rs"

[[bin]]
name = "api"
path = "src/main.rs"

[[bin]]
name = "seed"
path = "src/bin/seed.rs"

[dependencies]
axum = { version = "0.8", features = ["multipart", "macros"] }
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.6", features = ["cors"] }
sqlx = { version = "0.8", features = ["runtime-tokio", "tls-rustls", "postgres", "uuid", "chrono", "json", "migrate"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
jsonwebtoken = "9"
argon2 = { version = "0.5", features = ["std"] }
sha2 = "0.10"
hex = "0.4"
dotenvy = "0.15"
unicode-normalization = "0.1"
futures = "0.3"
tokio-stream = { version = "0.1", features = ["sync"] }

[dev-dependencies]
reqwest = { version = "0.12", features = ["json", "multipart"] }
```

`src/bin/seed.rs` must exist for the manifest to compile — create it now as a real (tiny) program that Task 15 will replace:

```rust
fn main() {
    println!("seed: implemented in Task 15");
}
```

- [ ] **Step 2: Write the schema migration (used by sqlx::migrate! from day one)**

`api/migrations/0001_init.sql`:

```sql
CREATE TABLE users (
  id UUID PRIMARY KEY,
  email TEXT NOT NULL UNIQUE,
  name TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  role TEXT NOT NULL,
  supplier_id UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE suppliers (
  id UUID PRIMARY KEY,
  cnpj TEXT NOT NULL UNIQUE,
  legal_name TEXT NOT NULL,
  contact_email TEXT NOT NULL,
  phone TEXT,
  status TEXT NOT NULL DEFAULT 'PENDING',
  status_reason TEXT,
  decided_at TIMESTAMPTZ,
  decided_by UUID,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

ALTER TABLE users
  ADD CONSTRAINT users_supplier_fk FOREIGN KEY (supplier_id) REFERENCES suppliers(id);

CREATE TABLE supplier_documents (
  id UUID PRIMARY KEY,
  supplier_id UUID NOT NULL REFERENCES suppliers(id),
  doc_type TEXT NOT NULL,
  file_name TEXT NOT NULL,
  file_path TEXT NOT NULL,
  valid_until DATE,
  uploaded_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX supplier_documents_supplier_idx ON supplier_documents(supplier_id);

CREATE TABLE quotations (
  id UUID PRIMARY KEY,
  code TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL DEFAULT 'DRAFT',
  passenger_name TEXT NOT NULL,
  passenger_cpf TEXT NOT NULL,
  passenger_sex TEXT NOT NULL,
  passenger_birth DATE NOT NULL,
  origin TEXT NOT NULL,
  destination TEXT NOT NULL,
  departure_at TIMESTAMPTZ NOT NULL,
  return_at TIMESTAMPTZ,
  reference_flight TEXT NOT NULL,
  reference_price_cents BIGINT NOT NULL,
  opens_at TIMESTAMPTZ,
  closes_at TIMESTAMPTZ,
  awarded_proposal_id UUID UNIQUE,
  awarded_at TIMESTAMPTZ,
  award_justification TEXT,
  ticket_deadline_at TIMESTAMPTZ,
  created_by UUID NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE proposals (
  id UUID PRIMARY KEY,
  quotation_id UUID NOT NULL REFERENCES quotations(id),
  supplier_id UUID NOT NULL REFERENCES suppliers(id),
  total_price_cents BIGINT NOT NULL,
  flight_info TEXT NOT NULL,
  notes TEXT,
  submitted_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (quotation_id, supplier_id)
);

CREATE TABLE service_orders (
  id UUID PRIMARY KEY,
  quotation_id UUID NOT NULL UNIQUE REFERENCES quotations(id),
  number TEXT NOT NULL UNIQUE,
  issued_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE tickets (
  id UUID PRIMARY KEY,
  quotation_id UUID NOT NULL UNIQUE REFERENCES quotations(id),
  file_name TEXT NOT NULL,
  file_path TEXT NOT NULL,
  passenger_name TEXT NOT NULL,
  flight_info TEXT NOT NULL,
  departure_at TIMESTAMPTZ NOT NULL,
  price_cents BIGINT NOT NULL,
  divergences JSONB NOT NULL,
  late BOOLEAN NOT NULL,
  uploaded_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  confirmed_at TIMESTAMPTZ,
  confirmed_by UUID
);

CREATE TABLE notifications (
  id UUID PRIMARY KEY,
  supplier_id UUID NOT NULL REFERENCES suppliers(id),
  quotation_id UUID REFERENCES quotations(id),
  kind TEXT NOT NULL,
  message TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  read_at TIMESTAMPTZ
);

CREATE TABLE audit_events (
  seq BIGSERIAL PRIMARY KEY,
  at TEXT NOT NULL,
  actor_id UUID,
  actor_role TEXT,
  event_type TEXT NOT NULL,
  entity TEXT NOT NULL,
  entity_id TEXT NOT NULL,
  quotation_id UUID,
  payload JSONB NOT NULL,
  prev_hash TEXT NOT NULL,
  hash TEXT NOT NULL
);
CREATE INDEX audit_events_quotation_idx ON audit_events(quotation_id);

CREATE TABLE counters (
  id TEXT PRIMARY KEY,
  value BIGINT NOT NULL
);
```

- [ ] **Step 3: Write config, error, db, lib, main**

`api/.env.example` (copy verbatim to `api/.env`):

```bash
DATABASE_URL=postgresql://tj:tj@localhost:5433/tjviagens
TEST_DATABASE_URL=postgresql://tj:tj@localhost:5433/tjviagens_test
JWT_SECRET=dev-secret-change-me
PORT=3001
PROPOSAL_WINDOW_MINUTES=60
TICKET_WINDOW_MINUTES=30
UPLOAD_DIR=uploads
WEB_ORIGIN=http://localhost:5173
```

`api/src/config.rs`:

```rust
#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub port: u16,
    pub proposal_window_minutes: i64,
    pub ticket_window_minutes: i64,
    pub upload_dir: String,
    pub web_origin: String,
}

fn var(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

impl Config {
    pub fn from_env() -> Config {
        Config {
            database_url: var("DATABASE_URL", "postgresql://tj:tj@localhost:5433/tjviagens"),
            jwt_secret: var("JWT_SECRET", "dev-secret-change-me"),
            port: var("PORT", "3001").parse().unwrap_or(3001),
            proposal_window_minutes: var("PROPOSAL_WINDOW_MINUTES", "60").parse().unwrap_or(60),
            ticket_window_minutes: var("TICKET_WINDOW_MINUTES", "30").parse().unwrap_or(30),
            upload_dir: var("UPLOAD_DIR", "uploads"),
            web_origin: var("WEB_ORIGIN", "http://localhost:5173"),
        }
    }
}
```

`api/src/error.rs`:

```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Forbidden(&'static str),
    NotFound(&'static str),
    Unprocessable(&'static str),
    UnprocessableWith(&'static str, serde_json::Value),
    Conflict(&'static str),
    Internal(String),
}

pub type ApiResult<T> = Result<T, ApiError>;

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> ApiError {
        ApiError::Internal(err.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match self {
            ApiError::Unauthorized => {
                (StatusCode::UNAUTHORIZED, json!({ "error": "NAO_AUTENTICADO" }))
            }
            ApiError::Forbidden(code) => (StatusCode::FORBIDDEN, json!({ "error": code })),
            ApiError::NotFound(code) => (StatusCode::NOT_FOUND, json!({ "error": code })),
            ApiError::Unprocessable(code) => {
                (StatusCode::UNPROCESSABLE_ENTITY, json!({ "error": code }))
            }
            ApiError::UnprocessableWith(code, detail) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                json!({ "error": code, "detail": detail }),
            ),
            ApiError::Conflict(code) => (StatusCode::CONFLICT, json!({ "error": code })),
            ApiError::Internal(msg) => {
                eprintln!("internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, json!({ "error": "ERRO_INTERNO" }))
            }
        };
        (status, Json(body)).into_response()
    }
}
```

`api/src/db.rs`:

```rust
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn connect(database_url: &str) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .expect("failed to connect to postgres");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");
    pool
}
```

`api/src/lib.rs` (grows over later tasks; this is the Task-1 version):

```rust
pub mod auth;
pub mod audit;
pub mod config;
pub mod db;
pub mod domain;
pub mod error;
pub mod html;
pub mod routes;
pub mod sse;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::routing::get;
use axum::{Json, Router};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::broadcast;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use crate::config::Config;

pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub channels: Mutex<HashMap<Uuid, broadcast::Sender<sse::SseMsg>>>,
}

pub type App = Arc<AppState>;

impl AppState {
    pub fn new(pool: PgPool, config: Config) -> App {
        Arc::new(AppState { pool, config, channels: Mutex::new(HashMap::new()) })
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "status": "ok", "serverNow": chrono::Utc::now().to_rfc3339() }))
}

async fn time_now() -> Json<serde_json::Value> {
    Json(json!({ "serverNow": chrono::Utc::now().to_rfc3339() }))
}

pub fn app(state: App) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);
    Router::new()
        .route("/health", get(health))
        .route("/time", get(time_now))
        .merge(routes::router())
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(10 * 1024 * 1024))
        .with_state(state)
}
```

Create the module stubs so `lib.rs` compiles — each is a REAL file that later tasks fill (they export nothing yet, which is valid Rust, not a placeholder value):

`api/src/auth.rs`, `api/src/audit.rs`, `api/src/html.rs`, `api/src/sse.rs` each start as an empty file **except** `api/src/sse.rs`, which needs `SseMsg` for `lib.rs`:

```rust
#[derive(Clone, Debug)]
pub struct SseMsg {
    pub event: String,
    pub data: String,
}
```

`api/src/domain/mod.rs`:

```rust
pub mod brl;
pub mod checklist;
pub mod cnpj;
pub mod cpf;
pub mod divergence;
pub mod economy;
pub mod types;
```

…with each listed file created empty for now (Task 2 fills them all — same commit day).

`api/src/routes/mod.rs`:

```rust
use axum::Router;

use crate::App;

pub mod auth;
pub mod award;
pub mod proposals;
pub mod quotations;
pub mod reports;
pub mod suppliers;
pub mod tickets;
pub mod views;

pub fn router() -> Router<App> {
    Router::new()
        .merge(auth::router())
        .merge(suppliers::router())
        .merge(quotations::router())
        .merge(proposals::router())
        .merge(award::router())
        .merge(tickets::router())
        .merge(reports::router())
}
```

…and each `routes/*.rs` starts as:

```rust
use axum::Router;

use crate::App;

pub fn router() -> Router<App> {
    Router::new()
}
```

(`routes/views.rs` starts empty.) These compile and are replaced by later tasks.

`api/src/main.rs`:

```rust
use tj_viagens_api::{app, config::Config, db, AppState};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    let pool = db::connect(&config.database_url).await;
    let port = config.port;
    let state = AppState::new(pool, config);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await.unwrap();
    println!("TJ-Viagens API on http://localhost:{port}");
    axum::serve(listener, app(state)).await.unwrap();
}
```

- [ ] **Step 4: Write the test harness + health test**

`api/tests/common/mod.rs`:

```rust
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tj_viagens_api::{app, config::Config, AppState};

pub struct TestApp {
    pub base: String,
    pub pool: PgPool,
    pub client: reqwest::Client,
}

pub fn test_config() -> Config {
    Config {
        database_url: std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://tj:tj@localhost:5433/tjviagens_test".to_string()),
        jwt_secret: "test-secret".to_string(),
        port: 0,
        proposal_window_minutes: 60,
        ticket_window_minutes: 30,
        upload_dir: std::env::temp_dir().join("tj-uploads").to_string_lossy().to_string(),
        web_origin: "http://localhost:5173".to_string(),
    }
}

pub async fn spawn_app() -> TestApp {
    let config = test_config();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("test db must be running (docker compose up -d)");
    sqlx::migrate!("./migrations").run(&pool).await.expect("migrations");
    sqlx::query(
        "TRUNCATE users, suppliers, supplier_documents, quotations, proposals, \
         service_orders, tickets, notifications, audit_events, counters \
         RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .expect("truncate");

    let state = AppState::new(pool.clone(), config);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app(state)).await.unwrap();
    });
    TestApp {
        base: format!("http://{addr}"),
        pool,
        client: reqwest::Client::new(),
    }
}
```

`api/tests/integration.rs` (first test):

```rust
mod common;

use common::spawn_app;

#[tokio::test]
async fn health_reports_server_time() {
    let app = spawn_app().await;
    let res = app.client.get(format!("{}/health", app.base)).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["serverNow"].as_str().unwrap().contains('T'));
}
```

- [ ] **Step 5: Build + test + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: compiles, `1 passed`. First build downloads crates (~2–4 min).

Run the dev server once: `cd api && cargo run --bin api` → `curl localhost:3001/health` → `{"status":"ok",...}` → Ctrl-C.

```bash
git add api
git commit -m "feat(api): axum skeleton with config, pg pool, migrations, health and test harness"
```

---

### Task 2: Domain layer — enums, CNPJ, BRL, CPF mask, checklist, divergence, economy (pure, unit-tested)

**Files:**
- Fill: `api/src/domain/types.rs`, `cnpj.rs`, `brl.rs`, `cpf.rs`, `checklist.rs`, `divergence.rs`, `economy.rs` (created empty in Task 1)

- [ ] **Step 1: Implement `types.rs`**

```rust
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Role {
    Admin,
    Servidor,
    Fornecedor,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "ADMIN",
            Role::Servidor => "SERVIDOR",
            Role::Fornecedor => "FORNECEDOR",
        }
    }
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "ADMIN" => Some(Role::Admin),
            "SERVIDOR" => Some(Role::Servidor),
            "FORNECEDOR" => Some(Role::Fornecedor),
            _ => None,
        }
    }
    pub fn is_staff(&self) -> bool {
        match self {
            Role::Admin | Role::Servidor => true,
            Role::Fornecedor => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SupplierStatus {
    Pending,
    Active,
    Rejected,
    Suspended,
}

impl SupplierStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SupplierStatus::Pending => "PENDING",
            SupplierStatus::Active => "ACTIVE",
            SupplierStatus::Rejected => "REJECTED",
            SupplierStatus::Suspended => "SUSPENDED",
        }
    }
    pub fn parse(s: &str) -> Option<SupplierStatus> {
        match s {
            "PENDING" => Some(SupplierStatus::Pending),
            "ACTIVE" => Some(SupplierStatus::Active),
            "REJECTED" => Some(SupplierStatus::Rejected),
            "SUSPENDED" => Some(SupplierStatus::Suspended),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QuotationStatus {
    Draft,
    Open,
    Closed,
    Awarded,
    Ticketed,
    Completed,
}

impl QuotationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            QuotationStatus::Draft => "DRAFT",
            QuotationStatus::Open => "OPEN",
            QuotationStatus::Closed => "CLOSED",
            QuotationStatus::Awarded => "AWARDED",
            QuotationStatus::Ticketed => "TICKETED",
            QuotationStatus::Completed => "COMPLETED",
        }
    }
    pub fn parse(s: &str) -> Option<QuotationStatus> {
        match s {
            "DRAFT" => Some(QuotationStatus::Draft),
            "OPEN" => Some(QuotationStatus::Open),
            "CLOSED" => Some(QuotationStatus::Closed),
            "AWARDED" => Some(QuotationStatus::Awarded),
            "TICKETED" => Some(QuotationStatus::Ticketed),
            "COMPLETED" => Some(QuotationStatus::Completed),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DocType {
    ContratoSocial,
    CndFederal,
    CrfFgts,
    Cndt,
}

impl DocType {
    pub fn as_str(&self) -> &'static str {
        match self {
            DocType::ContratoSocial => "CONTRATO_SOCIAL",
            DocType::CndFederal => "CND_FEDERAL",
            DocType::CrfFgts => "CRF_FGTS",
            DocType::Cndt => "CNDT",
        }
    }
    pub fn parse(s: &str) -> Option<DocType> {
        match s {
            "CONTRATO_SOCIAL" => Some(DocType::ContratoSocial),
            "CND_FEDERAL" => Some(DocType::CndFederal),
            "CRF_FGTS" => Some(DocType::CrfFgts),
            "CNDT" => Some(DocType::Cndt),
            _ => None,
        }
    }
}
```

- [ ] **Step 2: Implement the pure functions (each file ends with its `#[cfg(test)]` module)**

`api/src/domain/cnpj.rs`:

```rust
/// R1: CNPJ format validation with check digits (punctuation ignored).
pub fn is_valid_cnpj(input: &str) -> bool {
    let digits: Vec<u32> = input.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != 14 {
        return false;
    }
    if digits.iter().all(|&d| d == digits[0]) {
        return false;
    }
    let dv = |len: usize| -> u32 {
        let weights: &[u32] = if len == 12 {
            &[5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]
        } else {
            &[6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]
        };
        let sum: u32 = weights.iter().zip(&digits).map(|(w, d)| w * d).sum();
        let m = sum % 11;
        if m < 2 { 0 } else { 11 - m }
    };
    dv(12) == digits[12] && dv(13) == digits[13]
}

#[cfg(test)]
mod tests {
    use super::is_valid_cnpj;

    #[test]
    fn accepts_valid_cnpjs() {
        assert!(is_valid_cnpj("11.222.333/0001-81"));
        assert!(is_valid_cnpj("11444777000161"));
        assert!(is_valid_cnpj("12.345.678/0001-95"));
    }

    #[test]
    fn rejects_invalid_cnpjs() {
        assert!(!is_valid_cnpj("11.222.333/0001-82"));
        assert!(!is_valid_cnpj("11.111.111/1111-11"));
        assert!(!is_valid_cnpj("123"));
    }
}
```

`api/src/domain/brl.rs`:

```rust
/// Formats integer cents as pt-BR currency: 123456 -> "R$ 1.234,56".
pub fn format_brl(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    let frac = abs % 100;
    let mut int_str = (abs / 100).to_string();
    let mut grouped = String::new();
    while int_str.len() > 3 {
        let split = int_str.len() - 3;
        grouped = format!(".{}{}", &int_str[split..], grouped);
        int_str.truncate(split);
    }
    format!("{sign}R$ {int_str}{grouped},{frac:02}")
}

#[cfg(test)]
mod tests {
    use super::format_brl;

    #[test]
    fn formats_cents() {
        assert_eq!(format_brl(123456), "R$ 1.234,56");
        assert_eq!(format_brl(89000), "R$ 890,00");
        assert_eq!(format_brl(5), "R$ 0,05");
        assert_eq!(format_brl(185000000), "R$ 1.850.000,00");
    }
}
```

`api/src/domain/cpf.rs`:

```rust
/// LGPD masking for screens/reports: keeps middle 6 digits only.
pub fn mask_cpf(cpf: &str) -> String {
    let digits: String = cpf.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.len() != 11 {
        return "***".to_string();
    }
    format!("***.{}.{}-**", &digits[3..6], &digits[6..9])
}

#[cfg(test)]
mod tests {
    use super::mask_cpf;

    #[test]
    fn masks_first_three_and_check_digits() {
        assert_eq!(mask_cpf("123.456.789-09"), "***.456.789-**");
        assert_eq!(mask_cpf("bogus"), "***");
    }
}
```

`api/src/domain/checklist.rs`:

```rust
use std::collections::HashMap;

use chrono::NaiveDate;
use serde::Serialize;

use super::types::DocType;

pub const REQUIRED_DOCS: [DocType; 4] =
    [DocType::ContratoSocial, DocType::CndFederal, DocType::CrfFgts, DocType::Cndt];

#[derive(Debug, PartialEq, Serialize)]
pub struct ChecklistResult {
    pub missing: Vec<String>,
    pub expired: Vec<String>,
    pub ok: bool,
}

/// R1 pre-triage ("IA assistiva" deterministic fallback). Pass docs ordered by upload
/// time ascending — the latest document of each type wins.
pub fn checklist(docs: &[(DocType, Option<NaiveDate>)], today: NaiveDate) -> ChecklistResult {
    let mut latest: HashMap<&'static str, Option<NaiveDate>> = HashMap::new();
    for (doc_type, valid_until) in docs {
        latest.insert(doc_type.as_str(), *valid_until);
    }
    let missing: Vec<String> = REQUIRED_DOCS
        .iter()
        .filter(|t| !latest.contains_key(t.as_str()))
        .map(|t| t.as_str().to_string())
        .collect();
    let mut expired: Vec<String> = latest
        .iter()
        .filter(|(_, valid)| matches!(valid, Some(d) if *d < today))
        .map(|(k, _)| (*k).to_string())
        .collect();
    expired.sort();
    let ok = missing.is_empty() && expired.is_empty();
    ChecklistResult { missing, expired, ok }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;

    #[test]
    fn reports_missing_and_expired() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();
        let docs = vec![
            (DocType::CndFederal, NaiveDate::from_ymd_opt(2026, 1, 1)),
            (DocType::Cndt, NaiveDate::from_ymd_opt(2027, 1, 1)),
        ];
        let result = checklist(&docs, today);
        assert_eq!(result.missing, vec!["CONTRATO_SOCIAL", "CRF_FGTS"]);
        assert_eq!(result.expired, vec!["CND_FEDERAL"]);
        assert!(!result.ok);
    }

    #[test]
    fn ok_when_all_present_and_valid() {
        let today = NaiveDate::from_ymd_opt(2026, 8, 23).unwrap();
        let future = NaiveDate::from_ymd_opt(2027, 12, 31);
        let docs: Vec<_> = REQUIRED_DOCS.iter().map(|t| (*t, future)).collect();
        assert!(checklist(&docs, today).ok);
    }
}
```

`api/src/domain/divergence.rs`:

```rust
use chrono::{DateTime, Utc};
use unicode_normalization::UnicodeNormalization;

pub struct TicketFields<'a> {
    pub passenger_name: &'a str,
    pub departure_at: DateTime<Utc>,
    pub price_cents: i64,
}

fn norm(s: &str) -> String {
    s.trim()
        .to_uppercase()
        .nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}

fn day(d: DateTime<Utc>) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// R8: deterministic e-ticket conference — the substitutable "AI extraction" fallback.
pub fn compute_divergences(
    passenger_name: &str,
    departure_at: DateTime<Utc>,
    proposal_price_cents: i64,
    ticket: &TicketFields,
) -> Vec<String> {
    let mut divergences = Vec::new();
    if norm(ticket.passenger_name) != norm(passenger_name) {
        divergences.push("PASSAGEIRO_DIVERGENTE".to_string());
    }
    if ticket.price_cents != proposal_price_cents {
        divergences.push("VALOR_DIVERGENTE".to_string());
    }
    if day(ticket.departure_at) != day(departure_at) {
        divergences.push("DATA_DIVERGENTE".to_string());
    }
    divergences
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn accepts_accent_and_case_variations() {
        let dep = Utc.with_ymd_and_hms(2026, 9, 10, 8, 0, 0).unwrap();
        let ticket = TicketFields { passenger_name: "  MARIA DA SILVA ", departure_at: dep, price_cents: 149900 };
        assert!(compute_divergences("Maria da Silva", dep, 149900, &ticket).is_empty());
    }

    #[test]
    fn flags_wrong_passenger_price_and_date() {
        let dep = Utc.with_ymd_and_hms(2026, 9, 10, 8, 0, 0).unwrap();
        let ticket = TicketFields {
            passenger_name: "João Souza",
            departure_at: Utc.with_ymd_and_hms(2026, 9, 11, 8, 0, 0).unwrap(),
            price_cents: 155000,
        };
        assert_eq!(
            compute_divergences("Maria da Silva", dep, 149900, &ticket),
            vec!["PASSAGEIRO_DIVERGENTE", "VALOR_DIVERGENTE", "DATA_DIVERGENTE"]
        );
    }
}
```

`api/src/domain/economy.rs`:

```rust
use serde::Serialize;

#[derive(Debug, PartialEq, Serialize)]
pub struct Economy {
    pub reference_cents: i64,
    pub contracted_cents: i64,
    pub saved_cents: i64,
    pub saved_pct: f64,
}

/// R10: nominal + percentage economy per quotation.
pub fn compute_economy(reference_cents: i64, contracted_cents: i64) -> Economy {
    let saved_cents = reference_cents - contracted_cents;
    let saved_pct = if reference_cents > 0 {
        ((saved_cents as f64 / reference_cents as f64) * 10000.0).round() / 100.0
    } else {
        0.0
    };
    Economy { reference_cents, contracted_cents, saved_cents, saved_pct }
}

#[cfg(test)]
mod tests {
    use super::compute_economy;

    #[test]
    fn computes_nominal_and_pct() {
        let e = compute_economy(185000, 152300);
        assert_eq!(e.saved_cents, 32700);
        assert_eq!(e.saved_pct, 17.68);
    }
}
```

- [ ] **Step 3: Run unit tests, commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS — 10 domain unit tests + 1 integration test.

```bash
git add api/src/domain
git commit -m "feat(api): pure domain layer - enums, cnpj, brl, cpf mask, checklist, divergence, economy"
```

---

### Task 3: Auth — argon2, JWT, extractors (header OR ?token=), login, /me

**Files:**
- Fill: `api/src/auth.rs`
- Replace: `api/src/routes/auth.rs`
- Modify: `api/tests/common/mod.rs`, `api/tests/integration.rs`

- [ ] **Step 1: Implement `api/src/auth.rs`**

```rust
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::types::Role;
use crate::error::ApiError;
use crate::App;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub name: String,
    pub role: Role,
    pub supplier_id: Option<Uuid>,
    pub exp: i64,
}

pub fn sign_token(
    secret: &str,
    sub: Uuid,
    name: &str,
    role: Role,
    supplier_id: Option<Uuid>,
) -> String {
    let claims = Claims {
        sub,
        name: name.to_string(),
        role,
        supplier_id,
        exp: Utc::now().timestamp() + 8 * 3600,
    };
    encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_bytes()))
        .expect("jwt sign cannot fail with hs256")
}

pub fn verify_token(secret: &str, token: &str) -> Result<Claims, ApiError> {
    decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &Validation::default())
        .map(|data| data.claims)
        .map_err(|_| ApiError::Unauthorized)
}

/// Bearer header first; falls back to ?token= (EventSource and browser-opened
/// printable pages cannot set headers).
fn token_from_parts(parts: &Parts) -> Option<String> {
    if let Some(value) = parts.headers.get(axum::http::header::AUTHORIZATION) {
        if let Ok(s) = value.to_str() {
            if let Some(t) = s.strip_prefix("Bearer ") {
                return Some(t.to_string());
            }
        }
    }
    let query = parts.uri.query()?;
    query.split('&').find_map(|pair| pair.strip_prefix("token=").map(|t| t.to_string()))
}

pub struct AuthUser(pub Claims);

impl FromRequestParts<App> for AuthUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        let token = token_from_parts(parts).ok_or(ApiError::Unauthorized)?;
        Ok(AuthUser(verify_token(&state.config.jwt_secret, &token)?))
    }
}

pub struct Staff(pub Claims);

impl FromRequestParts<App> for Staff {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &App) -> Result<Self, Self::Rejection> {
        let AuthUser(claims) = AuthUser::from_request_parts(parts, state).await?;
        if claims.role.is_staff() {
            Ok(Staff(claims))
        } else {
            Err(ApiError::Forbidden("ACESSO_NEGADO"))
        }
    }
}

pub fn hash_password(password: &str) -> String {
    use argon2::password_hash::rand_core::OsRng;
    use argon2::password_hash::{PasswordHasher, SaltString};
    use argon2::Argon2;
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("argon2 hash")
        .to_string()
}

pub fn verify_password(hash: &str, password: &str) -> bool {
    use argon2::password_hash::{PasswordHash, PasswordVerifier};
    use argon2::Argon2;
    PasswordHash::new(hash)
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}
```

- [ ] **Step 2: Replace `api/src/routes/auth.rs`**

```rust
use std::sync::OnceLock;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::auth::{hash_password, sign_token, verify_password, AuthUser};
use crate::domain::types::Role;
use crate::error::{ApiError, ApiResult};
use crate::App;

#[derive(Deserialize)]
struct LoginBody {
    email: String,
    password: String,
}

/// Verified against on the unknown-email path so both failure branches pay the
/// same argon2 cost (prevents account/email enumeration by timing).
fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_password("dummy-timing-equalizer"))
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    name: String,
    password_hash: String,
    role: String,
    supplier_id: Option<Uuid>,
}

async fn login(State(state): State<App>, Json(body): Json<LoginBody>) -> ApiResult<Json<Value>> {
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, name, password_hash, role, supplier_id FROM users WHERE email = $1",
    )
    .bind(&body.email)
    .fetch_optional(&state.pool)
    .await?;
    let Some(user) = user else {
        let _ = verify_password(dummy_hash(), &body.password);
        return Err(ApiError::Unauthorized);
    };
    if !verify_password(&user.password_hash, &body.password) {
        return Err(ApiError::Unauthorized);
    }
    let role = Role::parse(&user.role).ok_or_else(|| ApiError::Internal("bad role in db".into()))?;
    let token = sign_token(&state.config.jwt_secret, user.id, &user.name, role, user.supplier_id);
    Ok(Json(json!({
        "token": token,
        "user": { "id": user.id, "name": user.name, "role": role, "supplierId": user.supplier_id }
    })))
}

async fn me(AuthUser(claims): AuthUser) -> Json<Value> {
    Json(json!({
        "sub": claims.sub,
        "name": claims.name,
        "role": claims.role,
        "supplierId": claims.supplier_id
    }))
}

pub fn router() -> Router<App> {
    Router::new().route("/auth/login", post(login)).route("/me", get(me))
}
```

- [ ] **Step 3: Add test factories and the auth integration test**

Append to `api/tests/common/mod.rs`:

```rust
use tj_viagens_api::auth::hash_password;
use uuid::Uuid;

pub async fn create_staff(pool: &PgPool, email: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role) \
         VALUES ($1, $2, 'Servidor SGA', $3, 'SERVIDOR')",
    )
    .bind(id)
    .bind(email)
    .bind(hash_password("demo1234"))
    .execute(pool)
    .await
    .unwrap();
    id
}

pub async fn create_supplier(
    pool: &PgPool,
    cnpj: &str,
    email: &str,
    status: &str,
    legal_name: &str,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO suppliers (id, cnpj, legal_name, contact_email, status) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(cnpj)
    .bind(legal_name)
    .bind(email)
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role, supplier_id) \
         VALUES ($1, $2, 'Titular', $3, 'FORNECEDOR', $4)",
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(hash_password("demo1234"))
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    id
}

pub async fn login(app: &TestApp, email: &str) -> String {
    let res = app
        .client
        .post(format!("{}/auth/login", app.base))
        .json(&serde_json::json!({ "email": email, "password": "demo1234" }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200, "login failed for {email}");
    let body: serde_json::Value = res.json().await.unwrap();
    body["token"].as_str().unwrap().to_string()
}
```

Append to `api/tests/integration.rs`:

```rust
#[tokio::test]
async fn login_rbac_and_query_token() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima")
        .await;

    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    let me: serde_json::Value = app
        .client
        .get(format!("{}/me", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["role"], "SERVIDOR");

    let wrong = app
        .client
        .post(format!("{}/auth/login", app.base))
        .json(&serde_json::json!({ "email": "servidor@tjrr.jus.br", "password": "nope" }))
        .send()
        .await
        .unwrap();
    assert_eq!(wrong.status(), 401);

    let anon = app.client.get(format!("{}/me", app.base)).send().await.unwrap();
    assert_eq!(anon.status(), 401);

    let via_query = app
        .client
        .get(format!("{}/me?token={staff_token}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(via_query.status(), 200, "query-param token must work (SSE + printable pages)");
}
```

- [ ] **Step 4: Run + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS — all previous + `login_rbac_and_query_token`.

```bash
git add api/src/auth.rs api/src/routes/auth.rs api/tests
git commit -m "feat(api): argon2 + jwt auth with header/query-token extractors and rbac"
```

---

### Task 4: Hash-chained audit trail (append + verify + endpoint)

**Files:**
- Fill: `api/src/audit.rs`
- Replace: `api/src/routes/reports.rs` (audit endpoints live here; Task 10 extends this file)
- Modify: `api/tests/integration.rs`

- [ ] **Step 1: Implement `api/src/audit.rs`**

```rust
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::ApiResult;

pub const GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// serde_json::Value objects are BTreeMaps by default, so to_string() is key-sorted
/// (canonical) on both write and re-read. Payload discipline: ints/strings/bools/null
/// only — floats would break jsonb round-trip determinism.
pub fn event_hash(prev_hash: &str, core: &Value) -> String {
    let canonical = serde_json::to_string(core).expect("serialize audit core");
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(b"|");
    hasher.update(canonical.as_bytes());
    hex::encode(hasher.finalize())
}

pub struct AuditInput<'a> {
    pub actor_id: Option<Uuid>,
    pub actor_role: Option<&'a str>,
    pub event_type: &'a str,
    pub entity: &'a str,
    pub entity_id: String,
    pub quotation_id: Option<Uuid>,
    pub payload: Value,
}

fn core_value(at: &str, input: &AuditInput) -> Value {
    json!({
        "at": at,
        "actorId": input.actor_id.map(|u| u.to_string()),
        "actorRole": input.actor_role,
        "type": input.event_type,
        "entity": input.entity,
        "entityId": input.entity_id,
        "quotationId": input.quotation_id.map(|u| u.to_string()),
        "payload": input.payload.clone(),
    })
}

/// jsonb round-trips do not preserve float formatting — a float in a payload
/// could make an untampered row verify as broken. Reject at append time.
fn assert_integer_payload(value: &Value) -> ApiResult<()> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
        Value::Number(n) => {
            if n.is_i64() || n.is_u64() {
                Ok(())
            } else {
                Err(crate::error::ApiError::Internal(
                    "audit payload must not contain floats".to_string(),
                ))
            }
        }
        Value::Array(items) => {
            for item in items {
                assert_integer_payload(item)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for item in map.values() {
                assert_integer_payload(item)?;
            }
            Ok(())
        }
    }
}

/// Appends inside the caller's transaction — use this to commit the audit entry
/// atomically with the business write it documents. Takes the global advisory
/// lock, so hold the transaction briefly.
pub async fn append_audit_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    input: AuditInput<'_>,
) -> ApiResult<()> {
    assert_integer_payload(&input.payload)?;
    sqlx::query("SET LOCAL lock_timeout = '5s'").execute(&mut **tx).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(4242)").execute(&mut **tx).await?;
    let prev_hash: String =
        sqlx::query_scalar("SELECT hash FROM audit_events ORDER BY seq DESC LIMIT 1")
            .fetch_optional(&mut **tx)
            .await?
            .unwrap_or_else(|| GENESIS_HASH.to_string());
    let at = Utc::now().to_rfc3339();
    let core = core_value(&at, &input);
    let hash = event_hash(&prev_hash, &core);
    sqlx::query(
        "INSERT INTO audit_events \
         (at, actor_id, actor_role, event_type, entity, entity_id, quotation_id, payload, prev_hash, hash) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(&at)
    .bind(input.actor_id)
    .bind(input.actor_role)
    .bind(input.event_type)
    .bind(input.entity)
    .bind(&input.entity_id)
    .bind(input.quotation_id)
    .bind(&input.payload)
    .bind(&prev_hash)
    .bind(&hash)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Standalone append (own transaction). Best-effort relative to any business
/// write that already committed — prefer append_audit_tx on hot paths where the
/// audit entry must be atomic with the write it documents.
pub async fn append_audit(pool: &PgPool, input: AuditInput<'_>) -> ApiResult<()> {
    let mut tx = pool.begin().await?;
    append_audit_tx(&mut tx, input).await?;
    tx.commit().await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
pub struct AuditRow {
    pub seq: i64,
    pub at: String,
    pub actor_id: Option<Uuid>,
    pub actor_role: Option<String>,
    pub event_type: String,
    pub entity: String,
    pub entity_id: String,
    pub quotation_id: Option<Uuid>,
    pub payload: Value,
    pub prev_hash: String,
    pub hash: String,
}

pub async fn list_events(pool: &PgPool, quotation_id: Option<Uuid>) -> ApiResult<Vec<AuditRow>> {
    let rows = match quotation_id {
        Some(qid) => {
            sqlx::query_as::<_, AuditRow>(
                "SELECT seq, at, actor_id, actor_role, event_type, entity, entity_id, \
                 quotation_id, payload, prev_hash, hash \
                 FROM audit_events WHERE quotation_id = $1 ORDER BY seq ASC",
            )
            .bind(qid)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, AuditRow>(
                "SELECT seq, at, actor_id, actor_role, event_type, entity, entity_id, \
                 quotation_id, payload, prev_hash, hash \
                 FROM audit_events ORDER BY seq ASC",
            )
            .fetch_all(pool)
            .await?
        }
    };
    Ok(rows)
}

pub async fn verify_chain(pool: &PgPool) -> ApiResult<Value> {
    let rows = list_events(pool, None).await?;
    let mut prev = GENESIS_HASH.to_string();
    for row in &rows {
        let core = json!({
            "at": row.at,
            "actorId": row.actor_id.map(|u| u.to_string()),
            "actorRole": row.actor_role,
            "type": row.event_type,
            "entity": row.entity,
            "entityId": row.entity_id,
            "quotationId": row.quotation_id.map(|u| u.to_string()),
            "payload": row.payload.clone(),
        });
        if event_hash(&prev, &core) != row.hash || row.prev_hash != prev {
            return Ok(json!({ "ok": false, "count": rows.len(), "brokenAtSeq": row.seq }));
        }
        prev = row.hash.clone();
    }
    Ok(json!({ "ok": true, "count": rows.len() }))
}
```

- [ ] **Step 2: Replace `api/src/routes/reports.rs` with the audit endpoints**

```rust
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::audit::{list_events, verify_chain};
use crate::auth::Staff;
use crate::error::ApiResult;
use crate::App;

async fn audit_verify(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    Ok(Json(verify_chain(&state.pool).await?))
}

async fn audit_events(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    let rows = list_events(&state.pool, None).await?;
    let events: Vec<Value> = rows
        .iter()
        .map(|e| {
            json!({
                "seq": e.seq, "at": e.at, "type": e.event_type, "entity": e.entity,
                "entityId": e.entity_id, "actorId": e.actor_id, "payload": e.payload
            })
        })
        .collect();
    Ok(Json(json!(events)))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/audit/verify", get(audit_verify))
        .route("/audit/events", get(audit_events))
}
```

- [ ] **Step 3: Add the integration test**

Append to `api/tests/integration.rs`:

```rust
use serde_json::json;
use tj_viagens_api::audit::{append_audit, AuditInput};

#[tokio::test]
async fn audit_chain_appends_verifies_and_detects_tampering() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima")
        .await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;

    for n in [1i64, 2] {
        append_audit(
            &app.pool,
            AuditInput {
                actor_id: None,
                actor_role: None,
                event_type: "TEST_EVENT",
                entity: "X",
                entity_id: n.to_string(),
                quotation_id: None,
                payload: json!({ "zeta": n, "flightInfo": "G3-1720 08:15 éão", "alpha": true, "note": null }),
            },
        )
        .await
        .unwrap();
    }

    let ok: serde_json::Value = app
        .client
        .get(format!("{}/audit/verify", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ok, json!({ "ok": true, "count": 2 }));

    // float payloads are rejected before they can poison the chain
    let float_err = append_audit(
        &app.pool,
        AuditInput {
            actor_id: None,
            actor_role: None,
            event_type: "BAD",
            entity: "X",
            entity_id: "f".to_string(),
            quotation_id: None,
            payload: json!({ "pct": 17.68 }),
        },
    )
    .await;
    assert!(float_err.is_err(), "float payload must be rejected");

    sqlx::query("UPDATE audit_events SET payload = '{\"n\": 999}'::jsonb WHERE seq = 2")
        .execute(&app.pool)
        .await
        .unwrap();
    let broken: serde_json::Value = app
        .client
        .get(format!("{}/audit/verify", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(broken["ok"], false);
    assert_eq!(broken["brokenAtSeq"], 2);

    let supplier_token = common::login(&app, "a@example.com").await;
    let denied = app
        .client
        .get(format!("{}/audit/verify", app.base))
        .bearer_auth(&supplier_token)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);
}
```

- [ ] **Step 4: Run + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS.

```bash
git add api/src/audit.rs api/src/routes/reports.rs api/tests/integration.rs
git commit -m "feat(api): append-only audit trail with sha256 chaining, verify endpoint"
```

---

### Task 5: Suppliers — registration, document upload, checklist, homologation, notifications

**Files:**
- Create: `api/src/uploads.rs` (add `pub mod uploads;` to `api/src/lib.rs`)
- Replace: `api/src/routes/suppliers.rs`
- Modify: `api/tests/common/mod.rs`, `api/tests/integration.rs`

> **As built (execution record — commits `9a7163e` + `c00f515`; supersedes the code blocks below where they differ):**
> `register` wraps both INSERTs in one tx and maps pg unique violations → 409 `JA_CADASTRADO` (closes the SELECT-then-INSERT race); password bounds are `chars().count() >= 8` and `len() <= 256` (`SENHA_LONGA`); `decide()` is atomic and single-shot (`SELECT … FOR UPDATE`, `UPDATE … AND status='PENDING'`, `append_audit_tx` + notification in the same tx — a double-click loses with 422 and exactly one audit row); `save_upload` keeps only `Path::file_name()` of the untrusted client filename; `my_notifications` uses a `NotificationRow` FromRow struct (clippy type_complexity). Integration adds: duplicate registration → 409, final `/audit/verify` ok, and `supplier_decision_is_atomic_and_single_shot` (concurrent decisions + traversal-filename neutralization).

- [ ] **Step 1: Implement `api/src/uploads.rs`**

```rust
use tokio::fs;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};

/// Saves multipart bytes under a uuid-prefixed name. Returns (original_name, path).
pub async fn save_upload(dir: &str, original_name: &str, bytes: &[u8]) -> ApiResult<(String, String)> {
    fs::create_dir_all(dir).await.map_err(|e| ApiError::Internal(e.to_string()))?;
    let file_name = original_name.to_string();
    let file_path = format!("{dir}/{}-{file_name}", Uuid::new_v4());
    fs::write(&file_path, bytes).await.map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok((file_name, file_path))
}
```

- [ ] **Step 2: Replace `api/src/routes/suppliers.rs`**

```rust
use std::collections::HashMap;

use axum::extract::{Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::NaiveDate;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::{append_audit, AuditInput};
use crate::auth::{hash_password, AuthUser, Claims, Staff};
use crate::domain::checklist::{checklist, ChecklistResult};
use crate::domain::cnpj::is_valid_cnpj;
use crate::domain::types::{DocType, Role};
use crate::error::{ApiError, ApiResult};
use crate::uploads::save_upload;
use crate::App;

pub fn require_supplier(claims: &Claims) -> Result<Uuid, ApiError> {
    match claims.role {
        Role::Fornecedor => claims.supplier_id.ok_or(ApiError::Forbidden("ACESSO_NEGADO")),
        Role::Admin | Role::Servidor => Err(ApiError::Forbidden("ACESSO_NEGADO")),
    }
}

#[derive(sqlx::FromRow)]
struct DocRow {
    doc_type: String,
    valid_until: Option<NaiveDate>,
}

pub async fn load_checklist(pool: &PgPool, supplier_id: Uuid) -> ApiResult<ChecklistResult> {
    let rows = sqlx::query_as::<_, DocRow>(
        "SELECT doc_type, valid_until FROM supplier_documents \
         WHERE supplier_id = $1 ORDER BY uploaded_at ASC",
    )
    .bind(supplier_id)
    .fetch_all(pool)
    .await?;
    let docs: Vec<(DocType, Option<NaiveDate>)> = rows
        .iter()
        .filter_map(|r| DocType::parse(&r.doc_type).map(|t| (t, r.valid_until)))
        .collect();
    Ok(checklist(&docs, Utc::now().date_naive()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RegisterBody {
    cnpj: String,
    legal_name: String,
    contact_email: String,
    phone: Option<String>,
    user_name: String,
    password: String,
}

async fn register(
    State(state): State<App>,
    Json(body): Json<RegisterBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    if !is_valid_cnpj(&body.cnpj) {
        return Err(ApiError::Unprocessable("CNPJ_INVALIDO"));
    }
    if body.password.len() < 8 {
        return Err(ApiError::Unprocessable("SENHA_CURTA"));
    }
    let cnpj: String = body.cnpj.chars().filter(|c| c.is_ascii_digit()).collect();
    let dup_supplier: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM suppliers WHERE cnpj = $1").bind(&cnpj)
            .fetch_optional(&state.pool).await?;
    let dup_user: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM users WHERE email = $1").bind(&body.contact_email)
            .fetch_optional(&state.pool).await?;
    if dup_supplier.is_some() || dup_user.is_some() {
        return Err(ApiError::Conflict("JA_CADASTRADO"));
    }
    let supplier_id = Uuid::new_v4();
    let mut tx = state.pool.begin().await?;
    sqlx::query(
        "INSERT INTO suppliers (id, cnpj, legal_name, contact_email, phone) \
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(supplier_id).bind(&cnpj).bind(&body.legal_name).bind(&body.contact_email).bind(&body.phone)
    .execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role, supplier_id) \
         VALUES ($1,$2,$3,$4,'FORNECEDOR',$5)",
    )
    .bind(Uuid::new_v4()).bind(&body.contact_email).bind(&body.user_name)
    .bind(hash_password(&body.password)).bind(supplier_id)
    .execute(&mut *tx).await?;
    tx.commit().await?;
    append_audit(&state.pool, AuditInput {
        actor_id: None,
        actor_role: None,
        event_type: "SUPPLIER_REGISTERED",
        entity: "Supplier",
        entity_id: supplier_id.to_string(),
        quotation_id: None,
        payload: json!({ "cnpj": cnpj, "legalName": body.legal_name }),
    }).await?;
    Ok((StatusCode::CREATED, Json(json!({ "supplierId": supplier_id }))))
}

#[derive(sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SupplierRow {
    id: Uuid,
    cnpj: String,
    legal_name: String,
    contact_email: String,
    phone: Option<String>,
    status: String,
    status_reason: Option<String>,
}

async fn me(State(state): State<App>, AuthUser(claims): AuthUser) -> ApiResult<Json<Value>> {
    let supplier_id = require_supplier(&claims)?;
    let supplier = sqlx::query_as::<_, SupplierRow>(
        "SELECT id, cnpj, legal_name, contact_email, phone, status, status_reason \
         FROM suppliers WHERE id = $1",
    )
    .bind(supplier_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound("NAO_ENCONTRADO"))?;
    let check = load_checklist(&state.pool, supplier_id).await?;
    Ok(Json(json!({ "supplier": supplier, "checklist": check })))
}

async fn upload_document(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let supplier_id = require_supplier(&claims)?;
    let mut fields: HashMap<String, String> = HashMap::new();
    let mut saved: Option<(String, String)> = None;
    while let Some(field) =
        multipart.next_field().await.map_err(|e| ApiError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let original = field.file_name().unwrap_or("upload.bin").to_string();
            let bytes = field.bytes().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            saved = Some(save_upload(&state.config.upload_dir, &original, &bytes).await?);
        } else {
            let value = field.text().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            fields.insert(name, value);
        }
    }
    let doc_type = fields
        .get("type")
        .and_then(|t| DocType::parse(t))
        .ok_or(ApiError::Unprocessable("DOCUMENTO_INVALIDO"))?;
    let Some((file_name, file_path)) = saved else {
        return Err(ApiError::Unprocessable("DOCUMENTO_INVALIDO"));
    };
    let valid_until: Option<NaiveDate> = fields.get("validUntil").and_then(|v| v.parse().ok());
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO supplier_documents (id, supplier_id, doc_type, file_name, file_path, valid_until) \
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(id).bind(supplier_id).bind(doc_type.as_str()).bind(&file_name).bind(&file_path).bind(valid_until)
    .execute(&state.pool).await?;
    append_audit(&state.pool, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "DOCUMENT_UPLOADED",
        entity: "SupplierDocument",
        entity_id: id.to_string(),
        quotation_id: None,
        payload: json!({ "docType": doc_type.as_str(), "validUntil": fields.get("validUntil") }),
    }).await?;
    Ok((StatusCode::CREATED, Json(json!({ "id": id, "type": doc_type.as_str(), "fileName": file_name }))))
}

#[derive(Deserialize)]
struct ListQuery {
    status: Option<String>,
}

async fn list_suppliers(
    State(state): State<App>,
    Staff(_claims): Staff,
    Query(query): Query<ListQuery>,
) -> ApiResult<Json<Value>> {
    let rows = match &query.status {
        Some(status) => sqlx::query_as::<_, SupplierRow>(
            "SELECT id, cnpj, legal_name, contact_email, phone, status, status_reason \
             FROM suppliers WHERE status = $1 ORDER BY created_at ASC",
        )
        .bind(status)
        .fetch_all(&state.pool)
        .await?,
        None => sqlx::query_as::<_, SupplierRow>(
            "SELECT id, cnpj, legal_name, contact_email, phone, status, status_reason \
             FROM suppliers ORDER BY created_at ASC",
        )
        .fetch_all(&state.pool)
        .await?,
    };
    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let check = load_checklist(&state.pool, row.id).await?;
        result.push(json!({ "supplier": row, "checklist": check }));
    }
    Ok(Json(json!(result)))
}

#[derive(Deserialize)]
struct DecisionBody {
    decision: String,
    reason: Option<String>,
}

async fn decide(
    State(state): State<App>,
    Staff(claims): Staff,
    Path(id): Path<Uuid>,
    Json(body): Json<DecisionBody>,
) -> ApiResult<Json<Value>> {
    let current: Option<(String,)> =
        sqlx::query_as("SELECT status FROM suppliers WHERE id = $1").bind(id)
            .fetch_optional(&state.pool).await?;
    let Some((status,)) = current else { return Err(ApiError::NotFound("NAO_ENCONTRADO")) };
    if status != "PENDING" {
        return Err(ApiError::Unprocessable("JA_DECIDIDO"));
    }
    let approve = match body.decision.as_str() {
        "APPROVE" => true,
        "REJECT" => false,
        _ => return Err(ApiError::Unprocessable("DECISAO_INVALIDA")),
    };
    if approve {
        let check = load_checklist(&state.pool, id).await?;
        if !check.ok {
            return Err(ApiError::UnprocessableWith(
                "CHECKLIST_PENDENTE",
                serde_json::to_value(&check).expect("serialize checklist"),
            ));
        }
    }
    let new_status = if approve { "ACTIVE" } else { "REJECTED" };
    sqlx::query(
        "UPDATE suppliers SET status = $1, status_reason = $2, decided_at = now(), decided_by = $3 \
         WHERE id = $4",
    )
    .bind(new_status).bind(&body.reason).bind(claims.sub).bind(id)
    .execute(&state.pool).await?;
    append_audit(&state.pool, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: if approve { "SUPPLIER_APPROVED" } else { "SUPPLIER_REJECTED" },
        entity: "Supplier",
        entity_id: id.to_string(),
        quotation_id: None,
        payload: json!({ "reason": body.reason }),
    }).await?;
    let message = if approve {
        "Credenciamento aprovado. Você já pode participar de cotações.".to_string()
    } else {
        format!("Credenciamento rejeitado: {}", body.reason.as_deref().unwrap_or("sem justificativa"))
    };
    sqlx::query("INSERT INTO notifications (id, supplier_id, kind, message) VALUES ($1,$2,'CREDENCIAMENTO',$3)")
        .bind(Uuid::new_v4()).bind(id).bind(&message)
        .execute(&state.pool).await?;
    Ok(Json(json!({ "id": id, "status": new_status })))
}

async fn my_notifications(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
) -> ApiResult<Json<Value>> {
    let supplier_id = require_supplier(&claims)?;
    let rows: Vec<(Uuid, Option<Uuid>, String, String, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, quotation_id, kind, message, created_at FROM notifications \
         WHERE supplier_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(supplier_id)
    .fetch_all(&state.pool)
    .await?;
    let items: Vec<Value> = rows
        .iter()
        .map(|(id, quotation_id, kind, message, created_at)| {
            json!({
                "id": id, "quotationId": quotation_id, "kind": kind,
                "message": message, "createdAt": created_at.to_rfc3339()
            })
        })
        .collect();
    Ok(Json(json!(items)))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/suppliers/register", post(register))
        .route("/suppliers/me", get(me))
        .route("/suppliers/me/documents", post(upload_document))
        .route("/suppliers", get(list_suppliers))
        .route("/suppliers/{id}/decision", post(decide))
        .route("/notifications", get(my_notifications))
}
```

(Axum 0.8 path params use `{id}` syntax, not `:id`.)

- [ ] **Step 3: Add the credenciamento flow test**

Append to `api/tests/common/mod.rs`:

```rust
/// Registers a supplier through the HTTP API, uploads all 4 required docs, returns supplier id.
pub async fn register_with_docs(app: &TestApp, cnpj: &str, email: &str, name: &str) -> String {
    let reg: serde_json::Value = app
        .client
        .post(format!("{}/suppliers/register", app.base))
        .json(&serde_json::json!({
            "cnpj": cnpj, "legalName": name, "contactEmail": email,
            "userName": "Titular", "password": "demo1234"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let supplier_id = reg["supplierId"].as_str().unwrap().to_string();
    let token = login(app, email).await;
    for doc_type in ["CONTRATO_SOCIAL", "CND_FEDERAL", "CRF_FGTS", "CNDT"] {
        let form = reqwest::multipart::Form::new()
            .text("type", doc_type)
            .text("validUntil", "2027-12-31")
            .part(
                "file",
                reqwest::multipart::Part::bytes(b"%PDF-1.4 fake".to_vec()).file_name("doc.pdf"),
            );
        let res = app
            .client
            .post(format!("{}/suppliers/me/documents", app.base))
            .bearer_auth(&token)
            .multipart(form)
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 201, "doc upload failed: {doc_type}");
    }
    supplier_id
}
```

Append to `api/tests/integration.rs`:

```rust
#[tokio::test]
async fn credenciamento_register_docs_checklist_and_homologation() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;

    let bad = app
        .client
        .post(format!("{}/suppliers/register", app.base))
        .json(&json!({
            "cnpj": "11.222.333/0001-82", "legalName": "X", "contactEmail": "x@example.com",
            "userName": "X Y", "password": "demo1234"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 422, "wrong check digit must be rejected");

    let supplier_id =
        common::register_with_docs(&app, "11.222.333/0001-81", "contato@voaroraima.com.br", "Voa Roraima Turismo").await;

    let token = common::login(&app, "contato@voaroraima.com.br").await;
    let me: serde_json::Value = app
        .client
        .get(format!("{}/suppliers/me", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(me["supplier"]["status"], "PENDING");
    assert_eq!(me["checklist"]["ok"], true);

    // approve without docs must fail for another supplier
    let no_docs = app
        .client
        .post(format!("{}/suppliers/register", app.base))
        .json(&json!({
            "cnpj": "11.444.777/0001-61", "legalName": "Sem Docs", "contactEmail": "semdocs@example.com",
            "userName": "Titular", "password": "demo1234"
        }))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let refused = app
        .client
        .post(format!("{}/suppliers/{}/decision", app.base, no_docs["supplierId"].as_str().unwrap()))
        .bearer_auth(&staff_token)
        .json(&json!({ "decision": "APPROVE" }))
        .send()
        .await
        .unwrap();
    assert_eq!(refused.status(), 422);
    assert_eq!(refused.json::<serde_json::Value>().await.unwrap()["error"], "CHECKLIST_PENDENTE");

    // approve the complete one
    let approved = app
        .client
        .post(format!("{}/suppliers/{supplier_id}/decision", app.base))
        .bearer_auth(&staff_token)
        .json(&json!({ "decision": "APPROVE" }))
        .send()
        .await
        .unwrap();
    assert_eq!(approved.status(), 200);
    assert_eq!(approved.json::<serde_json::Value>().await.unwrap()["status"], "ACTIVE");

    // supplier is notified in the panel
    let notifications: serde_json::Value = app
        .client
        .get(format!("{}/notifications", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(notifications[0]["kind"], "CREDENCIAMENTO");

    // staff-only listing blocked for suppliers
    let denied = app
        .client
        .get(format!("{}/suppliers", app.base))
        .bearer_auth(&token)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);
}
```

- [ ] **Step 4: Run + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS.

```bash
git add api/src/uploads.rs api/src/lib.rs api/src/routes/suppliers.rs api/tests
git commit -m "feat(api): supplier registration, doc upload, checklist pre-triage and homologation"
```

---

### Task 6: Quotations — create, open+notify, redacted views, SSE broker

> **As built (execution record — commits `6bf635c` + `c40a84e`; supersedes the blocks below where they differ):**
> `open()` is atomic and single-shot: guarded `UPDATE … AND status='DRAFT'` (0 rows → 422), notifications + `append_audit_tx` in the SAME tx, mail println + SSE publish only after commit. Notification copy uses the new `domain::timefmt::fmt_boa_vista` (dd/mm/YYYY HH:mm, "horário de Boa Vista") — never raw UTC. `load_quotation`'s lazy close guards `AND status='OPEN'` and audits/publishes only when `rows_affected()==1` (concurrent readers produce exactly one QUOTATION_CLOSED row). New test `lazy_close_audits_exactly_once_and_open_is_single_shot`.

**Files:**
- Fill: `api/src/sse.rs` (broker; HTTP route comes in Task 11), `api/src/routes/views.rs`
- Replace: `api/src/routes/quotations.rs`
- Modify: `api/tests/common/mod.rs`, `api/tests/integration.rs`

- [ ] **Step 1: Replace `api/src/sse.rs`**

```rust
use serde_json::Value;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::App;

#[derive(Clone, Debug)]
pub struct SseMsg {
    pub event: String,
    pub data: String,
}

pub fn channel_for(state: &App, quotation_id: Uuid) -> broadcast::Sender<SseMsg> {
    let mut channels = state.channels.lock().expect("channels lock");
    channels.entry(quotation_id).or_insert_with(|| broadcast::channel(64).0).clone()
}

/// R5 discipline: publish only status transitions and proposal COUNTS — never bid values.
pub fn publish(state: &App, quotation_id: Uuid, event: &str, data: Value) {
    let sender = channel_for(state, quotation_id);
    let _ = sender.send(SseMsg { event: event.to_string(), data: data.to_string() });
}
```

- [ ] **Step 2: Implement `api/src/routes/views.rs` (the ONLY quotation serializers)**

```rust
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::domain::types::QuotationStatus;

#[derive(sqlx::FromRow, Clone)]
pub struct QuotationRow {
    pub id: Uuid,
    pub code: String,
    pub status: String,
    pub passenger_name: String,
    pub passenger_cpf: String,
    pub passenger_sex: String,
    pub passenger_birth: chrono::NaiveDate,
    pub origin: String,
    pub destination: String,
    pub departure_at: DateTime<Utc>,
    pub return_at: Option<DateTime<Utc>>,
    pub reference_flight: String,
    pub reference_price_cents: i64,
    pub opens_at: Option<DateTime<Utc>>,
    pub closes_at: Option<DateTime<Utc>>,
    pub awarded_proposal_id: Option<Uuid>,
    pub awarded_at: Option<DateTime<Utc>>,
    pub award_justification: Option<String>,
    pub ticket_deadline_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow, Clone)]
pub struct ProposalRow {
    pub id: Uuid,
    pub quotation_id: Uuid,
    pub supplier_id: Uuid,
    pub total_price_cents: i64,
    pub flight_info: String,
    pub notes: Option<String>,
    pub submitted_at: DateTime<Utc>,
}

/// R4: server clock decides. OPEN past closes_at behaves as CLOSED.
pub fn effective_status(
    status: &str,
    closes_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> QuotationStatus {
    let parsed = QuotationStatus::parse(status).unwrap_or(QuotationStatus::Draft);
    match parsed {
        QuotationStatus::Open => match closes_at {
            Some(deadline) if now >= deadline => QuotationStatus::Closed,
            Some(_) | None => QuotationStatus::Open,
        },
        QuotationStatus::Draft
        | QuotationStatus::Closed
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => parsed,
    }
}

fn iso(d: Option<DateTime<Utc>>) -> Value {
    match d {
        Some(v) => json!(v.to_rfc3339()),
        None => Value::Null,
    }
}

fn base(q: &QuotationRow, status: QuotationStatus, now: DateTime<Utc>) -> Value {
    json!({
        "id": q.id, "code": q.code, "status": status.as_str(),
        "origin": q.origin, "destination": q.destination,
        "departureAt": q.departure_at.to_rfc3339(), "returnAt": iso(q.return_at),
        "referenceFlight": q.reference_flight,
        "opensAt": iso(q.opens_at), "closesAt": iso(q.closes_at),
        "serverNow": now.to_rfc3339(),
    })
}

fn passenger(q: &QuotationRow) -> Value {
    json!({
        "name": q.passenger_name, "cpf": q.passenger_cpf,
        "sex": q.passenger_sex, "birth": q.passenger_birth.format("%Y-%m-%d").to_string()
    })
}

fn proposal_json(p: &ProposalRow) -> Value {
    json!({
        "id": p.id, "supplierId": p.supplier_id, "totalPriceCents": p.total_price_cents,
        "flightInfo": p.flight_info, "notes": p.notes, "submittedAt": p.submitted_at.to_rfc3339()
    })
}

/// Staff see everything — but while OPEN, proposals collapse to a count (sealed bids, R5).
pub fn staff_view(q: &QuotationRow, proposals: &[ProposalRow], now: DateTime<Utc>) -> Value {
    let status = effective_status(&q.status, q.closes_at, now);
    let mut view = base(q, status, now);
    let obj = view.as_object_mut().expect("base is object");
    obj.insert("passenger".into(), passenger(q));
    obj.insert("referencePriceCents".into(), json!(q.reference_price_cents));
    obj.insert("awardedProposalId".into(), json!(q.awarded_proposal_id));
    obj.insert("awardedAt".into(), iso(q.awarded_at));
    obj.insert("awardJustification".into(), json!(q.award_justification));
    obj.insert("ticketDeadlineAt".into(), iso(q.ticket_deadline_at));
    let proposals_value = match status {
        QuotationStatus::Open => json!({ "count": proposals.len() }),
        QuotationStatus::Draft
        | QuotationStatus::Closed
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => json!(proposals.iter().map(proposal_json).collect::<Vec<_>>()),
    };
    obj.insert("proposals".into(), proposals_value);
    view
}

/// R2/R5: suppliers never see reference price, rival bids, or passenger PII.
/// The winner gains passenger data + ticket deadline after award (needed to emit the ticket).
pub fn supplier_view(
    q: &QuotationRow,
    proposals: &[ProposalRow],
    supplier_id: Uuid,
    now: DateTime<Utc>,
) -> Value {
    let status = effective_status(&q.status, q.closes_at, now);
    let own = proposals.iter().find(|p| p.supplier_id == supplier_id);
    let is_winner = matches!((own, q.awarded_proposal_id), (Some(p), Some(w)) if p.id == w);
    let mut view = base(q, status, now);
    let obj = view.as_object_mut().expect("base is object");
    obj.insert(
        "myProposal".into(),
        match own {
            Some(p) => proposal_json(p),
            None => Value::Null,
        },
    );
    obj.insert("isWinner".into(), json!(is_winner));
    if is_winner {
        obj.insert("passenger".into(), passenger(q));
        obj.insert("ticketDeadlineAt".into(), iso(q.ticket_deadline_at));
    }
    view
}
```

- [ ] **Step 3: Replace `api/src/routes/quotations.rs`**

```rust
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::audit::{append_audit, AuditInput};
use crate::auth::{AuthUser, Claims, Staff};
use crate::domain::types::{QuotationStatus, Role, SupplierStatus};
use crate::error::{ApiError, ApiResult};
use crate::sse::publish;
use crate::App;

use super::suppliers::require_supplier;
use super::views::{effective_status, staff_view, supplier_view, ProposalRow, QuotationRow};

pub const QUOTATION_COLUMNS: &str =
    "id, code, status, passenger_name, passenger_cpf, passenger_sex, passenger_birth, \
     origin, destination, departure_at, return_at, reference_flight, reference_price_cents, \
     opens_at, closes_at, awarded_proposal_id, awarded_at, award_justification, \
     ticket_deadline_at, created_by, created_at";

pub async fn fetch_quotation(pool: &PgPool, id: Uuid) -> ApiResult<Option<QuotationRow>> {
    Ok(sqlx::query_as::<_, QuotationRow>(&format!(
        "SELECT {QUOTATION_COLUMNS} FROM quotations WHERE id = $1"
    ))
    .bind(id)
    .fetch_optional(pool)
    .await?)
}

/// Always ordered (price asc, submitted_at asc) — this IS the ranking order (R6).
pub async fn fetch_proposals(pool: &PgPool, quotation_id: Uuid) -> ApiResult<Vec<ProposalRow>> {
    Ok(sqlx::query_as::<_, ProposalRow>(
        "SELECT id, quotation_id, supplier_id, total_price_cents, flight_info, notes, submitted_at \
         FROM proposals WHERE quotation_id = $1 \
         ORDER BY total_price_cents ASC, submitted_at ASC",
    )
    .bind(quotation_id)
    .fetch_all(pool)
    .await?)
}

/// Lazily persists OPEN -> CLOSED once the server clock passes closes_at (R4).
pub async fn load_quotation(
    state: &App,
    id: Uuid,
    now: DateTime<Utc>,
) -> ApiResult<Option<QuotationRow>> {
    let Some(q) = fetch_quotation(&state.pool, id).await? else { return Ok(None) };
    if q.status == "OPEN"
        && effective_status(&q.status, q.closes_at, now) == QuotationStatus::Closed
    {
        sqlx::query("UPDATE quotations SET status = 'CLOSED' WHERE id = $1")
            .bind(id)
            .execute(&state.pool)
            .await?;
        append_audit(&state.pool, AuditInput {
            actor_id: None,
            actor_role: None,
            event_type: "QUOTATION_CLOSED",
            entity: "Quotation",
            entity_id: id.to_string(),
            quotation_id: Some(id),
            payload: json!({ "closesAt": q.closes_at.map(|d| d.to_rfc3339()) }),
        })
        .await?;
        publish(state, id, "status", json!({ "status": "CLOSED" }));
        return fetch_quotation(&state.pool, id).await;
    }
    Ok(Some(q))
}

/// Atomic sequential codes: COT-2026-0001, OS-2026-0001.
pub async fn next_code(pool: &PgPool, prefix: &str) -> ApiResult<String> {
    let key = format!("{prefix}-{}", Utc::now().format("%Y"));
    let value: i64 = sqlx::query_scalar(
        "INSERT INTO counters (id, value) VALUES ($1, 1) \
         ON CONFLICT (id) DO UPDATE SET value = counters.value + 1 RETURNING value",
    )
    .bind(&key)
    .fetch_one(pool)
    .await?;
    Ok(format!("{key}-{value:04}"))
}

pub async fn require_active_supplier(pool: &PgPool, claims: &Claims) -> ApiResult<Uuid> {
    let supplier_id = require_supplier(claims)?;
    let status: Option<(String,)> = sqlx::query_as("SELECT status FROM suppliers WHERE id = $1")
        .bind(supplier_id)
        .fetch_optional(pool)
        .await?;
    match status.and_then(|(s,)| SupplierStatus::parse(&s)) {
        Some(SupplierStatus::Active) => Ok(supplier_id),
        Some(SupplierStatus::Pending)
        | Some(SupplierStatus::Rejected)
        | Some(SupplierStatus::Suspended)
        | None => Err(ApiError::Forbidden("FORNECEDOR_NAO_ATIVO")),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateBody {
    passenger_name: String,
    passenger_cpf: String,
    passenger_sex: String,
    passenger_birth: NaiveDate,
    origin: String,
    destination: String,
    departure_at: DateTime<Utc>,
    return_at: Option<DateTime<Utc>>,
    reference_flight: String,
    reference_price_cents: i64,
}

async fn create(
    State(state): State<App>,
    Staff(claims): Staff,
    Json(body): Json<CreateBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let cpf: String = body.passenger_cpf.chars().filter(|c| c.is_ascii_digit()).collect();
    if cpf.len() != 11 {
        return Err(ApiError::Unprocessable("CPF_INVALIDO"));
    }
    if !matches!(body.passenger_sex.as_str(), "F" | "M" | "O") {
        return Err(ApiError::Unprocessable("SEXO_INVALIDO"));
    }
    if body.reference_price_cents <= 0 {
        return Err(ApiError::Unprocessable("PRECO_INVALIDO"));
    }
    let id = Uuid::new_v4();
    let code = next_code(&state.pool, "COT").await?;
    sqlx::query(
        "INSERT INTO quotations \
         (id, code, passenger_name, passenger_cpf, passenger_sex, passenger_birth, \
          origin, destination, departure_at, return_at, reference_flight, \
          reference_price_cents, created_by) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
    )
    .bind(id).bind(&code).bind(&body.passenger_name).bind(&cpf).bind(&body.passenger_sex)
    .bind(body.passenger_birth).bind(&body.origin).bind(&body.destination)
    .bind(body.departure_at).bind(body.return_at).bind(&body.reference_flight)
    .bind(body.reference_price_cents).bind(claims.sub)
    .execute(&state.pool)
    .await?;
    append_audit(&state.pool, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "QUOTATION_CREATED",
        entity: "Quotation",
        entity_id: id.to_string(),
        quotation_id: Some(id),
        payload: json!({ "code": code }),
    })
    .await?;
    let q = fetch_quotation(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("quotation vanished after insert".into()))?;
    Ok((StatusCode::CREATED, Json(staff_view(&q, &[], Utc::now()))))
}

async fn open(
    State(state): State<App>,
    Staff(claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let Some(q) = fetch_quotation(&state.pool, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    if q.status != "DRAFT" {
        return Err(ApiError::Unprocessable("NAO_ESTA_EM_RASCUNHO"));
    }
    let now = Utc::now();
    let closes_at = now + Duration::minutes(state.config.proposal_window_minutes);
    sqlx::query("UPDATE quotations SET status = 'OPEN', opens_at = $1, closes_at = $2 WHERE id = $3")
        .bind(now).bind(closes_at).bind(id)
        .execute(&state.pool)
        .await?;
    // R3: simultaneous notification of every ACTIVE supplier. Message NEVER contains the reference price.
    let active: Vec<(Uuid, String)> =
        sqlx::query_as("SELECT id, contact_email FROM suppliers WHERE status = 'ACTIVE'")
            .fetch_all(&state.pool)
            .await?;
    let message = format!(
        "Nova cotação {}: {} → {}, embarque {}. Propostas até {}.",
        q.code, q.origin, q.destination, q.departure_at.to_rfc3339(), closes_at.to_rfc3339()
    );
    for (supplier_id, email) in &active {
        sqlx::query(
            "INSERT INTO notifications (id, supplier_id, quotation_id, kind, message) \
             VALUES ($1,$2,$3,'COTACAO_ABERTA',$4)",
        )
        .bind(Uuid::new_v4()).bind(supplier_id).bind(id).bind(&message)
        .execute(&state.pool)
        .await?;
        // Console mail adapter — swap for institutional SMTP without touching callers.
        println!("[mail] to={email} subject=\"Cotação {} aberta\"", q.code);
    }
    append_audit(&state.pool, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "QUOTATION_OPENED",
        entity: "Quotation",
        entity_id: id.to_string(),
        quotation_id: Some(id),
        payload: json!({
            "code": q.code,
            "closesAt": closes_at.to_rfc3339(),
            "notified": active.len()
        }),
    })
    .await?;
    publish(&state, id, "status", json!({ "status": "OPEN", "closesAt": closes_at.to_rfc3339() }));
    let q = fetch_quotation(&state.pool, id)
        .await?
        .ok_or_else(|| ApiError::Internal("quotation vanished after open".into()))?;
    Ok(Json(staff_view(&q, &[], now)))
}

async fn list(State(state): State<App>, AuthUser(claims): AuthUser) -> ApiResult<Json<Value>> {
    let now = Utc::now();
    match claims.role {
        Role::Fornecedor => {
            let supplier_id = require_active_supplier(&state.pool, &claims).await?;
            let rows = sqlx::query_as::<_, QuotationRow>(&format!(
                "SELECT {QUOTATION_COLUMNS} FROM quotations \
                 WHERE status <> 'DRAFT' AND (status = 'OPEN' OR id IN \
                   (SELECT quotation_id FROM proposals WHERE supplier_id = $1)) \
                 ORDER BY created_at DESC"
            ))
            .bind(supplier_id)
            .fetch_all(&state.pool)
            .await?;
            let mut result = Vec::new();
            for q in &rows {
                let proposals = fetch_proposals(&state.pool, q.id).await?;
                result.push(supplier_view(q, &proposals, supplier_id, now));
            }
            Ok(Json(json!(result)))
        }
        Role::Admin | Role::Servidor => {
            let rows = sqlx::query_as::<_, QuotationRow>(&format!(
                "SELECT {QUOTATION_COLUMNS} FROM quotations ORDER BY created_at DESC"
            ))
            .fetch_all(&state.pool)
            .await?;
            let mut result = Vec::new();
            for q in &rows {
                let proposals = fetch_proposals(&state.pool, q.id).await?;
                result.push(staff_view(q, &proposals, now));
            }
            Ok(Json(json!(result)))
        }
    }
}

async fn detail(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let now = Utc::now();
    let Some(q) = load_quotation(&state, id, now).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    let proposals = fetch_proposals(&state.pool, id).await?;
    match claims.role {
        Role::Fornecedor => {
            let supplier_id = require_active_supplier(&state.pool, &claims).await?;
            Ok(Json(supplier_view(&q, &proposals, supplier_id, now)))
        }
        Role::Admin | Role::Servidor => Ok(Json(staff_view(&q, &proposals, now))),
    }
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/quotations", post(create).get(list))
        .route("/quotations/{id}", get(detail))
        .route("/quotations/{id}/open", post(open))
}
```

- [ ] **Step 4: Test helpers + redaction/notification test**

Append to `api/tests/common/mod.rs`:

```rust
pub fn quotation_payload() -> serde_json::Value {
    serde_json::json!({
        "passengerName": "Maria da Silva",
        "passengerCpf": "123.456.789-09",
        "passengerSex": "F",
        "passengerBirth": "1985-04-12",
        "origin": "BVB",
        "destination": "BSB",
        "departureAt": "2026-09-10T08:00:00Z",
        "referenceFlight": "LA-4001",
        "referencePriceCents": 185000
    })
}

pub async fn create_open_quotation(app: &TestApp, staff_token: &str) -> String {
    let created: serde_json::Value = app
        .client
        .post(format!("{}/quotations", app.base))
        .bearer_auth(staff_token)
        .json(&quotation_payload())
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = created["id"].as_str().unwrap().to_string();
    let opened = app
        .client
        .post(format!("{}/quotations/{id}/open", app.base))
        .bearer_auth(staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(opened.status(), 200, "open failed");
    id
}

pub async fn time_travel_past_close(pool: &PgPool, quotation_id: &str) {
    sqlx::query("UPDATE quotations SET closes_at = now() - interval '1 second' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(quotation_id).unwrap())
        .execute(pool)
        .await
        .unwrap();
}
```

Append to `api/tests/integration.rs`:

```rust
#[tokio::test]
async fn quotation_open_notifies_active_suppliers_and_redacts_for_suppliers() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    common::create_supplier(&app.pool, "11444777000161", "b@example.com", "ACTIVE", "Amazônia Viagens").await;
    common::create_supplier(&app.pool, "12345678000195", "p@example.com", "PENDING", "Pendente Tur").await;

    let id = common::create_open_quotation(&app, &staff_token).await;

    // only the 2 ACTIVE suppliers were notified (R3)
    let notified: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE kind = 'COTACAO_ABERTA'")
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(notified, 2);

    // staff view: reference price + passenger + proposal count
    let staff_json: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(staff_json["code"], "COT-2026-0001");
    assert_eq!(staff_json["referencePriceCents"], 185000);
    assert_eq!(staff_json["proposals"], json!({ "count": 0 }));

    // supplier view: NO reference price, NO passenger PII, NO rival proposals (R2/R5)
    let supplier_token = common::login(&app, "a@example.com").await;
    let raw = app
        .client
        .get(format!("{}/quotations/{id}", app.base))
        .bearer_auth(&supplier_token)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(!raw.contains("185000"), "reference price leaked: {raw}");
    assert!(!raw.contains("Maria"), "passenger PII leaked: {raw}");
    assert!(!raw.contains("referencePriceCents"));
    let supplier_json: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(supplier_json["myProposal"], serde_json::Value::Null);

    // PENDING supplier is blocked entirely
    let pending_token = common::login(&app, "p@example.com").await;
    let denied = app
        .client
        .get(format!("{}/quotations/{id}", app.base))
        .bearer_auth(&pending_token)
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);
}
```

- [ ] **Step 5: Run + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS.

```bash
git add api/src/sse.rs api/src/routes/views.rs api/src/routes/quotations.rs api/tests
git commit -m "feat(api): quotations with secret reference price, active-supplier notify and redacted views"
```

---

### Task 7: Proposals — blind bids under the server-controlled window

**Files:**
- Replace: `api/src/routes/proposals.rs`
- Modify: `api/tests/integration.rs`

- [ ] **Step 1: Replace `api/src/routes/proposals.rs`**

```rust
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::audit::{append_audit_tx, AuditInput};
use crate::auth::AuthUser;
use crate::domain::types::QuotationStatus;
use crate::error::{ApiError, ApiResult};
use crate::sse::publish;
use crate::App;

use super::quotations::{load_quotation, require_active_supplier};
use super::views::effective_status;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProposalBody {
    total_price_cents: i64,
    flight_info: String,
    notes: Option<String>,
}

async fn submit(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<ProposalBody>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let supplier_id = require_active_supplier(&state.pool, &claims).await?;
    if body.total_price_cents <= 0 {
        return Err(ApiError::Unprocessable("PRECO_INVALIDO"));
    }
    if body.flight_info.trim().len() < 2 {
        return Err(ApiError::Unprocessable("VOO_INVALIDO"));
    }
    let now = Utc::now();
    let Some(q) = load_quotation(&state, id, now).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    match effective_status(&q.status, q.closes_at, now) {
        QuotationStatus::Open => {}
        QuotationStatus::Draft
        | QuotationStatus::Closed
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => return Err(ApiError::Unprocessable("COTACAO_FECHADA")),
    }
    // Replace-while-open semantics: one row per supplier, first submitted_at preserved.
    // Bid + audit entry commit ATOMICALLY — a bid can never exist without its trail row.
    let mut tx = state.pool.begin().await?;
    let (proposal_id, submitted_at): (Uuid, DateTime<Utc>) = sqlx::query_as(
        "INSERT INTO proposals (id, quotation_id, supplier_id, total_price_cents, flight_info, notes) \
         VALUES ($1,$2,$3,$4,$5,$6) \
         ON CONFLICT (quotation_id, supplier_id) DO UPDATE \
         SET total_price_cents = EXCLUDED.total_price_cents, \
             flight_info = EXCLUDED.flight_info, \
             notes = EXCLUDED.notes, \
             updated_at = now() \
         RETURNING id, submitted_at",
    )
    .bind(Uuid::new_v4()).bind(id).bind(supplier_id)
    .bind(body.total_price_cents).bind(&body.flight_info).bind(&body.notes)
    .fetch_one(&mut *tx)
    .await?;
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "PROPOSAL_SUBMITTED",
        entity: "Proposal",
        entity_id: proposal_id.to_string(),
        quotation_id: Some(id),
        payload: json!({ "totalPriceCents": body.total_price_cents, "flightInfo": body.flight_info }),
    })
    .await?;
    tx.commit().await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proposals WHERE quotation_id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    // Sealed bids: the live event carries only the COUNT.
    publish(&state, id, "proposal", json!({ "count": count }));
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "id": proposal_id,
            "totalPriceCents": body.total_price_cents,
            "submittedAt": submitted_at.to_rfc3339()
        })),
    ))
}

pub fn router() -> Router<App> {
    Router::new().route("/quotations/{id}/proposals", post(submit))
}
```

- [ ] **Step 2: Add the window/concurrency test**

Append to `api/tests/integration.rs`:

```rust
#[tokio::test]
async fn proposals_concurrent_bids_replacement_and_window_enforcement() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    common::create_supplier(&app.pool, "11444777000161", "b@example.com", "ACTIVE", "Amazônia Viagens").await;
    common::create_supplier(&app.pool, "12345678000195", "c@example.com", "ACTIVE", "Rio Branco Tur").await;
    let id = common::create_open_quotation(&app, &staff_token).await;

    // simultaneous blind bids
    let tokens = [
        common::login(&app, "a@example.com").await,
        common::login(&app, "b@example.com").await,
        common::login(&app, "c@example.com").await,
    ];
    let prices = [152300i64, 149900, 158000];
    let bids = futures::future::join_all(tokens.iter().zip(prices).map(|(token, price)| {
        app.client
            .post(format!("{}/quotations/{id}/proposals", app.base))
            .bearer_auth(token)
            .json(&json!({ "totalPriceCents": price, "flightInfo": "G3-1720 08:15" }))
            .send()
    }))
    .await;
    for bid in bids {
        assert_eq!(bid.unwrap().status(), 201);
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proposals WHERE quotation_id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count, 3);

    // replacement keeps first submitted_at and does not duplicate
    let first: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/proposals", app.base))
        .bearer_auth(&tokens[0])
        .json(&json!({ "totalPriceCents": 151000, "flightInfo": "G3-1720 08:15" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proposals WHERE quotation_id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(count_after, 3);
    assert!(first["submittedAt"].as_str().is_some());

    // window enforcement: server clock says no (R4)
    common::time_travel_past_close(&app.pool, &id).await;
    let late = app
        .client
        .post(format!("{}/quotations/{id}/proposals", app.base))
        .bearer_auth(&tokens[0])
        .json(&json!({ "totalPriceCents": 140000, "flightInfo": "G3-1720 08:15" }))
        .send()
        .await
        .unwrap();
    assert_eq!(late.status(), 422);
    assert_eq!(late.json::<serde_json::Value>().await.unwrap()["error"], "COTACAO_FECHADA");
    let status: String = sqlx::query_scalar("SELECT status FROM quotations WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(status, "CLOSED", "lazy close must persist");
}
```

Add `futures = "0.3"` usage note: already in `[dependencies]`; for tests it must ALSO be reachable — dev target uses the same dependency list, nothing to add.

- [ ] **Step 3: Run + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS.

```bash
git add api/src/routes/proposals.rs api/tests/integration.rs
git commit -m "feat(api): blind proposal submission with upsert-replace and server-clock window"
```

---

### Task 8: Ranking (lowest first) + award + OS number + 30-min ticket window

**Files:**
- Replace: `api/src/routes/award.rs`
- Modify: `api/src/routes/quotations.rs` (make `next_code` executor-generic), `api/tests/integration.rs`

- [ ] **Step 0: Make `next_code` executor-generic (usable inside the award transaction)**

In `api/src/routes/quotations.rs`, change the `next_code` signature (body unchanged except the executor):

```rust
/// Atomic sequential codes: COT-2026-0001, OS-2026-0001.
pub async fn next_code<'e, E: sqlx::PgExecutor<'e>>(executor: E, prefix: &str) -> ApiResult<String> {
    let key = format!("{prefix}-{}", Utc::now().format("%Y"));
    let value: i64 = sqlx::query_scalar(
        "INSERT INTO counters (id, value) VALUES ($1, 1) \
         ON CONFLICT (id) DO UPDATE SET value = counters.value + 1 RETURNING value",
    )
    .bind(&key)
    .fetch_one(executor)
    .await?;
    Ok(format!("{key}-{value:04}"))
}
```

The existing call site `next_code(&state.pool, "COT")` keeps compiling unchanged; the award flow passes `&mut *tx` so an OS number is never burned by a rolled-back award.

- [ ] **Step 1: Replace `api/src/routes/award.rs`**

```rust
use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::audit::{append_audit_tx, AuditInput};
use crate::auth::Staff;
use crate::domain::timefmt::fmt_boa_vista;
use crate::domain::types::QuotationStatus;
use crate::error::{ApiError, ApiResult};
use crate::sse::publish;
use crate::App;

use super::quotations::{fetch_proposals, load_quotation, next_code};
use super::views::{effective_status, staff_view};

async fn ranking(
    State(state): State<App>,
    Staff(_claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let now = Utc::now();
    let Some(q) = load_quotation(&state, id, now).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    match effective_status(&q.status, q.closes_at, now) {
        QuotationStatus::Draft | QuotationStatus::Open => {
            return Err(ApiError::Unprocessable("COTACAO_AINDA_ABERTA"))
        }
        QuotationStatus::Closed
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => {}
    }
    let proposals = fetch_proposals(&state.pool, id).await?;
    let supplier_ids: Vec<Uuid> = proposals.iter().map(|p| p.supplier_id).collect();
    let suppliers: Vec<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, legal_name, cnpj FROM suppliers WHERE id = ANY($1)")
            .bind(&supplier_ids)
            .fetch_all(&state.pool)
            .await?;
    let by_id: HashMap<Uuid, (String, String)> =
        suppliers.into_iter().map(|(id, name, cnpj)| (id, (name, cnpj))).collect();
    let ranking: Vec<Value> = proposals
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (legal_name, cnpj) = by_id.get(&p.supplier_id).cloned().unwrap_or_default();
            json!({
                "position": i + 1,
                "proposalId": p.id,
                "supplier": { "id": p.supplier_id, "legalName": legal_name, "cnpj": cnpj },
                "totalPriceCents": p.total_price_cents,
                "flightInfo": p.flight_info,
                "notes": p.notes,
                "submittedAt": p.submitted_at.to_rfc3339(),
                "deltaFromReferenceCents": p.total_price_cents - q.reference_price_cents,
            })
        })
        .collect();
    Ok(Json(json!({
        "quotation": staff_view(&q, &proposals, now),
        "referencePriceCents": q.reference_price_cents,
        "ranking": ranking
    })))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AwardBody {
    proposal_id: Uuid,
    justification: String,
}

async fn award(
    State(state): State<App>,
    Staff(claims): Staff,
    Path(id): Path<Uuid>,
    Json(body): Json<AwardBody>,
) -> ApiResult<Json<Value>> {
    if body.justification.trim().len() < 5 {
        return Err(ApiError::Unprocessable("JUSTIFICATIVA_CURTA"));
    }
    let now = Utc::now();
    let Some(q) = load_quotation(&state, id, now).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    match effective_status(&q.status, q.closes_at, now) {
        QuotationStatus::Closed => {}
        QuotationStatus::Draft
        | QuotationStatus::Open
        | QuotationStatus::Awarded
        | QuotationStatus::Ticketed
        | QuotationStatus::Completed => {
            return Err(ApiError::Unprocessable("NAO_ESTA_FECHADA"))
        }
    }
    let proposals = fetch_proposals(&state.pool, id).await?;
    let Some(winner) = proposals.iter().find(|p| p.id == body.proposal_id) else {
        return Err(ApiError::Unprocessable("PROPOSTA_INVALIDA"));
    };
    let deadline = now + Duration::minutes(state.config.ticket_window_minutes);
    // Award + OS number + both audit entries + winner notification commit ATOMICALLY.
    // The guarded UPDATE makes a double-submit lose with 422 (single-shot), and a
    // rolled-back award never burns an OS number (next_code runs inside the tx).
    let mut tx = state.pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE quotations SET status = 'AWARDED', awarded_proposal_id = $1, awarded_at = $2, \
         award_justification = $3, ticket_deadline_at = $4 WHERE id = $5 AND status = 'CLOSED'",
    )
    .bind(winner.id).bind(now).bind(&body.justification).bind(deadline).bind(id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::Unprocessable("NAO_ESTA_FECHADA"));
    }
    let os_number = next_code(&mut *tx, "OS").await?;
    sqlx::query("INSERT INTO service_orders (id, quotation_id, number) VALUES ($1,$2,$3)")
        .bind(Uuid::new_v4()).bind(id).bind(&os_number)
        .execute(&mut *tx)
        .await?;
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "QUOTATION_AWARDED",
        entity: "Quotation",
        entity_id: id.to_string(),
        quotation_id: Some(id),
        payload: json!({
            "proposalId": winner.id.to_string(),
            "supplierId": winner.supplier_id.to_string(),
            "totalPriceCents": winner.total_price_cents,
            "justification": body.justification
        }),
    })
    .await?;
    append_audit_tx(&mut tx, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "SERVICE_ORDER_ISSUED",
        entity: "ServiceOrder",
        entity_id: os_number.clone(),
        quotation_id: Some(id),
        payload: json!({ "number": os_number }),
    })
    .await?;
    let (winner_email,): (String,) =
        sqlx::query_as("SELECT contact_email FROM suppliers WHERE id = $1")
            .bind(winner.supplier_id)
            .fetch_one(&mut *tx)
            .await?;
    let message = format!(
        "Sua proposta venceu a cotação {}. Envie o e-ticket até {} (horário de Boa Vista).",
        q.code,
        fmt_boa_vista(deadline)
    );
    sqlx::query(
        "INSERT INTO notifications (id, supplier_id, quotation_id, kind, message) \
         VALUES ($1,$2,$3,'VENCEDORA',$4)",
    )
    .bind(Uuid::new_v4()).bind(winner.supplier_id).bind(id).bind(&message)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    // Side effects only after the durable commit.
    println!("[mail] to={winner_email} subject=\"Vencedora da cotação {}\"", q.code);
    publish(&state, id, "status", json!({ "status": "AWARDED", "ticketDeadlineAt": deadline.to_rfc3339() }));
    Ok(Json(json!({
        "serviceOrder": { "number": os_number },
        "ticketDeadlineAt": deadline.to_rfc3339()
    })))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/quotations/{id}/ranking", get(ranking))
        .route("/quotations/{id}/award", post(award))
}
```

- [ ] **Step 2: Add the ranking/award test**

Append to `api/tests/integration.rs`:

```rust
#[tokio::test]
async fn ranking_orders_lowest_first_with_tiebreak_then_award_starts_ticket_window() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    common::create_supplier(&app.pool, "11444777000161", "b@example.com", "ACTIVE", "Amazônia Viagens").await;
    let c_id = common::create_supplier(&app.pool, "12345678000195", "c@example.com", "ACTIVE", "Rio Branco Tur").await;
    let id = common::create_open_quotation(&app, &staff_token).await;

    for (email, price) in [("a@example.com", 152300i64), ("b@example.com", 149900), ("c@example.com", 149900)] {
        let token = common::login(&app, email).await;
        let res = app
            .client
            .post(format!("{}/quotations/{id}/proposals", app.base))
            .bearer_auth(&token)
            .json(&json!({ "totalPriceCents": price, "flightInfo": "G3-1720 08:15" }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 201);
    }
    // deterministic tie-break: c submitted a minute earlier than b
    sqlx::query(
        "UPDATE proposals SET submitted_at = now() - interval '1 minute' \
         WHERE quotation_id = $1 AND supplier_id = $2",
    )
    .bind(uuid::Uuid::parse_str(&id).unwrap())
    .bind(c_id)
    .execute(&app.pool)
    .await
    .unwrap();

    // ranking refused while open
    let early = app
        .client
        .get(format!("{}/quotations/{id}/ranking", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(early.status(), 422);

    common::time_travel_past_close(&app.pool, &id).await;
    let ranking: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/ranking", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let names: Vec<&str> = ranking["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["supplier"]["legalName"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["Rio Branco Tur", "Amazônia Viagens", "Voa Roraima"]);
    assert_eq!(ranking["ranking"][0]["deltaFromReferenceCents"], 149900 - 185000);

    let winner_proposal = ranking["ranking"][0]["proposalId"].as_str().unwrap();
    let before = chrono::Utc::now();
    let award: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/award", app.base))
        .bearer_auth(&staff_token)
        .json(&json!({ "proposalId": winner_proposal, "justification": "Menor preço e conformidade" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(award["serviceOrder"]["number"], "OS-2026-0001");
    let deadline: chrono::DateTime<chrono::Utc> =
        award["ticketDeadlineAt"].as_str().unwrap().parse().unwrap();
    let minutes = (deadline - before).num_minutes();
    assert!((29..=31).contains(&minutes), "ticket window must be ~30 min, got {minutes}");

    let status: String = sqlx::query_scalar("SELECT status FROM quotations WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(status, "AWARDED");

    // single-shot: a second award (double-click) loses with 422, exactly one audit row
    let again = app
        .client
        .post(format!("{}/quotations/{id}/award", app.base))
        .bearer_auth(&staff_token)
        .json(&json!({ "proposalId": winner_proposal, "justification": "Menor preço e conformidade" }))
        .send()
        .await
        .unwrap();
    assert_eq!(again.status(), 422);
    let award_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_events WHERE event_type = 'QUOTATION_AWARDED'",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert_eq!(award_events, 1);

    // winner notification copy is Boa Vista local, never raw UTC
    let msg: String = sqlx::query_scalar(
        "SELECT message FROM notifications WHERE kind = 'VENCEDORA' LIMIT 1",
    )
    .fetch_one(&app.pool)
    .await
    .unwrap();
    assert!(msg.contains("horário de Boa Vista"), "must state the timezone: {msg}");
    assert!(!msg.contains("+00:00"), "no raw UTC in supplier copy: {msg}");
}
```

- [ ] **Step 3: Run + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS.

```bash
git add api/src/routes/award.rs api/tests/integration.rs
git commit -m "feat(api): lowest-price ranking with tiebreak, award, OS number and 30-min ticket window"
```

---

### Task 9: E-ticket — winner upload, deterministic divergence check, staff confirmation

**Files:**
- Replace: `api/src/routes/tickets.rs`
- Modify: `api/tests/integration.rs`

- [ ] **Step 1: Replace `api/src/routes/tickets.rs`**

```rust
use std::collections::HashMap;

use axum::extract::{Multipart, Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::audit::{append_audit, AuditInput};
use crate::auth::{AuthUser, Staff};
use crate::domain::divergence::{compute_divergences, TicketFields};
use crate::error::{ApiError, ApiResult};
use crate::sse::publish;
use crate::uploads::save_upload;
use crate::App;

use super::quotations::{fetch_proposals, fetch_quotation};
use super::suppliers::require_supplier;

async fn upload_ticket(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let supplier_id = require_supplier(&claims)?;
    let now = Utc::now();
    let Some(q) = fetch_quotation(&state.pool, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    if q.status != "AWARDED" {
        return Err(ApiError::Unprocessable("NAO_AGUARDA_BILHETE"));
    }
    let proposals = fetch_proposals(&state.pool, id).await?;
    let Some(winner) = proposals.iter().find(|p| Some(p.id) == q.awarded_proposal_id) else {
        return Err(ApiError::Internal("awarded quotation without winner proposal".into()));
    };
    if winner.supplier_id != supplier_id {
        return Err(ApiError::Forbidden("ACESSO_NEGADO"));
    }

    let mut fields: HashMap<String, String> = HashMap::new();
    let mut saved: Option<(String, String)> = None;
    while let Some(field) =
        multipart.next_field().await.map_err(|e| ApiError::Internal(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" {
            let original = field.file_name().unwrap_or("eticket.pdf").to_string();
            let bytes = field.bytes().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            saved = Some(save_upload(&state.config.upload_dir, &original, &bytes).await?);
        } else {
            let value = field.text().await.map_err(|e| ApiError::Internal(e.to_string()))?;
            fields.insert(name, value);
        }
    }
    let passenger_name =
        fields.get("passengerName").cloned().ok_or(ApiError::Unprocessable("BILHETE_INVALIDO"))?;
    let flight_info =
        fields.get("flightInfo").cloned().ok_or(ApiError::Unprocessable("BILHETE_INVALIDO"))?;
    let departure_at: DateTime<Utc> = fields
        .get("departureAt")
        .and_then(|v| v.parse().ok())
        .ok_or(ApiError::Unprocessable("BILHETE_INVALIDO"))?;
    let price_cents: i64 = fields
        .get("priceCents")
        .and_then(|v| v.parse().ok())
        .ok_or(ApiError::Unprocessable("BILHETE_INVALIDO"))?;
    let Some((file_name, file_path)) = saved else {
        return Err(ApiError::Unprocessable("BILHETE_INVALIDO"));
    };

    let ticket_fields = TicketFields { passenger_name: &passenger_name, departure_at, price_cents };
    let divergences = compute_divergences(
        &q.passenger_name,
        q.departure_at,
        winner.total_price_cents,
        &ticket_fields,
    );
    // R8 as KPI, not hard block: accept late uploads but flag them.
    let late = matches!(q.ticket_deadline_at, Some(d) if now > d);
    let ticket_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tickets (id, quotation_id, file_name, file_path, passenger_name, \
         flight_info, departure_at, price_cents, divergences, late) \
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
    )
    .bind(ticket_id).bind(id).bind(&file_name).bind(&file_path).bind(&passenger_name)
    .bind(&flight_info).bind(departure_at).bind(price_cents)
    .bind(json!(divergences)).bind(late)
    .execute(&state.pool)
    .await?;
    sqlx::query("UPDATE quotations SET status = 'TICKETED' WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    append_audit(&state.pool, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "TICKET_UPLOADED",
        entity: "Ticket",
        entity_id: ticket_id.to_string(),
        quotation_id: Some(id),
        payload: json!({ "late": late, "divergences": divergences, "priceCents": price_cents }),
    })
    .await?;
    publish(&state, id, "status", json!({ "status": "TICKETED", "late": late, "divergences": divergences }));
    Ok((StatusCode::CREATED, Json(json!({ "id": ticket_id, "late": late, "divergences": divergences }))))
}

async fn confirm_ticket(
    State(state): State<App>,
    Staff(claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let Some(q) = fetch_quotation(&state.pool, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    if q.status != "TICKETED" {
        return Err(ApiError::Unprocessable("STATUS_INVALIDO"));
    }
    let ticket: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM tickets WHERE quotation_id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    let Some((ticket_id,)) = ticket else { return Err(ApiError::NotFound("BILHETE_NAO_ENVIADO")) };
    sqlx::query("UPDATE tickets SET confirmed_at = now(), confirmed_by = $1 WHERE id = $2")
        .bind(claims.sub)
        .bind(ticket_id)
        .execute(&state.pool)
        .await?;
    sqlx::query("UPDATE quotations SET status = 'COMPLETED' WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    append_audit(&state.pool, AuditInput {
        actor_id: Some(claims.sub),
        actor_role: Some(claims.role.as_str()),
        event_type: "TICKET_CONFIRMED",
        entity: "Ticket",
        entity_id: ticket_id.to_string(),
        quotation_id: Some(id),
        payload: json!({}),
    })
    .await?;
    publish(&state, id, "status", json!({ "status": "COMPLETED" }));
    Ok(Json(json!({ "status": "COMPLETED" })))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/quotations/{id}/ticket", post(upload_ticket))
        .route("/quotations/{id}/ticket/confirm", post(confirm_ticket))
}
```

- [ ] **Step 2: Add the e-ticket test**

Append to `api/tests/common/mod.rs` (awarded-quotation factory used here and in Task 10/11):

```rust
/// Full path to AWARDED: 2 active suppliers bid, window closes, lowest wins.
/// Returns (quotation_id, winner_email, winner_price_cents).
pub async fn setup_awarded(app: &TestApp, staff_token: &str) -> (String, &'static str, i64) {
    create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    create_supplier(&app.pool, "11444777000161", "b@example.com", "ACTIVE", "Amazônia Viagens").await;
    let id = create_open_quotation(app, staff_token).await;
    for (email, price) in [("a@example.com", 152300i64), ("b@example.com", 149900)] {
        let token = login(app, email).await;
        let res = app
            .client
            .post(format!("{}/quotations/{id}/proposals", app.base))
            .bearer_auth(&token)
            .json(&serde_json::json!({ "totalPriceCents": price, "flightInfo": "G3-1720 08:15" }))
            .send()
            .await
            .unwrap();
        assert_eq!(res.status(), 201);
    }
    time_travel_past_close(&app.pool, &id).await;
    let ranking: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/ranking", app.base))
        .bearer_auth(staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let winner_proposal = ranking["ranking"][0]["proposalId"].as_str().unwrap();
    let award = app
        .client
        .post(format!("{}/quotations/{id}/award", app.base))
        .bearer_auth(staff_token)
        .json(&serde_json::json!({ "proposalId": winner_proposal, "justification": "Menor preço" }))
        .send()
        .await
        .unwrap();
    assert_eq!(award.status(), 200);
    (id, "b@example.com", 149900)
}

pub fn ticket_form(passenger: &str, departure: &str, price_cents: i64) -> reqwest::multipart::Form {
    reqwest::multipart::Form::new()
        .text("passengerName", passenger.to_string())
        .text("flightInfo", "G3-1720 08:15")
        .text("departureAt", departure.to_string())
        .text("priceCents", price_cents.to_string())
        .part(
            "file",
            reqwest::multipart::Part::bytes(b"%PDF-1.4 fake ticket".to_vec()).file_name("eticket.pdf"),
        )
}
```

Append to `api/tests/integration.rs`:

```rust
#[tokio::test]
async fn ticket_upload_divergences_late_flag_and_confirmation() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    let (id, winner_email, winner_price) = common::setup_awarded(&app, &staff_token).await;

    // non-winner blocked
    let loser_token = common::login(&app, "a@example.com").await;
    let denied = app
        .client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&loser_token)
        .multipart(common::ticket_form("Maria da Silva", "2026-09-10T08:00:00Z", winner_price))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 403);

    // winner uploads clean ticket in time
    let winner_token = common::login(&app, winner_email).await;
    let clean: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&winner_token)
        .multipart(common::ticket_form("MARIA DA SILVA", "2026-09-10T08:00:00Z", winner_price))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(clean["late"], false);
    assert_eq!(clean["divergences"], json!([]));

    // staff confirms -> COMPLETED
    let confirm = app
        .client
        .post(format!("{}/quotations/{id}/ticket/confirm", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(confirm.status(), 200);
    let status: String = sqlx::query_scalar("SELECT status FROM quotations WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .fetch_one(&app.pool)
        .await
        .unwrap();
    assert_eq!(status, "COMPLETED");
}

#[tokio::test]
async fn ticket_late_and_divergent_price_are_flagged_not_rejected() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    let (id, winner_email, _) = common::setup_awarded(&app, &staff_token).await;
    sqlx::query("UPDATE quotations SET ticket_deadline_at = now() - interval '1 second' WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&app.pool)
        .await
        .unwrap();
    let winner_token = common::login(&app, winner_email).await;
    let flagged: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&winner_token)
        .multipart(common::ticket_form("Maria da Silva", "2026-09-10T08:00:00Z", 155000))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(flagged["late"], true);
    assert_eq!(flagged["divergences"], json!(["VALOR_DIVERGENTE"]));
}
```

- [ ] **Step 3: Run + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS.

```bash
git add api/src/routes/tickets.rs api/tests
git commit -m "feat(api): e-ticket upload with deterministic divergence check, late flag and confirmation"
```

---

### Task 10: Dossier (JSON), KPI metrics, printable OS + report pages (askama), tracing

**Files:**
- Modify: `api/Cargo.toml`, `api/src/main.rs`, `api/src/lib.rs`
- Fill: `api/src/html.rs`
- Create: `api/templates/os.html`, `api/templates/report.html`
- Replace: `api/src/routes/reports.rs` (keeps the Task-4 audit endpoints, adds the rest)
- Modify: `api/tests/integration.rs`

- [ ] **Step 1: Add crates (industry-standard, less code ours)**

In `api/Cargo.toml` `[dependencies]` add:

```toml
askama = "0.12"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

and change the tower-http line to:

```toml
tower-http = { version = "0.6", features = ["cors", "trace"] }
```

In `api/src/main.rs` add as the first line of `main`:

```rust
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "info,tower_http=debug".into()),
    ).init();
```

In `api/src/lib.rs` `app()`, add below the cors layer line — the span logs the PATH ONLY, never the query string, because SSE and the printable pages authenticate via `?token=<jwt>` and default TraceLayer spans would write live bearer tokens into logs:

```rust
        .layer(tower_http::trace::TraceLayer::new_for_http().make_span_with(
            |req: &axum::http::Request<axum::body::Body>| {
                tracing::info_span!("http", method = %req.method(), path = %req.uri().path())
            },
        ))
```

In `api/src/error.rs`, move the internal-error log into the structured pipeline — replace the `eprintln!("internal error: {msg}");` line inside `IntoResponse` with:

```rust
                tracing::error!(%msg, "internal error");
```

- [ ] **Step 2: Implement `api/src/html.rs` (askama template structs)**

```rust
use askama::Template;

#[derive(Template)]
#[template(path = "os.html")]
pub struct OsTemplate {
    pub number: String,
    pub code: String,
    pub supplier_name: String,
    pub supplier_cnpj: String,
    pub passenger_name: String,
    pub passenger_cpf: String,
    pub passenger_sex: String,
    pub passenger_birth: String,
    pub origin: String,
    pub destination: String,
    pub departure_at: String,
    pub flight_info: String,
    pub price: String,
    pub issued_at: String,
}

pub struct ReportProposal {
    pub position: usize,
    pub supplier: String,
    pub cnpj: String,
    pub price: String,
    pub flight_info: String,
    pub submitted_at: String,
}

pub struct ReportEvent {
    pub seq: i64,
    pub at: String,
    pub event_type: String,
}

#[derive(Template)]
#[template(path = "report.html")]
pub struct ReportTemplate {
    pub code: String,
    pub status: String,
    pub origin: String,
    pub destination: String,
    pub passenger_name: String,
    pub passenger_cpf_masked: String,
    pub reference_price: String,
    pub notified: i64,
    pub proposals: Vec<ReportProposal>,
    pub has_economy: bool,
    pub economy_saved: String,
    pub economy_pct: String,
    pub os_number: String,
    pub ticket_line: String,
    pub audit_ok: bool,
    pub timeline: Vec<ReportEvent>,
    pub generated_at: String,
}
```

- [ ] **Step 3: Write the templates**

`api/templates/os.html`:

```html
<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<title>Ordem de Serviço {{ number }}</title>
<style>
  body { font-family: Georgia, serif; max-width: 720px; margin: 2rem auto; color: #111; }
  header { text-align: center; border-bottom: 3px double #1e3a8a; padding-bottom: 1rem; }
  h1 { font-size: 1.1rem; letter-spacing: 0.05em; margin: 0; }
  h2 { font-size: 1.4rem; color: #1e3a8a; margin: 0.4rem 0 0; }
  section { margin-top: 1.5rem; }
  h3 { font-size: 0.9rem; text-transform: uppercase; color: #555; border-bottom: 1px solid #ddd; }
  table { width: 100%; border-collapse: collapse; }
  td { padding: 0.3rem 0; vertical-align: top; }
  td:first-child { width: 12rem; color: #555; }
  .price { font-size: 1.3rem; font-weight: bold; color: #14532d; }
  .print-btn { position: fixed; top: 1rem; right: 1rem; padding: 0.6rem 1.2rem;
               background: #1e3a8a; color: #fff; border: 0; border-radius: 6px; cursor: pointer; }
  @media print { .print-btn { display: none; } body { margin: 0; } }
</style>
</head>
<body>
<button class="print-btn" onclick="window.print()">Imprimir / Salvar PDF</button>
<header>
  <h1>TRIBUNAL DE JUSTIÇA DE RORAIMA</h1>
  <h2>ORDEM DE SERVIÇO {{ number }}</h2>
  <p>Cotação {{ code }} · Emitida em {{ issued_at }}</p>
</header>
<section>
  <h3>Fornecedor vencedor</h3>
  <table>
    <tr><td>Razão social</td><td>{{ supplier_name }}</td></tr>
    <tr><td>CNPJ</td><td>{{ supplier_cnpj }}</td></tr>
  </table>
</section>
<section>
  <h3>Dados do passageiro</h3>
  <table>
    <tr><td>Nome</td><td>{{ passenger_name }}</td></tr>
    <tr><td>CPF</td><td>{{ passenger_cpf }}</td></tr>
    <tr><td>Sexo</td><td>{{ passenger_sex }}</td></tr>
    <tr><td>Nascimento</td><td>{{ passenger_birth }}</td></tr>
  </table>
</section>
<section>
  <h3>Serviço contratado</h3>
  <table>
    <tr><td>Trecho</td><td>{{ origin }} → {{ destination }}</td></tr>
    <tr><td>Embarque</td><td>{{ departure_at }}</td></tr>
    <tr><td>Voo ofertado</td><td>{{ flight_info }}</td></tr>
    <tr><td>Valor contratado</td><td class="price">{{ price }}</td></tr>
  </table>
  <p>Prazo para envio do e-ticket: 30 minutos após a declaração da vencedora.</p>
</section>
</body>
</html>
```

`api/templates/report.html`:

```html
<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<title>Relatório {{ code }}</title>
<style>
  body { font-family: Georgia, serif; max-width: 760px; margin: 2rem auto; color: #111; }
  h1 { font-size: 1.3rem; color: #1e3a8a; text-align: center; }
  h2 { font-size: 0.95rem; text-transform: uppercase; color: #555; border-bottom: 1px solid #ddd; margin-top: 1.6rem; }
  table { width: 100%; border-collapse: collapse; font-size: 0.95rem; }
  th, td { padding: 0.35rem 0.5rem; text-align: left; border-bottom: 1px solid #eee; }
  th { color: #555; font-weight: normal; }
  .economy { background: #f0fdf4; border: 1px solid #bbf7d0; padding: 1rem; border-radius: 8px;
             font-size: 1.2rem; color: #14532d; }
  .audit-ok { color: #14532d; } .audit-bad { color: #b91c1c; }
  .print-btn { position: fixed; top: 1rem; right: 1rem; padding: 0.6rem 1.2rem;
               background: #1e3a8a; color: #fff; border: 0; border-radius: 6px; cursor: pointer; }
  @media print { .print-btn { display: none; } body { margin: 0; } }
</style>
</head>
<body>
<button class="print-btn" onclick="window.print()">Imprimir / Salvar PDF</button>
<h1>Relatório da Cotação {{ code }}</h1>
<p style="text-align:center">Status {{ status }} · Gerado em {{ generated_at }}</p>
<h2>Demanda</h2>
<table>
  <tr><th>Trecho</th><td>{{ origin }} → {{ destination }}</td></tr>
  <tr><th>Passageiro</th><td>{{ passenger_name }} (CPF {{ passenger_cpf_masked }})</td></tr>
  <tr><th>Preço de referência</th><td>{{ reference_price }}</td></tr>
  <tr><th>Fornecedores notificados</th><td>{{ notified }}</td></tr>
</table>
<h2>Propostas (menor → maior)</h2>
<table>
  <tr><th>#</th><th>Fornecedor</th><th>CNPJ</th><th>Valor</th><th>Voo</th><th>Enviada em</th></tr>
  {% for p in proposals %}
  <tr>
    <td>{{ p.position }}º</td><td>{{ p.supplier }}</td><td>{{ p.cnpj }}</td>
    <td>{{ p.price }}</td><td>{{ p.flight_info }}</td><td>{{ p.submitted_at }}</td>
  </tr>
  {% endfor %}
</table>
{% if has_economy %}
<h2>Economicidade</h2>
<p class="economy">Economia obtida: <strong>{{ economy_saved }}</strong> ({{ economy_pct }}% abaixo da referência)
· Ordem de Serviço {{ os_number }}</p>
{% endif %}
<h2>E-ticket</h2>
<p>{{ ticket_line }}</p>
<h2>Trilha de auditoria</h2>
<p>
  {% if audit_ok %}<span class="audit-ok">✔ Cadeia de hashes íntegra</span>
  {% else %}<span class="audit-bad">✘ VIOLAÇÃO detectada na cadeia de hashes</span>{% endif %}
</p>
<table>
  <tr><th>#</th><th>Quando (UTC)</th><th>Evento</th></tr>
  {% for e in timeline %}
  <tr><td>{{ e.seq }}</td><td>{{ e.at }}</td><td>{{ e.event_type }}</td></tr>
  {% endfor %}
</table>
</body>
</html>
```

- [ ] **Step 4: Replace `api/src/routes/reports.rs`**

```rust
use axum::extract::{Path, State};
use axum::response::Html;
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::audit::{list_events, verify_chain};
use crate::auth::{AuthUser, Staff};
use crate::domain::brl::format_brl;
use crate::domain::cpf::mask_cpf;
use crate::domain::economy::compute_economy;
use crate::domain::types::Role;
use crate::error::{ApiError, ApiResult};
use crate::html::{OsTemplate, ReportEvent, ReportProposal, ReportTemplate};
use crate::App;

use super::quotations::{fetch_proposals, fetch_quotation};

async fn audit_verify(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    Ok(Json(verify_chain(&state.pool).await?))
}

async fn audit_events(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    let rows = list_events(&state.pool, None).await?;
    let events: Vec<Value> = rows
        .iter()
        .map(|e| {
            json!({
                "seq": e.seq, "at": e.at, "type": e.event_type, "entity": e.entity,
                "entityId": e.entity_id, "actorId": e.actor_id, "payload": e.payload
            })
        })
        .collect();
    Ok(Json(json!(events)))
}

struct Dossier {
    q: super::views::QuotationRow,
    proposals: Vec<super::views::ProposalRow>,
    supplier_names: std::collections::HashMap<Uuid, (String, String)>,
    notified: i64,
    os: Option<(String, DateTime<Utc>)>,
    ticket: Option<(String, bool, Value, DateTime<Utc>, Option<DateTime<Utc>>)>,
}

async fn load_dossier(state: &App, id: Uuid) -> ApiResult<Option<Dossier>> {
    let Some(q) = fetch_quotation(&state.pool, id).await? else { return Ok(None) };
    let proposals = fetch_proposals(&state.pool, id).await?;
    let supplier_ids: Vec<Uuid> = proposals.iter().map(|p| p.supplier_id).collect();
    let names: Vec<(Uuid, String, String)> =
        sqlx::query_as("SELECT id, legal_name, cnpj FROM suppliers WHERE id = ANY($1)")
            .bind(&supplier_ids)
            .fetch_all(&state.pool)
            .await?;
    let supplier_names =
        names.into_iter().map(|(id, name, cnpj)| (id, (name, cnpj))).collect();
    let notified: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE quotation_id = $1 AND kind = 'COTACAO_ABERTA'",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    let os: Option<(String, DateTime<Utc>)> =
        sqlx::query_as("SELECT number, issued_at FROM service_orders WHERE quotation_id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;
    let ticket: Option<(String, bool, Value, DateTime<Utc>, Option<DateTime<Utc>>)> =
        sqlx::query_as(
            "SELECT file_name, late, divergences, uploaded_at, confirmed_at \
             FROM tickets WHERE quotation_id = $1",
        )
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;
    Ok(Some(Dossier { q, proposals, supplier_names, notified, os, ticket }))
}

/// R9/R10: the complete JSON dossier. CPF masked — full CPF lives only on the OS.
async fn report_json(
    State(state): State<App>,
    Staff(_claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    let Some(d) = load_dossier(&state, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    let events = list_events(&state.pool, Some(id)).await?;
    let winner = d.q.awarded_proposal_id.and_then(|w| d.proposals.iter().find(|p| p.id == w));
    let economy = winner.map(|w| compute_economy(d.q.reference_price_cents, w.total_price_cents));
    Ok(Json(json!({
        "generatedAt": Utc::now().to_rfc3339(),
        "quotation": {
            "code": d.q.code, "status": d.q.status,
            "origin": d.q.origin, "destination": d.q.destination,
            "departureAt": d.q.departure_at.to_rfc3339(),
            "referenceFlight": d.q.reference_flight,
            "opensAt": d.q.opens_at.map(|v| v.to_rfc3339()),
            "closesAt": d.q.closes_at.map(|v| v.to_rfc3339()),
            "awardedAt": d.q.awarded_at.map(|v| v.to_rfc3339()),
            "awardJustification": d.q.award_justification,
            "passengerName": d.q.passenger_name,
            "passengerCpfMasked": mask_cpf(&d.q.passenger_cpf),
        },
        "referencePriceCents": d.q.reference_price_cents,
        "notifiedSuppliers": d.notified,
        "proposals": d.proposals.iter().enumerate().map(|(i, p)| {
            let (name, cnpj) = d.supplier_names.get(&p.supplier_id).cloned().unwrap_or_default();
            json!({
                "position": i + 1, "supplier": name, "cnpj": cnpj,
                "totalPriceCents": p.total_price_cents, "flightInfo": p.flight_info,
                "submittedAt": p.submitted_at.to_rfc3339()
            })
        }).collect::<Vec<_>>(),
        "winner": winner.map(|w| {
            let (name, cnpj) = d.supplier_names.get(&w.supplier_id).cloned().unwrap_or_default();
            json!({ "supplier": name, "cnpj": cnpj, "totalPriceCents": w.total_price_cents })
        }),
        "serviceOrder": d.os.as_ref().map(|(number, issued_at)| {
            json!({ "number": number, "issuedAt": issued_at.to_rfc3339() })
        }),
        "ticket": d.ticket.as_ref().map(|(file_name, late, divergences, uploaded_at, confirmed_at)| {
            json!({
                "fileName": file_name, "late": late, "divergences": divergences,
                "uploadedAt": uploaded_at.to_rfc3339(),
                "confirmedAt": confirmed_at.map(|v| v.to_rfc3339())
            })
        }),
        "economy": economy,
        "timeline": events.iter().map(|e| json!({
            "seq": e.seq, "at": e.at, "type": e.event_type, "actorId": e.actor_id, "payload": e.payload
        })).collect::<Vec<_>>(),
    })))
}

/// Printable OS page — staff or the winning supplier (opened via ?token=).
async fn service_order_page(
    State(state): State<App>,
    AuthUser(claims): AuthUser,
    Path(id): Path<Uuid>,
) -> ApiResult<Html<String>> {
    let Some(d) = load_dossier(&state, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    let Some((os_number, issued_at)) = d.os.clone() else {
        return Err(ApiError::NotFound("OS_NAO_EMITIDA"));
    };
    let Some(winner) = d.q.awarded_proposal_id.and_then(|w| d.proposals.iter().find(|p| p.id == w))
    else {
        return Err(ApiError::NotFound("OS_NAO_EMITIDA"));
    };
    match claims.role {
        Role::Admin | Role::Servidor => {}
        Role::Fornecedor => {
            if claims.supplier_id != Some(winner.supplier_id) {
                return Err(ApiError::Forbidden("ACESSO_NEGADO"));
            }
        }
    }
    let (supplier_name, supplier_cnpj) =
        d.supplier_names.get(&winner.supplier_id).cloned().unwrap_or_default();
    let template = OsTemplate {
        number: os_number,
        code: d.q.code.clone(),
        supplier_name,
        supplier_cnpj,
        passenger_name: d.q.passenger_name.clone(),
        passenger_cpf: d.q.passenger_cpf.clone(),
        passenger_sex: d.q.passenger_sex.clone(),
        passenger_birth: d.q.passenger_birth.format("%d/%m/%Y").to_string(),
        origin: d.q.origin.clone(),
        destination: d.q.destination.clone(),
        departure_at: d.q.departure_at.format("%d/%m/%Y %H:%M UTC").to_string(),
        flight_info: winner.flight_info.clone(),
        price: format_brl(winner.total_price_cents),
        issued_at: issued_at.format("%d/%m/%Y %H:%M UTC").to_string(),
    };
    Ok(Html(template.render().map_err(|e| ApiError::Internal(e.to_string()))?))
}

/// Printable dossier page — staff only (SEI attachment, prestação de contas).
async fn report_page(
    State(state): State<App>,
    Staff(_claims): Staff,
    Path(id): Path<Uuid>,
) -> ApiResult<Html<String>> {
    let Some(d) = load_dossier(&state, id).await? else {
        return Err(ApiError::NotFound("NAO_ENCONTRADA"));
    };
    let events = list_events(&state.pool, Some(id)).await?;
    let audit = verify_chain(&state.pool).await?;
    let winner = d.q.awarded_proposal_id.and_then(|w| d.proposals.iter().find(|p| p.id == w));
    let economy = winner.map(|w| compute_economy(d.q.reference_price_cents, w.total_price_cents));
    let ticket_line = match &d.ticket {
        Some((file_name, late, divergences, _, _)) => format!(
            "{} — {} — divergências: {}",
            file_name,
            if *late { "enviado FORA do prazo de 30 min" } else { "enviado dentro do prazo" },
            divergences
                .as_array()
                .map(|a| {
                    if a.is_empty() {
                        "nenhuma".to_string()
                    } else {
                        a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", ")
                    }
                })
                .unwrap_or_else(|| "nenhuma".to_string())
        ),
        None => "Ainda não enviado.".to_string(),
    };
    let template = ReportTemplate {
        code: d.q.code.clone(),
        status: d.q.status.clone(),
        origin: d.q.origin.clone(),
        destination: d.q.destination.clone(),
        passenger_name: d.q.passenger_name.clone(),
        passenger_cpf_masked: mask_cpf(&d.q.passenger_cpf),
        reference_price: format_brl(d.q.reference_price_cents),
        notified: d.notified,
        proposals: d
            .proposals
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let (name, cnpj) =
                    d.supplier_names.get(&p.supplier_id).cloned().unwrap_or_default();
                ReportProposal {
                    position: i + 1,
                    supplier: name,
                    cnpj,
                    price: format_brl(p.total_price_cents),
                    flight_info: p.flight_info.clone(),
                    submitted_at: p.submitted_at.format("%d/%m/%Y %H:%M:%S UTC").to_string(),
                }
            })
            .collect(),
        has_economy: economy.is_some(),
        economy_saved: economy.as_ref().map(|e| format_brl(e.saved_cents)).unwrap_or_default(),
        economy_pct: economy.as_ref().map(|e| e.saved_pct.to_string()).unwrap_or_default(),
        os_number: d.os.as_ref().map(|(n, _)| n.clone()).unwrap_or_default(),
        ticket_line,
        audit_ok: audit["ok"] == json!(true),
        timeline: events
            .iter()
            .map(|e| ReportEvent { seq: e.seq, at: e.at.clone(), event_type: e.event_type.clone() })
            .collect(),
        generated_at: Utc::now().format("%d/%m/%Y %H:%M UTC").to_string(),
    };
    Ok(Html(template.render().map_err(|e| ApiError::Internal(e.to_string()))?))
}

/// KPI block — maps 1:1 to the edital's Indicativos de Sucesso.
async fn metrics_summary(State(state): State<App>, Staff(_claims): Staff) -> ApiResult<Json<Value>> {
    #[derive(sqlx::FromRow)]
    struct AwardedRow {
        id: Uuid,
        reference_price_cents: i64,
        awarded_proposal_id: Option<Uuid>,
    }
    let awarded = sqlx::query_as::<_, AwardedRow>(
        "SELECT id, reference_price_cents, awarded_proposal_id FROM quotations \
         WHERE status IN ('AWARDED','TICKETED','COMPLETED')",
    )
    .fetch_all(&state.pool)
    .await?;
    let mut total_saved: i64 = 0;
    let mut participants: i64 = 0;
    let mut tickets_total: i64 = 0;
    let mut tickets_on_time: i64 = 0;
    for row in &awarded {
        if let Some(winner_id) = row.awarded_proposal_id {
            let price: Option<i64> =
                sqlx::query_scalar("SELECT total_price_cents FROM proposals WHERE id = $1")
                    .bind(winner_id)
                    .fetch_optional(&state.pool)
                    .await?;
            if let Some(price) = price {
                total_saved += row.reference_price_cents - price;
            }
        }
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM proposals WHERE quotation_id = $1")
            .bind(row.id)
            .fetch_one(&state.pool)
            .await?;
        participants += count;
        let late: Option<bool> =
            sqlx::query_scalar("SELECT late FROM tickets WHERE quotation_id = $1")
                .bind(row.id)
                .fetch_optional(&state.pool)
                .await?;
        if let Some(late) = late {
            tickets_total += 1;
            if !late {
                tickets_on_time += 1;
            }
        }
    }
    let awarded_count = awarded.len() as i64;
    Ok(Json(json!({
        "awardedCount": awarded_count,
        "totalSavedCents": total_saved,
        "avgParticipants": if awarded_count > 0 {
            (participants as f64 / awarded_count as f64 * 10.0).round() / 10.0
        } else { 0.0 },
        "ticketsOnTimePct": if tickets_total > 0 {
            (tickets_on_time as f64 / tickets_total as f64 * 100.0).round()
        } else { 0.0 },
    })))
}

pub fn router() -> Router<App> {
    Router::new()
        .route("/audit/verify", get(audit_verify))
        .route("/audit/events", get(audit_events))
        .route("/quotations/{id}/report.json", get(report_json))
        .route("/quotations/{id}/report", get(report_page))
        .route("/quotations/{id}/service-order", get(service_order_page))
        .route("/metrics/summary", get(metrics_summary))
}
```

- [ ] **Step 5: Add the report/metrics/printable-pages test**

Append to `api/tests/integration.rs`:

```rust
#[tokio::test]
async fn report_metrics_and_printable_pages_after_completed_flow() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    let (id, winner_email, winner_price) = common::setup_awarded(&app, &staff_token).await;
    let winner_token = common::login(&app, winner_email).await;
    app.client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&winner_token)
        .multipart(common::ticket_form("Maria da Silva", "2026-09-10T08:00:00Z", winner_price))
        .send()
        .await
        .unwrap();
    app.client
        .post(format!("{}/quotations/{id}/ticket/confirm", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();

    let report: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/report.json", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["quotation"]["status"], "COMPLETED");
    assert_eq!(report["economy"]["saved_cents"], 35100);
    assert_eq!(report["economy"]["saved_pct"], 18.97);
    assert_eq!(report["notifiedSuppliers"], 2);
    assert_eq!(report["quotation"]["passengerCpfMasked"], "***.456.789-**");
    assert_eq!(report["serviceOrder"]["number"], "OS-2026-0001");
    assert!(report["timeline"].as_array().unwrap().len() >= 6);

    // printable OS: staff ok, winner ok (via ?token=), loser 403
    let os_staff = app
        .client
        .get(format!("{}/quotations/{id}/service-order", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();
    assert_eq!(os_staff.status(), 200);
    let os_html = os_staff.text().await.unwrap();
    assert!(os_html.contains("OS-2026-0001"));
    assert!(os_html.contains("Maria da Silva"));

    let os_winner = app
        .client
        .get(format!("{}/quotations/{id}/service-order?token={winner_token}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(os_winner.status(), 200);

    let loser_token = common::login(&app, "a@example.com").await;
    let os_loser = app
        .client
        .get(format!("{}/quotations/{id}/service-order?token={loser_token}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(os_loser.status(), 403);

    // printable report page renders with economy + audit badge
    let report_page = app
        .client
        .get(format!("{}/quotations/{id}/report", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(report_page.contains("Economia obtida"));
    assert!(report_page.contains("Cadeia de hashes íntegra"));

    let metrics: serde_json::Value = app
        .client
        .get(format!("{}/metrics/summary", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(metrics["awardedCount"], 1);
    assert_eq!(metrics["totalSavedCents"], 35100);
    assert_eq!(metrics["avgParticipants"], 2.0);
    assert_eq!(metrics["ticketsOnTimePct"], 100.0);
}
```

(`economy` serializes with snake_case keys because `Economy` derives Serialize without rename — hence `saved_cents`/`saved_pct` in assertions.)

- [ ] **Step 6: Run + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS.

```bash
git add api/Cargo.toml api/src/main.rs api/src/lib.rs api/src/html.rs api/templates api/src/routes/reports.rs api/tests/integration.rs
git commit -m "feat(api): json dossier, kpi metrics, printable os/report pages (askama) and tracing"
```

---

### Task 11: SSE route + full multi-supplier flow test

**Files:**
- Modify: `api/src/sse.rs`, `api/src/lib.rs`, `api/Cargo.toml`
- Modify: `api/tests/integration.rs`

- [ ] **Step 1: Append the SSE route to `api/src/sse.rs`**

```rust
use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::routing::get;
use axum::Router;
use chrono::Utc;
use futures::stream::Stream;
use futures::StreamExt;
use serde_json::json;

use crate::auth::AuthUser;

/// R4: live countdown/state. Auth via ?token= (EventSource cannot set headers).
/// Emits: hello {serverNow} once, tick {serverNow} every 5s, plus published
/// status/proposal events. Never carries bid values.
async fn events(
    State(state): State<App>,
    AuthUser(_claims): AuthUser,
    Path(id): Path<Uuid>,
) -> Sse<impl Stream<Item = Result<Event, std::convert::Infallible>>> {
    let receiver = channel_for(&state, id).subscribe();
    let hello = futures::stream::once(async {
        Ok(Event::default()
            .event("hello")
            .data(json!({ "serverNow": Utc::now().to_rfc3339() }).to_string()))
    });
    let updates = tokio_stream::wrappers::BroadcastStream::new(receiver).filter_map(|msg| async {
        match msg {
            Ok(m) => Some(Ok(Event::default().event(m.event).data(m.data))),
            Err(_) => None,
        }
    });
    let ticks = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(5),
    ))
    .map(|_| {
        Ok(Event::default()
            .event("tick")
            .data(json!({ "serverNow": Utc::now().to_rfc3339() }).to_string()))
    });
    let stream = hello.chain(futures::stream::select(updates, ticks));
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub fn router() -> Router<App> {
    Router::new().route("/quotations/{id}/events", get(events))
}
```

In `api/src/lib.rs` `app()`, add after `.merge(routes::router())`:

```rust
        .merge(sse::router())
```

In `api/Cargo.toml`, change the reqwest dev-dependency to include streaming:

```toml
reqwest = { version = "0.12", features = ["json", "multipart", "stream"] }
```

- [ ] **Step 2: SSE wire test + THE full-flow test**

Append to `api/tests/integration.rs`:

```rust
#[tokio::test]
async fn sse_stream_delivers_hello_then_proposal_count() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;
    common::create_supplier(&app.pool, "11222333000181", "a@example.com", "ACTIVE", "Voa Roraima").await;
    let id = common::create_open_quotation(&app, &staff_token).await;

    let res = app
        .client
        .get(format!("{}/quotations/{id}/events?token={staff_token}", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert!(res.headers()["content-type"].to_str().unwrap().contains("text/event-stream"));

    let mut stream = res.bytes_stream();
    let mut buffer = String::new();

    use futures::StreamExt;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(chunk) = stream.next().await {
            buffer.push_str(&String::from_utf8_lossy(&chunk.unwrap()));
            if buffer.contains("event: hello") {
                break;
            }
        }
    })
    .await
    .expect("hello event within 5s");

    // a bid publishes a count-only event
    let supplier_token = common::login(&app, "a@example.com").await;
    app.client
        .post(format!("{}/quotations/{id}/proposals", app.base))
        .bearer_auth(&supplier_token)
        .json(&json!({ "totalPriceCents": 150000, "flightInfo": "G3-1720" }))
        .send()
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        while let Some(chunk) = stream.next().await {
            buffer.push_str(&String::from_utf8_lossy(&chunk.unwrap()));
            if buffer.contains("\"count\":1") {
                break;
            }
        }
    })
    .await
    .expect("proposal count event within 5s");
    assert!(!buffer.contains("150000"), "SSE must never leak bid values");

    // no token -> 401
    let anon = app
        .client
        .get(format!("{}/quotations/{id}/events", app.base))
        .send()
        .await
        .unwrap();
    assert_eq!(anon.status(), 401);
}

#[tokio::test]
async fn full_flow_credenciamento_to_report_through_http_only() {
    let app = spawn_app().await;
    common::create_staff(&app.pool, "servidor@tjrr.jus.br").await;
    let staff_token = common::login(&app, "servidor@tjrr.jus.br").await;

    // 1. Credenciamento via API: register + 4 docs + homologation, three suppliers
    let companies = [
        ("11.222.333/0001-81", "voa@example.com", "Voa Roraima", 152300i64),
        ("11.444.777/0001-61", "ama@example.com", "Amazônia Viagens", 149900),
        ("12.345.678/0001-95", "rio@example.com", "Rio Branco Tur", 158000),
    ];
    for (cnpj, email, name, _) in &companies {
        let supplier_id = common::register_with_docs(&app, cnpj, email, name).await;
        let approved = app
            .client
            .post(format!("{}/suppliers/{supplier_id}/decision", app.base))
            .bearer_auth(&staff_token)
            .json(&json!({ "decision": "APPROVE" }))
            .send()
            .await
            .unwrap();
        assert_eq!(approved.status(), 200);
    }

    // 2. Demand + simultaneous notification
    let id = common::create_open_quotation(&app, &staff_token).await;
    let notified: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM notifications WHERE kind = 'COTACAO_ABERTA'")
            .fetch_one(&app.pool)
            .await
            .unwrap();
    assert_eq!(notified, 3);

    // 3. Concurrent blind bids
    let bids = futures::future::join_all(companies.iter().map(|(_, email, _, price)| {
        let app_ref = &app;
        let id_ref = &id;
        async move {
            let token = common::login(app_ref, email).await;
            app_ref
                .client
                .post(format!("{}/quotations/{id_ref}/proposals", app_ref.base))
                .bearer_auth(&token)
                .json(&json!({ "totalPriceCents": price, "flightInfo": "G3-1720 08:15" }))
                .send()
                .await
                .unwrap()
        }
    }))
    .await;
    for bid in bids {
        assert_eq!(bid.status(), 201);
    }

    // 4. Close + ranking lowest-first
    common::time_travel_past_close(&app.pool, &id).await;
    let ranking: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/ranking", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let prices: Vec<i64> = ranking["ranking"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["totalPriceCents"].as_i64().unwrap())
        .collect();
    assert_eq!(prices, vec![149900, 152300, 158000]);

    // 5. Award + OS
    let award: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/award", app.base))
        .bearer_auth(&staff_token)
        .json(&json!({
            "proposalId": ranking["ranking"][0]["proposalId"],
            "justification": "Menor preço entre as propostas válidas"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(award["serviceOrder"]["number"], "OS-2026-0001");

    // 6. Winner e-ticket + confirmation
    let winner_token = common::login(&app, "ama@example.com").await;
    let ticket: serde_json::Value = app
        .client
        .post(format!("{}/quotations/{id}/ticket", app.base))
        .bearer_auth(&winner_token)
        .multipart(common::ticket_form("Maria da Silva", "2026-09-10T08:00:00Z", 149900))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ticket["divergences"], json!([]));
    app.client
        .post(format!("{}/quotations/{id}/ticket/confirm", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap();

    // 7. Dossier + audit integrity + KPIs
    let report: serde_json::Value = app
        .client
        .get(format!("{}/quotations/{id}/report.json", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["quotation"]["status"], "COMPLETED");
    assert_eq!(report["economy"]["saved_cents"], 35100);
    assert_eq!(report["notifiedSuppliers"], 3);

    let audit: serde_json::Value = app
        .client
        .get(format!("{}/audit/verify", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(audit["ok"], true);

    let metrics: serde_json::Value = app
        .client
        .get(format!("{}/metrics/summary", app.base))
        .bearer_auth(&staff_token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(metrics["awardedCount"], 1);
}
```

- [ ] **Step 3: Run the whole suite + commit**

Run: `cd api && cargo test -- --test-threads=1 && cd ..`
Expected: PASS — every test green. If the full-flow test exposes a bug, fix the bug (not the test) before committing.

```bash
git add api/src/sse.rs api/src/lib.rs api/Cargo.toml api/tests
git commit -m "feat(api): sse live channel and full multi-supplier flow test"
```

**API is now demoable end-to-end.** Checkpoint: `cd api && cargo run --bin api` and walk the flow with curl/Insomnia if the UI is not ready yet.

---

## UX foundations (governs Tasks 12–14 — and the pitch video)

Who actually uses this, and what each screen must optimize for. E2C1 (weight 2.5) is scored on exactly this.

**Persona A — Servidor da SGA (fiscal de contrato).** Desk worker, Windows + Chrome, lives in SEI and e-mail, constantly interrupted; the cotação is one of a dozen daily tasks. Fears doing something irregular more than doing something slow.
- Dashboard is an **action queue**, not a table: "Aguardando abertura", "Encerradas — declarar vencedora", "Bilhetes para conferir" come first; everything else below.
- They will NOT watch a 1-hour timer. State must be re-findable at a glance; closing is announced by the queue (and SSE keeps any open screen current).
- Adjudication in one click: lowest bid **pre-selected**, justification **pre-filled** ("Menor preço entre as propostas válidas."), delta vs reference shown in green — the reassurance that this is defensible.
- Ticket conference is **side-by-side** (pedido vs bilhete) with divergence chips — verify in seconds, not by re-reading two PDFs.
- Zero retyping: passenger data flows form → OS → report automatically; every screen links to the printable dossiê ("pronto para anexar ao SEI").

**Persona B — Atendente da agência credenciada.** Small Boa Vista agency, 1–3 people; prices trips in a consolidator/airline site on another screen or device; often first sees the notification on a phone. The 1-hour window can catch them at lunch.
- Supplier side is **mobile-first** (R11): cards, big touch targets, single-column.
- Bid page is a **single-task screen**: route/dates/reference-flight brief readable in 5 seconds, huge countdown, ONE price field (pt-BR mask), flight info, submit. After submitting: explicit "Proposta registrada às HH:mm — enviar novamente substitui" state.
- Winning must be unmissable: banner + 30-min countdown + guided upload (passenger data and OS right there).
- Credenciamento status in plain language chips (✔ ok / ✖ pendente / ⚠ vencido) with what to do next.

**Persona C — Gestor/Controle (NPI, auditoria).** Occasional: KPIs on the dashboard, printable dossiê, audit-verify badge. Already covered; do not over-build.

**Cross-cutting rules (all tasks):**
1. **All timestamps render in `America/Boa_Vista`** via `fmtDateTime()` — never raw UTC ISO. Countdowns are server-offset corrected and labeled "horário oficial do servidor". (A supplier missing a deadline over a timezone misread is the worst possible failure.)
2. Every mutation → sonner toast (success and failure); buttons disable while pending. No silent outcomes.
3. Empty states teach in plain Portuguese ("Nenhuma cotação aberta no momento. Você será notificado por e-mail e neste painel.") — the edital names complex UI as the #1 boycott risk.
4. Inputs have visible labels; forms submit on Enter; FF components keep focus rings (accessibility comes with the kit — don't strip it).
5. Status vocabulary is consistent everywhere via `StatusBadge` (Rascunho/Aberta/Encerrada/Adjudicada/Bilhete enviado/Concluída).

---

### Task 12: Web scaffold — Vite + Tailwind 4 + shadcn + Fluid Functionalism, api client, auth, login

**Files:**
- Create: `web/` (Vite app), `web/src/lib/{api.ts, auth.tsx, domain.ts}`, `web/src/components/{Countdown.tsx, StatusBadge.tsx, Layout.tsx}`, `web/src/pages/Login.tsx`
- Replace: `web/src/App.tsx`, `web/src/main.tsx`, `web/src/index.css`

- [ ] **Step 1: Scaffold Vite + Tailwind 4 + path alias**

```bash
bun create vite web --template react-ts
cd web && bun install
bun add tailwindcss @tailwindcss/vite @tanstack/react-query @fontsource-variable/inter
bun add -d @types/node
cd ..
```

Replace `web/vite.config.ts`:

```ts
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import path from 'node:path';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  server: { port: 5173 },
});
```

Add to BOTH `web/tsconfig.json` and `web/tsconfig.app.json` under `"compilerOptions"`:

```json
    "baseUrl": ".",
    "paths": { "@/*": ["./src/*"] }
```

Replace `web/src/index.css`:

```css
@import 'tailwindcss';
@import '@fontsource-variable/inter';

:root {
  font-family: 'Inter Variable', system-ui, sans-serif;
}
```

- [ ] **Step 2: shadcn init + Fluid Functionalism components**

```bash
cd web
bunx shadcn@latest init
bunx shadcn@latest add input textarea label
bunx shadcn@latest add https://www.fluidfunctionalism.com/r/button.json
bunx shadcn@latest add https://www.fluidfunctionalism.com/r/card.json
bunx shadcn@latest add https://www.fluidfunctionalism.com/r/table.json
bunx shadcn@latest add https://www.fluidfunctionalism.com/r/badge.json
bunx shadcn@latest add https://www.fluidfunctionalism.com/r/dialog.json
bunx shadcn@latest add https://www.fluidfunctionalism.com/r/select.json
bunx shadcn@latest add https://www.fluidfunctionalism.com/r/tooltip.json
bunx shadcn@latest add sonner
cd ..
```

`shadcn init` prompts: pick defaults (style, base color slate/neutral, CSS variables yes). FF components land in `web/src/components/ui/` and are drop-in shadcn-compatible (spring motion + proximity hover included). Also install router:

```bash
cd web && bun add react-router-dom && cd ..
```

Note for the executor: if any FF registry URL 404s (component renamed upstream), fall back to `npx shadcn@latest add <name>` (plain shadcn) for that one component and continue — the app must not block on the kit.

- [ ] **Step 3: Domain utils (TS mirrors of the Rust domain — client-side validation/format only)**

`web/src/lib/domain.ts`:

```ts
export function isValidCnpj(input: string): boolean {
  const digits = input.replace(/\D/g, '');
  if (digits.length !== 14) return false;
  if (/^(\d)\1{13}$/.test(digits)) return false;
  const dv = (len: 12 | 13): number => {
    const weights =
      len === 12 ? [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2] : [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    const sum = weights.reduce((acc, w, i) => acc + w * Number(digits[i]), 0);
    const mod = sum % 11;
    return mod < 2 ? 0 : 11 - mod;
  };
  return dv(12) === Number(digits[12]) && dv(13) === Number(digits[13]);
}

export function parseBRL(input: string): number | null {
  const cleaned = input.replace(/[R$\s.]/g, '').replace(',', '.');
  if (!/^\d+(\.\d{1,2})?$/.test(cleaned)) return null;
  return Math.round(parseFloat(cleaned) * 100);
}

export function formatBRL(cents: number): string {
  return new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' }).format(cents / 100);
}

/** UX rule 1: everything renders in Boa Vista local time, never raw UTC. */
export function fmtDateTime(iso: string): string {
  return new Intl.DateTimeFormat('pt-BR', {
    dateStyle: 'short',
    timeStyle: 'short',
    timeZone: 'America/Boa_Vista',
  }).format(new Date(iso));
}

export function serverOffsetMs(serverNowIso: string, clientNowMs: number): number {
  return new Date(serverNowIso).getTime() - clientNowMs;
}

export function remainingMs(deadlineIso: string, offsetMs: number, clientNowMs: number): number {
  return Math.max(0, new Date(deadlineIso).getTime() - (clientNowMs + offsetMs));
}

export function formatMmSs(ms: number): string {
  const totalS = Math.floor(ms / 1000);
  const mm = String(Math.floor(totalS / 60)).padStart(2, '0');
  const ss = String(totalS % 60).padStart(2, '0');
  return `${mm}:${ss}`;
}
```

- [ ] **Step 4: API client + auth context**

`web/src/lib/api.ts`:

```ts
const API = import.meta.env.VITE_API_URL ?? 'http://localhost:3001';

export function getToken(): string | null {
  return localStorage.getItem('tj_token');
}

export function setToken(token: string | null): void {
  if (token === null) localStorage.removeItem('tj_token');
  else localStorage.setItem('tj_token', token);
}

export function apiUrl(path: string): string {
  return `${API}${path}`;
}

export async function api<T>(
  path: string,
  opts: { method?: string; body?: unknown; form?: FormData } = {},
): Promise<T> {
  const headers: Record<string, string> = {};
  const token = getToken();
  if (token) headers.Authorization = `Bearer ${token}`;
  let body: BodyInit | undefined;
  if (opts.form) {
    body = opts.form;
  } else if (opts.body !== undefined) {
    headers['Content-Type'] = 'application/json';
    body = JSON.stringify(opts.body);
  }
  const res = await fetch(apiUrl(path), { method: opts.method ?? 'GET', headers, body });
  if (!res.ok) {
    const detail = (await res.json().catch(() => ({}))) as { error?: string };
    throw new Error(detail.error ?? `HTTP ${res.status}`);
  }
  return res.json() as Promise<T>;
}

/** Opens a printable page (OS / relatório) in a new tab, authenticated via ?token=. */
export function openPage(path: string): void {
  window.open(`${apiUrl(path)}?token=${getToken()}`, '_blank');
}

/** SSE subscription with react-query invalidation on events. Returns cleanup. */
export function subscribeQuotation(id: string, onEvent: (event: string, data: unknown) => void): () => void {
  const source = new EventSource(`${apiUrl(`/quotations/${id}/events`)}?token=${getToken()}`);
  for (const name of ['hello', 'tick', 'status', 'proposal']) {
    source.addEventListener(name, (e) => onEvent(name, JSON.parse((e as MessageEvent).data)));
  }
  return () => source.close();
}
```

`web/src/lib/auth.tsx`:

```tsx
import { createContext, useContext, useState, type ReactNode } from 'react';
import { Navigate } from 'react-router-dom';
import { getToken, setToken } from './api';

export type SessionUser = {
  sub: string;
  name: string;
  role: 'ADMIN' | 'SERVIDOR' | 'FORNECEDOR';
  supplierId: string | null;
};

export function parseJwt(token: string): SessionUser | null {
  try {
    const payload = JSON.parse(atob(token.split('.')[1]));
    return {
      sub: payload.sub,
      name: payload.name,
      role: payload.role,
      supplierId: payload.supplier_id ?? null,
    };
  } catch {
    return null;
  }
}

type AuthCtx = { user: SessionUser | null; signIn: (token: string) => void; signOut: () => void };

const Ctx = createContext<AuthCtx>({ user: null, signIn: () => {}, signOut: () => {} });

export function AuthProvider({ children }: { children: ReactNode }) {
  const [user, setUser] = useState<SessionUser | null>(() => {
    const t = getToken();
    return t ? parseJwt(t) : null;
  });
  const signIn = (token: string) => {
    setToken(token);
    setUser(parseJwt(token));
  };
  const signOut = () => {
    setToken(null);
    setUser(null);
  };
  return <Ctx.Provider value={{ user, signIn, signOut }}>{children}</Ctx.Provider>;
}

export function useAuth(): AuthCtx {
  return useContext(Ctx);
}

export function RequireRole({
  roles,
  children,
}: {
  roles: SessionUser['role'][];
  children: ReactNode;
}) {
  const { user } = useAuth();
  if (!user || !roles.includes(user.role)) return <Navigate to="/login" replace />;
  return <>{children}</>;
}
```

(Note the JWT claim is `supplier_id` — Rust serde default — hence the mapping in `parseJwt`.)

- [ ] **Step 5: Shared components**

`web/src/components/StatusBadge.tsx`:

```tsx
import { Badge } from '@/components/ui/badge';

const LABELS: Record<string, string> = {
  DRAFT: 'Rascunho',
  OPEN: 'Aberta',
  CLOSED: 'Encerrada',
  AWARDED: 'Adjudicada',
  TICKETED: 'Bilhete enviado',
  COMPLETED: 'Concluída',
  PENDING: 'Pendente',
  ACTIVE: 'Credenciado',
  REJECTED: 'Rejeitado',
  SUSPENDED: 'Suspenso',
};

const CLASSES: Record<string, string> = {
  DRAFT: 'bg-slate-200 text-slate-700',
  OPEN: 'bg-green-100 text-green-800',
  CLOSED: 'bg-amber-100 text-amber-800',
  AWARDED: 'bg-blue-100 text-blue-800',
  TICKETED: 'bg-violet-100 text-violet-800',
  COMPLETED: 'bg-emerald-100 text-emerald-800',
  PENDING: 'bg-amber-100 text-amber-800',
  ACTIVE: 'bg-emerald-100 text-emerald-800',
  REJECTED: 'bg-red-100 text-red-800',
  SUSPENDED: 'bg-slate-200 text-slate-700',
};

export function StatusBadge({ status }: { status: string }) {
  return (
    <Badge className={CLASSES[status] ?? 'bg-slate-200 text-slate-700'} variant="secondary">
      {LABELS[status] ?? status}
    </Badge>
  );
}
```

`web/src/components/Countdown.tsx`:

```tsx
import { useEffect, useMemo, useState } from 'react';
import { formatMmSs, remainingMs, serverOffsetMs } from '@/lib/domain';

export function Countdown({
  deadline,
  serverNow,
  onExpire,
  size = 'lg',
}: {
  deadline: string;
  serverNow: string;
  onExpire?: () => void;
  size?: 'lg' | 'sm';
}) {
  const offset = useMemo(() => serverOffsetMs(serverNow, Date.now()), [serverNow]);
  const [ms, setMs] = useState(() => remainingMs(deadline, offset, Date.now()));
  useEffect(() => {
    const timer = setInterval(() => {
      const next = remainingMs(deadline, offset, Date.now());
      setMs(next);
      if (next === 0) {
        clearInterval(timer);
        onExpire?.();
      }
    }, 250);
    return () => clearInterval(timer);
  }, [deadline, offset, onExpire]);
  const urgent = ms > 0 && ms < 60_000;
  return (
    <span
      className={`font-mono font-bold tabular-nums ${size === 'lg' ? 'text-4xl' : 'text-lg'} ${
        urgent ? 'text-red-600 animate-pulse' : 'text-primary'
      }`}
      aria-live="polite"
    >
      {formatMmSs(ms)}
    </span>
  );
}
```

`web/src/components/Layout.tsx`:

```tsx
import type { ReactNode } from 'react';
import { Link } from 'react-router-dom';
import { Button } from '@/components/ui/button';
import { useAuth } from '@/lib/auth';

export function Layout({ children }: { children: ReactNode }) {
  const { user, signOut } = useAuth();
  const home = user?.role === 'FORNECEDOR' ? '/fornecedor' : '/';
  return (
    <div className="min-h-screen bg-muted/40">
      <header className="border-b bg-background">
        <div className="mx-auto flex max-w-5xl items-center justify-between p-3 md:p-4">
          <Link to={home} className="text-lg font-bold text-primary">
            TJ-Viagens <span className="text-sm font-normal text-muted-foreground">· TJRR</span>
          </Link>
          {user && (
            <div className="flex items-center gap-3 text-sm">
              <span className="hidden text-muted-foreground md:inline">{user.name}</span>
              <Button variant="outline" size="sm" onClick={signOut}>
                Sair
              </Button>
            </div>
          )}
        </div>
      </header>
      <main className="mx-auto max-w-5xl p-3 md:p-4">{children}</main>
    </div>
  );
}
```

- [ ] **Step 6: Login page, App shell, main**

`web/src/pages/Login.tsx`:

```tsx
import { useState, type FormEvent } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { api } from '@/lib/api';
import { parseJwt, useAuth } from '@/lib/auth';

export function Login() {
  const { signIn } = useAuth();
  const navigate = useNavigate();
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [pending, setPending] = useState(false);

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    setPending(true);
    try {
      const res = await api<{ token: string }>('/auth/login', {
        method: 'POST',
        body: { email, password },
      });
      signIn(res.token);
      const user = parseJwt(res.token);
      navigate(user?.role === 'FORNECEDOR' ? '/fornecedor' : '/');
    } catch {
      toast.error('E-mail ou senha inválidos.');
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-primary/95 p-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle className="text-2xl">TJ-Viagens</CardTitle>
          <p className="text-sm text-muted-foreground">
            Cotações competitivas de passagens aéreas · TJRR
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="space-y-4">
            <div className="space-y-1.5">
              <Label htmlFor="email">E-mail</Label>
              <Input id="email" type="email" value={email} onChange={(e) => setEmail(e.target.value)} required autoFocus />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="password">Senha</Label>
              <Input id="password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} required />
            </div>
            <Button type="submit" className="w-full" disabled={pending}>
              {pending ? 'Entrando…' : 'Entrar'}
            </Button>
            <p className="text-center text-sm text-muted-foreground">
              Agência de viagens ainda sem acesso?{' '}
              <Link to="/registro" className="text-primary underline">
                Solicite credenciamento
              </Link>
            </p>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
```

`web/src/App.tsx` (Task-12 version — supplier/staff routes land in Tasks 13/14, each replacing this file wholesale):

```tsx
import { Navigate, Route, Routes } from 'react-router-dom';
import { Login } from '@/pages/Login';

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="*" element={<Navigate to="/login" replace />} />
    </Routes>
  );
}
```

`web/src/main.tsx`:

```tsx
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { Toaster } from 'sonner';
import { App } from './App';
import { AuthProvider } from './lib/auth';
import './index.css';

const queryClient = new QueryClient({
  defaultOptions: { queries: { refetchOnWindowFocus: true, retry: 1 } },
});

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <AuthProvider>
          <App />
          <Toaster richColors position="top-center" />
        </AuthProvider>
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
);
```

Delete the Vite template leftovers: `web/src/App.css`, `web/src/assets/react.svg` (and its import if any).

- [ ] **Step 7: Verify + commit**

Run: `cd web && bunx tsc -b && bun run dev` — open http://localhost:5173 with the API running (`cd api && cargo run --bin api`). Create a supplier via curl and log in through the UI:

```bash
curl -s -X POST localhost:3001/suppliers/register -H 'content-type: application/json' -d '{"cnpj":"11.222.333/0001-81","legalName":"Voa Roraima","contactEmail":"contato@voaroraima.com.br","userName":"Ana","password":"demo1234"}'
```

Expected: login succeeds, redirects to `/fornecedor` (404s to login for now — routes come next). Toast on wrong password. Then:

```bash
git add web
git commit -m "feat(web): vite + tailwind4 + shadcn + fluid functionalism scaffold with auth and login"
```

---

### Task 13: Supplier area — register, home (docs + notifications + open cotações), single-task bid screen

**Files:**
- Create: `web/src/lib/types.ts`, `web/src/pages/Register.tsx`, `web/src/pages/supplier/Home.tsx`, `web/src/pages/supplier/QuotationBid.tsx`
- Replace: `web/src/App.tsx`

- [ ] **Step 1: Shared API types**

`web/src/lib/types.ts`:

```ts
export type ChecklistInfo = { missing: string[]; expired: string[]; ok: boolean };

export type SupplierInfo = {
  id: string;
  cnpj: string;
  legalName: string;
  contactEmail: string;
  phone: string | null;
  status: string;
  statusReason: string | null;
};

export type SupplierMe = { supplier: SupplierInfo; checklist: ChecklistInfo };

export type NotificationItem = {
  id: string;
  quotationId: string | null;
  kind: string;
  message: string;
  createdAt: string;
};

export type ProposalInfo = {
  id: string;
  supplierId: string;
  totalPriceCents: number;
  flightInfo: string;
  notes: string | null;
  submittedAt: string;
};

export type QuotationBase = {
  id: string;
  code: string;
  status: string;
  origin: string;
  destination: string;
  departureAt: string;
  returnAt: string | null;
  referenceFlight: string;
  opensAt: string | null;
  closesAt: string | null;
  serverNow: string;
};

export type SupplierQuotation = QuotationBase & {
  myProposal: ProposalInfo | null;
  isWinner: boolean;
  passenger?: { name: string; cpf: string; sex: string; birth: string };
  ticketDeadlineAt?: string | null;
};

export type StaffQuotation = QuotationBase & {
  passenger: { name: string; cpf: string; sex: string; birth: string };
  referencePriceCents: number;
  awardedProposalId: string | null;
  awardedAt: string | null;
  awardJustification: string | null;
  ticketDeadlineAt: string | null;
  proposals: { count: number } | ProposalInfo[];
};

export function proposalsCount(p: StaffQuotation['proposals']): number {
  return Array.isArray(p) ? p.length : p.count;
}

export type RankingRow = {
  position: number;
  proposalId: string;
  supplier: { id: string; legalName: string; cnpj: string };
  totalPriceCents: number;
  flightInfo: string;
  notes: string | null;
  submittedAt: string;
  deltaFromReferenceCents: number;
};

export type Metrics = {
  awardedCount: number;
  totalSavedCents: number;
  avgParticipants: number;
  ticketsOnTimePct: number;
};

export type SupplierListItem = { supplier: SupplierInfo; checklist: ChecklistInfo };

export type Report = {
  quotation: { code: string; status: string; passengerName: string; passengerCpfMasked: string };
  economy: { saved_cents: number; saved_pct: number } | null;
  serviceOrder: { number: string } | null;
  ticket: { fileName: string; late: boolean; divergences: string[] } | null;
};
```

- [ ] **Step 2: Public registration page**

`web/src/pages/Register.tsx`:

```tsx
import { useState, type FormEvent } from 'react';
import { Link } from 'react-router-dom';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { api } from '@/lib/api';
import { isValidCnpj } from '@/lib/domain';

export function Register() {
  const [form, setForm] = useState({
    cnpj: '',
    legalName: '',
    contactEmail: '',
    phone: '',
    userName: '',
    password: '',
  });
  const [done, setDone] = useState(false);
  const [pending, setPending] = useState(false);
  const set = (key: keyof typeof form) => (e: { target: { value: string } }) =>
    setForm({ ...form, [key]: e.target.value });

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    if (!isValidCnpj(form.cnpj)) {
      toast.error('CNPJ inválido — confira os dígitos.');
      return;
    }
    setPending(true);
    try {
      await api('/suppliers/register', {
        method: 'POST',
        body: { ...form, phone: form.phone || null },
      });
      setDone(true);
    } catch (err) {
      toast.error(
        err instanceof Error && err.message === 'JA_CADASTRADO'
          ? 'CNPJ ou e-mail já cadastrado.'
          : 'Falha no cadastro. Revise os dados.',
      );
    } finally {
      setPending(false);
    }
  }

  if (done) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-primary/95 p-4">
        <Card className="w-full max-w-md text-center">
          <CardHeader>
            <CardTitle>Solicitação enviada ✔</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <p className="text-sm text-muted-foreground">
              Entre com seu e-mail e senha para enviar os documentos obrigatórios e acompanhar a
              análise do credenciamento.
            </p>
            <Button asChild className="w-full">
              <Link to="/login">Ir para o login</Link>
            </Button>
          </CardContent>
        </Card>
      </div>
    );
  }

  const fields: Array<[keyof typeof form, string, string]> = [
    ['cnpj', 'CNPJ', '00.000.000/0000-00'],
    ['legalName', 'Razão social', 'Agência Exemplo Viagens LTDA'],
    ['contactEmail', 'E-mail de contato', 'contato@agencia.com.br'],
    ['phone', 'Telefone (opcional)', '(95) 99999-0000'],
    ['userName', 'Nome do responsável', 'Nome completo'],
    ['password', 'Senha (mínimo 8 caracteres)', ''],
  ];

  return (
    <div className="flex min-h-screen items-center justify-center bg-primary/95 p-4">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>Credenciamento de fornecedor</CardTitle>
          <p className="text-sm text-muted-foreground">
            Agências de viagens e companhias aéreas — credenciamento permanente do TJRR.
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="space-y-3">
            {fields.map(([key, label, placeholder]) => (
              <div key={key} className="space-y-1.5">
                <Label htmlFor={key}>{label}</Label>
                <Input
                  id={key}
                  type={key === 'password' ? 'password' : key === 'contactEmail' ? 'email' : 'text'}
                  placeholder={placeholder}
                  value={form[key]}
                  onChange={set(key)}
                  required={key !== 'phone'}
                  minLength={key === 'password' ? 8 : undefined}
                />
              </div>
            ))}
            <Button type="submit" className="w-full" disabled={pending}>
              {pending ? 'Enviando…' : 'Solicitar credenciamento'}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
```

- [ ] **Step 3: Supplier home (mobile-first cards)**

`web/src/pages/supplier/Home.tsx`:

```tsx
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type FormEvent } from 'react';
import { Link } from 'react-router-dom';
import { toast } from 'sonner';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select, SelectContent, SelectItem, SelectTrigger, SelectValue,
} from '@/components/ui/select';
import { api } from '@/lib/api';
import { fmtDateTime, formatBRL } from '@/lib/domain';
import type { NotificationItem, SupplierMe, SupplierQuotation } from '@/lib/types';

const DOC_TYPES = [
  ['CONTRATO_SOCIAL', 'Contrato social'],
  ['CND_FEDERAL', 'CND Federal (regularidade fiscal)'],
  ['CRF_FGTS', 'CRF do FGTS'],
  ['CNDT', 'CNDT (débitos trabalhistas)'],
] as const;

export function SupplierHome() {
  const queryClient = useQueryClient();
  const me = useQuery({ queryKey: ['me'], queryFn: () => api<SupplierMe>('/suppliers/me') });
  const notifications = useQuery({
    queryKey: ['notifications'],
    queryFn: () => api<NotificationItem[]>('/notifications'),
    refetchInterval: 15000,
  });
  const active = me.data?.supplier.status === 'ACTIVE';
  const quotations = useQuery({
    queryKey: ['quotations'],
    queryFn: () => api<SupplierQuotation[]>('/quotations'),
    enabled: active,
    refetchInterval: 15000,
  });

  const [docType, setDocType] = useState<string>(DOC_TYPES[0][0]);
  const [validUntil, setValidUntil] = useState('');
  const [file, setFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);

  async function uploadDoc(e: FormEvent) {
    e.preventDefault();
    if (!file) return;
    setUploading(true);
    const form = new FormData();
    form.append('type', docType);
    if (validUntil) form.append('validUntil', validUntil);
    form.append('file', file);
    try {
      await api('/suppliers/me/documents', { method: 'POST', form });
      toast.success('Documento enviado.');
      setFile(null);
      await queryClient.invalidateQueries({ queryKey: ['me'] });
    } catch {
      toast.error('Falha ao enviar o documento.');
    } finally {
      setUploading(false);
    }
  }

  if (!me.data) {
    return (
      <Layout>
        <p className="text-muted-foreground">Carregando…</p>
      </Layout>
    );
  }
  const { supplier, checklist } = me.data;

  return (
    <Layout>
      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center justify-between text-base">
              Credenciamento <StatusBadge status={supplier.status} />
            </CardTitle>
            <p className="text-sm text-muted-foreground">
              {supplier.legalName} · CNPJ {supplier.cnpj}
            </p>
          </CardHeader>
          <CardContent className="space-y-4">
            {supplier.statusReason && (
              <p className="rounded bg-amber-50 p-2 text-sm">{supplier.statusReason}</p>
            )}
            <ul className="space-y-1 text-sm">
              {DOC_TYPES.map(([key, label]) => {
                const state = checklist.missing.includes(key)
                  ? '✖ pendente'
                  : checklist.expired.includes(key)
                    ? '⚠ vencido — reenvie'
                    : '✔ ok';
                return (
                  <li key={key} className="flex justify-between gap-2">
                    <span>{label}</span>
                    <span className="text-muted-foreground">{state}</span>
                  </li>
                );
              })}
            </ul>
            <form onSubmit={uploadDoc} className="space-y-2 border-t pt-3">
              <Label>Enviar / atualizar documento</Label>
              <Select value={docType} onValueChange={setDocType}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {DOC_TYPES.map(([key, label]) => (
                    <SelectItem key={key} value={key}>
                      {label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <div className="space-y-1.5">
                <Label htmlFor="validUntil">Válido até</Label>
                <Input id="validUntil" type="date" value={validUntil} onChange={(e) => setValidUntil(e.target.value)} />
              </div>
              <Input type="file" onChange={(e) => setFile(e.target.files?.[0] ?? null)} required />
              <Button type="submit" disabled={uploading || !file} className="w-full md:w-auto">
                {uploading ? 'Enviando…' : 'Enviar documento'}
              </Button>
            </form>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Notificações</CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2 text-sm">
              {(notifications.data ?? []).map((n) => (
                <li key={n.id} className="rounded bg-muted p-2">
                  <p>{n.message}</p>
                  <p className="mt-1 text-xs text-muted-foreground">{fmtDateTime(n.createdAt)}</p>
                </li>
              ))}
              {(notifications.data ?? []).length === 0 && (
                <li className="text-muted-foreground">
                  Nenhuma notificação. Novas cotações aparecem aqui e no seu e-mail.
                </li>
              )}
            </ul>
          </CardContent>
        </Card>
      </div>

      {active && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Cotações</CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 md:grid-cols-2">
            {(quotations.data ?? []).map((q) => (
              <Link key={q.id} to={`/fornecedor/cotacoes/${q.id}`} className="block">
                <Card className="transition hover:border-primary">
                  <CardContent className="flex items-center justify-between gap-2 p-4">
                    <div>
                      <p className="font-semibold">
                        {q.origin} → {q.destination}
                        {q.isWinner && ' 🏆'}
                      </p>
                      <p className="text-sm text-muted-foreground">
                        {q.code} · embarque {fmtDateTime(q.departureAt)}
                      </p>
                      <p className="text-sm">
                        {q.myProposal
                          ? `Minha proposta: ${formatBRL(q.myProposal.totalPriceCents)}`
                          : 'Sem proposta ainda'}
                      </p>
                    </div>
                    <div className="text-right">
                      <StatusBadge status={q.status} />
                      {q.status === 'OPEN' && q.closesAt && (
                        <div className="mt-1">
                          <Countdown deadline={q.closesAt} serverNow={q.serverNow} size="sm" />
                        </div>
                      )}
                    </div>
                  </CardContent>
                </Card>
              </Link>
            ))}
            {(quotations.data ?? []).length === 0 && (
              <p className="text-sm text-muted-foreground">
                Nenhuma cotação aberta no momento. Você será notificado por e-mail e neste painel.
              </p>
            )}
          </CardContent>
        </Card>
      )}
    </Layout>
  );
}
```

- [ ] **Step 4: The bid screen (single task, big countdown, winner flow)**

`web/src/pages/supplier/QuotationBid.tsx`:

```tsx
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState, type FormEvent } from 'react';
import { useParams } from 'react-router-dom';
import { toast } from 'sonner';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';
import { api, openPage, subscribeQuotation } from '@/lib/api';
import { fmtDateTime, formatBRL, parseBRL } from '@/lib/domain';
import type { SupplierQuotation } from '@/lib/types';

export function SupplierQuotationPage() {
  const { id = '' } = useParams<{ id: string }>();
  const queryClient = useQueryClient();
  const { data: q } = useQuery({
    queryKey: ['quotation', id],
    queryFn: () => api<SupplierQuotation>(`/quotations/${id}`),
  });

  useEffect(() => {
    return subscribeQuotation(id, (event) => {
      if (event === 'status' || event === 'proposal') {
        void queryClient.invalidateQueries({ queryKey: ['quotation', id] });
      }
    });
  }, [id, queryClient]);

  const [price, setPrice] = useState('');
  const [flightInfo, setFlightInfo] = useState('');
  const [notes, setNotes] = useState('');
  const [pending, setPending] = useState(false);
  const [ticketPrice, setTicketPrice] = useState('');
  const [ticketFile, setTicketFile] = useState<File | null>(null);

  async function submitBid(e: FormEvent) {
    e.preventDefault();
    const cents = parseBRL(price);
    if (cents === null) {
      toast.error('Valor inválido — use o formato 1.523,00');
      return;
    }
    setPending(true);
    try {
      await api(`/quotations/${id}/proposals`, {
        method: 'POST',
        body: { totalPriceCents: cents, flightInfo, notes: notes || null },
      });
      toast.success('Proposta registrada. Enviar novamente substitui o valor.');
      await queryClient.invalidateQueries({ queryKey: ['quotation', id] });
    } catch (err) {
      toast.error(
        err instanceof Error && err.message === 'COTACAO_FECHADA'
          ? 'A janela de propostas já encerrou.'
          : 'Falha ao enviar a proposta.',
      );
    } finally {
      setPending(false);
    }
  }

  async function submitTicket(e: FormEvent) {
    e.preventDefault();
    if (!ticketFile || !q?.passenger) return;
    const cents = parseBRL(ticketPrice);
    if (cents === null) {
      toast.error('Valor do bilhete inválido.');
      return;
    }
    setPending(true);
    const form = new FormData();
    form.append('passengerName', q.passenger.name);
    form.append('flightInfo', q.myProposal?.flightInfo ?? '');
    form.append('departureAt', q.departureAt);
    form.append('priceCents', String(cents));
    form.append('file', ticketFile);
    try {
      const res = await api<{ late: boolean; divergences: string[] }>(`/quotations/${id}/ticket`, {
        method: 'POST',
        form,
      });
      if (res.divergences.length === 0) toast.success('E-ticket enviado sem divergências.');
      else toast.warning(`E-ticket enviado com divergências: ${res.divergences.join(', ')}`);
      await queryClient.invalidateQueries({ queryKey: ['quotation', id] });
    } catch {
      toast.error('Falha ao enviar o e-ticket.');
    } finally {
      setPending(false);
    }
  }

  if (!q) {
    return (
      <Layout>
        <p className="text-muted-foreground">Carregando…</p>
      </Layout>
    );
  }

  return (
    <Layout>
      {/* The brief: everything needed to quote, readable in 5 seconds */}
      <Card>
        <CardHeader>
          <CardTitle className="flex flex-wrap items-center justify-between gap-2 text-lg">
            <span>
              {q.code} · {q.origin} → {q.destination}
            </span>
            <StatusBadge status={q.status} />
          </CardTitle>
          <p className="text-sm text-muted-foreground">
            Embarque {fmtDateTime(q.departureAt)}
            {q.returnAt ? ` · retorno ${fmtDateTime(q.returnAt)}` : ''} · voo de referência{' '}
            {q.referenceFlight}
          </p>
        </CardHeader>
        {q.status === 'OPEN' && q.closesAt && (
          <CardContent className="rounded-lg bg-muted/60 py-6 text-center">
            <p className="mb-1 text-sm">Tempo restante para propostas</p>
            <Countdown
              deadline={q.closesAt}
              serverNow={q.serverNow}
              onExpire={() => void queryClient.invalidateQueries({ queryKey: ['quotation', id] })}
            />
            <p className="mt-1 text-xs text-muted-foreground">
              Encerramento pelo horário oficial do servidor · {q.closesAt && fmtDateTime(q.closesAt)}{' '}
              (horário de Boa Vista)
            </p>
          </CardContent>
        )}
      </Card>

      {q.status === 'OPEN' && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Minha proposta (sigilosa)</CardTitle>
            {q.myProposal && (
              <p className="text-sm text-muted-foreground">
                Registrada: {formatBRL(q.myProposal.totalPriceCents)} às{' '}
                {fmtDateTime(q.myProposal.submittedAt)} — enviar novamente substitui.
              </p>
            )}
          </CardHeader>
          <CardContent>
            <form onSubmit={submitBid} className="space-y-3">
              <div className="space-y-1.5">
                <Label htmlFor="price">Valor total (R$)</Label>
                <Input
                  id="price"
                  inputMode="decimal"
                  placeholder="1.523,00"
                  value={price}
                  onChange={(e) => setPrice(e.target.value)}
                  required
                  className="text-lg"
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="flight">Voo ofertado</Label>
                <Input
                  id="flight"
                  placeholder="G3-1720 · 10/09 08:15"
                  value={flightInfo}
                  onChange={(e) => setFlightInfo(e.target.value)}
                  required
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="notes">Observações (bagagem, conexões…)</Label>
                <Textarea id="notes" value={notes} onChange={(e) => setNotes(e.target.value)} />
              </div>
              <Button type="submit" className="w-full md:w-auto" disabled={pending}>
                {pending ? 'Enviando…' : q.myProposal ? 'Substituir proposta' : 'Enviar proposta'}
              </Button>
              <p className="text-xs text-muted-foreground">
                Você não vê as propostas concorrentes nem o preço de referência do Tribunal — a
                disputa é cega e isonômica.
              </p>
            </form>
          </CardContent>
        </Card>
      )}

      {q.status === 'CLOSED' && (
        <Card className="mt-4">
          <CardContent className="p-6 text-center text-sm text-muted-foreground">
            Janela encerrada. Aguardando a declaração da vencedora pelo TJRR — você será notificado.
          </CardContent>
        </Card>
      )}

      {q.isWinner && q.status === 'AWARDED' && q.passenger && (
        <Card className="mt-4 border-primary">
          <CardHeader>
            <CardTitle className="text-base">🏆 Sua proposta venceu — emita e anexe o e-ticket</CardTitle>
            {q.ticketDeadlineAt && (
              <p className="text-sm">
                Prazo de emissão:{' '}
                <Countdown deadline={q.ticketDeadlineAt} serverNow={q.serverNow} size="sm" />
              </p>
            )}
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="rounded bg-muted p-3 text-sm">
              <p className="font-semibold">Dados do passageiro</p>
              <p>
                {q.passenger.name} · CPF {q.passenger.cpf} · {q.passenger.sex} · nasc.{' '}
                {q.passenger.birth}
              </p>
            </div>
            <Button variant="outline" onClick={() => openPage(`/quotations/${q.id}/service-order`)}>
              Ver Ordem de Serviço
            </Button>
            <form onSubmit={submitTicket} className="space-y-3 border-t pt-3">
              <div className="space-y-1.5">
                <Label htmlFor="ticketPrice">Valor emitido (R$)</Label>
                <Input
                  id="ticketPrice"
                  inputMode="decimal"
                  placeholder="1.523,00"
                  value={ticketPrice}
                  onChange={(e) => setTicketPrice(e.target.value)}
                  required
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="ticketFile">Arquivo do e-ticket (PDF)</Label>
                <Input
                  id="ticketFile"
                  type="file"
                  onChange={(e) => setTicketFile(e.target.files?.[0] ?? null)}
                  required
                />
              </div>
              <Button type="submit" disabled={pending || !ticketFile} className="w-full md:w-auto">
                {pending ? 'Enviando…' : 'Anexar e-ticket'}
              </Button>
            </form>
          </CardContent>
        </Card>
      )}

      {(q.status === 'TICKETED' || q.status === 'COMPLETED') && (
        <Card className="mt-4">
          <CardContent className="p-6 text-center text-sm">
            {q.isWinner
              ? 'E-ticket enviado. O TJRR fará a conferência final.'
              : 'Cotação concluída.'}
          </CardContent>
        </Card>
      )}
    </Layout>
  );
}
```

- [ ] **Step 5: Wire routes, verify, commit**

Replace `web/src/App.tsx`:

```tsx
import { Navigate, Route, Routes } from 'react-router-dom';
import { RequireRole } from '@/lib/auth';
import { Login } from '@/pages/Login';
import { Register } from '@/pages/Register';
import { SupplierHome } from '@/pages/supplier/Home';
import { SupplierQuotationPage } from '@/pages/supplier/QuotationBid';

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/registro" element={<Register />} />
      <Route
        path="/fornecedor"
        element={
          <RequireRole roles={['FORNECEDOR']}>
            <SupplierHome />
          </RequireRole>
        }
      />
      <Route
        path="/fornecedor/cotacoes/:id"
        element={
          <RequireRole roles={['FORNECEDOR']}>
            <SupplierQuotationPage />
          </RequireRole>
        }
      />
      <Route path="*" element={<Navigate to="/login" replace />} />
    </Routes>
  );
}
```

Run: `cd web && bunx tsc -b` — no type errors. Manual smoke with API running: register at `/registro`, log in, see PENDING home with checklist, upload a doc (any small PDF), checklist flips. Resize to phone width (devtools) — cards stack single-column.

```bash
git add web/src
git commit -m "feat(web): supplier area - registration, docs/checklist home and single-task bid screen"
```

---

### Task 14: Staff area — action-queue dashboard, fast demand form, state-machine detail, homologation

**Files:**
- Create: `web/src/pages/staff/Dashboard.tsx`, `web/src/pages/staff/NewQuotation.tsx`, `web/src/pages/staff/QuotationDetail.tsx`, `web/src/pages/staff/Suppliers.tsx`
- Replace: `web/src/App.tsx` (final version)

- [ ] **Step 1: Action-queue dashboard with KPIs**

`web/src/pages/staff/Dashboard.tsx`:

```tsx
import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import { Link } from 'react-router-dom';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { api } from '@/lib/api';
import { fmtDateTime, formatBRL } from '@/lib/domain';
import { proposalsCount, type Metrics, type StaffQuotation } from '@/lib/types';

function QuotationLine({ q, cta }: { q: StaffQuotation; cta: string }) {
  return (
    <li className="flex items-center justify-between gap-2 rounded border p-3">
      <div>
        <p className="font-medium">
          {q.code} · {q.origin} → {q.destination}
        </p>
        <p className="text-sm text-muted-foreground">
          {q.passenger.name} · embarque {fmtDateTime(q.departureAt)} · {proposalsCount(q.proposals)}{' '}
          proposta(s)
        </p>
      </div>
      <div className="flex items-center gap-3">
        {q.status === 'OPEN' && q.closesAt && (
          <Countdown deadline={q.closesAt} serverNow={q.serverNow} size="sm" />
        )}
        <Button asChild size="sm">
          <Link to={`/cotacoes/${q.id}`}>{cta}</Link>
        </Button>
      </div>
    </li>
  );
}

export function StaffDashboard() {
  const metrics = useQuery({ queryKey: ['metrics'], queryFn: () => api<Metrics>('/metrics/summary') });
  const quotations = useQuery({
    queryKey: ['staff-quotations'],
    queryFn: () => api<StaffQuotation[]>('/quotations'),
    refetchInterval: 10000,
  });

  const groups = useMemo(() => {
    const g = {
      toAward: [] as StaffQuotation[],
      toConfirm: [] as StaffQuotation[],
      draft: [] as StaffQuotation[],
      open: [] as StaffQuotation[],
      done: [] as StaffQuotation[],
    };
    for (const q of quotations.data ?? []) {
      if (q.status === 'CLOSED') g.toAward.push(q);
      else if (q.status === 'TICKETED') g.toConfirm.push(q);
      else if (q.status === 'DRAFT') g.draft.push(q);
      else if (q.status === 'OPEN' || q.status === 'AWARDED') g.open.push(q);
      else g.done.push(q);
    }
    return g;
  }, [quotations.data]);

  const kpis = metrics.data;
  const queue: Array<[string, StaffQuotation[], string]> = [
    ['Encerradas — declarar vencedora', groups.toAward, 'Declarar'],
    ['Bilhetes para conferir', groups.toConfirm, 'Conferir'],
    ['Rascunhos — abrir disputa', groups.draft, 'Abrir'],
    ['Em andamento', groups.open, 'Acompanhar'],
  ];

  return (
    <Layout>
      <div className="mb-4 flex flex-wrap items-center justify-between gap-2">
        <h1 className="text-xl font-bold">Painel de cotações</h1>
        <div className="flex gap-2">
          <Button variant="outline" asChild>
            <Link to="/fornecedores">Fornecedores</Link>
          </Button>
          <Button asChild>
            <Link to="/cotacoes/nova">Nova cotação</Link>
          </Button>
        </div>
      </div>

      {kpis && (
        <div className="mb-4 grid grid-cols-2 gap-3 md:grid-cols-4">
          {(
            [
              ['Economia acumulada', formatBRL(kpis.totalSavedCents), 'text-emerald-700'],
              ['Cotações adjudicadas', String(kpis.awardedCount), ''],
              ['Média de participantes', String(kpis.avgParticipants), ''],
              ['E-tickets no prazo', `${kpis.ticketsOnTimePct}%`, ''],
            ] as const
          ).map(([label, value, cls]) => (
            <Card key={label}>
              <CardContent className="p-4">
                <p className="text-xs text-muted-foreground">{label}</p>
                <p className={`text-xl font-bold ${cls}`}>{value}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {queue.map(([title, items, cta]) =>
        items.length === 0 ? null : (
          <Card key={title} className="mb-4">
            <CardHeader>
              <CardTitle className="text-base">{title}</CardTitle>
            </CardHeader>
            <CardContent>
              <ul className="space-y-2">
                {items.map((q) => (
                  <QuotationLine key={q.id} q={q} cta={cta} />
                ))}
              </ul>
            </CardContent>
          </Card>
        ),
      )}

      {groups.done.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Concluídas</CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2 text-sm">
              {groups.done.map((q) => (
                <li key={q.id} className="flex items-center justify-between rounded border p-3">
                  <span>
                    {q.code} · {q.origin} → {q.destination}
                  </span>
                  <span className="flex items-center gap-2">
                    <StatusBadge status={q.status} />
                    <Link className="text-primary underline" to={`/cotacoes/${q.id}`}>
                      Ver dossiê
                    </Link>
                  </span>
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      )}

      {(quotations.data ?? []).length === 0 && (
        <Card>
          <CardContent className="p-8 text-center text-sm text-muted-foreground">
            Nenhuma cotação ainda. Clique em “Nova cotação” para registrar a primeira demanda.
          </CardContent>
        </Card>
      )}
    </Layout>
  );
}
```

- [ ] **Step 2: Fast demand form (defaults, masks, sigilo labeling)**

`web/src/pages/staff/NewQuotation.tsx`:

```tsx
import { useState, type FormEvent } from 'react';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import { Layout } from '@/components/Layout';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select, SelectContent, SelectItem, SelectTrigger, SelectValue,
} from '@/components/ui/select';
import { api } from '@/lib/api';
import { parseBRL } from '@/lib/domain';

export function NewQuotation() {
  const navigate = useNavigate();
  const [pending, setPending] = useState(false);
  const [sex, setSex] = useState('F');
  const [form, setForm] = useState({
    passengerName: '',
    passengerCpf: '',
    passengerBirth: '',
    origin: 'BVB',
    destination: '',
    departureAt: '',
    returnAt: '',
    referenceFlight: '',
    referencePrice: '',
  });
  const set = (key: keyof typeof form) => (e: { target: { value: string } }) =>
    setForm({ ...form, [key]: e.target.value });

  async function onSubmit(e: FormEvent) {
    e.preventDefault();
    const cents = parseBRL(form.referencePrice);
    if (cents === null) {
      toast.error('Preço de referência inválido — use o formato 1.850,00');
      return;
    }
    setPending(true);
    try {
      const q = await api<{ id: string }>('/quotations', {
        method: 'POST',
        body: {
          passengerName: form.passengerName,
          passengerCpf: form.passengerCpf,
          passengerSex: sex,
          passengerBirth: form.passengerBirth,
          origin: form.origin.toUpperCase(),
          destination: form.destination.toUpperCase(),
          departureAt: new Date(form.departureAt).toISOString(),
          returnAt: form.returnAt ? new Date(form.returnAt).toISOString() : null,
          referenceFlight: form.referenceFlight,
          referencePriceCents: cents,
        },
      });
      toast.success('Rascunho criado. Revise e abra a disputa.');
      navigate(`/cotacoes/${q.id}`);
    } catch {
      toast.error('Falha ao criar a cotação. Confira CPF e datas.');
    } finally {
      setPending(false);
    }
  }

  return (
    <Layout>
      <Card className="mx-auto max-w-2xl">
        <CardHeader>
          <CardTitle>Nova demanda de passagem</CardTitle>
          <p className="text-sm text-muted-foreground">
            Dados do formulário do passageiro (Nome, CPF, Sexo, Nascimento) preenchem a Ordem de
            Serviço automaticamente.
          </p>
        </CardHeader>
        <CardContent>
          <form onSubmit={onSubmit} className="grid gap-3 md:grid-cols-2">
            <div className="space-y-1.5 md:col-span-2">
              <Label htmlFor="passengerName">Nome do passageiro</Label>
              <Input id="passengerName" value={form.passengerName} onChange={set('passengerName')} required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="passengerCpf">CPF</Label>
              <Input id="passengerCpf" placeholder="000.000.000-00" value={form.passengerCpf} onChange={set('passengerCpf')} required />
            </div>
            <div className="space-y-1.5">
              <Label>Sexo</Label>
              <Select value={sex} onValueChange={setSex}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="F">Feminino</SelectItem>
                  <SelectItem value="M">Masculino</SelectItem>
                  <SelectItem value="O">Outro</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="passengerBirth">Nascimento</Label>
              <Input id="passengerBirth" type="date" value={form.passengerBirth} onChange={set('passengerBirth')} required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="origin">Origem</Label>
              <Input id="origin" value={form.origin} onChange={set('origin')} required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="destination">Destino</Label>
              <Input id="destination" placeholder="BSB" value={form.destination} onChange={set('destination')} required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="departureAt">Embarque</Label>
              <Input id="departureAt" type="datetime-local" value={form.departureAt} onChange={set('departureAt')} required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="returnAt">Retorno (opcional)</Label>
              <Input id="returnAt" type="datetime-local" value={form.returnAt} onChange={set('returnAt')} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="referenceFlight">Voo de referência</Label>
              <Input id="referenceFlight" placeholder="LA-4001" value={form.referenceFlight} onChange={set('referenceFlight')} required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="referencePrice">Preço de referência (R$)</Label>
              <Input id="referencePrice" inputMode="decimal" placeholder="1.850,00" value={form.referencePrice} onChange={set('referencePrice')} required />
              <p className="text-xs text-muted-foreground">
                🔒 Sigiloso — nunca é exibido aos fornecedores durante a disputa.
              </p>
            </div>
            <div className="md:col-span-2">
              <Button type="submit" disabled={pending} className="w-full md:w-auto">
                {pending ? 'Criando…' : 'Criar rascunho'}
              </Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </Layout>
  );
}
```

- [ ] **Step 3: State-machine detail (open → live count → one-click award → conference → dossiê)**

`web/src/pages/staff/QuotationDetail.tsx`:

```tsx
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';
import { toast } from 'sonner';
import { Countdown } from '@/components/Countdown';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle, DialogTrigger,
} from '@/components/ui/dialog';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { api, openPage, subscribeQuotation } from '@/lib/api';
import { fmtDateTime, formatBRL } from '@/lib/domain';
import {
  proposalsCount, type RankingRow, type Report, type StaffQuotation,
} from '@/lib/types';

export function StaffQuotationDetail() {
  const { id = '' } = useParams<{ id: string }>();
  const queryClient = useQueryClient();
  const [pending, setPending] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [justification, setJustification] = useState('Menor preço entre as propostas válidas.');

  const { data: q } = useQuery({
    queryKey: ['staff-quotation', id],
    queryFn: () => api<StaffQuotation>(`/quotations/${id}`),
  });
  const closedOrLater = q && ['CLOSED'].includes(q.status);
  const ranking = useQuery({
    queryKey: ['ranking', id],
    queryFn: () => api<{ ranking: RankingRow[] }>(`/quotations/${id}/ranking`),
    enabled: Boolean(closedOrLater),
  });
  const showDossier = q && ['TICKETED', 'COMPLETED'].includes(q.status);
  const report = useQuery({
    queryKey: ['report', id],
    queryFn: () => api<Report>(`/quotations/${id}/report.json`),
    enabled: Boolean(showDossier),
  });
  const audit = useQuery({
    queryKey: ['audit'],
    queryFn: () => api<{ ok: boolean }>('/audit/verify'),
    enabled: Boolean(showDossier),
  });

  useEffect(() => {
    return subscribeQuotation(id, (event) => {
      if (event === 'status' || event === 'proposal') {
        void queryClient.invalidateQueries({ queryKey: ['staff-quotation', id] });
        void queryClient.invalidateQueries({ queryKey: ['ranking', id] });
        void queryClient.invalidateQueries({ queryKey: ['report', id] });
      }
    });
  }, [id, queryClient]);

  // recommended winner: lowest price pre-selected (UX: one-click adjudication)
  useEffect(() => {
    const first = ranking.data?.ranking[0];
    if (first && selected === null) setSelected(first.proposalId);
  }, [ranking.data, selected]);

  async function act(path: string, body?: unknown, success?: string) {
    setPending(true);
    try {
      await api(`/quotations/${id}/${path}`, { method: 'POST', body });
      if (success) toast.success(success);
      await queryClient.invalidateQueries();
    } catch (err) {
      toast.error(err instanceof Error ? `Falha: ${err.message}` : 'Falha na operação.');
    } finally {
      setPending(false);
    }
  }

  if (!q) {
    return (
      <Layout>
        <p className="text-muted-foreground">Carregando…</p>
      </Layout>
    );
  }

  return (
    <Layout>
      <Card>
        <CardHeader>
          <CardTitle className="flex flex-wrap items-center justify-between gap-2 text-lg">
            <span>
              {q.code} · {q.origin} → {q.destination}
            </span>
            <StatusBadge status={q.status} />
          </CardTitle>
          <div className="grid gap-1 text-sm text-muted-foreground md:grid-cols-2">
            <p>
              Passageiro: {q.passenger.name} · CPF {q.passenger.cpf}
            </p>
            <p>Embarque: {fmtDateTime(q.departureAt)}</p>
            <p>Voo de referência: {q.referenceFlight}</p>
            <p className="font-medium text-foreground">
              🔒 Preço de referência (sigiloso): {formatBRL(q.referencePriceCents)}
            </p>
          </div>
        </CardHeader>
      </Card>

      {q.status === 'DRAFT' && (
        <Card className="mt-4">
          <CardContent className="p-6 text-center">
            <p className="mb-3 text-sm text-muted-foreground">
              Ao abrir, todos os fornecedores credenciados ativos serão notificados simultaneamente
              e a janela de propostas de 60 minutos começará a contar no horário do servidor.
            </p>
            <Dialog>
              <DialogTrigger asChild>
                <Button size="lg">Abrir cotação</Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Abrir a disputa {q.code}?</DialogTitle>
                </DialogHeader>
                <p className="text-sm text-muted-foreground">
                  A notificação é irreversível e o cronômetro de 60 minutos inicia imediatamente.
                </p>
                <DialogFooter>
                  <Button
                    disabled={pending}
                    onClick={() => void act('open', undefined, 'Cotação aberta — fornecedores notificados.')}
                  >
                    Confirmar abertura
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
          </CardContent>
        </Card>
      )}

      {q.status === 'OPEN' && q.closesAt && (
        <Card className="mt-4">
          <CardContent className="p-8 text-center">
            <p className="text-sm">Janela de propostas em andamento</p>
            <Countdown
              deadline={q.closesAt}
              serverNow={q.serverNow}
              onExpire={() => void queryClient.invalidateQueries({ queryKey: ['staff-quotation', id] })}
            />
            <p className="mt-4 text-4xl font-bold">{proposalsCount(q.proposals)}</p>
            <p className="text-sm text-muted-foreground">
              propostas recebidas — valores lacrados até o encerramento (isonomia)
            </p>
          </CardContent>
        </Card>
      )}

      {q.status === 'CLOSED' && ranking.data && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Propostas — menor para maior</CardTitle>
            <p className="text-sm text-muted-foreground">
              A 1ª colocada já vem selecionada. Confira a conformidade e declare a vencedora.
            </p>
          </CardHeader>
          <CardContent className="space-y-3">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead />
                  <TableHead>Fornecedor</TableHead>
                  <TableHead>Valor</TableHead>
                  <TableHead>Δ vs referência</TableHead>
                  <TableHead>Voo</TableHead>
                  <TableHead>Enviada às</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {ranking.data.ranking.map((r) => (
                  <TableRow
                    key={r.proposalId}
                    className={selected === r.proposalId ? 'bg-primary/5' : ''}
                    onClick={() => setSelected(r.proposalId)}
                  >
                    <TableCell>
                      <input
                        type="radio"
                        name="winner"
                        aria-label={`Selecionar ${r.supplier.legalName}`}
                        checked={selected === r.proposalId}
                        onChange={() => setSelected(r.proposalId)}
                      />
                    </TableCell>
                    <TableCell>
                      {r.position}º {r.supplier.legalName}
                    </TableCell>
                    <TableCell className="font-semibold">{formatBRL(r.totalPriceCents)}</TableCell>
                    <TableCell
                      className={r.deltaFromReferenceCents < 0 ? 'text-emerald-700' : 'text-red-700'}
                    >
                      {formatBRL(r.deltaFromReferenceCents)}
                    </TableCell>
                    <TableCell>{r.flightInfo}</TableCell>
                    <TableCell>{fmtDateTime(r.submittedAt)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
            <Textarea
              value={justification}
              onChange={(e) => setJustification(e.target.value)}
              aria-label="Justificativa"
            />
            <Button
              disabled={pending || selected === null || justification.trim().length < 5}
              onClick={() =>
                void act(
                  'award',
                  { proposalId: selected, justification },
                  'Vencedora declarada — Ordem de Serviço emitida.',
                )
              }
            >
              Declarar vencedora e emitir OS
            </Button>
          </CardContent>
        </Card>
      )}

      {q.status === 'AWARDED' && (
        <Card className="mt-4">
          <CardHeader>
            <CardTitle className="text-base">Aguardando e-ticket da vencedora</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {q.ticketDeadlineAt && (
              <p>
                Prazo de 30 minutos:{' '}
                <Countdown deadline={q.ticketDeadlineAt} serverNow={q.serverNow} size="sm" />
              </p>
            )}
            <Button variant="outline" onClick={() => openPage(`/quotations/${q.id}/service-order`)}>
              Ver Ordem de Serviço
            </Button>
          </CardContent>
        </Card>
      )}

      {showDossier && report.data && (
        <div className="mt-4 grid gap-4 md:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Conferência do e-ticket</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2 text-sm">
              {report.data.ticket && (
                <>
                  <div className="grid grid-cols-2 gap-2">
                    <div className="rounded bg-muted p-2">
                      <p className="text-xs text-muted-foreground">Pedido</p>
                      <p>{report.data.quotation.passengerName}</p>
                      <p>{fmtDateTime(q.departureAt)}</p>
                    </div>
                    <div className="rounded bg-muted p-2">
                      <p className="text-xs text-muted-foreground">Bilhete</p>
                      <p>{report.data.ticket.fileName}</p>
                      <p>
                        {report.data.ticket.late
                          ? '⚠ FORA do prazo de 30 min'
                          : '✔ dentro do prazo'}
                      </p>
                    </div>
                  </div>
                  <p>
                    {report.data.ticket.divergences.length === 0
                      ? '✔ Sem divergências detectadas'
                      : `⚠ Divergências: ${report.data.ticket.divergences.join(', ')}`}
                  </p>
                  {q.status === 'TICKETED' && (
                    <Button
                      disabled={pending}
                      onClick={() =>
                        void act('ticket/confirm', undefined, 'Cotação concluída.')
                      }
                    >
                      Confirmar e concluir
                    </Button>
                  )}
                </>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Economicidade e dossiê</CardTitle>
            </CardHeader>
            <CardContent className="space-y-2">
              {report.data.economy && (
                <p className="text-2xl font-bold text-emerald-700">
                  {formatBRL(report.data.economy.saved_cents)}{' '}
                  <span className="text-base font-normal">
                    ({report.data.economy.saved_pct}% abaixo da referência)
                  </span>
                </p>
              )}
              {report.data.serviceOrder && (
                <p className="text-sm text-muted-foreground">
                  OS {report.data.serviceOrder.number}
                </p>
              )}
              <p className="text-sm">
                {audit.data?.ok === true
                  ? '🔒 Trilha de auditoria íntegra'
                  : audit.data
                    ? '❌ Trilha de auditoria VIOLADA'
                    : ''}
              </p>
              <div className="flex flex-wrap gap-2">
                <Button variant="outline" onClick={() => openPage(`/quotations/${q.id}/report`)}>
                  Relatório (imprimir/PDF)
                </Button>
                <Button variant="outline" onClick={() => openPage(`/quotations/${q.id}/service-order`)}>
                  Ordem de Serviço
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </Layout>
  );
}
```

- [ ] **Step 4: Homologation screen**

`web/src/pages/staff/Suppliers.tsx`:

```tsx
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { toast } from 'sonner';
import { Layout } from '@/components/Layout';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle,
} from '@/components/ui/dialog';
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
} from '@/components/ui/table';
import { Textarea } from '@/components/ui/textarea';
import { api } from '@/lib/api';
import type { SupplierListItem } from '@/lib/types';

export function StaffSuppliers() {
  const queryClient = useQueryClient();
  const { data: rows } = useQuery({
    queryKey: ['suppliers'],
    queryFn: () => api<SupplierListItem[]>('/suppliers'),
  });
  const [pending, setPending] = useState(false);
  const [rejecting, setRejecting] = useState<string | null>(null);
  const [reason, setReason] = useState('');

  async function decide(id: string, decision: 'APPROVE' | 'REJECT', why?: string) {
    setPending(true);
    try {
      await api(`/suppliers/${id}/decision`, {
        method: 'POST',
        body: { decision, reason: why ?? null },
      });
      toast.success(decision === 'APPROVE' ? 'Fornecedor homologado.' : 'Credenciamento rejeitado.');
      setRejecting(null);
      setReason('');
      await queryClient.invalidateQueries({ queryKey: ['suppliers'] });
    } catch (err) {
      toast.error(
        err instanceof Error && err.message === 'CHECKLIST_PENDENTE'
          ? 'Checklist incompleto — documentos faltando ou vencidos.'
          : 'Falha na decisão.',
      );
    } finally {
      setPending(false);
    }
  }

  return (
    <Layout>
      <Card>
        <CardHeader>
          <CardTitle>Credenciamento de fornecedores</CardTitle>
          <p className="text-sm text-muted-foreground">
            A pré-triagem do checklist é automática e determinística; a homologação é decisão do
            servidor.
          </p>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Razão social</TableHead>
                <TableHead>CNPJ</TableHead>
                <TableHead>Checklist</TableHead>
                <TableHead>Status</TableHead>
                <TableHead />
              </TableRow>
            </TableHeader>
            <TableBody>
              {(rows ?? []).map(({ supplier, checklist }) => (
                <TableRow key={supplier.id}>
                  <TableCell>{supplier.legalName}</TableCell>
                  <TableCell>{supplier.cnpj}</TableCell>
                  <TableCell className="text-sm">
                    {checklist.ok
                      ? '✔ completo'
                      : `✖ ${[...checklist.missing.map((m) => `${m} pendente`), ...checklist.expired.map((x) => `${x} vencido`)].join(', ')}`}
                  </TableCell>
                  <TableCell>
                    <StatusBadge status={supplier.status} />
                  </TableCell>
                  <TableCell className="space-x-2">
                    {supplier.status === 'PENDING' && (
                      <>
                        <Button
                          size="sm"
                          disabled={pending || !checklist.ok}
                          onClick={() => void decide(supplier.id, 'APPROVE')}
                        >
                          Homologar
                        </Button>
                        <Button
                          size="sm"
                          variant="destructive"
                          disabled={pending}
                          onClick={() => setRejecting(supplier.id)}
                        >
                          Rejeitar
                        </Button>
                      </>
                    )}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
          {(rows ?? []).length === 0 && (
            <p className="p-4 text-center text-sm text-muted-foreground">
              Nenhuma solicitação de credenciamento ainda.
            </p>
          )}
        </CardContent>
      </Card>

      <Dialog open={rejecting !== null} onOpenChange={(open) => !open && setRejecting(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Justificativa da rejeição</DialogTitle>
          </DialogHeader>
          <Textarea
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder="Ex.: Documentação fiscal vencida e não reapresentada."
          />
          <DialogFooter>
            <Button
              variant="destructive"
              disabled={pending || reason.trim().length < 5}
              onClick={() => rejecting && void decide(rejecting, 'REJECT', reason)}
            >
              Confirmar rejeição
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Layout>
  );
}
```

- [ ] **Step 5: Final routes, verify, commit**

Replace `web/src/App.tsx` (final):

```tsx
import { Navigate, Route, Routes } from 'react-router-dom';
import { RequireRole } from '@/lib/auth';
import { Login } from '@/pages/Login';
import { Register } from '@/pages/Register';
import { StaffDashboard } from '@/pages/staff/Dashboard';
import { NewQuotation } from '@/pages/staff/NewQuotation';
import { StaffQuotationDetail } from '@/pages/staff/QuotationDetail';
import { StaffSuppliers } from '@/pages/staff/Suppliers';
import { SupplierHome } from '@/pages/supplier/Home';
import { SupplierQuotationPage } from '@/pages/supplier/QuotationBid';

const STAFF: Array<'SERVIDOR' | 'ADMIN'> = ['SERVIDOR', 'ADMIN'];

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<Login />} />
      <Route path="/registro" element={<Register />} />
      <Route path="/" element={<RequireRole roles={STAFF}><StaffDashboard /></RequireRole>} />
      <Route path="/cotacoes/nova" element={<RequireRole roles={STAFF}><NewQuotation /></RequireRole>} />
      <Route path="/cotacoes/:id" element={<RequireRole roles={STAFF}><StaffQuotationDetail /></RequireRole>} />
      <Route path="/fornecedores" element={<RequireRole roles={STAFF}><StaffSuppliers /></RequireRole>} />
      <Route path="/fornecedor" element={<RequireRole roles={['FORNECEDOR']}><SupplierHome /></RequireRole>} />
      <Route path="/fornecedor/cotacoes/:id" element={<RequireRole roles={['FORNECEDOR']}><SupplierQuotationPage /></RequireRole>} />
      <Route path="*" element={<Navigate to="/login" replace />} />
    </Routes>
  );
}
```

Run: `cd web && bunx tsc -b` — clean. **UX acceptance walk** (two browser windows, API running with `PROPOSAL_WINDOW_MINUTES=2`):
1. Staff window: create demand → confirm-open dialog → live count + countdown.
2. Supplier window (phone width): bid page brief readable without scrolling past the fold; countdown huge; submit shows toast + "substituir" state.
3. After close (2 min): staff sees ranking with 1st pre-selected; declare in one click; supplier sees winner banner; upload ticket; staff conference shows side-by-side + confirm; dashboard KPIs update; printable OS/report open via buttons.

```bash
git add web/src
git commit -m "feat(web): staff area - action-queue dashboard, demand form, state-machine detail, homologation"
```

---

### Task 15: Seed data, README quickstart, demo environment

**Files:**
- Replace: `api/src/bin/seed.rs`
- Create: `README.md`

- [ ] **Step 1: Replace `api/src/bin/seed.rs`**

```rust
use chrono::{Duration, Utc};
use serde_json::json;
use sqlx::PgPool;
use tj_viagens_api::audit::{append_audit, AuditInput};
use tj_viagens_api::auth::hash_password;
use tj_viagens_api::config::Config;
use tj_viagens_api::db;
use tj_viagens_api::routes::quotations::next_code;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let config = Config::from_env();
    let pool = db::connect(&config.database_url).await;
    println!("⚠ seed: limpando e repopulando {}", config.database_url);
    sqlx::query(
        "TRUNCATE users, suppliers, supplier_documents, quotations, proposals, \
         service_orders, tickets, notifications, audit_events, counters \
         RESTART IDENTITY CASCADE",
    )
    .execute(&pool)
    .await
    .unwrap();

    let hash = hash_password("demo1234");
    for (email, name, role) in [
        ("admin@tjrr.jus.br", "Administrador STI", "ADMIN"),
        ("servidor@tjrr.jus.br", "Servidor SGA", "SERVIDOR"),
    ] {
        sqlx::query(
            "INSERT INTO users (id, email, name, password_hash, role) VALUES ($1,$2,$3,$4,$5)",
        )
        .bind(Uuid::new_v4())
        .bind(email)
        .bind(name)
        .bind(&hash)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    }
    let servidor_id: Uuid =
        sqlx::query_scalar("SELECT id FROM users WHERE email = 'servidor@tjrr.jus.br'")
            .fetch_one(&pool)
            .await
            .unwrap();

    let active = [
        ("11222333000181", "Voa Roraima Turismo LTDA", "contato@voaroraima.com.br"),
        ("11444777000161", "Amazônia Viagens LTDA", "contato@amazoniaviagens.com.br"),
        ("12345678000195", "Rio Branco Turismo LTDA", "contato@riobrancotur.com.br"),
    ];
    let mut supplier_ids = Vec::new();
    for (cnpj, name, email) in active {
        supplier_ids.push(seed_supplier(&pool, &hash, cnpj, name, email, "ACTIVE", 4).await);
    }
    // PENDING supplier missing CNDT — demos the deterministic checklist pre-triage
    seed_supplier(
        &pool,
        &hash,
        "98765432000198",
        "Monte Roraima Travel LTDA",
        "contato@monteroraima.com.br",
        "PENDING",
        3,
    )
    .await;

    seed_completed_quotation(&pool, servidor_id, &supplier_ids).await;

    println!("Seed ok. Senha universal: demo1234");
    println!("  Servidor:     servidor@tjrr.jus.br");
    println!("  Fornecedores: contato@voaroraima.com.br | contato@amazoniaviagens.com.br | contato@riobrancotur.com.br");
    println!("  Pendente:     contato@monteroraima.com.br (falta CNDT — demo do checklist)");
}

async fn seed_supplier(
    pool: &PgPool,
    hash: &str,
    cnpj: &str,
    name: &str,
    email: &str,
    status: &str,
    docs: usize,
) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO suppliers (id, cnpj, legal_name, contact_email, status) \
         VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(id)
    .bind(cnpj)
    .bind(name)
    .bind(email)
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO users (id, email, name, password_hash, role, supplier_id) \
         VALUES ($1,$2,'Titular',$3,'FORNECEDOR',$4)",
    )
    .bind(Uuid::new_v4())
    .bind(email)
    .bind(hash)
    .bind(id)
    .execute(pool)
    .await
    .unwrap();
    for doc_type in ["CONTRATO_SOCIAL", "CND_FEDERAL", "CRF_FGTS", "CNDT"].iter().take(docs) {
        sqlx::query(
            "INSERT INTO supplier_documents (id, supplier_id, doc_type, file_name, file_path, valid_until) \
             VALUES ($1,$2,$3,'documento.pdf','seed/documento.pdf','2027-12-31'::date)",
        )
        .bind(Uuid::new_v4())
        .bind(id)
        .bind(doc_type)
        .execute(pool)
        .await
        .unwrap();
    }
    id
}

/// One COMPLETED quotation ~2h in the past: feeds the KPI cards, the dossier and
/// the audit timeline so the very first demo screen already tells the story.
async fn seed_completed_quotation(pool: &PgPool, servidor_id: Uuid, supplier_ids: &[Uuid]) {
    let q_id = Uuid::new_v4();
    let code = next_code(pool, "COT").await.unwrap();
    let opened = Utc::now() - Duration::hours(2);
    let closed = opened + Duration::hours(1);
    let awarded_at = closed + Duration::minutes(4);
    sqlx::query(
        "INSERT INTO quotations (id, code, status, passenger_name, passenger_cpf, passenger_sex, \
         passenger_birth, origin, destination, departure_at, reference_flight, \
         reference_price_cents, opens_at, closes_at, awarded_at, award_justification, \
         ticket_deadline_at, created_by) \
         VALUES ($1,$2,'COMPLETED','Maria da Silva','12345678909','F','1985-04-12'::date, \
         'BVB','BSB',$3,'LA-4001',185000,$4,$5,$6,'Menor preço entre as propostas válidas.',$7,$8)",
    )
    .bind(q_id)
    .bind(&code)
    .bind(opened + Duration::days(20))
    .bind(opened)
    .bind(closed)
    .bind(awarded_at)
    .bind(awarded_at + Duration::minutes(30))
    .bind(servidor_id)
    .execute(pool)
    .await
    .unwrap();

    let bids: [(Uuid, i64); 3] = [
        (supplier_ids[0], 152300),
        (supplier_ids[1], 149900),
        (supplier_ids[2], 158000),
    ];
    let mut winner_proposal = Uuid::nil();
    for (i, (supplier_id, price)) in bids.iter().enumerate() {
        let pid = Uuid::new_v4();
        if *price == 149900 {
            winner_proposal = pid;
        }
        sqlx::query(
            "INSERT INTO proposals (id, quotation_id, supplier_id, total_price_cents, flight_info, submitted_at) \
             VALUES ($1,$2,$3,$4,'G3-1720 08:15',$5)",
        )
        .bind(pid)
        .bind(q_id)
        .bind(supplier_id)
        .bind(price)
        .bind(opened + Duration::minutes(10 + i as i64 * 7))
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE quotations SET awarded_proposal_id = $1 WHERE id = $2")
        .bind(winner_proposal)
        .bind(q_id)
        .execute(pool)
        .await
        .unwrap();
    let os_number = next_code(pool, "OS").await.unwrap();
    sqlx::query(
        "INSERT INTO service_orders (id, quotation_id, number, issued_at) VALUES ($1,$2,$3,$4)",
    )
    .bind(Uuid::new_v4())
    .bind(q_id)
    .bind(&os_number)
    .bind(awarded_at)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tickets (id, quotation_id, file_name, file_path, passenger_name, flight_info, \
         departure_at, price_cents, divergences, late, uploaded_at, confirmed_at, confirmed_by) \
         VALUES ($1,$2,'eticket-maria.pdf','seed/eticket-maria.pdf','Maria da Silva','G3-1720 08:15', \
         $3,149900,$4,false,$5,$6,$7)",
    )
    .bind(Uuid::new_v4())
    .bind(q_id)
    .bind(opened + Duration::days(20))
    .bind(json!([]))
    .bind(awarded_at + Duration::minutes(12))
    .bind(awarded_at + Duration::minutes(20))
    .bind(servidor_id)
    .execute(pool)
    .await
    .unwrap();
    for supplier_id in supplier_ids {
        sqlx::query(
            "INSERT INTO notifications (id, supplier_id, quotation_id, kind, message) \
             VALUES ($1,$2,$3,'COTACAO_ABERTA',$4)",
        )
        .bind(Uuid::new_v4())
        .bind(supplier_id)
        .bind(q_id)
        .bind(format!("Nova cotação {code}: BVB → BSB."))
        .execute(pool)
        .await
        .unwrap();
    }

    // Audit trail in the real event order — verify_chain() must return ok:true after seeding.
    let events: Vec<(&str, &str, String, serde_json::Value)> = vec![
        ("QUOTATION_CREATED", "Quotation", q_id.to_string(), json!({ "code": code })),
        ("QUOTATION_OPENED", "Quotation", q_id.to_string(), json!({ "code": code, "notified": 3 })),
        ("PROPOSAL_SUBMITTED", "Proposal", bids[0].0.to_string(), json!({ "totalPriceCents": 152300 })),
        ("PROPOSAL_SUBMITTED", "Proposal", bids[1].0.to_string(), json!({ "totalPriceCents": 149900 })),
        ("PROPOSAL_SUBMITTED", "Proposal", bids[2].0.to_string(), json!({ "totalPriceCents": 158000 })),
        ("QUOTATION_CLOSED", "Quotation", q_id.to_string(), json!({})),
        ("QUOTATION_AWARDED", "Quotation", q_id.to_string(), json!({ "proposalId": winner_proposal.to_string(), "totalPriceCents": 149900 })),
        ("SERVICE_ORDER_ISSUED", "ServiceOrder", os_number.clone(), json!({ "number": os_number })),
        ("TICKET_UPLOADED", "Ticket", q_id.to_string(), json!({ "late": false, "divergences": [] })),
        ("TICKET_CONFIRMED", "Ticket", q_id.to_string(), json!({})),
    ];
    for (event_type, entity, entity_id, payload) in events {
        append_audit(
            pool,
            AuditInput {
                actor_id: Some(servidor_id),
                actor_role: Some("SERVIDOR"),
                event_type,
                entity,
                entity_id,
                quotation_id: Some(q_id),
                payload,
            },
        )
        .await
        .unwrap();
    }
}
```

- [ ] **Step 2: Write `README.md`**

```markdown
# TJ-Viagens — Cotações competitivas de passagens aéreas (TJRR)

Protótipo (Etapa 2, Prêmio de Inovação TJRR — Edital 13/2026, Tema 1 / Desafio 1) da equipe
**Entropy Code**. Credenciamento permanente de agências/companhias + disputa cega com preço de
referência sigiloso, janela de 1 hora controlada pelo relógio do servidor, seleção pelo menor
preço, Ordem de Serviço automática, e-ticket em 30 minutos e trilha de auditoria com
encadeamento de hashes.

## Stack

- **API**: Rust · Axum · SQLx · PostgreSQL 16 · SSE · askama (OS e relatório imprimíveis)
- **Web**: Bun · Vite · React · Tailwind 4 · shadcn/ui + Fluid Functionalism · TanStack Query
- Sem APIs comerciais de voo, sem scraping, sem IA obrigatória (pré-triagem e conferência são
  regras determinísticas), conforme restrições do desafio.

## Subir do zero

```bash
docker compose up -d              # postgres :5433 (dev + test)
cd api && cargo run --bin seed    # migra + dados fictícios de demonstração
cargo run --bin api               # http://localhost:3001
# noutro terminal:
cd web && bun install && bun run dev   # http://localhost:5173
```

Testes: `cd api && cargo test -- --test-threads=1` (usa o banco `tjviagens_test`).

## Credenciais de demonstração (senha `demo1234`)

| Perfil | E-mail |
|---|---|
| Servidor SGA | servidor@tjrr.jus.br |
| Admin | admin@tjrr.jus.br |
| Fornecedor (ativo) | contato@voaroraima.com.br |
| Fornecedor (ativo) | contato@amazoniaviagens.com.br |
| Fornecedor (ativo) | contato@riobrancotur.com.br |
| Fornecedor (pendente, falta CNDT) | contato@monteroraima.com.br |

## Roteiro de demonstração (~90s, espelha o Ato III do pitch)

Para demo ao vivo, encurte as janelas em `api/.env`:
`PROPOSAL_WINDOW_MINUTES=2` e `TICKET_WINDOW_MINUTES=5` (reinicie a API).

1. **Servidor** (janela A, `servidor@tjrr.jus.br`): dashboard já mostra economia acumulada da
   cotação semeada. "Nova cotação" → dados do passageiro → criar → **Abrir** (diálogo avisa a
   notificação simultânea).
2. **Fornecedor** (janela B em largura de celular, `contato@voaroraima.com.br`): notificação no
   painel → cotação → cronômetro grande → envia proposta (ex.: `1.523,00`). Repita com
   `contato@amazoniaviagens.com.br` com preço menor (ex.: `1.499,00`).
3. Janela A: contagem de propostas sobe ao vivo (valores lacrados). Ao encerrar: ranking
   menor→maior com a 1ª pré-selecionada → **Declarar vencedora e emitir OS** (1 clique).
4. Janela B (vencedora): banner 🏆 + prazo de 30 min → anexa e-ticket.
5. Janela A: conferência lado a lado sem divergências → **Confirmar** → card de economia,
   selo "trilha de auditoria íntegra", botões OS/Relatório imprimíveis (anexáveis ao SEI).

## Estrutura

- `api/` — crate Rust (rotas em `src/routes/`, domínio puro em `src/domain/`, auditoria em
  `src/audit.rs`, templates imprimíveis em `templates/`)
- `web/` — SPA React (área do servidor e do fornecedor)
- `docs/pitch/` — canvas, roteiro do vídeo (≤5 min) e checklist de gravação da Etapa 2
- `docs/superpowers/plans/` — plano de implementação executável
```

- [ ] **Step 3: Verify + commit**

Run: `cd api && cargo run --bin seed` → prints credentials. Then `cargo run --bin api` and:

```bash
curl -s -X POST localhost:3001/auth/login -H 'content-type: application/json' -d '{"email":"servidor@tjrr.jus.br","password":"demo1234"}'
```

Expected: token JSON. Log in on the web app: dashboard shows KPI cards fed by the seeded quotation; `/audit/verify` (via dossier screen) shows íntegra.

```bash
git add api/src/bin/seed.rs README.md
git commit -m "feat: demo seed with completed quotation, kpis and audit trail; readme with demo script"
```

---

### Task 16: Phase-2 pitch assets — canvas, 5-minute video script, recording checklist

**Files:**
- Create: `docs/pitch/canvas.md`, `docs/pitch/roteiro-video.md`, `docs/pitch/checklist-gravacao.md`

- [ ] **Step 1: Write `docs/pitch/canvas.md`** (o Canvas é o esqueleto arquitetônico do vídeo — blueprint, p. 6)

```markdown
# Canvas do Protótipo — TJ-Viagens (Entropy Code)

## O que é?
Plataforma web funcional (não mockup) de credenciamento permanente e cotação competitiva de
passagens aéreas para o TJRR. O público interage com dois ambientes: o painel do servidor da SGA
(demanda, disputa, adjudicação, conferência, dossiê) e o portal do fornecedor credenciado
(documentos, proposta sigilosa com cronômetro, e-ticket).

## Por quê?
Hoje a aquisição depende de contrato com fornecedor único: tarifas acima do balcão, fiscais
comparando telas e planilhas manualmente, histórico disperso e difícil de auditar (dores do
Edital 13/2026, Desafio 1). O art. 79 da Lei 14.133/2021 autoriza credenciamento para mercados
fluidos — faltava a ferramenta.

## Para quem?
- Servidores da SGA/fiscais (registram demanda, declaram vencedora, conferem bilhete)
- Agências e companhias credenciadas (disputam e emitem)
- Gestão e controle interno (KPIs, dossiê pronto para o SEI, trilha auditável)

## Resultados esperados
- Economia mensurável por cotação (referência sigilosa × preço contratado) e acumulada
- Esforço do servidor reduzido de horas para minutos por cotação
- 100% das disputas com notificação simultânea, janela isonômica de 1h e ranking automático
- E-ticket vinculado em até 30 min (medido) e rastreabilidade integral verificável por hash

## Funcionamento
1. Credenciamento aberto: cadastro com validação de CNPJ + documentos fiscais/trabalhistas com
   checklist determinístico; homologação humana.
2. Servidor registra a demanda (passageiro Nome/CPF/Sexo/Nascimento, trecho, voo e preço de
   referência sigiloso) e abre a disputa: todos os ativos notificados, cronômetro de 1h no
   relógio do servidor.
3. Propostas cegas (valor + voo), substituíveis até o fim; ninguém vê referência nem rivais.
4. Encerrada: tabela menor→maior, 1ª pré-selecionada, declaração em 1 clique, OS emitida.
5. Vencedora anexa e-ticket em 30 min; conferência automática aponta divergências; servidor
   confirma; relatório de economicidade e dossiê imprimível são gerados.

## Características
- Rust + Axum + PostgreSQL, SSE em tempo real, código aberto e conteinerizável (STI)
- Trilha append-only com encadeamento SHA-256 e endpoint de verificação de integridade
- LGPD: preço de referência e PII segregados por papel; CPF mascarado em relatórios
- Regras determinísticas no lugar de IA obrigatória (pré-triagem documental e conferência de
  bilhete) — sem custo por requisição, sem dependência externa
- UI Fluid Functionalism responsiva: fila de ações do servidor, disputa mobile-first
```

- [ ] **Step 2: Write `docs/pitch/roteiro-video.md`** (5 atos do blueprint, com beats de tela)

```markdown
# Roteiro do Vídeo Pitch — TJ-Viagens · máx. 5:00

> Regra de ouro (E2C4): protagonista é o PROTÓTIPO. Nada de currículos, nada de slides narrados
> sem interface. Gravar a demo com `PROPOSAL_WINDOW_MINUTES=2` e dados do seed.

## Ato I — O Gancho (0:00–1:00) · Canvas: Por quê? + Para quem?
NARRAÇÃO: Comece pela dor, não pela tecnologia. "Hoje, cada passagem aérea do TJRR nasce de um
contrato com um único fornecedor. O fiscal da SGA abre buscadores, compara telas, preenche
planilha — e ainda assim paga acima do balcão. E se precisar auditar? E-mails e planilhas
dispersas." Cite art. 79 da Lei 14.133/2021 (credenciamento p/ mercados fluidos).
TELA: foto/ilustração rápida do fluxo atual (planilha + abas) → corta para a logo TJ-Viagens.

## Ato II — A Proposta (1:00–2:00) · Canvas: O que é? · **E2C3 peso 3.0**
NARRAÇÃO: "TJ-Viagens: credenciamento permanente + disputa cega de 1 hora com preço de
referência sigiloso. Na Etapa 1 isso era um conceito; hoje é um sistema funcional: Rust,
PostgreSQL, tempo real, trilha de auditoria com hash encadeado — pronto para o container da
STI." Explicitar a EVOLUÇÃO: conceito → regras do edital implementadas uma a uma.
TELA: diagrama simples dos 3 módulos (credenciamento → disputa → auditoria) e a tabela de
regras do edital com ✔.

## Ato III — O Motor sob o Capô (2:00–3:30) · Canvas: Funcionamento + Características · **E2C1/E2C2 peso 2.5+2.5 — SHOW, DON'T TELL**
DEMO AO VIVO (roteiro do README, ensaiado; janela A servidor, janela B fornecedor em largura de
celular):
- 2:00 Servidor cria demanda (form do passageiro) e ABRE → diálogo "notificará todos os ativos".
- 2:20 Fornecedor no celular: notificação → cronômetro gigante → envia 1.523,00. Segundo
  fornecedor envia 1.499,00. Narrar: "cada um vê só a própria proposta — sigilo e isonomia".
- 2:45 Painel do servidor: contagem sobe ao vivo, valores lacrados. Janela encerra sozinha
  (relógio do servidor).
- 2:55 Ranking menor→maior com Δ vs referência; 1ª pré-selecionada → 1 clique "Declarar
  vencedora e emitir OS".
- 3:10 Fornecedora vencedora: banner 🏆, prazo de 30 min, anexa e-ticket → conferência
  automática sem divergências → servidor confirma.
- 3:25 Mostrar OS imprimível e o selo "trilha de auditoria íntegra" (mencionar SEI/integração).

## Ato IV — O Impacto (3:30–4:30) · Canvas: Resultados esperados
NARRAÇÃO: converter em valor institucional: "Nesta cotação, R$ 351,00 abaixo da referência —
18,97%. O painel acumula economia, participantes por disputa e % de bilhetes no prazo: os
indicadores do próprio edital, medidos pelo sistema, não por planilha." Horas → minutos por
cotação; auditoria deixa de ser reconstrução manual.
TELA: dashboard com KPIs + relatório de economicidade da cotação semeada.

## Ato V — O Fechamento (4:30–5:00) · Síntese · E2C4 peso 2.0
NARRAÇÃO: "Código aberto, PostgreSQL, sem APIs pagas, sem raspagem, LGPD desde a concepção —
pronto para o sandbox da Etapa 3 com a STI. A Entropy Code entrega hoje o que o desafio pediu:
menor preço, celeridade e rastreabilidade total." Encerrar com nome da equipe + TJ-Viagens.
TELA: tela final com logo, stack e QR/URL do repositório.
```

- [ ] **Step 3: Write `docs/pitch/checklist-gravacao.md`**

```markdown
# Checklist de Gravação e Submissão — Etapa 2

## Antes de gravar
- [ ] `api/.env`: `PROPOSAL_WINDOW_MINUTES=2`, `TICKET_WINDOW_MINUTES=5`; reiniciar API
- [ ] `cargo run --bin seed` executado (KPIs e dossiê pré-populados)
- [ ] `cd web && bun run demo:record` rodado — screenshots auditados (Task 17) e clipes webm em
      `demo/out/video/` disponíveis como b-roll do Ato III (narração por cima, ou gravação ao
      vivo espelhando o mesmo roteiro)
- [ ] Janela A: servidor logado no dashboard; Janela B: fornecedor em largura de celular
- [ ] Ensaiar o Ato III completo 2× — a demo tem 90s, sem improviso
- [ ] Microfone testado: **áudio limpo vale mais que imagem 4K** (blueprint)
- [ ] Cronômetro visível na gravação para não passar de 5:00

## Regras críticas (nota zero se violar)
- [ ] Duração ≤ **5:00** — NÃO ultRAPASSAR
- [ ] YouTube como **Público ou Não listado** (link Privado = zero por falha de acesso)
- [ ] Vídeo mostra a interface interativa real (não apenas slides narrados)
- [ ] Sem leitura de currículos — o protótipo é o protagonista

## Submissão (antes de 14h de 26/08/2026)
- [ ] Upload no YouTube concluído e processado
- [ ] Testar o link em aba anônima (sem login) — abre e reproduz?
- [ ] Acessar npi.tjrr.jus.br/si com o e-mail cadastrado na inscrição (daviciencia1@gmail.com)
- [ ] Colar o link, confirmar o envio e guardar o comprovante/print
- [ ] Margem de segurança: submeter até 12h do dia 26/08

## Desempate (se precisar priorizar polimento)
Ordem da banca: E2C3 (evolução do conceito) → E2C1 (usabilidade) → E2C2 (integração) → E2C4
(defesa) → Potencial de Impacto da Etapa 1. Invista o tempo extra nessa ordem.
```

- [ ] **Step 4: Commit**

```bash
git add docs/pitch README.md
git commit -m "docs: phase-2 pitch canvas, 5-minute video script and recording checklist"
```

---

### Task 17: Automated demo recording (Playwright) + visual self-audit

Records the golden path as video clips (b-roll for Ato III) and screenshots every demo beat so the UI gets a vision review BEFORE the pitch is recorded. Runs after Tasks 12–15 (full app + seed).

**Files:**
- Create: `web/e2e/record.ts`
- Modify: `.gitignore` (add `demo/out/`), `web/package.json` (script)

- [ ] **Step 1: Install Playwright (Bun tooling)**

```bash
cd web && bun add -d playwright && bunx playwright install chromium && cd ..
```

Append to root `.gitignore`:

```gitignore
demo/out/
```

Add to `web/package.json` `"scripts"`:

```json
    "demo:record": "bun e2e/record.ts"
```

- [ ] **Step 2: Write `web/e2e/record.ts`**

```ts
/**
 * Records the TJ-Viagens golden path with two browser contexts:
 *  - staff (desktop 1280x800), supplier (iPhone-ish 390x844 mobile emulation)
 * Outputs: ../demo/out/video/*.webm (b-roll for the pitch's Ato III) and
 *          ../demo/out/shots/NN-*.png (visual self-audit).
 * Prereqs: docker compose db up, seeded DB (cargo run --bin seed), API on :3001,
 *          web dev server on :5173. Window length doesn't matter — the script
 *          force-closes the bidding window via SQL for a tight recording.
 * Run: cd web && bun run demo:record
 */
import { execSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { chromium, type BrowserContext, type Page } from 'playwright';

const WEB = process.env.WEB_URL ?? 'http://localhost:5173';
const API = process.env.API_URL ?? 'http://localhost:3001';
const OUT = '../demo/out';

let shot = 0;
async function snap(page: Page, name: string): Promise<void> {
  shot += 1;
  const path = `${OUT}/shots/${String(shot).padStart(2, '0')}-${name}.png`;
  await page.screenshot({ path });
  console.log(`📸 ${path}`);
}

async function login(page: Page, email: string, landing: RegExp): Promise<void> {
  await page.goto(`${WEB}/login`);
  await page.fill('#email', email);
  await page.fill('#password', 'demo1234');
  await page.getByRole('button', { name: 'Entrar' }).click();
  await page.waitForURL(landing);
}

function sql(statement: string): void {
  execSync(
    `docker compose exec -T db psql -U tj -d tjviagens -c "${statement.replace(/"/g, '\\"')}"`,
    { cwd: '..', stdio: 'inherit' },
  );
}

async function main(): Promise<void> {
  mkdirSync(`${OUT}/shots`, { recursive: true });
  writeFileSync(`${OUT}/eticket-demo.pdf`, '%PDF-1.4 demo e-ticket TJ-Viagens\n');

  const browser = await chromium.launch();
  const staffCtx = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    recordVideo: { dir: `${OUT}/video/staff`, size: { width: 1280, height: 800 } },
  });
  const supplierCtx = await browser.newContext({
    viewport: { width: 390, height: 844 },
    isMobile: true,
    hasTouch: true,
    recordVideo: { dir: `${OUT}/video/supplier`, size: { width: 390, height: 844 } },
  });
  const winnerCtx = await browser.newContext({
    viewport: { width: 390, height: 844 },
    isMobile: true,
    hasTouch: true,
    recordVideo: { dir: `${OUT}/video/winner`, size: { width: 390, height: 844 } },
  });

  // ── Staff: dashboard with seeded KPIs ─────────────────────────────
  const staff = await staffCtx.newPage();
  await login(staff, 'servidor@tjrr.jus.br', /\/$/);
  await staff.getByText('Economia acumulada').waitFor();
  await snap(staff, 'staff-dashboard-kpis');

  // ── Staff: new demand ─────────────────────────────────────────────
  await staff.getByRole('link', { name: 'Nova cotação' }).click();
  await staff.fill('#passengerName', 'Maria da Silva');
  await staff.fill('#passengerCpf', '123.456.789-09');
  await staff.fill('#passengerBirth', '1985-04-12');
  await staff.fill('#origin', 'BVB');
  await staff.fill('#destination', 'BSB');
  await staff.fill('#departureAt', '2026-09-15T08:00');
  await staff.fill('#referenceFlight', 'LA-4001');
  await staff.fill('#referencePrice', '1.850,00');
  await snap(staff, 'staff-new-demand-form');
  await staff.getByRole('button', { name: 'Criar rascunho' }).click();
  await staff.waitForURL(/\/cotacoes\//);
  const quotationId = staff.url().split('/').pop() ?? '';
  await snap(staff, 'staff-draft-detail');

  // ── Staff: open the dispute (confirm dialog) ──────────────────────
  await staff.getByRole('button', { name: 'Abrir cotação' }).click();
  await staff.getByRole('button', { name: 'Confirmar abertura' }).click();
  await staff.getByText('propostas recebidas').waitFor();
  await snap(staff, 'staff-open-countdown');

  // ── Supplier 1 (mobile): notification -> blind bid ────────────────
  const supplier = await supplierCtx.newPage();
  await login(supplier, 'contato@voaroraima.com.br', /\/fornecedor$/);
  await snap(supplier, 'supplier-home-mobile');
  await supplier.goto(`${WEB}/fornecedor/cotacoes/${quotationId}`);
  await supplier.getByText('Tempo restante').waitFor();
  await snap(supplier, 'supplier-bid-countdown-mobile');
  await supplier.fill('#price', '1.523,00');
  await supplier.fill('#flight', 'G3-1720 · 15/09 08:15');
  await supplier.getByRole('button', { name: 'Enviar proposta' }).click();
  await supplier.getByText('Proposta registrada').first().waitFor();
  await snap(supplier, 'supplier-bid-submitted');

  // ── Supplier 2 (winner-to-be) bids lower ──────────────────────────
  const winner = await winnerCtx.newPage();
  await login(winner, 'contato@amazoniaviagens.com.br', /\/fornecedor$/);
  await winner.goto(`${WEB}/fornecedor/cotacoes/${quotationId}`);
  await winner.fill('#price', '1.499,00');
  await winner.fill('#flight', 'G3-1720 · 15/09 08:15');
  await winner.getByRole('button', { name: 'Enviar proposta' }).click();
  await winner.getByText('Proposta registrada').first().waitFor();

  // ── Staff: sealed live count ──────────────────────────────────────
  await staff.getByText('2', { exact: true }).first().waitFor();
  await snap(staff, 'staff-live-count-sealed');

  // ── Force-close the window (server clock) -> ranking ──────────────
  sql(`UPDATE quotations SET closes_at = now() - interval '1 second' WHERE id = '${quotationId}'`);
  await staff.reload();
  await staff.getByText('menor para maior').waitFor();
  await snap(staff, 'staff-ranking-preselected');

  // ── One-click award ───────────────────────────────────────────────
  await staff.getByRole('button', { name: 'Declarar vencedora e emitir OS' }).click();
  await staff.getByText('Aguardando e-ticket').waitFor();
  await snap(staff, 'staff-awarded-waiting-ticket');

  // ── Winner: banner + e-ticket upload ──────────────────────────────
  await winner.reload();
  await winner.getByText('Sua proposta venceu').waitFor();
  await snap(winner, 'winner-banner-mobile');
  await winner.fill('#ticketPrice', '1.499,00');
  await winner.setInputFiles('#ticketFile', `${OUT}/eticket-demo.pdf`);
  await winner.getByRole('button', { name: 'Anexar e-ticket' }).click();
  await winner.getByText('E-ticket enviado').first().waitFor();
  await snap(winner, 'winner-ticket-uploaded');

  // ── Staff: conference -> complete -> economy ──────────────────────
  await staff.reload();
  await staff.getByText('Conferência do e-ticket').waitFor();
  await snap(staff, 'staff-ticket-conference');
  await staff.getByRole('button', { name: 'Confirmar e concluir' }).click();
  await staff.getByText('abaixo da referência').waitFor();
  await snap(staff, 'staff-economy-and-audit');

  // ── Printable pages (token via localStorage) ──────────────────────
  const token = await staff.evaluate(() => localStorage.getItem('tj_token'));
  await staff.goto(`${API}/quotations/${quotationId}/service-order?token=${token}`);
  await snap(staff, 'printable-service-order');
  await staff.goto(`${API}/quotations/${quotationId}/report?token=${token}`);
  await snap(staff, 'printable-report');

  // ── Final dashboard ───────────────────────────────────────────────
  await staff.goto(`${WEB}/`);
  await staff.getByText('Economia acumulada').waitFor();
  await snap(staff, 'staff-dashboard-final');

  await staffCtx.close();
  await supplierCtx.close();
  await winnerCtx.close();
  await browser.close();
  console.log(`🎬 vídeos em ${OUT}/video/{staff,supplier,winner} · ${shot} screenshots em ${OUT}/shots`);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
```

- [ ] **Step 3: Record a run**

Prereqs in three terminals: `docker compose up -d --wait`; `cd api && cargo run --bin seed && cargo run --bin api`; `cd web && bun run dev`. Then:

```bash
cd web && bun run demo:record
```

Expected: 14 screenshots + 3 webm videos under `demo/out/`. If Playwright under Bun hits a runtime incompatibility, fall back to `npx tsx e2e/record.ts` (node) — note it in the commit message.

- [ ] **Step 4: Visual self-audit (controller/vision review — gate before the pitch)**

Review EVERY screenshot in `demo/out/shots/` against this checklist; fix UI issues found (small CSS/copy fixes directly; larger ones as follow-up tasks), then re-run Step 3 until clean:

1. No text truncation/overflow; no horizontal scroll on the 390px mobile shots.
2. Countdown legible at a glance; urgent state readable.
3. The bid screen brief (route/dates/reference flight) fits above the fold on mobile.
4. Ranking table: pre-selected row visibly highlighted; Δ column green/red correct.
5. KPI cards aligned, currency formatted pt-BR, no `NaN`/`Invalid Date` anywhere.
6. Toasts visible in the shots that follow an action (not off-screen/covered).
7. Printable OS/report pages: print button present, tables not clipped, accents render.
8. Consistent status vocabulary + badge colors across screens; Fluid Functionalism styling coherent (no unstyled raw-HTML islands).
9. All timestamps in Boa Vista local format (no raw ISO/UTC strings).

- [ ] **Step 5: Commit**

```bash
git add web/e2e/record.ts web/package.json .gitignore
git commit -m "feat(demo): playwright golden-path recorder with screenshots and b-roll videos"
```

---

## Execution recap

- Priority under deadline (26/08 14h): Tasks 0–11 (API) → 15 (seed/demo) → 12–14 (UI) → 17 (recorder + visual audit) → 16 (pitch). Never record the video against an unseeded database.
- Everything runs on `main` of the fresh `hacka-roraima` repo (Task 0 verifies the git root to dodge the `~/gits` parent-repo trap).
- API test invocation is always `cargo test -- --test-threads=1` (shared test DB).
- The plan embeds exact code; executors should treat compiler disagreements as reality winning — fix forward, keep the behavior and tests specified here.







