# 03 — FeedbackService: Close the Learning Loop on Live Paths

> Foundation Phase 0.3 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Cross-references audit `tmp/workflow/12-learning-feedback-audit.md`.

---

## Status (2026-05-01)

**PARTIAL.** Trait + service exist; ~half of live paths attached; learning loop closed for `WorkflowEngine` runs but **not** for chat or HTTP inference; `observe_multi_objective` still only fires from tests / dead code.

**What's done:**

- `roko_core::foundation::FeedbackSink` trait + `FeedbackEvent` enum — `crates/roko-core/src/foundation.rs:325-332` (and event variants 278-323)
- `roko_learn::FeedbackService` (concrete impl) — `crates/roko-learn/src/feedback_service.rs`. Implements `roko_core::FeedbackSink`. Builders: `from_roko_dir_with_episodes`, `with_cascade_router`, `with_section_effectiveness`, knowledge scores.
- `WorkflowEngine::record_workflow_feedback` emits `FeedbackEvent::WorkflowComplete` and flushes the sink — `crates/roko-runtime/src/workflow_engine.rs:~598-627`
- `EffectDriver` records `ModelCall` and `GateResult` events — `crates/roko-runtime/src/effect_driver.rs:~203,~249,~491`
- `EpisodeLogger` (canonical Episode schema) — `crates/roko-learn/src/episode_logger.rs:Episode`
- `CascadeRouter::observe(...)` is called from `FeedbackService::observe_model_call` for every `ModelCall`
- `ServiceFactory::build` attaches the `FeedbackService` to `EffectServices`

**What's not:**

- **Chat API path** does not attach `FeedbackService`. `crates/roko-cli/src/chat_session.rs:~630` constructs `ModelCallService` without `.with_feedback_sink(...)`. So **none of the most-used path's runs are recorded**.
- **HTTP `/api/inference/complete`** uses `ModelCallService` from `AppState` (which can have a sink attached via `ServiceFactory`), but **no prompt-section feedback** because the route bypasses `PromptAssemblyService` (see plan 02).
- **Naming collision:** there are TWO `FeedbackSink` traits and TWO `FeedbackEvent` enums:
  1. `roko_core::foundation::FeedbackSink` / `FeedbackEvent` (canonical for `WorkflowEngine`)
  2. `roko_cli::runtime_feedback::FeedbackSink` / `FeedbackEvent` (CLI-only, used by `roko plan run` via `FeedbackFacade`)
- **`observe_multi_objective`** is referenced only from tests in `crates/roko-cli/tests/phase0_wiring.rs` and from the legacy-orchestrate-gated `orchestrate.rs:~11018`. Live paths use the simpler `observe(...)` and `record_confidence_outcome(...)`.
- **`run_learning_subscriber`** in `crates/roko-learn/src/event_subscriber.rs` is `tokio::spawn`'d only in `#[cfg(test)]` blocks. No production wiring.
- **No `ThresholdSink` symbol** — gate threshold updates happen inline inside `GateService` and ACP runner separately, not via a sink.
- **No `PlaybookSink` symbol** — playbook record/query is bypassed entirely on `roko run` and chat. Only orchestrate (gated) uses it.
- The 12 noisy hooks listed in plan §0.3.3 (HDC, affect, anomaly, etc.) are still emitted from orchestrate when enabled; not pruned.
- Chat-side `CostMeter` is in-memory only; chat episodes never persist.

---

## Goal

**Every** model call, gate run, and workflow completion in the binary records a `FeedbackEvent` to one canonical `FeedbackService`, regardless of entry point. After two runs of the same task, observable changes occur in:

- `.roko/episodes.jsonl` (two records)
- `.roko/learn/cascade-router.json` (router updated)
- `.roko/learn/section-effects.json` (per-section trial counters incremented)
- `.roko/learn/efficiency.jsonl` (two records)
- `.roko/learn/playbooks/` (if task succeeded, playbook recorded with confidence > 0.5)
- `.roko/learn/gate-thresholds.json` (EMA updated for each rung that ran)

The CLI `runtime_feedback::FeedbackSink` trait collapses into the canonical one.

---

## Why This Exists (Anti-Patterns Eliminated)

