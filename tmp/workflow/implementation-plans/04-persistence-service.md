# 04 — PersistenceService: Crash-Safe Unified Run State

> Foundation Phase 0.4 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Cross-references audit `tmp/workflow/16-http-serve-persistence-audit.md`.

---

## Status (2026-05-01)

**NOT IMPLEMENTED as a unified service.** Three separate snapshot schemas; `WorkflowEngine` does not auto-checkpoint; runner persistence works only for the legacy `event_loop` plan runner.

**What's done:**

- `roko_runtime::run_ledger::RunLedger` — in-memory typed ledger, recording phase transitions / agent / gate / commit outcomes — `crates/roko-runtime/src/run_ledger.rs`
- `EffectDriver::save_checkpoint(state, path)` — atomic tmp-then-rename write, emits `RuntimeEvent::StateCheckpointed` — `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-cli/src/runner/persist.rs` — `save_run_state` / `atomic_write` / `RunStateSnapshot { schema_version, fingerprints, completed_tasks, cascade_router_json, ... }`
- `crates/roko-cli/src/runner/resume.rs` — strict resume: schema check, fingerprint validation, `prepare_resume`, `recover_jsonl` (truncate partial trailing line)
- `crates/roko-orchestrator/src/executor/snapshot.rs` — `ExecutorSnapshot { plan_states, queue_order, speculative_executions, ... }` (legacy)
- `crates/roko-orchestrator/src/runtime_snapshot.rs` — `OrchestratorSnapshot` wrapping executor + merge queue + worktrees + event log
- Tests: `jsonl_recovery_truncates_partial_trailing_line`, `jsonl_recovery_drops_invalid_tail_line`, `test_checkpoint_resume_round_trip`

**What's not:**

- `RunLedger` is in-memory only. There is no `RunLedger::persist(path)`.
- `WorkflowEngine::run_with_cancel` does **not** call `save_checkpoint`. So workflow runs that crash mid-pipeline lose state.
- Three snapshot schemas (`RunStateSnapshot` for runner, `ExecutorSnapshot` for orchestrator, `OrchestratorSnapshot` wrapping it) coexist. They are not interchangeable.
- `CascadeRouter` state and `AdaptiveThresholds` state are saved separately by their owning services; there is no transactional multi-file write.
- No "crash at every phase" test matrix as called out in the unified plan.
- `.roko/episodes.jsonl` and `.roko/learn/episodes.jsonl` duplicate (audit doc 16, Section 7).

---

## Goal

A single `PersistenceService` that:

1. Owns the canonical `RunLedger` schema on disk
2. Provides atomic snapshot + recovery for all live entry points (chat, `roko run`, `roko plan run`, ACP, HTTP plan execution)
3. Validates resume strictly (fingerprint, schema version, plan presence)
4. Persists transactionally — when CascadeRouter and gate thresholds and section effects change in one workflow, all three end up consistent (or all roll back)
5. Replaces `RunStateSnapshot`, `ExecutorSnapshot`, and `OrchestratorSnapshot` over time
6. Has tests for crash at every documented phase (active agent, post-agent/pre-gate, in-gate, post-gate/pre-snapshot)

---

## Why This Exists (Anti-Patterns Eliminated)

- **#3 Build Another Runtime** — three snapshot schemas
- **#7 Copy-Paste Between Runtimes** — runner persist + orchestrator snapshot + ledger duplicating concerns
- **#10 God file** — `runner/persist.rs` mixes JSONL recovery + snapshot writing + fingerprint validation in one file

The audit doc 16 also identifies "no transactional multi-file writes" — this plan provides one.

---

## Existing Code — Read These First

Read in this order:

