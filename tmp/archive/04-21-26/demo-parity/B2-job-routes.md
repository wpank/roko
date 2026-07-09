# B2: Job API routes in roko-serve

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
- `AppState` uses `tokio::sync::RwLock` — all lock ops are `.read().await` / `.write().await` (NOT `.unwrap()`)
- Event bus: `state.event_bus.publish(event)` — always present, no Option wrapping

### Pre-commit (MANDATORY)

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

---

## What this task does

Wire the job type system (from B1) into the HTTP server as REST endpoints. Add `FileJobStore` to `AppState`, create route handlers for the full job lifecycle with correct HTTP status codes and JSON error responses, emit `ServerEvent` variants on every state change, and add `http://localhost:5173` to CORS origins.

---

## Prerequisite

B1 must be complete. `roko_core::jobs` must exist and compile before starting this task.

---

## Existing patterns to follow

Read `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` first. Key conventions:

- Path params use `{id}` syntax: `.route("/plans/{id}", get(get_plan))`
- State extractor: `State(state): State<Arc<AppState>>`
- Path extractor: `Path(id): Path<String>`
- Error type: `ApiError` from `crate::error` — it must provide `ApiError::bad_request`, `ApiError::not_found`, `ApiError::internal`
- Success responses: `Json(json!({...}))` for 200, `(StatusCode::CREATED, Json(...))` for 201
- State machine violations → 409 Conflict, not 400

---

## Steps

### Step 1 — CORS fix

- [ ] Ensure `http://localhost:5173` is allowed as a CORS origin.

  Search for where CORS origins are assembled:
  ```bash
  grep -rn "cors_origins\|CorsLayer\|allow_origin" crates/roko-serve/src/ --include='*.rs'
  ```

  If the result shows `CorsLayer::permissive()` with no origin list, CORS is already fully open and **no change is needed**.

  If a whitelist is found (e.g., an array or `Vec<String>`), add `"http://localhost:5173".to_string()` to that array.

  This step must be done before the dashboard can call the job endpoints during development.

### Step 2 — ServerEvent variants

- [ ] Open `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/events.rs`.

  Before the `ServerShutdown` variant (near the end of the `ServerEvent` enum), add:

  ```rust
      /// A new job was created.
      JobCreated {
          job_id: String,
          job_type: String,
          title: String,
      },

      /// A job changed state.
      JobStateChanged {
          job_id: String,
          old_state: String,
          new_state: String,
      },

      /// A job submission was received.
      JobSubmitted {
          job_id: String,
          agent_id: String,
      },

      /// A job evaluation completed.
      JobEvaluated {
          job_id: String,
          accepted: bool,
      },
  ```

### Step 3 — Add `job_store` to AppState

- [ ] Open `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs`.

  Add import near the top (after the existing `roko_core` imports):
  ```rust
  use roko_core::jobs::FileJobStore;
  ```

  Add field to `AppState`, after `aggregator_cache`:
  ```rust
      /// File-backed job store at `.roko/jobs/`.
      pub job_store: std::sync::Arc<FileJobStore>,
  ```

  In `AppState::new_with_daimon_strategy()`, after `aggregator_cache: RwLock::new(HashMap::new()),`, add:
  ```rust
              job_store: std::sync::Arc::new(
                  FileJobStore::for_workdir(&workdir)
                      .unwrap_or_else(|e| panic!("cannot create job store at .roko/jobs/: {e}")),
              ),
  ```

  Note: `unwrap_or_else` + `panic!` is acceptable here because a missing or unwritable `.roko/` directory is a non-recoverable configuration error that should fail loudly at startup.

### Step 4 — Create the jobs route module

- [ ] Create `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/jobs.rs` with the full contents below.

### Step 5 — Wire into the router

- [ ] Open `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs`.

  Add `mod jobs;` after `mod integrations;`.

  In `build_router()`, add `.merge(jobs::routes())` after `.merge(integrations::routes())`.

---

## Full contents of `routes/jobs.rs`

