# Task 07: WebSocket/SSE Event Format Normalization

**Priority**: P0
**Crate**: `roko-serve`
**Files**: `crates/roko-serve/src/routes/ws.rs`, `crates/roko-serve/src/routes/sse.rs`, `crates/roko-serve/src/state.rs` (ServerEvent enum)

## Problem

The dashboard expects WebSocket/SSE events with specific `type` field values in snake_case.
Roko-serve's `ServerEvent` enum may serialize with PascalCase variant names (Rust default
for serde) or different field names.

### Dashboard expected event types

From `nunchi-dashboard/src/services/rokoWs.ts` and `wsInvalidation.ts`:

```typescript
type WsEventPayload =
  | { type: "run_started";          run_id: string; prompt_preview: string }
  | { type: "run_completed";        run_id: string; success: boolean }
  | { type: "agent_output";         agent_id: string; content: string; done: boolean }
  | { type: "plan_started";         plan_id: string }
  | { type: "plan_completed";       plan_id: string; success: boolean }
  | { type: "task_started";         task_id: string; plan_id?: string }
  | { type: "task_completed";       task_id: string; plan_id?: string; success?: boolean }
  | { type: "task_failed";          task_id: string; plan_id?: string }
  | { type: "gate_result";          task_id: string; rung: number; passed: boolean }
  | { type: "heartbeat";            agent_id: string; block_number: number | null }
  | { type: "heartbeat_received";   sender_id: string; active_tasks: number; active_agents: number }
  | { type: "job_created";          job_id: string; job_type: string; title: string }
  | { type: "job_state_changed";    job_id: string; old_state: string; new_state: string }
  | { type: "job_submitted";        job_id: string; agent_id: string }
  | { type: "job_evaluated";        job_id: string; accepted: boolean }
  | { type: "error";                message: string }
  | { type: "operation_completed";  op_id: string; kind: string; success: boolean }
```

### Dashboard query invalidation mapping

When the dashboard receives a WS event, it invalidates TanStack Query caches based on
the `type` field:

```
run_started          → invalidate [status]
run_completed        → invalidate [status, plans, agents]
plan_started         → invalidate [plans]
plan_completed       → invalidate [plans, status]
task_started         → invalidate [plans]
task_completed       → invalidate [plans]
task_failed          → invalidate [plans, diagnosis]
gate_result          → invalidate [plans, adaptiveThresholds]
agent_output         → invalidate [agents]
agent_started        → invalidate [agents]
agent_stopped        → invalidate [agents]
operation_completed  → invalidate [plans, prds, status]
job_created          → invalidate [jobs]
job_state_changed    → invalidate [jobs]
job_submitted        → invalidate [jobs]
job_evaluated        → invalidate [jobs]
heartbeat            → invalidate [agents, health]
heartbeat_received   → invalidate [agents, health, networkStats]
efficiency_event     → invalidate [efficiency, cfactor]
episode_recorded     → invalidate [cfactor, metricsSummary]
experiment_updated   → invalidate [experiments]
cascade_updated      → invalidate [cascade]
```

If the event `type` strings don't match, the dashboard won't invalidate queries and the
UI will appear stale (not updating) until the next polling interval.

### Likely Rust serialization issue

Roko-serve's `ServerEvent` enum (in `state.rs`) likely uses serde's default serialization:

```rust
#[derive(Serialize)]
enum ServerEvent {
    PlanStarted { plan_id: String },
    PlanCompleted { plan_id: String, success: bool },
    // ...
}
```

This serializes as `{"PlanStarted": {"plan_id": "..."}}` (externally tagged) or
`{"type": "PlanStarted", ...}` (internally tagged with `#[serde(tag = "type")]`).

The dashboard expects `{"type": "plan_started", ...}` — **internally tagged, snake_case**.

## Implementation

### Step 1: Audit current ServerEvent serialization

Read `crates/roko-serve/src/state.rs` and find the `ServerEvent` enum. Check:

1. Does it have `#[serde(tag = "type")]`? (internally tagged)
2. Does it have `#[serde(rename_all = "snake_case")]`? (snake_case variants)
3. What are the exact variant names and fields?

### Step 2: Add serde attributes if missing

