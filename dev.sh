#!/usr/bin/env bash
set -euo pipefail

# 1) Make/replace container and map host ports → container ports
docker rm -f ledger-dev 2>/dev/null || true
docker run --name ledger-dev --rm -d --privileged \
  -v "$(pwd):/app" -w /app \
  --publish 6001:15001 --publish 9998:19999 \
  ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest

# 2) Build + (re)start Speculos inside the container
docker exec ledger-dev bash -lc '
  set -e
  cargo ledger build nanox
  pkill -f speculos || true
  nohup speculos --apdu-port 19999 --api-port 15001 --display headless \
    --model nanox target/nanox/release/app-boilerplate-rust \
    >/tmp/speculos.log 2>&1 &
'

# 3) Health check (expect 6d00)
echo "[health] expecting 6d00:"
curl -s -X POST http://localhost:6001/apdu \
  -H "Content-Type: application/json" \
  -d '{"data":"E001000000"}'
echo

# 4) Echo test (expect 64 'a' + 9000)
DATA="$(printf 'AA%.0s' {1..32})"
APDU="E010000020${DATA}"
echo "[echo test] expecting 64 'a' + 9000:"
curl -s -X POST http://localhost:6001/apdu \
  -H "Content-Type: application/json" \
  -d "{\"data\":\"$APDU\"}"
echo
