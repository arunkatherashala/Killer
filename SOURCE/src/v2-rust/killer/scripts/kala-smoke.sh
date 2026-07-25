#!/usr/bin/env bash
# Kala HTTP smoke test — requires a running server (see ../KALA_SMOKE.md).
set -euo pipefail
PORT="${KALA_PORT:-8080}"
BASE="http://127.0.0.1:${PORT}"

echo "Kala smoke: GET ${BASE}/"
if ! curl -sfS --max-time 10 "${BASE}/" | head -c 4096 | grep -qE 'Kala|<!DOCTYPE'; then
  echo "FAIL: home page missing expected HTML."
  exit 1
fi

echo "Kala smoke: POST ${BASE}/api/kala"
JSON='{"mode":"ask","question":"Reply with exactly: SMOKE_OK","style":"essay","lang":"killer","history":[],"uname":""}'
RESP="$(curl -sfS --max-time 120 -X POST "${BASE}/api/kala" \
  -H 'Content-Type: application/json' \
  -d "${JSON}")"

if ! echo "${RESP}" | grep -q '"response"'; then
  echo "FAIL: response JSON has no response field."
  echo "${RESP}" | head -c 500
  exit 1
fi

echo "Kala smoke: POST /api/kala/clear-session"
curl -sfS --max-time 10 -X POST "${BASE}/api/kala/clear-session" \
  -H 'Content-Type: application/json' \
  -d '{}' | grep -q '"ok"' || { echo "FAIL: clear-session"; exit 1; }

echo "PASS."
