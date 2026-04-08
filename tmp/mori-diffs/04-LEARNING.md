# Path 4: Learning Feedback -- Adaptive Routing, Episodes, Knowledge

## Current State (What's Broken)

The learning subsystems exist as well-engineered modules in `roko-learn` but are **not wired
into the runner v2 event loop**. The runner dispatches agents with a static model from
`RunConfig.model` (or task `model_hint`) and never consults the `CascadeRouter`. Outcomes
are logged but never fed back to influence future decisions.

### Specific gaps (3/10 wired)

| # | Problem | Where |
|---|---------|-------|
| L1 | **CascadeRouter never consulted** -- `dispatch_action` in `event_loop.rs:470` picks model from `task_def.model_hint.unwrap_or(&config.model)`. The router's `route()` method is never called. | `runner/event_loop.rs` |
| L2 | **No routing observations recorded** -- After a task passes or fails, the runner emits an `Episode` and `AgentEfficiencyEvent` to JSONL, but never calls `CascadeRouter::record_observation()`. The router never learns. | `runner/event_loop.rs:emit_episode` |
| L3 | **Episodes missing key fields** -- `emit_episode` populates model, tokens, cost, gate verdicts, and wall time. But `role` defaults to the agent_id string (not the task role), `files_changed` is always empty, and `provider` is hardcoded to `"claude"`. | `runner/event_loop.rs:586-637` |
| L4 | **Efficiency events per-task, not per-turn** -- `emit_efficiency_event` fires once per gate completion. Per-turn token events (`AgentEvent::TokenUsage`) are accumulated in `RunState` but never individually emitted as efficiency records. | `runner/event_loop.rs:641-687` |
| L5 | **Adaptive gate thresholds not loaded** -- Thresholds exist in `roko-learn` but the runner creates a fresh `ParallelExecutor` with `ExecutorConfig::default()`. Persisted thresholds from `.roko/learn/gate-thresholds.json` are never read on startup. | `runner/event_loop.rs:90-96` |
| L6 | **No knowledge ingestion on gate success** -- When a task passes all gates, the winning pattern (prompt + model + gate config) should be lowered-threshold ingested into the neuro store. This never happens. | Not wired |
| L7 | **force_backend not recorded** -- When a task specifies `model_hint`, the runner uses it but never records the outcome back through `CascadeRouter::record_override_outcome()`. Manual overrides don't feed learning. | `runner/event_loop.rs:470-474` |

### What works (3/10)

| # | Working | Where |
|---|---------|-------|
| W1 | Episode JSONL logging (basic fields) | `emit_episode` -> `persist::append_jsonl` |
| W2 | Efficiency JSONL logging (per-task) | `emit_efficiency_event` -> `persist::append_jsonl` |
| W3 | Token accumulation in `RunState` | `agent_events.rs:42-54` sums per-turn tokens |


## Design Goals

1. **CascadeRouter consulted at dispatch time** -- every `SpawnAgent` action routes through the cascade before selecting a model.
2. **Routing observations recorded after every task outcome** -- success/fail, cost, latency, and model all feed back to the router.
3. **Episodes enriched with full context** -- model, provider, tokens, cost, gate results, files changed, duration, role, task category.
4. **Efficiency events per-turn** -- each `TokenUsage` + `TurnCompleted` pair emits an efficiency record, not just one per gate cycle.
5. **Adaptive gate thresholds loaded from disk on startup** -- thresholds survive restarts.
6. **Knowledge ingestion on gate success** -- winning patterns ingested into neuro store with lowered admission threshold.
7. **force_backend outcomes recorded as observations** -- manual overrides feed back into routing via dampened learning rate.
8. **Cascade state persisted across runs** -- router snapshot saved periodically and loaded on startup.


## Architecture

### New Types

