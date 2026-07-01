# B10: End-to-end integration test script

## Context

**Repo:** `/Users/will/dev/nunchi/roko/roko`
**Branch:** `demo-backend`
**Language:** Rust (workspace with ~29 crates)
**Key crate paths:**
- CLI + orchestrator: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/`
- Core types: `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/`
- HTTP server: `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/`
- Agent dispatch: `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/`

**Key files:**
- Orchestrator (20K lines): `crates/roko-cli/src/orchestrate.rs`
- CLI entry: `crates/roko-cli/src/main.rs`
- Server routes: `crates/roko-serve/src/routes/mod.rs`
- Server state: `crates/roko-serve/src/state.rs`
- Server events: `crates/roko-serve/src/events.rs`
- Server WS: `crates/roko-serve/src/routes/ws.rs`

**Architecture:**
- `roko-serve` is an axum HTTP server on port 6677 with ~85 REST routes + WebSocket
- `AppState` uses `tokio::sync::RwLock` -- all lock ops are `.read().await` / `.write().await` (NOT `.unwrap()`)
- Event bus: `state.event_bus.publish(event)` -- always present, no Option wrapping
- The TUI gets data two ways: (1) StateHub push via `watch<DashboardSnapshot>` channel, (2) file polling via `DashboardData::tick()` reading `.roko/` files

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

## What this task does

Create an integration test script that exercises all the endpoints and flows added in B1-B9. The script starts the server, runs curl-based tests against every endpoint, tests error cases (404, 400, invalid transitions), and verifies the complete bounty lifecycle. Cleans up test data after completion.

**Audit update (2026-04-22):** Rust integration coverage exists and passes in `crates/roko-serve/tests/job_lifecycle.rs` and `job_runner_integration.rs`, but the requested `tmp/04-21-26/demo-parity/integration-test.sh` script is still missing.

- [ ] Add the bash `integration-test.sh` harness or explicitly retire that requirement in favor of the Rust integration suites.

## Prerequisites

B1 through B9 must all be complete and compiling.

## Steps

- [ ] **Run the full build and test suite first.**

```bash
cd /Users/will/dev/nunchi/roko/roko

# Verify everything compiles
cargo build --workspace 2>&1 | tail -10

# Verify all tests pass
cargo test --workspace 2>&1 | tail -30

# Verify clippy is clean
cargo clippy --workspace --no-deps -- -D warnings 2>&1 | tail -10
```

- [ ] **Create the integration test script.** Create `/Users/will/dev/nunchi/roko/roko/tmp/04-21-26/demo-parity/integration-test.sh`:

```bash
#!/usr/bin/env bash
# End-to-end integration test for B1-B9 demo backend features.
#
# Prerequisites: cargo build --workspace must succeed.
# Usage: bash tmp/04-21-26/demo-parity/integration-test.sh
#
# Exit codes:
#   0 -- all tests passed
#   1 -- one or more tests failed

set -euo pipefail

REPO="/Users/will/dev/nunchi/roko/roko"
cd "$REPO"

BASE_URL="http://localhost:6677"
PASS=0
FAIL=0
TOTAL=0

# Track IDs for cleanup
declare -a CREATED_JOB_IDS=()

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

green() { printf '\033[32m%s\033[0m\n' "$1"; }
red()   { printf '\033[31m%s\033[0m\n' "$1"; }
bold()  { printf '\033[1m%s\033[0m\n'  "$1"; }
dim()   { printf '\033[2m%s\033[0m\n'  "$1"; }

assert_status() {
    local label="$1" expected="$2" actual="$3"
    TOTAL=$((TOTAL + 1))
    if [ "$actual" = "$expected" ]; then
        green "  PASS: $label (HTTP $actual)"
        PASS=$((PASS + 1))
    else
        red "  FAIL: $label -- expected HTTP $expected, got $actual"
        FAIL=$((FAIL + 1))
    fi
}

