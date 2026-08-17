# 10 — Observability: Unified RuntimeEvent + Projection

> Phase 4 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Cross-references audit `tmp/workflow/16-http-serve-persistence-audit.md`.

---

## Status (2026-05-01)

**PARTIAL.** `RuntimeEvent` exists, `RuntimeProjection` exists, two persistent JSONL streams exist with conflicting purposes.

**What's done:**

- `roko_core::runtime_event::RuntimeEvent` enum — `crates/roko-core/src/runtime_event.rs`
- `RuntimeEventEnvelope { run_id, seq, source, event }` — round-trips JSON
- `roko_runtime::projection::RuntimeProjection` and `RunSummary` — `crates/roko-runtime/src/projection.rs`
- `WorkflowEngine` emits `RuntimeEvent`s via `event_bus::emit_runtime_event`
- `JsonlLogger` writes them to `.roko/runtime-events.jsonl` — `crates/roko-runtime/src/jsonl_logger.rs`
- `crates/roko-serve/src/state.rs` seeds the StateHub with workflow events
- HTTP `GET /api/dashboard/runs` reads `runtime-events.jsonl` via `RuntimeProjection` — `crates/roko-serve/src/routes/runs.rs`
- StateHub still runs in parallel: `.roko/events.jsonl` carries `DashboardEvent` envelopes for the legacy TUI / scope.

**What's not:**

- **Two persistent streams disagree:**
  - `.roko/events.jsonl` (StateHub `DashboardEvent`) — `crates/roko-runtime/src/event_bus.rs` + `crates/roko-runtime/.../state_hub.rs`
  - `.roko/runtime-events.jsonl` (workflow `RuntimeEvent`) — `JsonlLogger`
- **Two HTTP surfaces disagree:**
  - `GET /api/dashboard/runs` reads `RuntimeEvent` (workflow projection)
  - `GET /api/runs/{id}` (`shared_runs.rs::load_transcript`) reads share-transcript (different store)
- `InvalidationPolicy.max_age_secs` defined but no TTL check
- TUI dashboard does not read `RuntimeEvent`s — it has its own `DashboardData` from `tui/dashboard.rs`
- HTTP SSE `/api/events` and WS `/ws` send `DashboardEvent`s; nothing fans `RuntimeEvent` to web subscribers
- `RuntimeEvent` enum is missing variants for tool calls (`ToolCallStarted`, `ToolCallCompleted`, `ToolOutputDelta`)
- No coordinated proof that "workflow event emitted → StateHub broadcast → SSE client receives within X ms"

---

## Goal

One canonical event stream (`RuntimeEvent`) consumed by everything: TUI, HTTP SSE, WebSocket, dashboards, shared transcript pages. The two JSONL files merge into one; the two HTTP routes converge on the same projection. Projection TTL is enforced.

---

## Why This Exists (Anti-Patterns Eliminated)

- **#3 Build Another Runtime** — observability today has two parallel stacks
- **#7 Copy-Paste** — `RuntimeEvent` and `DashboardEvent` are 80% the same fields
- **#10 God file** — `tui/dashboard.rs` (6K LOC) and `tui/state.rs` (5K LOC) load disk independently because no shared projection serves them

---

## Existing Code — Read These First

```rust
// crates/roko-core/src/runtime_event.rs
pub enum RuntimeEvent {
    WorkflowStarted { run_id, template, prompt },
    PhaseTransition { run_id, from, to },
    AgentSpawned { run_id, agent_id, role, model },
    AgentOutput { run_id, agent_id, chunk },
    AgentCompleted { run_id, agent_id, output, tokens_used, cost_usd },
    AgentFailed { run_id, agent_id, error },
    GateStarted { run_id, gate_name, rung },
    GatePassed { run_id, gate_name, duration_ms },
    GateFailed { run_id, gate_name, output, duration_ms },
    FeedbackRecorded { run_id, kind, summary },
    StateCheckpointed { run_id, path },
    WorkflowCompleted { run_id, outcome },
}

pub struct RuntimeEventEnvelope {
    pub run_id: String,
    pub seq: u64,
    pub source: String,
    pub event: RuntimeEvent,
    pub timestamp_ms: u64,
}
```

```rust
// crates/roko-runtime/src/projection.rs
pub struct RuntimeProjection {
    runs: HashMap<String, RunSummary>,
}

pub struct RunSummary {
    pub run_id: String,
    pub started_at_ms: u64,
    pub last_event_ms: u64,
    pub status: RunStatus,
    pub agent_count: u32,
    pub gate_count: u32,
    pub total_cost_usd: f64,
    pub total_tokens: u64,
}
```