```rust
// runner/learning.rs

use std::path::Path;
use std::sync::Arc;

use roko_learn::cascade_router::CascadeRouter;
use roko_learn::model_router::RoutingContext;
use roko_learn::episode_logger::{Episode, Usage, GateVerdict};
use roko_learn::efficiency::AgentEfficiencyEvent;
use roko_core::agent::{AgentRole, ModelSpec};
use roko_core::task::TaskCategory;

/// Collects learning data during a task lifecycle and flushes it on
/// completion. Replaces the ad-hoc `emit_episode` / `emit_efficiency_event`
/// free functions with a structured, per-task collector.
pub struct LearningCollector {
    /// The cascade router to consult and record observations into.
    router: Arc<CascadeRouter>,

    /// Routing context built at dispatch time (frozen for the task duration).
    routing_ctx: Option<RoutingContext>,

    /// Model actually used (may differ from router recommendation if overridden).
    model_used: String,

    /// Whether the model was a force_backend override (dampened learning).
    is_override: bool,

    /// Per-turn token snapshots collected from AgentEvent::TokenUsage.
    turn_snapshots: Vec<TurnSnapshot>,

    /// Files changed during agent execution (populated from git diff).
    files_changed: Vec<String>,

    /// Task role (from task def, not agent_id).
    role: String,

    /// Task category for routing context.
    task_category: TaskCategory,

    /// Iteration number (0-based retry count).
    iteration: u32,
}

/// Snapshot of one agent turn's resource consumption.
#[derive(Debug, Clone)]
pub struct TurnSnapshot {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_usd: f64,
    pub wall_ms: u64,
    pub turn_index: u32,
}

/// Full routing context built from task metadata + runner state at dispatch
/// time. This wraps `roko_learn::model_router::RoutingContext` with the
/// additional runner-local fields needed to record observations.
pub struct DispatchRoutingContext {
    /// The RoutingContext passed to CascadeRouter::route().
    pub ctx: RoutingContext,
    /// Model recommended by the router.
    pub recommended_model: String,
    /// Model actually used (same as recommended unless overridden).
    pub actual_model: String,
    /// Whether this was a force_backend override.
    pub is_override: bool,
}

impl LearningCollector {
    /// Create a new collector for a task dispatch.
    pub fn new(
        router: Arc<CascadeRouter>,
        role: &str,
        task_category: TaskCategory,
        iteration: u32,
    ) -> Self { /* ... */ }

    /// Set the routing context and model after dispatch.
    pub fn set_dispatch(
        &mut self,
        routing_ctx: RoutingContext,
        model_used: String,
        is_override: bool,
    ) { /* ... */ }

    /// Record a per-turn token snapshot (called on each TokenUsage event).
    pub fn record_turn(&mut self, snapshot: TurnSnapshot) { /* ... */ }

    /// Record files changed (called before episode emission).
    pub fn set_files_changed(&mut self, files: Vec<String>) { /* ... */ }

    /// Flush all learning data after gate completion.
    ///
    /// This:
    /// 1. Records a routing observation into the CascadeRouter.
    /// 2. Writes an enriched Episode to the episode log.
    /// 3. Writes per-turn efficiency events to the efficiency log.
    /// 4. Optionally triggers knowledge ingestion on success.
    pub fn flush(
        &self,
        gate_passed: bool,
        gate_verdicts: &[GateVerdict],
        total_cost_usd: f64,
        total_wall_ms: u64,
        persist_paths: &PersistPaths,
    ) -> anyhow::Result<()> { /* ... */ }
}
```

```rust
// runner/learning.rs (continued)

/// Manages cascade router lifecycle: load from disk, save periodically,
/// and provide shared access to the event loop.
pub struct RouterState {
    /// The cascade router instance.
    pub router: Arc<CascadeRouter>,
    /// Path to the persisted router snapshot.
    snapshot_path: PathBuf,
    /// Whether the router has unsaved changes.
    dirty: bool,
}

impl RouterState {
    /// Load or create a cascade router.
    ///
    /// Tries to deserialize from `snapshot_path`. Falls back to a fresh
    /// router with the given model slugs.
    pub fn load_or_create(
        snapshot_path: &Path,
        model_slugs: Vec<String>,
    ) -> Self { /* ... */ }

    /// Save the router state to disk (called periodically from flush_interval).
    pub fn save(&mut self) -> anyhow::Result<()> { /* ... */ }

    /// Mark the router as having unsaved observations.
    pub fn mark_dirty(&mut self) { /* ... */ }
}
```

### Module Layout

```
crates/roko-cli/src/runner/
  learning.rs       # NEW: LearningCollector, RouterState, DispatchRoutingContext
  event_loop.rs     # MODIFIED: integrate LearningCollector + RouterState
  agent_events.rs   # MODIFIED: feed TurnSnapshot to collector
  state.rs          # MODIFIED: add learning_collector field
  types.rs          # UNCHANGED
  tui_bridge.rs     # UNCHANGED
```

### Integration Points

#### 1. Router consulted at dispatch time