```rust
//! Job marketplace API routes.
//!
//! Provides CRUD and lifecycle endpoints for the job system defined in
//! [`roko_core::jobs`]. Every state change emits a [`ServerEvent`] to the
//! event bus so WebSocket clients see updates in real time.
//!
//! ## HTTP status codes
//!
//! | Situation | Status |
//! |-----------|--------|
//! | Job created | 201 Created |
//! | Job found | 200 OK |
//! | Job not found | 404 Not Found |
//! | Invalid state transition | 409 Conflict |
//! | Missing / blank required field | 400 Bad Request |
//! | Internal I/O or serialization error | 500 Internal Server Error |

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use roko_core::jobs::{
    CreateJobRequest, JobError, JobEvaluation, JobFilter, JobGateResult, JobState, JobSubmission,
};

use crate::error::ApiError;
use crate::events::ServerEvent;
use crate::state::AppState;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/jobs",             get(list_jobs).post(create_job))
        .route("/jobs/stats",       get(job_stats))
        .route("/jobs/{id}",        get(get_job))
        .route("/jobs/{id}/assign", post(assign_job))
        .route("/jobs/{id}/start",  post(start_job))
        .route("/jobs/{id}/submit", post(submit_job))
        .route("/jobs/{id}/evaluate", post(evaluate_job))
        .route("/jobs/{id}/cancel", post(cancel_job))
}

// ---------------------------------------------------------------------------
// Query params
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct JobListQuery {
    #[serde(default)]
    state: Option<JobState>,
    #[serde(default)]
    assigned_to: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /api/jobs` — list jobs with optional filters.
///
/// Query parameters:
/// - `state` — filter by job state (`open`, `assigned`, `in_progress`, etc.)
/// - `assigned_to` — filter by assignee agent ID
async fn list_jobs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<JobListQuery>,
) -> Result<Json<Value>, ApiError> {
    let filter = JobFilter {
        state: query.state,
        assigned_to: query.assigned_to,
        ..Default::default()
    };
    let jobs = state
        .job_store
        .list(&filter)
        .map_err(|e| ApiError::internal(format!("list jobs: {e}")))?;

    let items: Vec<Value> = jobs.iter().map(job_summary).collect();
    Ok(Json(json!({ "jobs": items, "count": items.len() })))
}

/// `POST /api/jobs` — create a new job.
///
/// Returns **201 Created** with the full job object on success.
/// Returns **400 Bad Request** if title or description is blank.
async fn create_job(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateJobRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if body.title.trim().is_empty() {
        return Err(ApiError::bad_request("title must not be blank"));
    }
    if body.description.trim().is_empty() {
        return Err(ApiError::bad_request("description must not be blank"));
    }

    let job = state
        .job_store
        .create(body)
        .map_err(|e| ApiError::internal(format!("create job: {e}")))?;

    state.event_bus.publish(ServerEvent::JobCreated {
        job_id: job.id.clone(),
        job_type: job.job_type.to_string(),
        title: job.title.clone(),
    });

    Ok((StatusCode::CREATED, Json(job_to_json(&job))))
}

/// `GET /api/jobs/stats` — aggregate job statistics.
async fn job_stats(State(state): State<Arc<AppState>>) -> Result<Json<Value>, ApiError> {
    let stats = state
        .job_store
        .stats()
        .map_err(|e| ApiError::internal(format!("job stats: {e}")))?;
    Ok(Json(
        serde_json::to_value(stats).unwrap_or_else(|_| json!({"error": "serialization failed"})),
    ))
}

/// `GET /api/jobs/{id}` — fetch a single job by ID.
///
/// Returns **404 Not Found** if the job does not exist.
async fn get_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let job = load_job(&state, &id)?;
    Ok(Json(job_to_json(&job)))
}

/// `POST /api/jobs/{id}/assign` — assign a job to an agent.
///
/// Body: `{ "agent_id": "<id>" }`
///
/// Returns **409 Conflict** if the job is not in the `Open` state.
async fn assign_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<AssignRequest>,
) -> Result<Json<Value>, ApiError> {
    if body.agent_id.trim().is_empty() {
        return Err(ApiError::bad_request("agent_id must not be blank"));
    }

    let mut job = load_job(&state, &id)?;
    let old_state = job.state.to_string();

    job.transition(JobState::Assigned)
        .map_err(job_state_error)?;
    job.assigned_to = Some(body.agent_id.clone());

    state
        .job_store
        .update(&job)
        .map_err(|e| ApiError::internal(format!("update job: {e}")))?;

    state.event_bus.publish(ServerEvent::JobStateChanged {
        job_id: job.id.clone(),
        old_state,
        new_state: job.state.to_string(),
    });

    Ok(Json(job_to_json(&job)))
}

