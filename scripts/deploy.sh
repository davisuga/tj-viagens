#!/usr/bin/env bash
# Build, ship and redeploy TJ-Viagens on the Dokploy VPS.
#
# Nothing compiles on the VPS: it serves traffic and a Rust build would starve
# it. The binary is cross-compiled here with cargo-zigbuild and the images are
# streamed over the SSH connection, so no container registry is involved.
#
# Requires an SSH connection to the host. The simplest form is a ControlMaster
# opened once by hand:
#   ssh -M -S /tmp/tjv-cm -o ControlPersist=6h -L 3000:127.0.0.1:3000 -N -f root@$VPS
# which also forwards the Dokploy API (port 3000 is deliberately not public).
#
# Env: DOKPLOY_API_KEY (required), VPS (default below), SSH_SOCK (default below).
set -euo pipefail

VPS=${VPS:-74.208.159.201}
SSH_SOCK=${SSH_SOCK:-/tmp/tjv-cm}
DOKPLOY_URL=${DOKPLOY_URL:-http://127.0.0.1:3000}
COMPOSE_ID=${COMPOSE_ID:-2DPIBYWl70kZNLTt3qwEb}
HOST=${HOST:-sh-tjviagens-74-208-159-201.sslip.io}

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
TAG=$(git -C "$ROOT" rev-parse --short HEAD)
SSH="ssh -S $SSH_SOCK root@$VPS"

: "${DOKPLOY_API_KEY:?set DOKPLOY_API_KEY}"
$SSH true || { echo "no SSH master at $SSH_SOCK — open one first (see header)"; exit 1; }

echo "==> cross-compiling api for linux/amd64 (tag $TAG)"
# cargo is aliased to nightly on the author's machine; pin stable explicitly.
(cd "$ROOT/api" && "$HOME/.cargo/bin/cargo" +stable zigbuild \
    --release --target x86_64-unknown-linux-gnu --bins)

echo "==> building web bundle"
# Relative base: the bundle carries no hostname, so it works on any domain.
(cd "$ROOT/web" && VITE_API_URL=/api bun run build)

echo "==> building images"
docker build --platform linux/amd64 -f "$ROOT/docker/Dockerfile.api" -t "tj-viagens-api:$TAG" "$ROOT"
docker build --platform linux/amd64 -f "$ROOT/docker/Dockerfile.web" -t "tj-viagens-web:$TAG" "$ROOT"

echo "==> shipping images to $VPS"
docker save "tj-viagens-api:$TAG" "tj-viagens-web:$TAG" | gzip -1 | $SSH 'gunzip | docker load'

echo "==> pointing the stack at $TAG"
# The Dokploy env store is REPLACE-not-merge: read it, edit the one line, send
# it back whole. Sending only TAG would silently drop the secrets.
ENV=$(curl -fsS "$DOKPLOY_URL/api/compose.one?composeId=$COMPOSE_ID" \
        -H "x-api-key: $DOKPLOY_API_KEY" | jq -r '.env')
ENV=$(printf '%s\n' "$ENV" | sed -E "s/^TAG=.*/TAG=$TAG/")
jq -n --arg id "$COMPOSE_ID" --arg e "$ENV" '{composeId:$id, env:$e}' \
  | curl -fsS -X POST "$DOKPLOY_URL/api/compose.update" \
      -H "x-api-key: $DOKPLOY_API_KEY" -H 'Content-Type: application/json' -d @- >/dev/null

echo "==> deploying"
curl -fsS -X POST "$DOKPLOY_URL/api/compose.deploy" \
  -H "x-api-key: $DOKPLOY_API_KEY" -H 'Content-Type: application/json' \
  -d "{\"composeId\":\"$COMPOSE_ID\"}"
echo

echo "==> waiting for health"
for _ in $(seq 1 40); do
  if curl -fsS --max-time 5 "https://$HOST/api/health" >/dev/null 2>&1; then
    echo "live: https://$HOST"
    exit 0
  fi
  sleep 6
done
echo "timed out waiting for https://$HOST/api/health" >&2
exit 1
