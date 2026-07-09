#!/usr/bin/env bash
set -euo pipefail

# End-to-end smoke test for the demo-parity backend job and heartbeat flow.
# It intentionally complements the Rust integration tests by exercising the
# compiled CLI server over HTTP with curl.

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
PORT="${ROKO_TEST_PORT:-6678}"
BASE_URL="http://127.0.0.1:${PORT}"
WORKDIR="$(mktemp -d "${TMPDIR:-/tmp}/roko-b10.XXXXXX")"
RESPONSE_FILE="${WORKDIR}/response.json"
SERVER_PID=""

cleanup() {
  if [[ -n "${SERVER_PID}" ]] && kill -0 "${SERVER_PID}" 2>/dev/null; then
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
  rm -rf "${WORKDIR}"
}
trap cleanup EXIT

require() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

json_get() {
  python3 - "${RESPONSE_FILE}" "$1" <<'PY'
import json
import sys
path = sys.argv[2].split(".")
with open(sys.argv[1], encoding="utf-8") as fh:
    data = json.load(fh)
for part in path:
    if isinstance(data, list):
        data = data[int(part)]
    else:
        data = data[part]
print(data)
PY
}

wait_for_server() {
  for _ in $(seq 1 80); do
    if curl -fsS "${BASE_URL}/api/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.25
  done
  echo "server did not become healthy at ${BASE_URL}" >&2
  return 1
}

assert_status() {
  local expected="$1"
  local method="$2"
  local path="$3"
  local body="${4:-}"
  local code
  if [[ -n "${body}" ]]; then
    code="$(curl -sS -o "${RESPONSE_FILE}" -w '%{http_code}' \
      -X "${method}" "${BASE_URL}${path}" \
      -H 'content-type: application/json' \
      -d "${body}")"
  else
    code="$(curl -sS -o "${RESPONSE_FILE}" -w '%{http_code}' \
      -X "${method}" "${BASE_URL}${path}")"
  fi
  if [[ "${code}" != "${expected}" ]]; then
    echo "${method} ${path}: expected ${expected}, got ${code}" >&2
    cat "${RESPONSE_FILE}" >&2 || true
    exit 1
  fi
}

require cargo
require curl
require python3

cd "${ROOT}"
cargo build -p roko-cli >/dev/null

cat >"${WORKDIR}/roko.toml" <<EOF
[server]
bind = "127.0.0.1"
port = ${PORT}
cors_origins = ["http://localhost:5173"]
EOF

cargo run -q -p roko-cli -- --repo "${WORKDIR}" serve --port "${PORT}" >"${WORKDIR}/serve.log" 2>&1 &
SERVER_PID="$!"
wait_for_server

assert_status 201 POST /api/jobs '{"id":"b10-research","title":"Research demo","description":"Summarize the demo backend","job_type":"research","posted_by":"b10"}'
assert_status 200 GET /api/jobs/b10-research
[[ "$(json_get status)" == "open" ]]

assert_status 200 POST /api/jobs/b10-research/assign '{"agent_id":"agent-b10"}'
[[ "$(json_get status)" == "assigned" ]]

assert_status 200 POST /api/jobs/b10-research/start '{}'
[[ "$(json_get status)" == "in_progress" ]]

assert_status 200 POST /api/jobs/b10-research/submit '{"result_summary":"done","artifacts":[{"path":"report.md"}],"gate_results":[{"gate":"compile","passed":true}]}'
[[ "$(json_get status)" == "submitted" ]]

assert_status 200 POST /api/jobs/b10-research/evaluate '{"accepted":true,"feedback":"ok"}'
[[ "$(json_get status)" == "completed" ]]

assert_status 422 POST /api/jobs/b10-research/cancel '{}'
assert_status 400 POST /api/jobs '{"title":"   "}'
assert_status 404 GET /api/jobs/does-not-exist

assert_status 202 POST /api/heartbeats '{"sender_id":"agent-b10","timestamp":"2026-04-22T00:00:00Z","active_tasks":2,"completed_tasks":1,"failed_tasks":0,"active_agents":1,"frequency":7.0,"metrics":{"session_spend_usd":0.01}}'
assert_status 200 GET /api/heartbeats
[[ "$(json_get 0.sender_id)" == "agent-b10" ]]

assert_status 200 GET /api/network/stats
[[ "$(json_get 0.sender_id)" == "agent-b10" ]]

assert_status 200 GET /api/jobs/stats
[[ "$(json_get total)" == "1" ]]

echo "B10 integration smoke passed against ${BASE_URL}"