/// `POST /api/jobs/{id}/start` — transition a job to in-progress.
///
/// Returns **409 Conflict** if the job is not in the `Assigned` state.
async fn start_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let mut job = load_job(&state, &id)?;
    let old_state = job.state.to_string();

    job.transition(JobState::InProgress)
        .map_err(job_state_error)?;

    state
        .job_store
        .update(&job)
        .map_err(|e| ApiError::internal(format!("update job: {e}")))?;

    state.event_bus.publish(ServerEvent::JobStateChanged {
        job_id: job.id.clone(),
        old_state,
        new_state: job.state.to_string(),
    });

    Ok(Json(job_to_json(&job)))
}

/// `POST /api/jobs/{id}/submit` — submit results for a job.
///
/// Returns **409 Conflict** if the job is not in the `InProgress` state.
async fn submit_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<SubmitRequest>,
) -> Result<Json<Value>, ApiError> {
    if body.result_summary.trim().is_empty() {
        return Err(ApiError::bad_request("result_summary must not be blank"));
    }

    let mut job = load_job(&state, &id)?;
    let old_state = job.state.to_string();

    job.transition(JobState::Submitted)
        .map_err(job_state_error)?;

    let agent_id = body
        .agent_id
        .filter(|s| !s.is_empty())
        .or_else(|| job.assigned_to.clone())
        .unwrap_or_else(|| "unknown".to_owned());

    job.submission = Some(JobSubmission {
        agent_id: agent_id.clone(),
        result_summary: body.result_summary,
        artifacts: body.artifacts,
        gate_results: body
            .gate_results
            .into_iter()
            .map(|g| JobGateResult {
                gate: g.gate,
                passed: g.passed,
                detail: g.detail.unwrap_or_default(),
            })
            .collect(),
        submitted_at: chrono::Utc::now(),
    });

    state
        .job_store
        .update(&job)
        .map_err(|e| ApiError::internal(format!("update job: {e}")))?;

    state.event_bus.publish(ServerEvent::JobStateChanged {
        job_id: job.id.clone(),
        old_state,
        new_state: job.state.to_string(),
    });
    state.event_bus.publish(ServerEvent::JobSubmitted {
        job_id: job.id.clone(),
        agent_id,
    });

    Ok(Json(job_to_json(&job)))
}

/// `POST /api/jobs/{id}/evaluate` — evaluate a job submission.
///
/// Returns **409 Conflict** if the job is not in the `Submitted` state.
async fn evaluate_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<EvaluateRequest>,
) -> Result<Json<Value>, ApiError> {
    let mut job = load_job(&state, &id)?;
    let old_state = job.state.to_string();

    job.transition(JobState::Evaluated)
        .map_err(job_state_error)?;

    job.evaluation = Some(JobEvaluation {
        evaluator: body.evaluator.unwrap_or_else(|| "system".to_owned()),
        accepted: body.accepted,
        score: body.score,
        feedback: body.feedback.unwrap_or_default(),
        evaluated_at: chrono::Utc::now(),
    });

    state
        .job_store
        .update(&job)
        .map_err(|e| ApiError::internal(format!("update job: {e}")))?;

    state.event_bus.publish(ServerEvent::JobStateChanged {
        job_id: job.id.clone(),
        old_state,
        new_state: job.state.to_string(),
    });
    state.event_bus.publish(ServerEvent::JobEvaluated {
        job_id: job.id.clone(),
        accepted: body.accepted,
    });

    Ok(Json(job_to_json(&job)))
}