assert_json_eq() {
    local label="$1" json="$2" field="$3" expected="$4"
    TOTAL=$((TOTAL + 1))
    local actual
    actual=$(printf '%s' "$json" | jq -r "$field" 2>/dev/null || echo "__PARSE_ERROR__")
    if [ "$actual" = "$expected" ]; then
        green "  PASS: $label ($field = \"$actual\")"
        PASS=$((PASS + 1))
    else
        red "  FAIL: $label -- $field expected \"$expected\", got \"$actual\""
        FAIL=$((FAIL + 1))
    fi
}

assert_json_exists() {
    local label="$1" json="$2" field="$3"
    TOTAL=$((TOTAL + 1))
    local val
    val=$(printf '%s' "$json" | jq -r "$field" 2>/dev/null || echo "null")
    if [ "$val" != "null" ] && [ -n "$val" ]; then
        green "  PASS: $label ($field present)"
        PASS=$((PASS + 1))
    else
        red "  FAIL: $label -- $field missing or null"
        FAIL=$((FAIL + 1))
    fi
}

assert_ge() {
    local label="$1" actual="$2" min="$3"
    TOTAL=$((TOTAL + 1))
    if [ "$actual" -ge "$min" ] 2>/dev/null; then
        green "  PASS: $label ($actual >= $min)"
        PASS=$((PASS + 1))
    else
        red "  FAIL: $label -- expected >= $min, got $actual"
        FAIL=$((FAIL + 1))
    fi
}

wait_for_server() {
    local retries=30
    dim "Waiting for server..."
    while [ $retries -gt 0 ]; do
        if curl -sf "$BASE_URL/api/status" >/dev/null 2>&1; then
            return 0
        fi
        sleep 1
        retries=$((retries - 1))
    done
    red "Server failed to start within 30 seconds"
    exit 1
}

# ---------------------------------------------------------------------------
# Setup
# ---------------------------------------------------------------------------

bold "=== Integration test: B1-B9 demo backend ==="
echo ""

mkdir -p .roko

bold "Building workspace (release)..."
cargo build -p roko-cli --release 2>&1 | tail -3

bold "Starting roko serve..."
cargo run -p roko-cli --release -- serve &
SERVER_PID=$!

cleanup() {
    bold "Cleaning up..."
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
    # Remove test jobs from the filesystem (if persisted).
    for jid in "${CREATED_JOB_IDS[@]}"; do
        rm -f ".roko/jobs/${jid}.json" 2>/dev/null || true
    done
    dim "Done."
}
trap cleanup EXIT

wait_for_server
bold "Server ready on $BASE_URL"
echo ""

# ---------------------------------------------------------------------------
# [1/9] Server health
# ---------------------------------------------------------------------------

bold "[1/9] Server health"
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE_URL/api/status")
assert_status "GET /api/status" "200" "$STATUS"
echo ""

# ---------------------------------------------------------------------------
# [2/9] Job creation
# ---------------------------------------------------------------------------

bold "[2/9] Job creation"

# Empty list
RESP=$(curl -sf "$BASE_URL/api/jobs")
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE_URL/api/jobs")
assert_status "GET /api/jobs (initial)" "200" "$STATUS"
assert_json_eq "initial job count" "$RESP" ".count" "0"

# Create research job
RESP=$(curl -sf -X POST "$BASE_URL/api/jobs" \
    -H 'Content-Type: application/json' \
    -d '{"title":"Research Uniswap v4","description":"Survey hook patterns","job_type":"research","posted_by":"integration-test"}')
JOB_ID=$(printf '%s' "$RESP" | jq -r '.id')
CREATED_JOB_IDS+=("$JOB_ID")
assert_json_exists "research job has id" "$RESP" ".id"
assert_json_eq "research job state" "$RESP" ".state" "open"
assert_json_eq "research job type" "$RESP" ".job_type" "research"
dim "  Created job: $JOB_ID"

# Verify creation returns 201
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' -X POST "$BASE_URL/api/jobs" \
    -H 'Content-Type: application/json' \
    -d '{"title":"Second job","description":"test","job_type":"coding_task"}')