1. `crates/roko-runtime/src/run_ledger.rs` — current `RunLedger` shape and what it tracks
2. `crates/roko-runtime/src/effect_driver.rs` (search for `save_checkpoint`) — the existing atomic write
3. `crates/roko-cli/src/runner/persist.rs` — `RunStateSnapshot`, `save_run_state`, `recover_jsonl`
4. `crates/roko-cli/src/runner/resume.rs` — `prepare_resume`, fingerprint matching
5. `crates/roko-orchestrator/src/executor/snapshot.rs` — `ExecutorSnapshot` (legacy reference; do not replicate)
6. `crates/roko-fs/src/atomic.rs` — atomic write helpers (use these; do not roll your own)

---

## Implementation Steps

### Step 1 — Define the `PersistenceService` trait + canonical schema

```rust
// crates/roko-runtime/src/persistence.rs
#[async_trait]
pub trait PersistenceService: Send + Sync {
    /// Atomically write the current run state. Safe to call frequently.
    async fn checkpoint(&self, snapshot: &RunStateV2) -> Result<()>;

    /// Load and validate the most recent checkpoint. Returns None if no checkpoint.
    async fn load_checkpoint(&self, run_id: &str) -> Result<Option<RunStateV2>>;

    /// Recover any append-only logs (truncate partial last lines).
    async fn recover_logs(&self) -> Result<RecoveryReport>;

    /// Atomically commit a multi-file change set.
    async fn transactional_write(&self, batch: WriteBatch) -> Result<()>;

    /// Validate a snapshot can resume: returns the strict reasons it cannot.
    fn validate_resume(&self, snapshot: &RunStateV2, current_plan: &Plan) -> ResumeValidation;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStateV2 {
    pub schema_version: u32,             // bump when fields change
    pub run_id: String,
    pub started_at_ms: u64,
    pub last_checkpoint_ms: u64,

    pub workflow: WorkflowConfig,
    pub plan_id: Option<String>,
    pub phase: Phase,
    pub iteration: u32,

    // Per-task state for plan runs
    pub completed_tasks: Vec<TaskCompleted>,
    pub in_flight_tasks: Vec<TaskInFlight>,
    pub failed_tasks: Vec<TaskFailed>,
    pub skipped_tasks: Vec<String>,

    // Per-task fingerprints for strict resume
    pub task_fingerprints: HashMap<String, String>,

    // Counters
    pub agent_turns: u32,
    pub gate_runs: u32,
    pub total_cost_usd: f64,
    pub total_tokens: u64,

    // Embedded state of dependent services (transactional checkpoint of the whole run)
    pub cascade_router_state: serde_json::Value,
    pub adaptive_thresholds_state: serde_json::Value,
    pub section_effects_state: Option<serde_json::Value>,

    // Pending merge queue state (when applicable)
    pub merge_queue: Option<MergeQueueState>,
}

pub struct WriteBatch {
    pub run_state: Option<RunStateV2>,
    pub jsonl_appends: Vec<(PathBuf, Vec<String>)>,    // (path, lines)
    pub atomic_files: Vec<(PathBuf, Vec<u8>)>,         // (path, contents)
}
```

The schema is intentionally a **superset** of `RunStateSnapshot` and `ExecutorSnapshot` so we can migrate incrementally.

### Step 2 — Implement `FsPersistenceService`