```rust
// event_loop.rs, in dispatch_action SpawnAgent branch
// BEFORE (current):
let model = task_def
    .model_hint
    .as_deref()
    .unwrap_or(&ctx.config.model)
    .to_string();

// AFTER:
let (model, is_override) = if let Some(hint) = &task_def.model_hint {
    (hint.clone(), true) // force_backend override
} else {
    let routing_ctx = build_routing_context(task_def, ctx.state, ctx.config);
    let cascade_model = ctx.state.router.router.route(&routing_ctx);
    (cascade_model.primary.slug.clone(), false)
};

// Create the learning collector for this task
let mut collector = LearningCollector::new(
    ctx.state.router.router.clone(),
    task_def.role.as_deref().unwrap_or("implementer"),
    infer_task_category(task_def),
    ctx.state.iteration,
);
collector.set_dispatch(routing_ctx, model.clone(), is_override);
ctx.state.learning_collector = Some(collector);
```

#### 2. Per-turn snapshots recorded

```rust
// agent_events.rs, in handle_agent_event TokenUsage branch
AgentEvent::TokenUsage { input_tokens, output_tokens, cache_read_tokens, cache_write_tokens } => {
    state.tokens_in += input_tokens;
    state.tokens_out += output_tokens;
    state.cache_read_tokens += cache_read_tokens;
    state.cache_write_tokens += cache_write_tokens;

    // NEW: record per-turn snapshot
    if let Some(ref mut collector) = state.learning_collector {
        collector.record_turn(TurnSnapshot {
            input_tokens: *input_tokens,
            output_tokens: *output_tokens,
            cache_read_tokens: *cache_read_tokens,
            cache_write_tokens: *cache_write_tokens,
            cost_usd: 0.0, // filled from TurnCompleted
            wall_ms: state.task_elapsed_ms(),
            turn_index: state.task_agent_calls,
        });
    }
}
```

#### 3. Observations recorded on gate completion

```rust
// event_loop.rs, replacing emit_episode + emit_efficiency_event
if let Some(collector) = state.learning_collector.take() {
    // Populate files changed from git diff
    let files = git_diff_files(&config.workdir);
    collector.set_files_changed(files);

    // Flush: records observation, writes episode, writes efficiency events
    if let Err(e) = collector.flush(
        completion.passed,
        &gate_verdicts,
        state.cost_usd,
        state.task_elapsed_ms(),
        &paths,
    ) {
        error!(err = %e, "learning flush failed");
    }

    // Mark router dirty for periodic save
    state.router.mark_dirty();
}
```

#### 4. Router state persisted periodically

```rust
// event_loop.rs, in flush_interval branch
_ = flush_interval.tick() => {
    save_snapshot(&executor, &paths, &mut state);
    state.router.save().ok(); // NEW: persist router state
    // ...
}
```

#### 5. Gate thresholds loaded on startup

```rust
// event_loop.rs, in run() setup
let gate_thresholds = load_gate_thresholds(&paths.learn_dir);
let exec_config = ExecutorConfig {
    max_concurrent_plans: 4,
    max_concurrent_tasks: 1,
    max_auto_fix_iterations: config.max_retries,
    task_timeout_secs: config.timeout_secs,
    gate_thresholds, // NEW: loaded from disk
    ..Default::default()
};
```


## Detailed Specification

### 4.1 CascadeRouter Integration

**On startup** (`run()` function):
1. Load persisted cascade snapshot from `.roko/learn/cascade-router.json`.
2. If the file is missing or corrupt, create a fresh `CascadeRouter` with model slugs from config.
3. Wrap in `Arc<CascadeRouter>` and store in `RunState.router`.

**On dispatch** (`SpawnAgent` action):
1. Build a `RoutingContext` from the task definition:
   - `task_category`: inferred from task title/tags (implementation, test, review, etc.)
   - `complexity`: inferred from task description length, dependency count, and file scope
   - `iteration`: from `state.iteration`
   - `role`: from `task_def.role`
   - `crate_familiarity`: 0.0 initially (requires episodic memory lookup in future)
   - `has_prior_failure`: true if `state.iteration > 0`
   - `conductor_load`: 0.0 (single-agent runner)
2. Call `router.route(&routing_ctx)` to get a `CascadeModel`.
3. Use `cascade_model.primary.slug` as the model to spawn.
4. If `task_def.model_hint` is set, override the router selection but record `is_override = true`.