assert_status "POST /api/jobs returns 201" "201" "$STATUS"

# Blank title rejected with 400
STATUS=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE_URL/api/jobs" \
    -H 'Content-Type: application/json' \
    -d '{"title":"   ","description":"d","job_type":"research"}')
assert_status "blank title rejected (400)" "400" "$STATUS"

# List now has jobs
RESP=$(curl -sf "$BASE_URL/api/jobs")
COUNT=$(printf '%s' "$RESP" | jq -r '.count')
assert_ge "job list count >= 2" "$COUNT" "2"

echo ""

# ---------------------------------------------------------------------------
# [3/9] Job GET + stats
# ---------------------------------------------------------------------------

bold "[3/9] Job GET + stats"

RESP=$(curl -sf "$BASE_URL/api/jobs/$JOB_ID")
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE_URL/api/jobs/$JOB_ID")
assert_status "GET /api/jobs/:id" "200" "$STATUS"
assert_json_eq "job title" "$RESP" ".title" "Research Uniswap v4"
assert_json_eq "job state" "$RESP" ".state" "open"

RESP=$(curl -sf "$BASE_URL/api/jobs/stats")
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE_URL/api/jobs/stats")
assert_status "GET /api/jobs/stats" "200" "$STATUS"
assert_json_exists "stats.total" "$RESP" ".total"

echo ""

# ---------------------------------------------------------------------------
# [4/9] Full job lifecycle: open -> assigned -> in_progress -> submitted -> evaluated
# ---------------------------------------------------------------------------

bold "[4/9] Full job lifecycle"

# Assign
RESP=$(curl -sf -X POST "$BASE_URL/api/jobs/$JOB_ID/assign" \
    -H 'Content-Type: application/json' \
    -d '{"agent_id":"test-agent-1"}')
assert_json_eq "state after assign" "$RESP" ".state" "assigned"
assert_json_eq "assigned_to" "$RESP" ".assigned_to" "test-agent-1"

# Start
RESP=$(curl -sf -X POST "$BASE_URL/api/jobs/$JOB_ID/start" \
    -H 'Content-Type: application/json' \
    -d '{}')
assert_json_eq "state after start" "$RESP" ".state" "in_progress"

# Submit
RESP=$(curl -sf -X POST "$BASE_URL/api/jobs/$JOB_ID/submit" \
    -H 'Content-Type: application/json' \
    -d '{"result_summary":"Found 5 hook patterns","artifacts":["report.md"],"gate_results":[{"gate":"format","passed":true}]}')
assert_json_eq "state after submit" "$RESP" ".state" "submitted"
assert_json_exists "submission object" "$RESP" ".submission"

# Evaluate
RESP=$(curl -sf -X POST "$BASE_URL/api/jobs/$JOB_ID/evaluate" \
    -H 'Content-Type: application/json' \
    -d '{"accepted":true,"score":0.9,"feedback":"Good work"}')
assert_json_eq "state after evaluate" "$RESP" ".state" "evaluated"
assert_json_exists "evaluation object" "$RESP" ".evaluation"

echo ""

# ---------------------------------------------------------------------------
# [5/9] Error cases
# ---------------------------------------------------------------------------

bold "[5/9] Error cases"

# 404 for missing job
STATUS=$(curl -s -o /dev/null -w '%{http_code}' "$BASE_URL/api/jobs/nonexistent-job-id-xxx")
assert_status "GET missing job (404)" "404" "$STATUS"

# 404 for assign on missing job
STATUS=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE_URL/api/jobs/missing-xxx/assign" \
    -H 'Content-Type: application/json' -d '{"agent_id":"x"}')
assert_status "assign missing job (404)" "404" "$STATUS"

# Invalid state transition: try to cancel an evaluated job (must fail with 4xx)
STATUS=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE_URL/api/jobs/$JOB_ID/cancel" \
    -H 'Content-Type: application/json' -d '{}')