- **#6 Feedback as Afterthought** — most live paths today record nothing
- **#7 Copy-Paste Between Runtimes** — two `FeedbackSink` traits doing the same thing
- **#3 Build Another Runtime** — CLI built its own facade rather than wiring into the canonical one

---

## Existing Code — Read These First

```325:332:crates/roko-core/src/foundation.rs
#[async_trait]
pub trait FeedbackSink: Send + Sync {
    async fn record(&self, event: FeedbackEvent) -> Result<()>;
    async fn flush(&self) -> Result<()>;
}
```

```278:323:crates/roko-core/src/foundation.rs
pub enum FeedbackEvent {
    ModelCall {
        run_id: Option<String>,
        request_id: Option<String>,
        prompt_section_ids: Vec<String>,
        knowledge_ids: Vec<String>,
        model: Option<String>,
        provider: Option<String>,
        token_usage: Option<u64>,
        cost: Option<f64>,
        role: String,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: f64,
        latency_ms: u64,
        success: bool,
    },
    GateResult { run_id, gate_name, passed, duration_ms },
    WorkflowComplete { event_type, run_id, model, success, outcome, total_cost_usd, total_tokens, duration_ms },
}
```

`crates/roko-cli/src/runtime_feedback/mod.rs:125-138` defines the **other** `FeedbackSink`:

```rust
pub trait FeedbackSink: Send + Sync {
    fn name(&self) -> &str;
    fn interested(&self, event: &FeedbackEvent) -> bool { true }
    async fn on_event(&self, event: &FeedbackEvent) -> Result<()>;
}
```

with `FeedbackEvent` variants like `TaskCompleted { task_id, success, model, duration_ms, ... }`. These are **per-task** events, while the canonical sink emits **per-call** events. They serve different layers.

The CLI sinks include `EpisodeSink`, `RoutingObservationSink`, `KnowledgeIngestionSink`, `ConductorObservationSink`, `DreamTriggerSink`. They run via `FeedbackFacade` from `crates/roko-cli/src/commands/plan.rs`.

---

## Implementation Steps

### Step 1 — Promote one canonical `FeedbackEvent` and collapse the CLI duplicate

The canonical one (`roko_core::foundation::FeedbackEvent`) is per-call. Add **per-task** events to it so the CLI variants can fold in:

```rust
// roko_core::foundation::FeedbackEvent (extended)
pub enum FeedbackEvent {
    ModelCall { /* unchanged */ },
    GateResult { /* unchanged */ },

    TaskStarted {
        run_id: String, plan_id: String, task_id: String, role: String,
    },
    TaskCompleted {
        run_id: String, plan_id: String, task_id: String, role: String,
        success: bool, model: String, model_source: ModelSource,
        duration_ms: u64, cost_usd: f64, tokens_used: u64,
        gate_verdicts: Vec<GateVerdict>,
    },
    TaskFailed {
        run_id: String, plan_id: String, task_id: String,
        error: String, retry_eligible: bool,
    },

    PlanStarted { run_id, plan_id, task_count: u32 },
    PlanCompleted { run_id, plan_id, success: bool },

    WorkflowComplete { /* unchanged */ },
}
```

Rename `roko_cli::runtime_feedback::FeedbackEvent` to `RuntimeFeedbackEvent` temporarily during migration to avoid name shadowing. Then delete it once all CLI sinks consume the canonical event.

### Step 2 — Move CLI sinks into `roko-learn`

The five CLI-side sinks (`EpisodeSink`, `RoutingObservationSink`, `KnowledgeIngestionSink`, `ConductorObservationSink`, `DreamTriggerSink`) belong in `roko-learn` so all crates can use them. Move:

- `crates/roko-cli/src/runtime_feedback/episodes.rs` → `crates/roko-learn/src/sinks/episodes.rs`
- `crates/roko-cli/src/runtime_feedback/routing.rs` → `crates/roko-learn/src/sinks/routing.rs`
- `crates/roko-cli/src/runtime_feedback/knowledge.rs` → `crates/roko-learn/src/sinks/knowledge.rs`
- `crates/roko-cli/src/runtime_feedback/conductor.rs` → `crates/roko-learn/src/sinks/conductor.rs`
- `crates/roko-cli/src/runtime_feedback/dreams.rs` → `crates/roko-learn/src/sinks/dreams.rs`