**On completion** (`gate_rx.recv` branch):
1. Compute routing reward: `compute_routing_reward_v2(quality, cost, latency, sla)` where:
   - `quality` = 1.0 if passed, 0.0 if failed
   - `cost` = `state.cost_usd`
   - `latency` = `state.task_elapsed_ms()`
   - `sla` = `default_latency_sla(model_tier)` from the cascade helpers
2. If `is_override`:
   - Call `router.record_override_outcome(model, &routing_ctx, passed)` (dampened rate)
3. Else:
   - Call `router.record_observation(&routing_ctx, model, reward, passed)`
4. Check for stage transitions: `router.check_stage_transition()` and log if occurred.

**Periodic save** (every 2s flush):
- If `router_state.dirty`, serialize the cascade snapshot and write to `.roko/learn/cascade-router.json`.

### 4.2 Enriched Episodes

The `LearningCollector::flush` method builds an `Episode` with these fields:

| Field | Source | Current | New |
|-------|--------|---------|-----|
| `agent_id` | `plan_id/task_id` | Yes | Same |
| `task_id` | `completion.task_id` | Yes | Same |
| `model` | `state.agent_model` | Yes | Same |
| `backend` | hardcoded | `"claude"` | From config (claude/codex/etc) |
| `role` | Missing | agent_id string | `task_def.role` |
| `usage.input_tokens` | state | Yes | Same |
| `usage.output_tokens` | state | Yes | Same |
| `usage.cache_read_tokens` | state | Yes | Same |
| `usage.cache_write_tokens` | state | Yes | Same |
| `usage.cost_usd` | state | Yes | Same |
| `usage.wall_ms` | state | Yes | Same |
| `gate_verdicts` | completion | Yes | Same |
| `turns` | state | Yes | Same |
| `files_changed` | Missing | Empty | `git diff --name-only` |
| `task_category` | Missing | Not set | From RoutingContext |
| `extra.plan_id` | completion | Yes | Same |
| `extra.routing_stage` | Missing | Not set | CascadeRouter stage |
| `extra.was_override` | Missing | Not set | bool |
| `extra.recommended_model` | Missing | Not set | Router's pick |

### 4.3 Per-Turn Efficiency Events

Currently, `emit_efficiency_event` fires once per task (after gate completion). The new design emits **two levels** of efficiency events:

1. **Per-turn**: On each `TurnCompleted` event, emit a lightweight efficiency record to `.roko/learn/efficiency-turns.jsonl`:
   ```rust
   pub struct TurnEfficiencyEvent {
       pub agent_id: String,
       pub plan_id: String,
       pub task_id: String,
       pub turn_index: u32,
       pub model: String,
       pub input_tokens: u64,
       pub output_tokens: u64,
       pub cache_read_tokens: u64,
       pub cache_write_tokens: u64,
       pub cost_usd: f64,
       pub wall_ms: u64,
       pub timestamp: String,
   }
   ```

2. **Per-task** (existing): The existing `AgentEfficiencyEvent` continues to be emitted on gate completion with the full summary.

### 4.4 Adaptive Gate Thresholds

**On startup**:
1. Read `.roko/learn/gate-thresholds.json`.
2. Deserialize into `HashMap<String, f64>` (rung name -> threshold).
3. Pass into `ExecutorConfig.gate_thresholds` so the executor uses learned thresholds.

**On gate completion**:
1. Update the EMA threshold for the rung that ran:
   ```
   new_threshold = alpha * observed_pass_rate + (1 - alpha) * old_threshold
   ```
   where `alpha = 0.1` (slow learning rate to avoid oscillation).
2. Write the updated thresholds to disk.

### 4.5 Knowledge Ingestion on Success

When a task passes all gates:
1. Build a `KnowledgeEntry` from the successful episode:
   - `topic`: task title / description
   - `content`: summary of what worked (model, prompt structure, gate config)
   - `source`: `"episode:{episode_id}"`
   - `confidence`: start at 0.7 (lowered admission threshold for winning patterns)
2. Call `neuro_store.ingest(entry)` if the neuro store is available.
3. This is **best-effort**: failure to ingest does not block the run.

### 4.6 Force-Backend Override Learning

When a task has `model_hint` set:
1. The runner uses the hint directly (bypassing the router).
2. On completion, call `router.record_override_outcome(model_slug, &routing_ctx, success)`.
3. This uses the existing `OVERRIDE_LEARNING_RATE` (dampened) so manual overrides influence but don't dominate the bandit.
4. Record `extra.was_override = true` in the episode for audit.

### 4.7 Data Flow Diagram