The legacy StateHub / `DashboardEvent` lives in `roko-runtime/src/event_bus.rs`. It has overlapping fields plus extra flexibility (`Topic`, `Subscription`).

---

## Implementation Steps

### Step 1 — Extend `RuntimeEvent` to subsume `DashboardEvent`

Audit the `DashboardEvent` enum and add missing variants to `RuntimeEvent`:

```rust
// crates/roko-core/src/runtime_event.rs (additions)
pub enum RuntimeEvent {
    // existing
    WorkflowStarted { ... },
    PhaseTransition { ... },
    AgentSpawned { ... },
    AgentOutput { ... },
    AgentCompleted { ... },
    AgentFailed { ... },
    GateStarted { ... },
    GatePassed { ... },
    GateFailed { ... },
    FeedbackRecorded { ... },
    StateCheckpointed { ... },
    WorkflowCompleted { ... },

    // NEW — for TUI / SSE consumers
    ToolCallStarted { run_id, agent_id, tool: String, args_summary: String },
    ToolCallCompleted { run_id, agent_id, tool: String, result_summary: String, duration_ms: u64 },
    ToolOutputDelta { run_id, agent_id, tool: String, chunk: String },
    AgentThinkingDelta { run_id, agent_id, chunk: String },

    PlanStarted { run_id, plan_id: String, task_count: u32 },
    PlanCompleted { run_id, plan_id: String, success: bool },
    TaskStarted { run_id, plan_id: String, task_id: String, role: String },
    TaskCompleted { run_id, plan_id: String, task_id: String, success: bool, duration_ms: u64, cost_usd: f64 },

    MergeStarted { run_id, plan_id: String, strategy: String },
    MergeSucceeded { run_id, plan_id: String, commit: String },
    MergeFailed { run_id, plan_id: String, conflict: String },

    ReviewStarted { run_id, agent_id, role: String },
    ReviewApproved { run_id, agent_id, summary: String },
    ReviewRevised { run_id, agent_id, findings: Vec<String> },

    AutoFixStarted { run_id, error_summary: String },
    AutoFixCompleted { run_id, fixed: bool },

    ModelCallStarted { run_id, request_id, model: String, role: String },
    ModelCallCompleted { run_id, request_id, model: String, usage: TokenUsage },

    SafetyAlert { run_id, agent_id, severity: String, reason: String },
    SafetyWarning { run_id, agent_id, warning: String },
    AgentBlocked { run_id, agent_id, reason: String },

    CostUpdate { run_id, total_cost_usd: f64, total_tokens: u64 },
    PromptAssembled { run_id, agent_id, diagnostics: PromptDiagnostics },
}
```

Bump `RuntimeEventEnvelope::source` semantics: `"workflow_engine" | "effect_driver" | "model_call_service" | "safety" | "tool_dispatcher" | "feedback_service"`.

### Step 2 — Migrate `DashboardEvent` consumers to `RuntimeEvent`

Find every `DashboardEvent` use:

```bash
rg 'DashboardEvent' crates/ --type rust
```

For each, either:
- Replace the variant with the equivalent `RuntimeEvent` variant
- Or, if dashboard-specific (e.g. `DashboardLayoutChanged`), keep as a separate `UiEvent` enum **inside the TUI crate** — not a runtime concern

Key consumers:

- `crates/roko-cli/src/tui/app.rs` — drain `DashboardEvent` from `apply_dashboard_snapshot`. Switch to `RuntimeEvent` + `apply_runtime_event`.
- `crates/roko-cli/src/tui/state.rs` — `TuiState` updates from events; rewire from `RuntimeEvent`.
- `crates/roko-serve/src/routes/sse.rs` and `ws.rs` — SSE / WS broadcasters; today they send `DashboardEvent`. Switch to `RuntimeEventEnvelope`.

### Step 3 — Single canonical JSONL — `.roko/events.jsonl`

Pick `.roko/events.jsonl` (matches `RokoLayout::events_path`) as canonical.

- `JsonlLogger` writes `RuntimeEventEnvelope` to `.roko/events.jsonl`
- StateHub broadcast uses the same envelope shape (no separate `DashboardEvent` envelope)
- Migration helper at startup: if `.roko/runtime-events.jsonl` exists and `.roko/events.jsonl` does not, move it. If both exist, merge by `seq` (deduplicate).