The enum needs both attributes:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    PlanStarted { plan_id: String },
    PlanCompleted { plan_id: String, success: bool },
    TaskStarted { task_id: String, plan_id: Option<String> },
    TaskCompleted { task_id: String, plan_id: Option<String>, success: Option<bool> },
    TaskFailed { task_id: String, plan_id: Option<String> },
    RunStarted { run_id: String, prompt_preview: String },
    RunCompleted { run_id: String, success: bool },
    AgentOutput { agent_id: String, content: String, done: bool },
    GateResult { task_id: String, rung: u32, passed: bool },
    HeartbeatReceived { sender_id: String, active_tasks: u32, active_agents: u32 },
    JobCreated { job_id: String, job_type: String, title: String },
    JobStateChanged { job_id: String, old_state: String, new_state: String },
    JobSubmitted { job_id: String, agent_id: String },
    JobEvaluated { job_id: String, accepted: bool },
    OperationCompleted { op_id: String, kind: String, success: bool },
    Error { message: String },
    // ... other variants
}
```

### Step 3: Add missing event variants

Cross-reference the dashboard's expected types against the enum's actual variants. Add
any missing variants. Common gaps:

- `task_failed` — the dashboard expects this separately from `task_completed`
- `heartbeat_received` — may be different from the internal heartbeat processing
- `job_*` variants — may not exist if job events were added later
- `operation_completed` — for background operation tracking
- `agent_started` / `agent_stopped` — for agent lifecycle
- `efficiency_event` / `episode_recorded` / `experiment_updated` / `cascade_updated` — learning events

For any variant that doesn't have natural data to emit, add the variant to the enum
and emit it at the appropriate point in the code.

### Step 4: Verify field names match

For each variant, verify field names match exactly:
- `plan_id` not `planId`
- `run_id` not `runId`
- `agent_id` not `agentId`
- `task_id` not `taskId`
- `job_id` not `jobId`
- `op_id` not `opId`

Serde's `#[serde(rename_all = "snake_case")]` only affects the tag name, not field names.
If fields are already snake_case in Rust, they'll serialize correctly. But if any field
uses camelCase, add `#[serde(rename = "snake_case_name")]`.

### Step 5: Emit events at correct points

Verify that events are actually emitted (published to the broadcast channel) at the
right points in the code:

| Event | Where it should be emitted |
|-------|---------------------------|
| `plan_started` | When `POST /api/plans/{id}/execute` begins |
| `plan_completed` | When plan execution finishes |
| `task_started` | When a task within a plan begins |
| `task_completed` | When a task passes gates |
| `task_failed` | When a task fails gates |
| `run_started` | When `POST /api/run` spawns a background run |
| `run_completed` | When the background run finishes |
| `gate_result` | After each gate check |
| `agent_output` | When agent produces output (streaming or final) |
| `job_*` | In job lifecycle handlers |
| `heartbeat_received` | In `POST /api/heartbeats` handler |
| `operation_completed` | When any background operation finishes |

Search for `event_bus.send()` or `broadcast.send()` calls in the codebase to see where
events are currently emitted, and add any missing emission points.

## Files to modify

| File | Change |
|------|--------|
| `crates/roko-serve/src/state.rs` | ServerEvent serde attributes + missing variants |
| `crates/roko-serve/src/routes/plans.rs` | Emit events if missing |
| `crates/roko-serve/src/routes/run.rs` | Emit events if missing |
| `crates/roko-serve/src/routes/jobs.rs` | Emit events if missing |
| `crates/roko-serve/src/routes/heartbeats.rs` | Emit events if missing |
| Other route files | Wherever events should be emitted but aren't |

## Verification

### Automated

```bash
cargo build -p roko-serve
cargo test -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
```

### Manual — event format

```bash
cargo run -p roko-cli -- serve &

# Connect to WebSocket and capture events
websocat ws://127.0.0.1:6677/roko-ws &
WS_PID=$!

# Trigger a plan execution or idea creation
curl -s -X POST http://127.0.0.1:6677/api/prds/ideas \
  -H 'Content-Type: application/json' \
  -d '{"text": "test WS events"}'

sleep 2

# The WebSocket should have received events
# Verify event format:
# - Has "type" field
# - Type is snake_case (e.g., "plan_started" not "PlanStarted")
# - Fields are snake_case

kill $WS_PID
```

### Manual — round-trip type check

Write a quick test that serializes each ServerEvent variant and checks the `type` field:

```bash
# In a Rust test or script:
# let event = ServerEvent::PlanStarted { plan_id: "test".into() };
# let json = serde_json::to_string(&event).unwrap();
# assert!(json.contains(r#""type":"plan_started""#));
```

This should be a unit test in `state.rs` — add it as part of this task.

### Manual — dashboard integration

```bash
# Start serve + dashboard
# Open dashboard, open browser DevTools Network tab, filter WS
# Trigger actions via chat (/idea, /plan, etc.)
# Verify WS messages have correct type fields
# Verify UI updates when events arrive (plans tab refreshes, etc.)
```

## Acceptance criteria

- [ ] `ServerEvent` uses `#[serde(tag = "type", rename_all = "snake_case")]`
- [ ] All variant names serialize to snake_case (e.g., `plan_started`, not `PlanStarted`)
- [ ] All field names are snake_case in JSON output
- [ ] All event types expected by dashboard exist as variants
- [ ] Events are emitted at the correct code points (plan exec, task exec, job lifecycle, etc.)
- [ ] Unit test verifies serialization format for each variant
- [ ] SSE events use the same format as WebSocket events
- [ ] All existing tests still pass
- [ ] No new clippy warnings
