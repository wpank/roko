# Runner 04 — Persistence Service

> **Give this entire file to a fresh agent.**

---

## Context

Codebase: `/Users/will/dev/nunchi/roko/roko` (Rust workspace). Goal: unified crash-safe persistence so `WorkflowEngine` can checkpoint + resume, replacing three separate snapshot schemas.

**Read first:**

1. `tmp/workflow/implementation-plans/04-persistence-service.md`
2. `crates/roko-runtime/src/run_ledger.rs` — current in-memory ledger
3. `crates/roko-runtime/src/effect_driver.rs` — `save_checkpoint`
4. `crates/roko-runtime/src/workflow_engine.rs` — note: does NOT call `save_checkpoint` in production loop
5. `crates/roko-cli/src/runner/persist.rs` — `RunStateSnapshot`, `save_run_state`, `recover_jsonl`
6. `crates/roko-cli/src/runner/resume.rs` — `prepare_resume`, fingerprint validation
7. `crates/roko-fs/src/atomic.rs` — atomic write helpers (USE these)

---

## Work Items

### Step 1: Define trait + schema

Create `crates/roko-runtime/src/persistence.rs`:

```rust
#[async_trait]
pub trait PersistenceService: Send + Sync {
    async fn checkpoint(&self, snapshot: &RunStateV2) -> Result<()>;
    async fn load_checkpoint(&self, run_id: &str) -> Result<Option<RunStateV2>>;
    async fn recover_logs(&self) -> Result<RecoveryReport>;
    async fn transactional_write(&self, batch: WriteBatch) -> Result<()>;
    fn validate_resume(&self, snapshot: &RunStateV2, plan: &PlanMeta) -> ResumeValidation;
}
```

Define `RunStateV2` with: `schema_version`, `run_id`, `started_at_ms`, `last_checkpoint_ms`, `workflow`, `plan_id`, `phase`, `iteration`, `completed_tasks`, `in_flight_tasks`, `failed_tasks`, `skipped_tasks`, `task_fingerprints`, `agent_turns`, `gate_runs`, `total_cost_usd`, `total_tokens`, `cascade_router_state: serde_json::Value`, `adaptive_thresholds_state: serde_json::Value`.

Define `WriteBatch` with: `run_state: Option<RunStateV2>`, `jsonl_appends: Vec<(PathBuf, Vec<String>)>`, `atomic_files: Vec<(PathBuf, Vec<u8>)>`.

### Step 2: Implement `FsPersistenceService`

Create `crates/roko-runtime/src/persistence_fs.rs`. Use `roko_fs::atomic::write` for checkpoint. For `transactional_write`: write tmp files → append JSONL → write run state (commit point) → rename tmps.

### Step 3: Wire into `WorkflowEngine`

1. Add `persistence: Arc<dyn PersistenceService>` to `EffectServices`
2. In `WorkflowEngine::run_with_cancel`, after each `pipeline.step(input)`, call `persistence.checkpoint(&snapshot)`
3. Implement `WorkflowEngine::resume(config, checkpoint_json)` that loads and validates

### Step 4: Migrate runner callers

Convert `RunStateSnapshot` users in `runner/persist.rs` to `RunStateV2`. Add `From<RunStateSnapshot>` for legacy file reads.

### Step 5: Fix duplicate episodes.jsonl

Canonical: `.roko/episodes.jsonl`. Delete all reads/writes to `.roko/learn/episodes.jsonl`. Add startup migration.

### Step 6: Crash-recovery tests

Create `crates/roko-runtime/tests/crash_recovery_matrix.rs` with 8 crash points (DuringStrategist, PostStrategist, DuringImplementer, PostImplementer, DuringGate, PostGate, DuringCommit, PostCommit). Each: inject panic → load checkpoint → resume → verify no duplicate completions.

---

## Verification

```bash
rg 'pub trait PersistenceService' crates/roko-runtime/src/ --type rust
# returns 1

rg 'persistence\.checkpoint' crates/roko-runtime/src/workflow_engine.rs
# returns 1+

rg 'learn/episodes.jsonl' crates/ --type rust | grep -v test
# returns 0

cargo test --workspace
```