After migration, `runtime-events.jsonl` references in code are deleted.

### Step 4 — Single HTTP projection layer

Today:
- `GET /api/dashboard/runs` returns workflow projection (run summaries from `RuntimeProjection`)
- `GET /api/runs/{id}` returns share transcript (different schema, from `shared_runs.rs::load_transcript`)

After:
- `GET /api/runs` → list all runs (paginated, recent first)
- `GET /api/runs/{id}` → full run state (replaces both)
- `GET /api/runs/{id}/events` → `RuntimeEventEnvelope[]`, filtered + paginated
- `GET /api/runs/{id}/transcript` → human-readable transcript view (was `shared_runs.rs::load_transcript`); built from events
- Public-share routes (no auth) at `/runs/{id}` redirect through `/api/runs/{id}` with public token check

Keep the shareable `/runs/{token}` HTML page (it's a product feature) — but it now consumes the same projection.

```rust
// crates/roko-serve/src/routes/runs.rs (new unified routes)
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/runs", get(list_runs))
        .route("/api/runs/{id}", get(get_run))
        .route("/api/runs/{id}/events", get(get_run_events))
        .route("/api/runs/{id}/transcript", get(get_run_transcript))
}

async fn get_run_events(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<EventQuery>,
) -> impl IntoResponse {
    let projection = state.projection.runtime();
    let events = projection.events_for_run(&id, q).await?;
    Json(events)
}
```

### Step 5 — Add TTL enforcement to `RuntimeProjectionSet`

**File:** `crates/roko-runtime/src/projection.rs`

```rust
pub struct ProjectionEnvelope<T> {
    pub schema_version: u32,
    pub cursor: Option<String>,
    pub generated_at_ms: u64,
    pub max_age_secs: u64,
    pub data: T,
}

impl<T> ProjectionEnvelope<T> {
    pub fn is_stale(&self, now_ms: u64) -> bool {
        (now_ms - self.generated_at_ms) > self.max_age_secs * 1000
    }
}

impl RuntimeProjectionSet {
    pub async fn load_or_refresh<T>(&self, name: &str) -> Result<ProjectionEnvelope<T>>
    where T: DeserializeOwned + Default + Send,
    {
        let envelope = self.load(name).await?;
        if envelope.is_stale(now_ms()) {
            self.refresh(name).await?;
            return self.load(name).await;
        }
        Ok(envelope)
    }
}
```

HTTP routes use `load_or_refresh` instead of `load`. Audit doc 16 § 8 specifically calls this gap out.

### Step 6 — TUI dashboard consumes `RuntimeProjection`

**Files:** `crates/roko-cli/src/tui/dashboard.rs`, `tui/state.rs`

Today TUI loads `.roko/` files independently via `DashboardData::load_from_disk`. Replace with `RuntimeProjection::dashboard_view` (a new convenience method):

```rust
// crates/roko-runtime/src/projection.rs
impl RuntimeProjection {
    pub fn dashboard_view(&self) -> DashboardView {
        DashboardView {
            active_runs: self.runs.values().filter(|r| r.status.is_active()).cloned().collect(),
            total_cost_today: self.cost_for_period(today_start_ms(), now_ms()),
            top_models_by_calls: self.top_models(10),
            recent_failures: self.recent_failures(20),
            // ... whatever TUI needs
        }
    }
}
```

TUI then:

```rust
// in TUI render loop
let view = state.projection.dashboard_view();
draw_dashboard(&view, frame);
```

This eliminates the `DashboardData` god file's disk-load logic.

### Step 7 — End-to-end SSE proof

```rust
// crates/roko-serve/tests/sse_runtime_event_proof.rs
#[tokio::test]
async fn sse_client_receives_runtime_events_within_100ms() {
    let server = test_server_with_workflow_engine().await?;
    let url = format!("{}/api/events?run_id=*", server.url());

    let mut sse = reqwest::Client::new().get(&url).send().await?.bytes_stream();
    let received = Arc::new(Mutex::new(Vec::new()));
    let recv = received.clone();
    tokio::spawn(async move {
        while let Some(Ok(chunk)) = sse.next().await {
            let envelope: RuntimeEventEnvelope = parse_sse_chunk(chunk)?;
            recv.lock().await.push((envelope, now_ms()));
        }
    });

    // Trigger a workflow run
    let trigger_at = now_ms();
    server.engine.run(test_run_config()).await?;

    let events = received.lock().await.clone();
    assert!(!events.is_empty(), "client should receive events");

    // First event should arrive within 100ms of WorkflowStarted emission
    let first = events.first().unwrap();
    assert!(first.1 - trigger_at < 100, "first event delay too high: {}ms", first.1 - trigger_at);
}
```

### Step 8 — `caller` + provenance fields surfaced in events

Every `RuntimeEvent::ModelCallStarted` carries:

- `caller: String` (from `ModelCallRequest.caller`)
- `prompt_section_ids: Vec<String>`
- `knowledge_ids: Vec<String>`

So a TUI run inspector (see plan 17 / demo) can show "this call used these prompt sections + this knowledge".

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #3 Build another runtime | Adding a new event enum for SSE | One `RuntimeEvent` only |
| #7 Copy-paste | Duplicating `DashboardEvent` fields in `RuntimeEvent` | Migrate consumers, then delete `DashboardEvent` |
| #10 God file | `tui/dashboard.rs` 6K-LOC blob | Replace its disk-load with `RuntimeProjection::dashboard_view` |

---

## Things NOT To Do

1. **Don't keep both event enums "for compatibility".** They diverge. Pick one (RuntimeEvent), deprecate the other in one release, delete next release.
2. **Don't emit per-character text deltas.** `AgentOutput { chunk }` should batch (50ms or 200 chars, whichever first) to avoid SSE flood.
3. **Don't store full `output: String` in the JSONL.** Truncate to 4KB; full output goes in episode JSONL (different file).
4. **Don't expose internal state via SSE without auth filtering.** Per-run subscription must require auth or be opt-in via a public-share token.
5. **Don't tie projection refresh to user requests.** Background `RuntimeProjection::refresh_loop()` runs every N seconds; HTTP requests serve cached data + check TTL.
6. **Don't create a `DashboardEvent::FromRuntime(RuntimeEvent)` adapter variant.** It's the same data twice. Migrate consumers fully.
7. **Don't mix `tracing::info!` with `RuntimeEvent` emission.** Tracing is for developer debug; events are for user-visible state. Separate.
8. **Don't lose seq numbers across crashes.** `seq` must be monotonic across restarts; persist `last_seq` to `.roko/state/event-cursor.json`.

---

## Tests / Proof Criteria

```bash
# 1. One canonical event enum
rg 'pub enum RuntimeEvent|pub enum DashboardEvent' crates/ --type rust
# expected: only RuntimeEvent (DashboardEvent removed or deprecated)

# 2. One canonical JSONL
ls .roko/*.jsonl 2>/dev/null
# expected: events.jsonl present; runtime-events.jsonl absent

# 3. Projection TTL enforced
rg 'is_stale|load_or_refresh' crates/roko-runtime/src/projection.rs
# expected: 2+ matches

# 4. HTTP routes consolidated
rg 'route.*runs' crates/roko-serve/src/routes/ --type rust
# verify only the new unified routes
```

Functional proofs:

- [ ] SSE client test (Step 7) passes: events arrive within 100ms
- [ ] `roko dashboard` shows live cost ticker that updates within 1s of each model call
- [ ] `GET /api/runs/{id}/events?phase=Gating` returns only Gate* events
- [ ] Public share page at `/runs/{token}` uses the same projection backend; data is identical to authed `/api/runs/{id}`
- [ ] Migration: existing `.roko/runtime-events.jsonl` file successfully merges into `.roko/events.jsonl` on first startup after the change
- [ ] `RuntimeProjection::dashboard_view` returns same data as `DashboardData::load_from_disk` for the same run
- [ ] TTL test: projection with `max_age_secs: 5`, no events for 6s → request triggers refresh

---

## Dependencies

- **Plan 04 (PersistenceService)** — uses transactional writes for envelope cursor
- **Plan 07 (EffectDriver)** — emits the new event variants
- **Plan 09 (Safety)** — emits Safety* events

Can start in parallel with 04, 07, 09 once event enum extension is decided.

---

## Estimated Effort

**L.** ~1.5-2 weeks.

- Step 1 (event enum extension) — S (1 day)
- Step 2 (DashboardEvent migration) — L (4-5 days; ~30+ call sites)
- Step 3 (single JSONL) — S (1 day)
- Step 4 (HTTP route consolidation) — M (2-3 days)
- Step 5 (TTL) — S (1 day)
- Step 6 (TUI on projection) — M (3-4 days; TUI is sensitive)
- Step 7 (SSE proof) — S (1 day)
- Step 8 (caller surfacing) — S (1 day)