```
                    Task Dispatch
                         |
                         v
               +-------------------+
               | build_routing_ctx |
               +-------------------+
                         |
                         v
               +-------------------+
               | CascadeRouter     |
               |   .route(ctx)     |
               +-------------------+
                    |           |
             recommended    model_hint?
                model       (override)
                    |           |
                    +-----+-----+
                          |
                          v
                  [Agent Execution]
                          |
                          v
              +------------------------+
              | Per-turn: TokenUsage   |
              |   -> TurnSnapshot      |
              |   -> TurnEfficiency    |
              +------------------------+
                          |
                          v
                  [Gate Pipeline]
                          |
                    pass / fail
                          |
                          v
            +----------------------------+
            | LearningCollector.flush()  |
            |   1. record_observation()  |
            |   2. write Episode         |
            |   3. write Efficiency      |
            |   4. update thresholds     |
            |   5. ingest knowledge      |
            +----------------------------+
                          |
                          v
            +----------------------------+
            | Periodic: save router      |
            |   cascade-router.json      |
            +----------------------------+
```


## Error Handling

| Scenario | Handling |
|----------|----------|
| Router snapshot corrupt on load | Log warning, create fresh router. Router starts in Static stage. |
| Router `route()` panics | Catch via `std::panic::catch_unwind`, fall back to config model. |
| Episode append fails (disk full) | Log error, continue execution. Learning degrades gracefully. |
| Gate threshold file missing | Use defaults (same as current behavior). |
| Knowledge ingestion fails | Log warning, continue. Best-effort only. |
| Unknown model slug in observation | `record_observation` returns early (existing behavior). |


## Testing Strategy

### Unit tests

1. **`LearningCollector` flush correctness**: Create a collector, feed it turn snapshots and a gate result, call `flush()`, verify the episode JSONL contains all enriched fields.
2. **Router selection integration**: Create a `CascadeRouter` with 2 models, feed 100 observations favoring model A, verify `route()` returns model A.
3. **Override dampening**: Record 50 observations via `record_override_outcome`, verify the effective weight is scaled by `OVERRIDE_LEARNING_RATE`.
4. **Gate threshold persistence**: Write thresholds to a temp file, load them back, verify values match.

### Integration tests

5. **Full task lifecycle**: Run a mock plan with one task through the event loop, verify:
   - CascadeRouter was consulted (router observation count > 0)
   - Episode contains model, role, cost, gate verdicts
   - Efficiency event was written
6. **Override path**: Run a task with `model_hint` set, verify `record_override_outcome` was called.
7. **Router persistence across runs**: Run two sequential plan executions, verify the second run loads the router state from the first.

### Property tests

8. **Reward bounded**: Verify `compute_routing_reward_v2` always returns values in `[0.0, 1.0]`.
9. **Turn snapshot accumulation**: Verify sum of per-turn tokens equals the per-task total.


## Open Questions

1. **Neuro store availability**: The runner currently doesn't hold a reference to the neuro store. Should knowledge ingestion be async (fire-and-forget task) or synchronous (blocking the flush)?
   - **Recommendation**: Fire-and-forget `tokio::spawn` with a `neuro_store.ingest()` call. The runner should not block on knowledge ingestion.

2. **Router warm-up data**: Should we seed the cascade router with historical episode data on first load, or start cold?
   - **Recommendation**: Start cold. The Static stage (< 50 observations) already handles cold start gracefully with the role-model table.

3. **Per-turn vs per-task efficiency granularity**: Is the per-turn efficiency log worth the disk I/O overhead?
   - **Recommendation**: Yes. Per-turn data is essential for prompt section attribution and tool-call efficiency analysis. Use buffered writes to amortize I/O.

## Implementation Packet

This work turns learning from passive logging into an active feedback loop in the runner path.

### Required Context

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-cli/src/runner/agent_events.rs`
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-learn/src/episode_logger.rs`
- `crates/roko-learn/src/efficiency.rs`
- `crates/roko-learn/src/routing_log.rs`
- `crates/roko-learn/src/runtime_feedback.rs`
- `docs/05-learning/00-episode-logger.md`
- `docs/05-learning/04-cascade-router.md`
- `tmp/unified/07-LEARNING.md`

### Target Files