Each implements `roko_core::FeedbackSink`. Add a unified `MultiSink` that fans events out:

```rust
// crates/roko-learn/src/multi_sink.rs
pub struct MultiSink {
    sinks: Vec<Arc<dyn FeedbackSink>>,
}

#[async_trait]
impl FeedbackSink for MultiSink {
    async fn record(&self, event: FeedbackEvent) -> Result<()> {
        for sink in &self.sinks {
            if let Err(e) = sink.record(event.clone()).await {
                tracing::warn!(?e, "feedback sink failed; continuing");
            }
        }
        Ok(())
    }
    async fn flush(&self) -> Result<()> {
        for sink in &self.sinks {
            sink.flush().await?;
        }
        Ok(())
    }
}
```

`ServiceFactory::build` constructs a `MultiSink` containing all live sinks and passes one `Arc<dyn FeedbackSink>` to every consumer.

### Step 3 — Add new sinks: `ThresholdSink` and `PlaybookSink`

#### ThresholdSink (replaces inline gate threshold updates)

```rust
// crates/roko-learn/src/sinks/threshold.rs
pub struct ThresholdSink {
    thresholds: Arc<Mutex<AdaptiveThresholds>>,    // shared with GateService
    path: PathBuf,                                  // .roko/learn/gate-thresholds.json
}

#[async_trait]
impl FeedbackSink for ThresholdSink {
    async fn record(&self, event: FeedbackEvent) -> Result<()> {
        if let FeedbackEvent::GateResult { gate_name, passed, .. } = event {
            let rung = rung_for_gate(&gate_name);
            self.thresholds.lock().await.observe(rung, passed);
        }
        Ok(())
    }
    async fn flush(&self) -> Result<()> {
        let snapshot = self.thresholds.lock().await.snapshot();
        atomic_write_json(&self.path, &snapshot).await
    }
}
```

Wire the same `Arc<Mutex<AdaptiveThresholds>>` into `GateService::with_adaptive_thresholds` so both read/write the same in-memory state. This lets `ServiceFactory` finally call `with_adaptive_thresholds` (currently it does not, per audit doc 11).

#### PlaybookSink

```rust
// crates/roko-learn/src/sinks/playbook.rs
pub struct PlaybookSink {
    store: Arc<PlaybookStore>,
}

#[async_trait]
impl FeedbackSink for PlaybookSink {
    async fn record(&self, event: FeedbackEvent) -> Result<()> {
        if let FeedbackEvent::TaskCompleted { task_id, role, success, gate_verdicts, .. } = event {
            // Build playbook iff task succeeded with all gates passing
            if success && gate_verdicts.iter().all(|v| v.passed) {
                let playbook = self.store.build_from_task(&task_id, &role).await?;
                self.store.upsert(playbook).await?;
            }
            self.store.record_confidence(&task_id, success).await?;
        }
        Ok(())
    }
}
```

### Step 4 — Attach `FeedbackService` to chat path

**File:** `crates/roko-cli/src/chat_session.rs:~630`

Today `ModelCallService::new(...)` is constructed without `.with_feedback_sink(...)`. Use `ServiceFactory` instead:

```rust
let services = ServiceFactory::for_chat(&workdir, &config).await?;
let model_caller = services.model_caller();    // already has feedback wired
```

After this, every chat turn writes an episode and updates the cascade router. Verify via:

```bash
roko "what is the meaning of life"
ls -la .roko/episodes.jsonl
# expect: a new line appended; "caller": "cli", "role": "interactive_chat"
```

**Caution:** Chat episodes can be voluminous. Add a config knob `[learn].chat_episode_recording = true` (default `true`); allow operators to disable for privacy.

### Step 5 — Wire `observe_multi_objective` in `FeedbackService`

Today `FeedbackService::observe_model_call` calls only `router.observe(...)`. The richer `observe_multi_objective` (from `crates/roko-learn/src/cascade_router.rs`) takes additional fields: latency_ms, cost_usd, success, c_factor.

Replace the call:

```rust
// crates/roko-learn/src/feedback_service.rs (sketch)
async fn observe_model_call(&self, event: &FeedbackEvent::ModelCall) -> Result<()> {
    if let Some(router) = &self.router {
        let mut router = router.lock().await;
        router.observe_multi_objective(MultiObjectiveObservation {
            model: event.model.as_deref().unwrap_or(""),
            role: &event.role,
            success: event.success,
            cost_usd: event.cost_usd,
            latency_ms: event.latency_ms,
            tokens: event.input_tokens + event.output_tokens,
            c_factor: 0.0,                  // computed from gate verdicts upstream when available
        });
    }
    Ok(())
}
```

Update tests to assert the multi-objective path is the one being exercised, not the simple one.

### Step 6 — Prune the 12 noisy hooks

Per `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md` § 0.3.3 the following hooks should be deleted:

1. HDC fingerprinting — `roko-learn/src/hdc.rs` (delete or feature-gate)
2. Affect stamping — daimon hook in `EpisodeLogger::stamp_affect()` (remove call site, leave function for backward compat reads of old episodes)
3. Crate familiarity — `crate_familiarity_score` in episode loader (delete if unread)
4. Anomaly detection per-episode — `anomaly_detector.rs` write path (keep read path for historical episodes; do not write new ones)
5. Context attribution — `context_attribution.rs` (delete or feature-gate)
6. Section effectiveness per-episode — keep aggregate only (already in plan)
7. Strategy metadata — `strategy_metadata.rs` (delete)
8. Force-backend override learning — `force_backend_override.rs` (delete)
9. Somatic markers — `somatic_markers.rs` (delete; daimon cleanup in plan 15)
10. Emotional tags — keep in episode struct for compat but stop populating (set `None`)
11. Predictive calibration — `calibration.rs` (delete)
12. Enriched run recording — `enriched_run_recorder.rs` (delete)

For each deletion: search call sites, replace with no-op or remove, delete file. After all 12, re-run benchmarks; you should see noticeably lower episode write latency.

### Step 7 — Two-run proof test

```rust
// crates/roko-learn/tests/two_run_proof.rs
#[tokio::test]
async fn second_run_uses_first_runs_feedback() {
    let temp = tempdir()?;
    let services = ServiceFactory::for_test(temp.path()).await?;

    // Run 1: simple task that fails on the first prompt section
    services.model_caller().call(req_with_section("section_X")).await?;
    services.feedback_sink().record(FeedbackEvent::ModelCall {
        prompt_section_ids: vec!["section_X".into()], success: false, ..base()
    }).await?;
    services.feedback_sink().flush().await?;

    // Run 2: same task, fresh service factory (reload from disk)
    let services2 = ServiceFactory::for_test(temp.path()).await?;
    let prompt = services2.assembler().assemble(spec()).await?;

    // Verify: section_X is dropped or down-weighted in run 2
    assert!(prompt.diagnostics.dropped_sections.iter().any(|s| s.id == "section_X")
        || prompt.diagnostics.effectiveness_baseline["section_X"] < 0.5);
}
```

Three more tests:

- `cascade_router_changes_choice_after_failure`
- `playbook_records_after_successful_task_with_passing_gates`
- `gate_threshold_ema_decreases_after_repeated_failures`

### Step 8 — Delete `FeedbackFacade` and CLI runtime_feedback module

After Steps 1–4, the CLI no longer needs its own facade. Delete:

- `crates/roko-cli/src/runtime_feedback/mod.rs`
- `crates/roko-cli/src/runtime_feedback/*.rs` (all sinks moved in Step 2)
- All `use crate::runtime_feedback::FeedbackSink as CliFeedbackSink` aliases

Replace facade usage in `crates/roko-cli/src/commands/plan.rs` with `ServiceFactory::build(...).feedback_sink()`.

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #3 Build another runtime | Adding chat-specific feedback sinks alongside the canonical ones | All sinks live in `roko-learn/src/sinks/` |
| #6 Feedback afterthought | New caller forgets to attach `FeedbackService` | `ServiceFactory` is the only sanctioned way to construct services |
| #7 Copy-paste | Re-defining `FeedbackEvent` per-crate | Only `roko_core::foundation::FeedbackEvent` |
| #10 God file | Putting all sinks into `feedback_service.rs` | Each sink is its own module under `roko-learn/src/sinks/` |