if [ "$STATUS" = "400" ] || [ "$STATUS" = "409" ]; then
    green "  PASS: cancel evaluated job returns 4xx (got $STATUS)"
    PASS=$((PASS + 1))
else
    red "  FAIL: cancel evaluated job expected 400 or 409, got $STATUS"
    FAIL=$((FAIL + 1))
fi
TOTAL=$((TOTAL + 1))

# Create a fresh job and try invalid transition: start without assigning first
RESP=$(curl -sf -X POST "$BASE_URL/api/jobs" \
    -H 'Content-Type: application/json' \
    -d '{"title":"Skip assign test","description":"test","job_type":"research"}')
SKIP_ID=$(printf '%s' "$RESP" | jq -r '.id')
CREATED_JOB_IDS+=("$SKIP_ID")

STATUS=$(curl -s -o /dev/null -w '%{http_code}' -X POST "$BASE_URL/api/jobs/$SKIP_ID/start" \
    -H 'Content-Type: application/json' -d '{}')
if [ "$STATUS" = "400" ] || [ "$STATUS" = "409" ]; then
    green "  PASS: start without assign returns 4xx (got $STATUS)"
    PASS=$((PASS + 1))
else
    red "  FAIL: start without assign expected 400 or 409, got $STATUS"
    FAIL=$((FAIL + 1))
fi
TOTAL=$((TOTAL + 1))

echo ""

# ---------------------------------------------------------------------------
# [6/9] Job cancellation
# ---------------------------------------------------------------------------

bold "[6/9] Job cancellation"

RESP=$(curl -sf -X POST "$BASE_URL/api/jobs" \
    -H 'Content-Type: application/json' \
    -d '{"title":"Cancel me","description":"test cancel","job_type":"testing"}')
CANCEL_ID=$(printf '%s' "$RESP" | jq -r '.id')
CREATED_JOB_IDS+=("$CANCEL_ID")

RESP=$(curl -sf -X POST "$BASE_URL/api/jobs/$CANCEL_ID/cancel" \
    -H 'Content-Type: application/json' -d '{}')
assert_json_eq "state after cancel" "$RESP" ".state" "cancelled"

echo ""

# ---------------------------------------------------------------------------
# [7/9] Heartbeat endpoints (B7)
# ---------------------------------------------------------------------------

bold "[7/9] Heartbeat endpoints"

STATUS=$(curl -sf -o /dev/null -w '%{http_code}' -X POST "$BASE_URL/api/heartbeats" \
    -H 'Content-Type: application/json' \
    -d '{"sender_id":"integration-test","timestamp":"2026-04-21T12:00:00Z","active_tasks":3,"active_agents":2}')
assert_status "POST /api/heartbeats" "202" "$STATUS"

# Second heartbeat from a different sender
curl -sf -X POST "$BASE_URL/api/heartbeats" \
    -H 'Content-Type: application/json' \
    -d '{"sender_id":"second-sender","timestamp":"2026-04-21T12:01:00Z","active_tasks":1}' >/dev/null

RESP=$(curl -sf "$BASE_URL/api/heartbeats")
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE_URL/api/heartbeats")
assert_status "GET /api/heartbeats" "200" "$STATUS"
HB_COUNT=$(printf '%s' "$RESP" | jq -r '.count')
assert_ge "heartbeat count >= 2" "$HB_COUNT" "2"

RESP=$(curl -sf "$BASE_URL/api/network/stats")
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' "$BASE_URL/api/network/stats")
assert_status "GET /api/network/stats" "200" "$STATUS"
UNIQUE_SENDERS=$(printf '%s' "$RESP" | jq -r '.unique_senders')
assert_ge "unique_senders >= 2" "$UNIQUE_SENDERS" "2"
assert_json_exists "senders array" "$RESP" ".senders"

echo ""

# ---------------------------------------------------------------------------
# [8/9] Job filtering
# ---------------------------------------------------------------------------

bold "[8/9] Job filtering"

RESP=$(curl -sf "$BASE_URL/api/jobs?state=open")
assert_json_exists "state=open returns jobs" "$RESP" ".jobs"