/// `POST /api/jobs/{id}/cancel` — cancel a job.
///
/// Returns **409 Conflict** if the job is already in a terminal state
/// (`Evaluated` or `Cancelled`), or in the `Submitted` state (which cannot
/// be cancelled per the state machine).
async fn cancel_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let mut job = load_job(&state, &id)?;
    let old_state = job.state.to_string();

    job.transition(JobState::Cancelled)
        .map_err(job_state_error)?;

    state
        .job_store
        .update(&job)
        .map_err(|e| ApiError::internal(format!("update job: {e}")))?;

    state.event_bus.publish(ServerEvent::JobStateChanged {
        job_id: job.id.clone(),
        old_state,
        new_state: job.state.to_string(),
    });

    Ok(Json(job_to_json(&job)))
}

// ---------------------------------------------------------------------------
// Request bodies
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct AssignRequest {
    agent_id: String,
}

#[derive(Deserialize)]
struct SubmitRequest {
    #[serde(default)]
    agent_id: Option<String>,
    result_summary: String,
    #[serde(default)]
    artifacts: Vec<String>,
    #[serde(default)]
    gate_results: Vec<GateResultInput>,
}

#[derive(Deserialize)]
struct GateResultInput {
    gate: String,
    passed: bool,
    #[serde(default)]
    detail: Option<String>,
}

#[derive(Deserialize)]
struct EvaluateRequest {
    accepted: bool,
    #[serde(default)]
    evaluator: Option<String>,
    #[serde(default)]
    score: Option<f64>,
    #[serde(default)]
    feedback: Option<String>,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Load a job by ID, mapping `NotFound` to 404 and other errors to 500.
fn load_job(state: &AppState, id: &str) -> Result<roko_core::jobs::Job, ApiError> {
    state.job_store.get(id).map_err(|e| match e {
        JobError::NotFound(_) => ApiError::not_found(format!("job '{id}' not found")),
        other => ApiError::internal(format!("load job: {other}")),
    })
}

/// Map a [`JobError::InvalidTransition`] to 409 Conflict.
/// Other errors become 500 Internal Server Error.
fn job_state_error(e: JobError) -> ApiError {
    match e {
        JobError::InvalidTransition { .. } => {
            ApiError::with_status(StatusCode::CONFLICT, e.to_string())
        }
        other => ApiError::internal(other.to_string()),
    }
}

/// Serialize a full [`Job`] to JSON, with a safe fallback.
fn job_to_json(job: &roko_core::jobs::Job) -> Value {
    serde_json::to_value(job)
        .unwrap_or_else(|_| json!({ "error": "serialization failed", "id": job.id }))
}

/// Serialize a [`Job`] to a compact summary (for list responses).
fn job_summary(job: &roko_core::jobs::Job) -> Value {
    json!({
        "id":          job.id,
        "title":       job.title,
        "job_type":    job.job_type.to_string(),
        "state":       job.state.to_string(),
        "assigned_to": job.assigned_to,
        "created_at":  job.created_at.to_rfc3339(),
        "updated_at":  job.updated_at.to_rfc3339(),
    })
}
```

---

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Compile the server
cargo check -p roko-serve 2>&1 | head -30

# Run server tests
cargo test -p roko-serve 2>&1 | tail -20

# Clippy
cargo clippy -p roko-serve --no-deps -- -D warnings 2>&1 | head -20

# Format
cargo +nightly fmt --all -- --check
```

Manual smoke test (run in a separate terminal):

```bash
# Start the server
cargo run -p roko-cli -- serve &

# List jobs (empty initially)
curl -s http://localhost:6677/api/jobs | python3 -m json.tool

# Create a job
curl -s -X POST http://localhost:6677/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{"title":"Test research","description":"Survey DeFi protocols","job_type":"research"}' \
  | python3 -m json.tool

# Capture the ID from the response and use it in subsequent calls:
# curl -s -X POST http://localhost:6677/api/jobs/job-<uuid>/assign \
#   -H 'Content-Type: application/json' \
#   -d '{"agent_id":"test-agent"}' | python3 -m json.tool

# Stats
curl -s http://localhost:6677/api/jobs/stats | python3 -m json.tool
```

Expected: routes respond with JSON, correct HTTP status codes, and the job store file appears at `.roko/jobs/`.