```rust
// crates/roko-runtime/src/persistence_fs.rs
pub struct FsPersistenceService {
    root: PathBuf,                                    // .roko/
    layout: RokoLayout,                               // helper for paths
    fingerprinter: Arc<dyn TaskFingerprinter>,        // hashes task definitions
    write_lock: Arc<Mutex<()>>,                       // serialize transactional writes
}

#[async_trait]
impl PersistenceService for FsPersistenceService {
    async fn checkpoint(&self, snapshot: &RunStateV2) -> Result<()> {
        let path = self.layout.run_state_path(&snapshot.run_id);
        let json = serde_json::to_vec_pretty(snapshot)?;
        roko_fs::atomic::write(&path, &json).await?;
        Ok(())
    }

    async fn transactional_write(&self, batch: WriteBatch) -> Result<()> {
        let _guard = self.write_lock.lock().await;
        // Phase 1: write all atomic files to .tmp
        let mut tmp_files = Vec::new();
        for (path, bytes) in &batch.atomic_files {
            let tmp = path.with_extension("tmp.txn");
            tokio::fs::write(&tmp, bytes).await?;
            tmp_files.push((tmp, path.clone()));
        }
        // Phase 2: append JSONL with size markers (so we can truncate on failure)
        let mut jsonl_offsets = Vec::new();
        for (path, lines) in &batch.jsonl_appends {
            let pre_len = file_size(path).await?;
            append_lines(path, lines).await?;
            jsonl_offsets.push((path.clone(), pre_len));
        }
        // Phase 3: write run state last (commit point)
        if let Some(state) = &batch.run_state {
            self.checkpoint(state).await?;
        }
        // Phase 4: rename all .tmp to final
        for (tmp, final_path) in &tmp_files {
            tokio::fs::rename(tmp, final_path).await?;
        }
        Ok(())
        // If any phase fails, callers must call recover_logs() on next start to truncate JSONL appends.
    }

    fn validate_resume(&self, snapshot: &RunStateV2, plan: &Plan) -> ResumeValidation {
        if snapshot.schema_version != CURRENT_SCHEMA {
            return ResumeValidation::Reject("schema mismatch".into());
        }
        if let Some(plan_id) = &snapshot.plan_id {
            if plan_id != &plan.id {
                return ResumeValidation::Reject("plan_id mismatch".into());
            }
            for task in &plan.tasks {
                let now_fp = self.fingerprinter.fingerprint(task);
                if let Some(then_fp) = snapshot.task_fingerprints.get(&task.id) {
                    if &now_fp != then_fp {
                        return ResumeValidation::Reject(format!(
                            "task {} definition changed since checkpoint", task.id));
                    }
                }
            }
        }
        ResumeValidation::Ok
    }
    /* ...recover_logs, load_checkpoint... */
}
```

`atomic::write` already exists in `crates/roko-fs/src/atomic.rs` — use it.

### Step 3 — Wire `PersistenceService` into `WorkflowEngine`

**File:** `crates/roko-runtime/src/workflow_engine.rs`

Add to `EffectServices`:

```rust
pub struct EffectServices {
    pub default_model: String,
    pub model_caller: Arc<dyn ModelCaller>,
    pub prompt_assembler: Arc<dyn PromptAssembler>,
    pub feedback_sink: Arc<dyn FeedbackSink>,
    pub gate_runner: Arc<dyn GateRunner>,
    pub affect_policy: Option<Arc<dyn AffectPolicy>>,
    pub persistence: Arc<dyn PersistenceService>,    // NEW
}
```

In `run_with_cancel`, call `persistence.checkpoint(&snapshot).await` after each phase transition:

```rust
loop {
    if token.is_cancelled() { /* ... */ }
    let action = pipeline.step(input);
    let outcome = driver.execute(action).await;
    input = outcome.into_input();
    ledger.record(&outcome);

    // NEW: transactional checkpoint
    let snapshot = build_run_state_v2(&pipeline, &ledger);
    persistence.checkpoint(&snapshot).await?;

    if matches!(input, PipelineInput::WorkflowComplete | PipelineInput::Halted) {
        break;
    }
}
```

Add a `[runtime].checkpoint_interval_ms` config (default 0 = checkpoint every phase). For high-throughput runs, raise to 5_000ms to batch.

### Step 4 — Delete `RunStateSnapshot` overlap

`crates/roko-cli/src/runner/persist.rs` defines `RunStateSnapshot` with overlapping fields. Migrate the runner to use `RunStateV2`:

1. Add a conversion `From<RunStateSnapshot> for RunStateV2` for legacy on-disk files
2. Update `save_run_state` to write `RunStateV2`
3. Update `runner::resume::prepare_resume` to use `PersistenceService::load_checkpoint`
4. Once all callers migrated, delete `RunStateSnapshot` struct (keep one release with `#[deprecated]` for safety)

### Step 5 — Resolve duplicate `episodes.jsonl`

