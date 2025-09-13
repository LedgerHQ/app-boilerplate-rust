#!/usr/bin/env bash
set -euo pipefail
if [[ $# -ne 1 ]]; then
  echo "usage: $0 <32-byte-hex (64 chars)>"
  exit 1
fi
HASH="$1"
if [[ ${#HASH} -ne 64 ]]; then
  echo "error: expected 64 hex chars"; exit 2
fi
APDU="E010000020${HASH}"
curl -s -X POST http://localhost:6001/apdu \
  -H 'Content-Type: application/json' \
  -d "{\"data\":\"$APDU\"}"
echo
