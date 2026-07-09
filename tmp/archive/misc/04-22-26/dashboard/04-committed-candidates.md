# Task 4: Add committed_candidates to job creation

## Objective

When the user accepts a matchmaking quote in the dashboard, the frontend calls
`POST /api/jobs` with the list of agent IDs the user approved. The server needs to:
1. Store the `committed_candidates` list on the `JobRecord`.
2. Emit a `job.posted` event per candidate (so relay/sidecar can notify them).

## Files to modify

| File | What to change |
|---|---|
| `crates/roko-serve/src/routes/jobs.rs` | Add `committed_candidates` to `JobRecord` and `CreateJobRequest`; emit per-candidate events |
| `crates/roko-serve/src/events.rs` | Add `JobPostedToCandidate` event variant (if not already present) |

## Detailed changes

### 1. `crates/roko-serve/src/routes/jobs.rs` — JobRecord struct (line 66)

Add a new field after `plan_id` (line 92):

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
committed_candidates: Vec<String>,
```

### 2. `crates/roko-serve/src/routes/jobs.rs` — CreateJobRequest struct (line 116)

Add a matching field after `plan_id` (line 137):

```rust
#[serde(default)]
committed_candidates: Vec<String>,
```

### 3. `crates/roko-serve/src/routes/jobs.rs` — create_job handler (line 284)

In the `JobRecord` construction (line 301), add:

```rust
committed_candidates: body.committed_candidates.clone(),
```

(It currently does not have this field — add it after the `plan_id` line at ~314.)

After the existing `publish_job_event` call (line 319), add per-candidate event emission:

```rust
for candidate_id in &job.committed_candidates {
    state.event_bus.publish(ServerEvent::JobPostedToCandidate {
        job_id: job.id.clone(),
        agent_id: candidate_id.clone(),
        reward: job.reward.clone(),
    });
}
```

### 4. `crates/roko-serve/src/events.rs` — ServerEvent enum

Check if a `JobPostedToCandidate` variant already exists. If not, add it alongside the other
job events (after `JobTransitioned`, around line 194):

```rust
/// A newly posted job has been broadcast to a specific candidate agent.
JobPostedToCandidate {
    job_id: String,
    agent_id: String,
    reward: String,
},
```

Also add the variant to the `event_type()` match arm (this method returns a string tag for
SSE/WebSocket consumers):

```rust
ServerEvent::JobPostedToCandidate { .. } => "job.posted_to_candidate",
```

## Verification

### Compile check
```bash
cargo build -p roko-serve
```

### Existing tests must pass
```bash
cargo test -p roko-serve
```

### Unit test

Add to the test module in `jobs.rs`:

```rust
#[tokio::test]
async fn create_job_with_committed_candidates_emits_events() {
    let tempdir = tempdir().expect("tempdir");
    let state = Arc::new(AppState::new(
        tempdir.path().to_path_buf(),
        Arc::new(crate::runtime::NoOpRuntime),
        roko_core::config::schema::RokoConfig::default(),
        Arc::new(crate::deploy::manual::ManualBackend::default()),
    ));

    let result = create_job(
        State(Arc::clone(&state)),
        ValidJson(CreateJobRequest {
            id: Some("job-test".into()),
            title: "test job".into(),
            committed_candidates: vec!["agent-1".into(), "agent-2".into()],
            reward: "500 KORAI".into(),
            ..Default::default()
        }),
    ).await.expect("create job");

    // Verify the job was persisted with candidates
    let job = load_job(tempdir.path(), "job-test").await.expect("load job");
    assert_eq!(job.committed_candidates, vec!["agent-1", "agent-2"]);

    // Verify events were emitted
    let events = state.event_bus.replay_from(0);
    let posted_events: Vec<_> = events.iter().filter(|e| matches!(
        &e.payload,
        ServerEvent::JobPostedToCandidate { .. }
    )).collect();
    assert_eq!(posted_events.len(), 2, "expected one event per candidate");
}
```

**Note:** `CreateJobRequest` needs `Default` derive for the test. It currently does not have
it. Add `#[derive(Default)]` — all fields already have `#[serde(default)]` or are `String`
(which defaults to empty), except `title` which is just `String`. This is fine for tests.
Alternatively, spell out all fields explicitly in the test.

### Manual HTTP verification
```bash
cargo run -p roko-cli -- serve &

# Create a job with committed candidates
curl -s -X POST http://localhost:6677/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "implement walrus gateway relay",
    "description": "Build a relay",
    "reward": "2500 KORAI",
    "committed_candidates": ["agent-rustsmith", "agent-ethdev"]
  }' | jq .

# Verify committed_candidates in the response
# Expected: "committed_candidates": ["agent-rustsmith", "agent-ethdev"]

# Verify persisted to disk
ls .roko/jobs/
cat .roko/jobs/*.json | jq '.committed_candidates'
```

### Backward compatibility
```bash
# Job creation WITHOUT committed_candidates still works (empty default)
curl -s -X POST http://localhost:6677/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{"title": "simple job"}' | jq .committed_candidates
# Expected: field absent (skip_serializing_if = "Vec::is_empty")
```

## What NOT to do

- Do NOT add escrow locking logic — that's a Daeji (blockchain) concern, not roko-serve.
- Do NOT modify the job state machine transitions — `committed_candidates` is metadata, not
  a status change.
- Do NOT validate that committed_candidates are registered agents — the dashboard handles
  this via the match flow, and we don't want to couple job creation to agent discovery.
- Do NOT modify any fields in `JobRecord` besides adding the new one.