- [ ] Create `crates/roko-cli/src/runtime_feedback/mod.rs`.
- [ ] Create `crates/roko-cli/src/runtime_feedback/episodes.rs`.
- [ ] Create `crates/roko-cli/src/runtime_feedback/routing.rs`.
- [ ] Create `crates/roko-cli/src/runtime_feedback/efficiency.rs`.
- [ ] Update `crates/roko-cli/src/lib.rs` module exports if needed.
- [ ] Update `runner/event_loop.rs` to call the feedback facade.

### Checklist

- [ ] Load persisted `CascadeRouter` state before first dispatch.
- [ ] Build a routing context from task role, domain, complexity, model hint, budget, affect state when available, and provider health when available.
- [ ] Record override outcomes when `model_hint` forces a model.
- [ ] Emit per-turn efficiency records on usage events.
- [ ] Emit per-task efficiency records after gate completion.
- [ ] Include role, provider, model, cost, latency, task id, plan id, gate result, and files changed in each episode.
- [ ] Record failed gate classes as learning observations.
- [ ] Flush router state after every terminal task event or during periodic flush.
- [ ] Add a feedback facade method such as `on_runtime_event(&RuntimeEvent)`.

### Acceptance Criteria

- [ ] A second run can see a routing observation from the first run.
- [ ] `episodes.jsonl` contains provider and model values that match the actual dispatch path.
- [ ] `efficiency.jsonl` receives per-turn token events.
- [ ] Task-level pass/fail updates the router.
- [ ] Learning code is no longer spread directly through the event loop.

## Worker 9 Evidence Checklist (2026-04-26)

Learning abstractions already implemented outside the active runner:

- [x] `crates/roko-learn/src/runtime_feedback.rs` defines `LearningRuntime`, `LearningPaths`, `record_completed_run`, `append_efficiency_event`, `append_knowledge_seed`, `update_cascade_router`, and provider/model outcome recording.
- [x] `LearningPaths` includes persistent paths for episodes, efficiency, gate outcomes, retry outcomes, knowledge seeds, cascade router state, gate thresholds, and provider/model outcomes.
- [x] `crates/roko-learn/src/event_subscriber.rs` can consume learning `AgentEvent`s and update provider health, routing, calibration, costs, and efficiency.
- [x] `crates/roko-cli/src/runner/event_loop.rs` writes simple task-level episodes and efficiency records after gate completion.
- [x] The no-mock proof produced `.roko/episodes.jsonl` and `.roko/learn/efficiency.jsonl` through the live runner path.

Required work before this doc is archivable:

- [ ] Add or choose a runner feedback facade; the planned `crates/roko-cli/src/runtime_feedback/` module does not exist.
- [ ] Replace runner-local `emit_episode` and `emit_efficiency_event` with `roko_learn::runtime_feedback::LearningRuntime` or an equivalent facade.
- [ ] Remove hardcoded runner episode fields such as backend `"claude"` and role `"implementer"`; provider/model values must come from the actual dispatch path.
- [ ] Emit per-turn efficiency records from usage events, not only task-level gate summaries.
- [ ] Load and update `CascadeRouter` state from the active `runner/event_loop.rs` dispatch path.
- [ ] Write routing observations, knowledge candidates, conductor observations, and dream trigger signals from one live feedback surface.
- [ ] Prove a second run can consume a routing or learning observation produced by the first run.

## 2026-04-27 Deepening Pass - Source-Corrected Feedback Loop

Self-grade for this pass:

- Initial rating: 9.91 / 10.
- Reasoning: this section updates the doc from a stale "feedback facade absent" claim to the current, narrower failure: the facade exists and is wired, but high-fidelity dispatch data and durable second-run influence are still incomplete. The score is not higher because no clean-clone two-run learning proof has been executed in this pass.

This section supersedes the initial "Current State" table where source has moved forward.

### Current Source Truth

- [x] `crates/roko-cli/src/runtime_feedback/mod.rs` exists and defines `FeedbackEvent`, `FeedbackSink`, `FeedbackFacade`, sink stats, and fire-and-forget fan-out semantics.
- [x] `crates/roko-cli/src/runtime_feedback/episodes.rs` writes `FeedbackEvent::TaskCompleted` into `.roko/episodes.jsonl` through `EpisodeLogger`.
- [x] `crates/roko-cli/src/runtime_feedback/routing.rs` records task-completed outcomes into `CascadeRouter::record_outcome`.
- [x] `crates/roko-cli/src/runtime_feedback/knowledge.rs` writes `.roko/learn/knowledge_candidates.jsonl` and supports an optional live `KnowledgeIngestor`.
- [x] `crates/roko-cli/src/runtime_feedback/conductor.rs` writes conductor observations.
- [x] `crates/roko-cli/src/runtime_feedback/dreams.rs` writes `.roko/learn/dream_triggers.jsonl` and supports an optional live `DreamRunner`.
- [x] `crates/roko-cli/src/commands/plan.rs` builds a `FeedbackFacade` with episode, routing, knowledge, conductor, and dream sinks for `plan run`.
- [x] `crates/roko-cli/src/runner/event_loop.rs` fans runner lifecycle events into the feedback facade through `runner_event_to_feedback`.
- [x] `crates/roko-cli/src/runner/event_loop.rs` also records `RunnerFeedbackEvent::CompletedRun` into `roko_learn::runtime_feedback::LearningRuntime`.

