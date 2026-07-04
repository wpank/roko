# Task 5: End-to-end integration test for matchmaking flow

## Objective

Write an integration test that exercises the full dashboard flow against a real axum router:

1. Register agents with enriched fields (tier, reputation, skills)
2. Call `POST /api/jobs/match` and verify ranking
3. Create a job with `committed_candidates` from the match result
4. Assign the job to the top candidate
5. Walk the job through start → submit → evaluate(accept)
6. Verify the final state is `completed`

This test validates that all four prior tasks work together correctly.

## Dependencies

Tasks 1–4 must be complete.

## Files to modify

| File | What to change |
|---|---|
| `crates/roko-serve/src/routes/jobs.rs` | Add integration test to the `#[cfg(test)] mod tests` block |

Alternatively, if `jobs.rs` tests are getting long, create a new test file at
`crates/roko-serve/tests/matchmaking_flow.rs`. However, this requires that the internal
handler functions are accessible. Since they're module-private, the simpler approach is to
test via the axum `Router` using `tower::ServiceExt::oneshot`.

**Recommended:** Add the test in `crates/roko-serve/src/routes/jobs.rs` within the existing
(or newly created from Task 2) `#[cfg(test)] mod tests` block, since it needs access to
private types.

## Test implementation

```rust
#[tokio::test]
async fn matchmaking_full_flow() {
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    let tempdir = tempdir().expect("tempdir");
    let state = Arc::new(AppState::new(
        tempdir.path().to_path_buf(),
        Arc::new(crate::runtime::NoOpRuntime),
        roko_core::config::schema::RokoConfig::default(),
        Arc::new(crate::deploy::manual::ManualBackend::default()),
    ));

    // --- Step 1: Register agents with enriched fields ---

    state.upsert_discovered_agent(crate::state::AgentRegistrationRecord {
        agent_id: "agent-alpha".into(),
        label: Some("alpha".into()),
        tier: Some("Expert".into()),
        reputation: 95,
        skills: vec!["rust".into(), "networking".into()],
        past_jobs_completed: 42,
        max_concurrent_jobs: 5,
        ..Default::default()
    }).await;

    state.upsert_discovered_agent(crate::state::AgentRegistrationRecord {
        agent_id: "agent-beta".into(),
        label: Some("beta".into()),
        tier: Some("Verified".into()),
        reputation: 70,
        skills: vec!["rust".into(), "testing".into()],
        past_jobs_completed: 15,
        max_concurrent_jobs: 3,
        ..Default::default()
    }).await;

    state.upsert_discovered_agent(crate::state::AgentRegistrationRecord {
        agent_id: "agent-gamma".into(),
        label: Some("gamma".into()),
        tier: Some("Verified".into()),
        reputation: 80,
        skills: vec!["javascript".into(), "react".into()],
        past_jobs_completed: 25,
        max_concurrent_jobs: 4,
        ..Default::default()
    }).await;

    // Build the router — must include both agent and job routes
    let app = axum::Router::new()
        .nest("/api", super::routes())
        .with_state(Arc::clone(&state));

    // --- Step 2: Match agents ---

    let match_req = Request::builder()
        .method("POST")
        .uri("/api/jobs/match")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&serde_json::json!({
            "title": "build relay module",
            "description": "Implement a p2p relay in Rust",
            "language": "Rust",
            "minTier": "Verified",
            "reward": "1000 KORAI",
            "skills": ["rust"]
        })).unwrap()))
        .unwrap();

    let match_resp = app.clone().oneshot(match_req).await.unwrap();
    assert_eq!(match_resp.status(), StatusCode::OK);

    let match_body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(match_resp.into_body(), usize::MAX).await.unwrap()
    ).unwrap();

    let candidates = match_body["candidates"].as_array().unwrap();
    // agent-alpha and agent-beta have "rust" skill; agent-gamma does not
    assert_eq!(candidates.len(), 2);
    // agent-alpha should rank first (higher reputation)
    assert_eq!(candidates[0]["agentId"], "agent-alpha");
    assert_eq!(candidates[1]["agentId"], "agent-beta");
    assert_eq!(match_body["totalFee"], "1000 KORAI");
    assert!(match_body["etaHours"].as_u64().unwrap() > 0);

    // --- Step 3: Create job with committed candidates ---

    let candidate_ids: Vec<String> = candidates.iter()
        .map(|c| c["agentId"].as_str().unwrap().to_string())
        .collect();

    let create_req = Request::builder()
        .method("POST")
        .uri("/api/jobs")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&serde_json::json!({
            "title": "build relay module",
            "reward": "1000 KORAI",
            "committed_candidates": candidate_ids,
        })).unwrap()))
        .unwrap();

    let create_resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);

    let create_body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(create_resp.into_body(), usize::MAX).await.unwrap()
    ).unwrap();
    let job_id = create_body["id"].as_str().unwrap().to_string();
    assert_eq!(create_body["status"], "open");
    assert_eq!(
        create_body["committed_candidates"].as_array().unwrap().len(),
        2
    );

    // --- Step 4: Assign job to top candidate ---

    let assign_req = Request::builder()
        .method("POST")
        .uri(&format!("/api/jobs/{job_id}/assign"))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"agent_id":"agent-alpha"}"#))
        .unwrap();

    let assign_resp = app.clone().oneshot(assign_req).await.unwrap();
    assert_eq!(assign_resp.status(), StatusCode::OK);
    let assign_body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(assign_resp.into_body(), usize::MAX).await.unwrap()
    ).unwrap();
    assert_eq!(assign_body["status"], "assigned");
    assert_eq!(assign_body["assigned_to"], "agent-alpha");

    // --- Step 5: Start → Submit → Evaluate ---

    // Start
    let start_req = Request::builder()
        .method("POST")
        .uri(&format!("/api/jobs/{job_id}/start"))
        .body(Body::empty())
        .unwrap();
    let start_resp = app.clone().oneshot(start_req).await.unwrap();
    assert_eq!(start_resp.status(), StatusCode::OK);

    // Submit
    let submit_req = Request::builder()
        .method("POST")
        .uri(&format!("/api/jobs/{job_id}/submit"))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"result_summary":"relay module implemented","artifacts":[],"gate_results":[]}"#))
        .unwrap();
    let submit_resp = app.clone().oneshot(submit_req).await.unwrap();
    assert_eq!(submit_resp.status(), StatusCode::OK);
    let submit_body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(submit_resp.into_body(), usize::MAX).await.unwrap()
    ).unwrap();
    assert_eq!(submit_body["status"], "submitted");

    // Evaluate (accept)
    let eval_req = Request::builder()
        .method("POST")
        .uri(&format!("/api/jobs/{job_id}/evaluate"))
        .header("content-type", "application/json")
        .body(Body::from(r#"{"accepted":true,"feedback":"LGTM"}"#))
        .unwrap();
    let eval_resp = app.clone().oneshot(eval_req).await.unwrap();
    assert_eq!(eval_resp.status(), StatusCode::OK);
    let eval_body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(eval_resp.into_body(), usize::MAX).await.unwrap()
    ).unwrap();
    assert_eq!(eval_body["status"], "completed");

    // --- Step 6: Verify final state ---

    let get_req = Request::builder()
        .method("GET")
        .uri(&format!("/api/jobs/{job_id}"))
        .body(Body::empty())
        .unwrap();
    let get_resp = app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    let final_body: serde_json::Value = serde_json::from_slice(
        &axum::body::to_bytes(get_resp.into_body(), usize::MAX).await.unwrap()
    ).unwrap();
    assert_eq!(final_body["status"], "completed");
    assert_eq!(final_body["assigned_to"], "agent-alpha");
    assert!(final_body["submission"].is_object());
    assert!(final_body["evaluation"].is_object());

    // Verify events were emitted
    let events = state.event_bus.replay_from(0);
    assert!(events.iter().any(|e| matches!(
        &e.payload,
        crate::events::ServerEvent::JobCreated { .. }
    )));
    assert!(events.iter().any(|e| matches!(
        &e.payload,
        crate::events::ServerEvent::JobPostedToCandidate { agent_id, .. }
            if agent_id == "agent-alpha"
    )));
}
```