RESP=$(curl -sf "$BASE_URL/api/jobs?state=evaluated")
EVAL_COUNT=$(printf '%s' "$RESP" | jq -r '.count')
assert_ge "evaluated jobs >= 1" "$EVAL_COUNT" "1"

RESP=$(curl -sf "$BASE_URL/api/jobs?state=cancelled")
CANCEL_COUNT=$(printf '%s' "$RESP" | jq -r '.count')
assert_ge "cancelled jobs >= 1" "$CANCEL_COUNT" "1"

echo ""

# ---------------------------------------------------------------------------
# [9/9] Server persistence (B4)
# ---------------------------------------------------------------------------

bold "[9/9] Server persistence"

SNAP_PATH=".roko/state/server-state.json"
if [ -f "$SNAP_PATH" ]; then
    TOTAL=$((TOTAL + 1))
    green "  PASS: snapshot file exists at $SNAP_PATH"
    PASS=$((PASS + 1))
    SAVED_AT=$(jq -r '.saved_at' "$SNAP_PATH" 2>/dev/null || echo "missing")
    if [ "$SAVED_AT" != "null" ] && [ "$SAVED_AT" != "missing" ]; then
        TOTAL=$((TOTAL + 1))
        green "  PASS: snapshot has saved_at: $SAVED_AT"
        PASS=$((PASS + 1))
    else
        TOTAL=$((TOTAL + 1))
        red "  FAIL: snapshot missing saved_at field"
        FAIL=$((FAIL + 1))
    fi
else
    # Not a failure -- auto-save triggers on a timer, may not have fired yet.
    dim "  SKIP: snapshot not yet written (test runs faster than auto-save interval)"
fi

echo ""

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

bold "=== Results ==="
printf "  Total:  %d\n" "$TOTAL"
green "  Passed: $PASS"
if [ "$FAIL" -gt 0 ]; then
    red "  Failed: $FAIL"
    echo ""
    red "INTEGRATION TEST FAILED"
    exit 1
else
    green "  Failed: 0"
    echo ""
    green "ALL INTEGRATION TESTS PASSED"
    exit 0
fi
```

- [ ] **Make the script executable.**
```bash
chmod +x /Users/will/dev/nunchi/roko/roko/tmp/04-21-26/demo-parity/integration-test.sh
```

- [ ] **Create a Rust integration test.** Create `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/tests/job_lifecycle.rs`:

```rust
//! In-process integration test for the job lifecycle API.
//!
//! Starts an axum test router, exercises the full job state machine
//! (open -> assigned -> in_progress -> submitted -> evaluated), and
//! verifies error cases (404, invalid transitions).

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use roko_core::config::schema::RokoConfig;
use roko_core::config::ServeAuthConfig;
use serde_json::{json, Value};
use tempfile::tempdir;
use tower::ServiceExt;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn test_app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempdir().expect("tempdir");
    let workdir = dir.path().to_path_buf();
    // NOTE: Adjust these calls to match the actual pub API of roko-serve.
    // Check: grep -n "pub mod\|pub fn\|pub use" crates/roko-serve/src/lib.rs
    let state = Arc::new(
        roko_serve::state::AppState::new(workdir, RokoConfig::default())
            .expect("AppState::new"),
    );
    let router = roko_serve::routes::build_router(state, &[], ServeAuthConfig::default());
    (dir, router)
}

async fn post_json(app: &axum::Router, path: &str, body: Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    (status, json)
}

