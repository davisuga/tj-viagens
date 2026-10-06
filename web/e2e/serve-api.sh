#!/bin/sh
# Starts the API for the e2e suite on its own port and database, so a test run
# never touches dev data (every test reseeds tjviagens_e2e). Launched by
# playwright.config.ts — Playwright starts web servers before globalSetup, so the
# database has to be created here.
set -eu
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"

docker compose up -d --wait db >/dev/null
docker compose exec -T db sh -c \
  "psql -U tj -d postgres -tAc \"SELECT 1 FROM pg_database WHERE datname = 'tjviagens_e2e'\" | grep -q 1 \
   || createdb -U tj tjviagens_e2e"

cd api
# Both binaries: the tests reseed by running target/debug/seed directly.
"$HOME/.cargo/bin/cargo" +stable build --bins

export DATABASE_URL="postgresql://tj:tj@localhost:5433/tjviagens_e2e"
export PORT=3101
export JWT_SECRET=e2e-secret
export PROPOSAL_WINDOW_MINUTES=60
export TICKET_WINDOW_MINUTES=30
export UPLOAD_DIR="$ROOT/api/target/e2e-uploads"
exec ./target/debug/api