## Imports needed

The test uses types from multiple modules. Ensure these are in scope at the top of the
`#[cfg(test)] mod tests` block:

```rust
use std::sync::Arc;
use tempfile::tempdir;
use crate::state::AppState;
use axum::extract::State;
use crate::extract::ValidJson;
```

Most of these should already be present from Task 2's tests.

## Verification

### Run the test
```bash
cargo test -p roko-serve matchmaking_full_flow -- --nocapture
```

### Run all roko-serve tests to ensure no regressions
```bash
cargo test -p roko-serve
```

### Clippy
```bash
cargo clippy -p roko-serve --no-deps -- -D warnings
```

## Acceptance criteria

- [ ] Test compiles and passes
- [ ] Match endpoint returns only agents with overlapping skills
- [ ] Agents are ranked by reputation (highest first)
- [ ] Job creation persists committed_candidates
- [ ] Full job lifecycle (open → assigned → in_progress → submitted → completed) works
- [ ] Events are emitted for job creation and per-candidate notification
- [ ] All existing roko-serve tests still pass
- [ ] Clippy passes clean

## What NOT to do

- Do NOT create a separate integration test binary — keep it in `jobs.rs` mod tests.
- Do NOT use `reqwest` or start a real TCP server — use `tower::ServiceExt::oneshot`.
- Do NOT skip any lifecycle step (assign, start, submit, evaluate) — test the full flow.
- Do NOT hardcode UUIDs — let the create endpoint generate them, then extract from response.