---

## Things NOT To Do

1. **Don't keep the CLI `FeedbackEvent` alongside the canonical one.** Two `FeedbackEvent`s mean future bugs. Migrate fully or not at all.
2. **Don't make sinks block model calls.** Sinks must be fire-and-forget. Use `tokio::spawn` for slow sinks (knowledge ingestion can take seconds). The trait's `record` should return quickly.
3. **Don't drop `flush()` on shutdown.** `Drop` for `MultiSink` does NOT flush (futures need an executor). Add an explicit `services.shutdown().await` in the CLI / serve shutdown handler.
4. **Don't record one `FeedbackEvent` per chunk** in streaming. Record one `ModelCall` event when the stream completes (with full token counts). Per-chunk events flood disk.
5. **Don't put `Episode` schema in `roko-core`.** It belongs in `roko-learn::episode_logger::Episode`. `roko-core` only defines `EpisodeSummary` for dashboard read paths.
6. **Don't skip the playbook sink.** It's the only thing that closes the "system improves with use" loop. If you skip it, the cascade router learns but the prompts never get better.
7. **Don't re-add the 12 noisy hooks "in case we need them later".** They added 30%+ to episode write latency in benchmarks. Delete fully.
8. **Don't write episodes from inside `ModelCallService::call`.** The service emits a `FeedbackEvent::ModelCall`; the sink writes it. Keep separation of concerns.

---

## Tests / Proof Criteria

```bash
# 1. One canonical FeedbackEvent
rg 'pub enum FeedbackEvent' crates/ --type rust
# expected: exactly 1 result (in roko-core/foundation.rs)

# 2. One canonical FeedbackSink trait
rg 'pub trait FeedbackSink' crates/ --type rust
# expected: exactly 1 result (in roko-core/foundation.rs)

# 3. CLI runtime_feedback module deleted
ls crates/roko-cli/src/runtime_feedback/
# expected: directory does not exist

# 4. observe_multi_objective is the live path
rg 'fn observe_multi_objective' crates/roko-learn/src/ --type rust
rg 'observe_multi_objective\b' crates/ --type rust | grep -v 'crates/roko-learn/src/'
# expected: live caller in feedback_service.rs (not just tests/orchestrate.rs)

# 5. The 12 hooks are gone
for f in hdc anomaly_detector strategy_metadata force_backend_override somatic_markers calibration enriched_run_recorder context_attribution; do
    test -f crates/roko-learn/src/${f}.rs && echo "STILL PRESENT: $f"
done
# expected: nothing printed
```

Functional proofs:

- [ ] `roko "hello"` writes one episode with `caller: "cli"`, `role: "interactive_chat"`
- [ ] `roko run "fix typo"` writes one episode + one efficiency event + updates section-effects.json
- [ ] `roko plan run plans/sample` writes one episode per task and one playbook entry per successful task
- [ ] After 5 sequential `roko run` invocations on the same prompt, `cascade-router.json` has different model choice probabilities than baseline
- [ ] Two-run proof test (Step 7) passes
- [ ] Benchmark: episode write latency < 5ms p99 (was ~15ms before pruning)

---

## Dependencies

- **Plan 01 (ModelCallService)** — for `caller` and `request_id` fields
- **Plan 02 (PromptAssemblyService)** — for `prompt_section_ids` and `knowledge_ids` to flow

Can start in parallel with Plan 02 once `FeedbackEvent` extension is decided.

---

## Estimated Effort

**L.** ~1.5 weeks.

- Step 1 (extend canonical FeedbackEvent) — S (1 day)
- Step 2 (move sinks) — M (2 days)
- Step 3 (new sinks) — M (2 days)
- Step 4 (chat attachment) — S (half day)
- Step 5 (multi-objective wiring) — S (1 day)
- Step 6 (delete 12 hooks) — M (2-3 days; lots of grep + delete + test fix)
- Step 7 (two-run proof) — S (1 day)
- Step 8 (delete CLI facade) — S (1 day)