Audit doc 16 § 7: episodes are written to BOTH `.roko/episodes.jsonl` AND `.roko/learn/episodes.jsonl`. This is wasteful and inconsistent.

Decide:

- Canonical location: `.roko/episodes.jsonl` (matches `roko_fs::layout::RokoLayout::episodes_path`)
- `.roko/learn/episodes.jsonl` is a legacy alias

Implementation:

1. `EpisodeLogger` writes ONLY to `.roko/episodes.jsonl`
2. Migration: on `roko serve` startup, if `.roko/learn/episodes.jsonl` exists and `.roko/episodes.jsonl` does not, move it
3. Delete read paths that target `.roko/learn/episodes.jsonl`
4. Same treatment for `.roko/learn/knowledge-seeds.jsonl` vs `.roko/memory/knowledge-seeds.jsonl`

### Step 6 — Crash-recovery test matrix

Add a test that uses `tokio::test` with deterministic kill points:

```rust
// crates/roko-runtime/tests/crash_recovery_matrix.rs
#[derive(Debug, Clone, Copy)]
enum CrashPoint {
    DuringStrategist,
    PostStrategistPreImplementer,
    DuringImplementer,
    PostImplementerPreGate,
    DuringGate,
    PostGatePreSnapshot,
    DuringCommit,
    PostCommit,
}

async fn run_with_crash(crash_at: CrashPoint, scenario: TestScenario) -> RecoveryOutcome {
    let temp = tempdir()?;
    let services = test_services(temp.path()).with_kill_point(crash_at);
    let engine = WorkflowEngine::new(services);
    let _ = engine.run(test_config()).await;     // expected to panic at crash_at
    let services2 = test_services(temp.path());
    let engine2 = WorkflowEngine::new(services2);
    engine2.resume(test_config(), &load_checkpoint_path(temp.path())).await
}

#[tokio::test]
async fn crash_recovery_full_matrix() {
    for crash_at in [/* all variants */] {
        let outcome = run_with_crash(crash_at, simple_express_run()).await?;
        assert_eq!(outcome.duplicate_completions, 0);
        assert!(outcome.replayed_phases <= 2);   // re-do at most one phase
    }
}
```

Use `unsafe { std::process::abort() }` or a panic injection point inside `EffectDriver` to deterministically crash at each point. Mark these tests `#[ignore]` from default `cargo test` and run via `cargo test -- --ignored crash_recovery`.

### Step 7 — Add transactional CascadeRouter + thresholds save

Today CascadeRouter saves on every observation. After this plan:

- `CascadeRouter::save_to(path)` becomes `CascadeRouter::serialize() -> serde_json::Value`
- `AdaptiveThresholds::save_to(path)` becomes `AdaptiveThresholds::serialize() -> serde_json::Value`
- `WorkflowEngine` checkpoint embeds both as `cascade_router_state` and `adaptive_thresholds_state` in `RunStateV2`
- On startup, `ServiceFactory::build` reads them out of the most-recent checkpoint and seeds the live instances

This guarantees router + thresholds + run state are consistent (single transaction). Audit doc 16 § 8 explicitly calls this out as a gap.

### Step 8 — Long-poll TTL invalidation for projections (audit doc 16 § 8)

Outside the strict scope of `PersistenceService`, but related:

- `RuntimeProjectionSet` has `InvalidationPolicy.max_age_secs` defined but no TTL check
- Add `ProjectionEnvelope::is_stale(now)` and update `RuntimeProjectionSet::load(name)` to return `Stale(envelope)` when expired

This goes into `crates/roko-runtime/src/projection.rs`. See plan 10 for the projection layer details.

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #3 Build another runtime | Adding a new snapshot schema instead of consolidating | One `RunStateV2` |
| #7 Copy-paste | Re-defining JSONL recovery in multiple crates | One implementation in `roko-fs::atomic` + `PersistenceService` |
| #10 God file | Putting all of persistence into one 1000-line file | Split: `persistence.rs` (trait), `persistence_fs.rs` (impl), `recovery.rs` (JSONL utilities), `migrations.rs` (legacy schema reads) |