### Current Learning Gaps

- [ ] `runner_event_to_feedback` currently synthesizes `AgentOutcome` with empty `model`, empty `provider`, zero tokens, zero cost, and zero duration for task-completed events when the real dispatch outcome is not attached.
- [ ] `RoutingObservationSink` currently records both override and non-override outcomes through `record_outcome`; override damping and full `RoutingContext` feature vectors are not wired end to end.
- [ ] `KnowledgeIngestionSink` writes candidates by default; durable `roko-neuro` admission/reinforcement/lifecycle is optional and not proven in active plan-run.
- [ ] `DreamTriggerSink` writes triggers by default; actual dream consolidation and next-run prompt/routing influence are optional and not proven in active plan-run.
- [ ] `FeedbackFacade` errors are counted per sink but those counters are not yet exposed through a stable HTTP/query projection.
- [ ] Cascade router persistence is not yet proven across two real runs; recording into an in-memory router is not enough for Mori parity.
- [ ] Gate threshold learning is still separate from feedback facade completion and needs durable proof.
- [ ] Per-turn token events are not yet represented as `FeedbackEvent::TurnCompleted` from normalized provider runtime events in the active path.
- [ ] Files changed, task category, failure class, prompt section ids, knowledge ids, dream advice ids, and provider health are not yet consistently included in the learning observation.

### Target Learning Transaction

Every task attempt should produce one `LearningTransaction` object before any sink writes:

```rust
pub struct LearningTransaction {
    pub run_id: String,
    pub plan_id: String,
    pub task_id: String,
    pub attempt: u32,
    pub role: String,
    pub task_category: String,
    pub failure_class: Option<String>,
    pub model_choice: crate::dispatch::ModelChoice,
    pub provider: String,
    pub model: String,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
    pub duration_ms: u64,
    pub gate_passed: Option<bool>,
    pub gate_rung: Option<u32>,
    pub files_changed: Vec<String>,
    pub prompt_section_ids: Vec<String>,
    pub knowledge_ids: Vec<String>,
    pub playbook_ids: Vec<String>,
    pub dream_advice_ids: Vec<String>,
}
```

Rules:

- [ ] The transaction is assembled once from dispatch outcome, runner event, gate result, prompt diagnostics, and git diff.
- [ ] Sinks consume the transaction by reference; no sink recomputes provider/model/cost fields.
- [ ] The router sink receives the full routing context and the actual outcome, not a lossy default `AgentOutcome`.
- [ ] Episode sink, efficiency sink, knowledge sink, conductor sink, and dream sink write records with the same run id and attempt id.
- [ ] The projection layer exposes the transaction id so HTTP/TUI users can trace every derived observation back to one runtime event.

### Implementation Batches

#### LRN-01: Replace Lossy Feedback Translation

- [ ] Add a real `TaskAttemptRuntimeSummary` or equivalent to `RunnerEvent::TaskAttemptCompleted`.
- [ ] Populate it from `AgentOutcome` returned by `Dispatcher`.
- [ ] Include provider, model, model choice source, token counts, cost, duration, exit code, and session/run id.
- [ ] Update `runner_event_to_feedback` to use real fields and fail loudly if a completed agent task lacks dispatch outcome data.
- [ ] Add a regression test that fails if `model`, `provider`, or tokens are blank for a real dispatch completion.

#### LRN-02: Make Router Persistence Real

- [ ] Ensure the cascade router loaded in `commands/plan.rs` is the same router instance used by dispatch and feedback.
- [ ] After every terminal task event, mark the router dirty.
- [ ] Save router state to `.roko/learn/cascade-router.json` or a versioned replacement.
- [ ] Record `ModelChoiceSource::Override` through an override-aware method once the feature vector is available.
- [ ] Prove run 2 can read a routing observation written by run 1.