async fn get_json(app: &axum::Router, path: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(path)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(json!({}));
    (status, json)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn full_job_lifecycle() {
    let (_dir, app) = test_app();

    // Create
    let (status, resp) = post_json(
        &app,
        "/api/jobs",
        json!({
            "title": "Test research job",
            "description": "Integration test",
            "job_type": "research",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create job");
    let job_id = resp["id"].as_str().expect("id").to_string();
    assert_eq!(resp["state"], "open");

    // List
    let (status, resp) = get_json(&app, "/api/jobs").await;
    assert_eq!(status, StatusCode::OK);
    assert!(resp["count"].as_u64().unwrap_or(0) >= 1, "at least one job");

    // Get
    let (status, resp) = get_json(&app, &format!("/api/jobs/{job_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(resp["state"], "open");

    // Stats
    let (status, resp) = get_json(&app, "/api/jobs/stats").await;
    assert_eq!(status, StatusCode::OK);
    assert!(resp["total"].as_u64().unwrap_or(0) >= 1);

    // Assign
    let (status, resp) = post_json(
        &app,
        &format!("/api/jobs/{job_id}/assign"),
        json!({ "agent_id": "test-agent" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "assign");
    assert_eq!(resp["state"], "assigned");
    assert_eq!(resp["assigned_to"], "test-agent");

    // Start
    let (status, resp) = post_json(
        &app,
        &format!("/api/jobs/{job_id}/start"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "start");
    assert_eq!(resp["state"], "in_progress");

    // Submit
    let (status, resp) = post_json(
        &app,
        &format!("/api/jobs/{job_id}/submit"),
        json!({
            "result_summary": "Completed research",
            "gate_results": [{"gate": "quality", "passed": true}],
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "submit");
    assert_eq!(resp["state"], "submitted");
    assert!(resp.get("submission").is_some(), "submission object present");

    // Evaluate
    let (status, resp) = post_json(
        &app,
        &format!("/api/jobs/{job_id}/evaluate"),
        json!({ "accepted": true, "score": 0.95 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "evaluate");
    assert_eq!(resp["state"], "evaluated");
    assert!(resp.get("evaluation").is_some(), "evaluation object present");
}

#[tokio::test]
async fn invalid_transition_returns_4xx() {
    let (_dir, app) = test_app();

    let (_, resp) = post_json(
        &app,
        "/api/jobs",
        json!({
            "title": "Transition test",
            "description": "d",
            "job_type": "research",
        }),
    )
    .await;
    let job_id = resp["id"].as_str().expect("id").to_string();

    // Try to start without assigning first -- must be rejected.
    let (status, _) = post_json(
        &app,
        &format!("/api/jobs/{job_id}/start"),
        json!({}),
    )
    .await;
    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::CONFLICT,
        "start without assign must return 400 or 409, got {status}",
    );
}

#[tokio::test]
async fn cancel_from_open_succeeds() {
    let (_dir, app) = test_app();

    let (_, resp) = post_json(
        &app,
        "/api/jobs",
        json!({
            "title": "Cancel test",
            "description": "d",
            "job_type": "research",
        }),
    )
    .await;
    let job_id = resp["id"].as_str().expect("id").to_string();

    let (status, resp) = post_json(
        &app,
        &format!("/api/jobs/{job_id}/cancel"),
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "cancel from open");
    assert_eq!(resp["state"], "cancelled");
}

#[tokio::test]
async fn missing_job_returns_404() {
    let (_dir, app) = test_app();
    let (status, _) = get_json(&app, "/api/jobs/nonexistent-job-id-zzz").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn blank_title_rejected() {
    let (_dir, app) = test_app();
    let (status, _) = post_json(
        &app,
        "/api/jobs",
        json!({
            "title": "   ",
            "description": "d",
            "job_type": "research",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "blank title must be 400");
}
```

  NOTE: The test file uses `roko_serve::` paths. Check the actual pub exports:
  ```
  grep -n "pub mod\|pub use\|pub fn" crates/roko-serve/src/lib.rs | head -20
  ```
  Adjust imports to match actual module visibility. If `state`, `routes`, or `config` are not pub, make them pub or restructure the test accordingly.

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Run the bash integration test
bash tmp/04-21-26/demo-parity/integration-test.sh

# Run the Rust integration test
cargo test -p roko-serve --test job_lifecycle -- --nocapture

# Full workspace verification
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

Expected: bash script reports all tests passed (with cleanup), Rust integration test passes all five scenarios.