---

## Things NOT To Do

1. **Don't write to multiple files without `transactional_write`.** A crash between two related writes is the common cause of "router got smarter but the run state says we never ran the task" bugs.
2. **Don't use file locks across processes.** Multiple `roko` processes can run concurrently against the same `.roko/`. Atomic rename + JSONL append is enough; advisory locks add fragility.
3. **Don't skip schema versioning.** `RunStateV2.schema_version` MUST be checked on every read. If it doesn't match, `validate_resume` returns `Reject` — never silently coerce.
4. **Don't store secrets in checkpoints.** `cascade_router_state` is fine (model names + counts); `system_prompt` text is NOT (may contain user data). Audit serialization paths.
5. **Don't keep `OrchestratorSnapshot`.** It's a wrapper that adds nothing once `RunStateV2` includes merge queue + worktrees fields. Delete after migration.
6. **Don't make checkpoints synchronous from the model call hot path.** `EffectDriver` already runs `execute(action)` async; `persistence.checkpoint` is also async. Do NOT call it inside the model call's tight loop.
7. **Don't checkpoint mid-tool-call.** The `ToolLoop` (in `roko-agent/src/tool_loop/`) is not resume-safe (per audit doc 17). Either make it resume-safe (large effort) or document that resume happens only at phase boundaries.
8. **Don't trust `mtime` for staleness.** Use the `last_checkpoint_ms` field stored inside the snapshot — filesystem clocks vary.

---

## Tests / Proof Criteria

```bash
# 1. One canonical RunState struct
rg 'pub struct RunStateV2|pub struct RunStateSnapshot|pub struct ExecutorSnapshot' crates/ --type rust
# expected: only RunStateV2 (others #[deprecated] for one release)

# 2. PersistenceService trait exists
rg 'pub trait PersistenceService' crates/ --type rust
# expected: 1 result in crates/roko-runtime/src/persistence.rs

# 3. WorkflowEngine wires persistence
rg 'persistence\.checkpoint' crates/roko-runtime/src/workflow_engine.rs
# expected: at least 1 call site

# 4. Episodes only in canonical location
rg 'episodes\.jsonl' crates/ --type rust | grep -v '\.roko/episodes\.jsonl' | grep -v test
# expected: 0 results
```

Functional proofs:

- [ ] `roko run "fix typo"` writes `.roko/state/run-{id}.json` and resumes correctly after `kill -9` mid-run
- [ ] `roko plan run plans/sample` survives `kill -9` mid-task and resumes without re-running completed tasks
- [ ] Crash recovery matrix test (Step 6) passes for all 8 crash points
- [ ] After `kill -9` mid-CascadeRouter-save, the router state at startup is consistent with the run state
- [ ] Two concurrent `roko serve` processes against same `.roko/` do not corrupt each other's writes
- [ ] Schema version mismatch triggers a clear error, not silent corruption
- [ ] `.roko/episodes.jsonl` and `.roko/learn/episodes.jsonl` no longer both exist after migration

---

## Dependencies

- **Plan 01 (ModelCallService)** — for `request_id` to appear in episodes
- **Plan 03 (FeedbackService)** — needs the `transactional_write` API for atomic threshold + router + episode commits

Can start in parallel with Plans 01-03; integrates after Plan 07 (EffectDriver completion).

---

## Estimated Effort

**L–XL.** ~2 weeks.

- Step 1 (schema + trait) — S (1 day)
- Step 2 (FsPersistenceService) — M (3 days, JSONL + atomic + transactional logic + tests)
- Step 3 (WorkflowEngine wiring) — S (1 day)
- Step 4 (delete RunStateSnapshot) — M (2 days, lots of caller updates)
- Step 5 (episodes consolidation) — S (1 day)
- Step 6 (crash matrix) — M (3-4 days, need deterministic injection)
- Step 7 (transactional router + thresholds) — M (2 days)
- Step 8 (projection TTL) — S (1 day)