#### LRN-03: Add Per-Turn Efficiency

- [ ] Translate `AgentRuntimeEvent::TokenUsage` into `FeedbackEvent::TurnCompleted` or a more precise `TurnUsageRecorded` event.
- [ ] Include run id, plan id, task id, attempt id, provider, model, turn index, token counts, cache tokens, cost, and duration.
- [ ] Write `.roko/learn/efficiency.jsonl` from the feedback facade, not from ad-hoc event-loop helpers.
- [ ] Prove token totals in per-turn records sum to task-level totals.

#### LRN-04: Close Knowledge Lifecycle

- [ ] Keep writing `.roko/learn/knowledge_candidates.jsonl` as the durable hot-path outbox.
- [ ] Add a worker or post-run pass that consumes candidates into `roko-neuro::lifecycle`.
- [ ] Record admission result: admitted, reinforced, rejected, stale, or falsified.
- [ ] Include source episode ids and gate falsifier ids.
- [ ] Prove a successful task creates a knowledge candidate and a durable lifecycle observation.

#### LRN-05: Close Dream Loop

- [ ] Keep writing `.roko/learn/dream_triggers.jsonl` as the durable hot-path outbox.
- [ ] Add a worker or post-run pass that consumes triggers into `roko-dreams`.
- [ ] Persist dream outputs and routing advice with ids.
- [ ] Feed dream advice ids into the next run's routing and prompt assembly.
- [ ] Prove run 2 can show a prompt diagnostic or routing decision referencing dream output from run 1.

#### LRN-06: Expose Learning Observability

- [ ] Add projection events for feedback sink delivery, skip, and failure counters.
- [ ] Add HTTP/query access for recent episodes, routing observations, knowledge candidates, dream triggers, and feedback sink errors.
- [ ] Add a `roko inspect learning` or equivalent CLI read path if HTTP is not running.
- [ ] Redact prompt bodies while preserving prompt diagnostic ids.

### Generated Proof Contract

An agent implementing this file must produce `tmp/mori-diffs/generated/learning-feedback-proof.json`:

```json
{
  "schema": "mori-diffs.learning-feedback-proof.v1",
  "generated_at": "ISO-8601 timestamp",
  "git_commit": "HEAD sha",
  "run_1": {
    "run_id": "string",
    "provider": "string",
    "model": "string",
    "episodes_written": 0,
    "routing_observations_written": 0,
    "efficiency_turn_records_written": 0,
    "knowledge_candidates_written": 0,
    "dream_triggers_written": 0
  },
  "run_2": {
    "run_id": "string",
    "router_loaded_prior_observation": false,
    "knowledge_or_dream_influenced_prompt": false,
    "knowledge_or_dream_influenced_routing": false
  },
  "sink_stats": [],
  "http_or_cli_queries": [],
  "remaining_gaps": []
}
```

### No-Context Handoff Checklist

- [ ] Open `crates/roko-cli/src/runner/event_loop.rs` and find `runner_event_to_feedback`.
- [ ] Replace synthetic blank `AgentOutcome` creation with real dispatch outcome data.
- [ ] Open `crates/roko-cli/src/commands/plan.rs` and verify the same `CascadeRouter` is passed into dispatch and `RoutingObservationSink`.
- [ ] Open `crates/roko-cli/src/runtime_feedback/routing.rs` and make override observations feature-aware.
- [ ] Open `crates/roko-cli/src/runtime_feedback/knowledge.rs` and add durable lifecycle ingestion proof.
- [ ] Open `crates/roko-cli/src/runtime_feedback/dreams.rs` and add durable consolidation proof.
- [ ] Add the generated proof file above.
- [ ] Update [29-CURRENT-RUNTIME-GAP-LEDGER.md](29-CURRENT-RUNTIME-GAP-LEDGER.md), [38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md](38-COGNITIVE-FEEDBACK-LOOP-AUDIT.md), and [README.md](README.md).

### Archive Gate

- [ ] The old "runtime_feedback module does not exist" checklist item is corrected.
- [ ] Real model/provider/token/cost fields reach all learning sinks.
- [ ] Router state persists across two runs.
- [ ] Knowledge candidate outbox reaches durable neuro lifecycle.
- [ ] Dream trigger outbox reaches durable dream consolidation.
- [ ] HTTP or CLI queries can show feedback artifacts and sink failures.
- [ ] `learning-feedback-proof.json` exists and is linked from README.
