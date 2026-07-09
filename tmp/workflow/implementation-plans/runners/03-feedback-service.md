# Runner 03 — Feedback Service Completion

> **Give this entire file to a fresh agent.**

---

## Context

Codebase: `/Users/will/dev/nunchi/roko/roko` (Rust workspace). Goal: close the learning loop so **every** model call, gate run, and workflow completion records feedback, regardless of entry point. Currently chat and several CLI paths record nothing.

**Read first:**

1. `tmp/workflow/ANTI-PATTERNS.md`
2. `tmp/workflow/implementation-plans/03-feedback-service-completion.md`
3. `crates/roko-core/src/foundation.rs` — `FeedbackSink` trait, `FeedbackEvent` enum
4. `crates/roko-learn/src/feedback_service.rs` — existing `FeedbackService`
5. `crates/roko-cli/src/runtime_feedback/mod.rs` — the **other** `FeedbackSink` + `FeedbackEvent` (to be eliminated)
6. `crates/roko-cli/src/runtime_feedback/routing.rs` — `RoutingObservationSink`
7. `crates/roko-cli/src/chat_session.rs` — search for `with_feedback_sink` (not present = the bug)
8. `crates/roko-learn/src/cascade_router.rs` — `observe` vs `observe_multi_objective`

---

## Work Items

### Step 1: Extend canonical `FeedbackEvent`

**File:** `crates/roko-core/src/foundation.rs`

Add new variants to `FeedbackEvent`:

```rust
TaskStarted { run_id: String, plan_id: String, task_id: String, role: String },
TaskCompleted { run_id: String, plan_id: String, task_id: String, role: String,
    success: bool, model: String, duration_ms: u64, cost_usd: f64, tokens_used: u64,
    gate_verdicts: Vec<GateVerdict> },
TaskFailed { run_id: String, plan_id: String, task_id: String, error: String },
PlanStarted { run_id: String, plan_id: String, task_count: u32 },
PlanCompleted { run_id: String, plan_id: String, success: bool },
```

### Step 2: Move CLI sinks to `roko-learn/src/sinks/`

Create `crates/roko-learn/src/sinks/` directory. For each sink in `crates/roko-cli/src/runtime_feedback/`:

1. Copy file to `crates/roko-learn/src/sinks/`
2. Change `impl runtime_feedback::FeedbackSink` → `impl roko_core::foundation::FeedbackSink`
3. Adapt `on_event(&FeedbackEvent)` → `record(&self, event: FeedbackEvent)`
4. Update all callers

Files to move: `episodes.rs`, `routing.rs`, `knowledge.rs`, `conductor.rs`, `dreams.rs`

### Step 3: Create `MultiSink`

**File:** `crates/roko-learn/src/multi_sink.rs`

```rust
pub struct MultiSink { sinks: Vec<Arc<dyn FeedbackSink>> }

#[async_trait]
impl FeedbackSink for MultiSink {
    async fn record(&self, event: FeedbackEvent) -> Result<()> {
        for sink in &self.sinks {
            if let Err(e) = sink.record(event.clone()).await {
                tracing::warn!(?e, "feedback sink failed");
            }
        }
        Ok(())
    }
    async fn flush(&self) -> Result<()> {
        for sink in &self.sinks { sink.flush().await?; }
        Ok(())
    }
}
```

### Step 4: Create `ThresholdSink` and `PlaybookSink`

- `crates/roko-learn/src/sinks/threshold.rs` — on `GateResult` events, calls `AdaptiveThresholds::observe(rung, passed)`
- `crates/roko-learn/src/sinks/playbook.rs` — on `TaskCompleted` with success=true and all gates passed, builds + upserts playbook

### Step 5: Attach `FeedbackService` to chat

**File:** `crates/roko-cli/src/chat_session.rs`

Find where `ModelCallService` is constructed. Use `ServiceFactory` instead of manual construction so the `FeedbackService` gets attached:

```rust
let services = ServiceFactory::for_chat(&workdir, &config).await?;
let model_caller = services.model_caller();  // already has feedback wired
```

Verify: after `roko "hello"`, `.roko/episodes.jsonl` has a new line with `"caller":"cli"`.

### Step 6: Wire `observe_multi_objective`

**File:** `crates/roko-learn/src/feedback_service.rs`

Find `observe_model_call` method. Replace `router.observe(...)` with `router.observe_multi_objective(MultiObjectiveObservation { ... })`.

### Step 7: Prune 12 noisy hooks

For each of these files in `crates/roko-learn/src/`, check if it has live callers (via `rg`). If no live callers outside tests/orchestrate, delete:

1. `hdc.rs` 2. `anomaly_detector.rs` 3. `strategy_metadata.rs` 4. `force_backend_override.rs` 5. `somatic_markers.rs` 6. `calibration.rs` 7. `enriched_run_recorder.rs` 8. `context_attribution.rs`

Also: stop populating `emotional_tags` in new episodes (set to `None`).

Remove any `mod` declarations and `use` statements for deleted modules.

### Step 8: Delete CLI `runtime_feedback/` module

After Steps 2+3, no code in `roko-cli` should reference `runtime_feedback::FeedbackSink`. Delete the entire directory. Update `crates/roko-cli/src/lib.rs` to remove `pub mod runtime_feedback`.

Replace `FeedbackFacade` usage in `crates/roko-cli/src/commands/plan.rs` with `ServiceFactory.feedback_sink()`.

---

## Verification Checklist

```bash
rg 'pub enum FeedbackEvent' crates/ --type rust
# MUST return exactly 1 (in roko-core)

rg 'pub trait FeedbackSink' crates/ --type rust
# MUST return exactly 1 (in roko-core)

ls crates/roko-cli/src/runtime_feedback/
# MUST fail (directory deleted)

rg 'observe_multi_objective' crates/roko-learn/src/feedback_service.rs
# MUST return 1+ (live path)

cargo test --workspace
```

---

## Critical Rules

1. **ONE `FeedbackEvent` enum, ONE `FeedbackSink` trait.** Both in `roko-core`.
2. **Sinks are fire-and-forget.** `record()` returns quickly; slow work spawns onto tokio.
3. **NEVER record feedback per stream chunk.** One `ModelCall` event when the call completes.
4. **ALWAYS flush on shutdown.** Add explicit `services.shutdown().await` in CLI/serve shutdown.
5. **Episode schema stays in `roko-learn`.** Only `EpisodeSummary` in `roko-core`.
