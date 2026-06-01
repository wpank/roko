# W12-A: Gate Semaphore -- Global Singleton to Per-Run

**Priority**: P1 -- correctness (multi-run concurrency broken)
**Effort**: 30 minutes
**Files to modify**: 5 files
**Dependencies**: None

## Cross-Batch Overlap Warning (W12 event_loop.rs)

All four W12 batches touch `event_loop.rs`. This batch (W12-A) touches:
- **Line 95**: RunContext struct -- adds `gate_sem` field (W12-B also modifies this struct, changing `agent_handle` to `agent_handles` on the same line. Non-conflicting: different fields.)
- **Line 981**: RunContext construction -- adds `gate_sem: gate_sem.clone()` (W12-B also modifies this block, changing `agent_handle` to `agent_handles`. Non-conflicting: different lines within the block.)
- **Line 2558**: `spawn_gate` call -- adds `gate_sem` arg (no other batch touches this)
- **Line 2627**: `spawn_plan_verify` call -- adds `gate_sem` arg (no other batch touches this)

**Apply order**: W12-A can be applied independently of the other W12 batches. When applying alongside W12-B, the RunContext struct and construction site will both have changes from both batches -- they are additive and non-conflicting.

## Problem

`gate_dispatch.rs` uses a `OnceLock<Arc<Semaphore>>` initialized with `Semaphore::new(1)` -- a process-level singleton. All gate evaluations across all plans and all `roko plan run` invocations in the same process funnel through a single permit. This makes `max_concurrent_tasks` effectively inert for gate parallelism. If two runs share a process (e.g. `roko serve` triggering plans), one starves the other.

## Exact Code to Change

### File 1: `crates/roko-cli/src/runner/gate_dispatch.rs`

#### Change 1: Remove OnceLock import, global singleton, and helper function

**Find this code** (line 5):
```rust
use std::sync::{Arc, OnceLock};
```

**Replace with:**
```rust
use std::sync::Arc;
```

**Find this code** (line 20):
```rust
static GATE_SEMAPHORE: OnceLock<Arc<Semaphore>> = OnceLock::new();

fn gate_semaphore() -> Arc<Semaphore> {
    GATE_SEMAPHORE
        .get_or_init(|| Arc::new(Semaphore::new(1)))
        .clone()
}
```

**Replace with:**
```rust
// Gate semaphore is now passed per-run via the `gate_sem` parameter.
// This allows each `roko plan run` invocation to have its own concurrency
// limit instead of sharing a global singleton.
```

#### Change 2: Add `gate_sem` parameter to `spawn_gate`

**Find this code** (line 29):
```rust
pub fn spawn_gate(
    plan_id: String,
    task_id: String,
    rung: u32,
    workdir: PathBuf,
    verify_steps: Vec<VerifyStep>,
    timeout_secs: u64,
    gate_tx: mpsc::Sender<GateCompletion>,
) {
    tokio::spawn(async move {
        let t_wait = Instant::now();
        let Ok(_permit) = gate_semaphore().acquire_owned().await else {
            return;
        };
```

**Replace with:**
```rust
pub fn spawn_gate(
    plan_id: String,
    task_id: String,
    rung: u32,
    workdir: PathBuf,
    verify_steps: Vec<VerifyStep>,
    timeout_secs: u64,
    gate_tx: mpsc::Sender<GateCompletion>,
    gate_sem: Arc<Semaphore>,
) {
    tokio::spawn(async move {
        let t_wait = Instant::now();
        let Ok(_permit) = gate_sem.acquire_owned().await else {
            return;
        };
```

#### Change 3: Add `gate_sem` parameter to `spawn_plan_verify`

**Find this code** (line 131-132):
```rust
/// Spawn plan-level verify steps as a background task.
pub fn spawn_plan_verify(
    plan_id: String,
    workdir: PathBuf,
    verify_steps: Vec<(String, Vec<VerifyStep>)>,
    timeout_secs: u64,
    gate_tx: mpsc::Sender<GateCompletion>,
) {
    tokio::spawn(async move {
        let t_wait = Instant::now();
        let Ok(_permit) = gate_semaphore().acquire_owned().await else {
            return;
        };
```

**Replace with:**
```rust
/// Spawn plan-level verify steps as a background task.
pub fn spawn_plan_verify(
    plan_id: String,
    workdir: PathBuf,
    verify_steps: Vec<(String, Vec<VerifyStep>)>,
    timeout_secs: u64,
    gate_tx: mpsc::Sender<GateCompletion>,
    gate_sem: Arc<Semaphore>,
) {
    tokio::spawn(async move {
        let t_wait = Instant::now();
        let Ok(_permit) = gate_sem.acquire_owned().await else {
            return;
        };
```

### File 2: `crates/roko-cli/src/runner/types.rs`

#### Change 4: Add `gate_concurrency` to `RunConfig`

**Find this code** (line 1249):
```rust
    /// Maximum number of tasks that may execute concurrently within a plan.
    pub max_concurrent_tasks: usize,
```

**Replace with:**
```rust
    /// Maximum number of tasks that may execute concurrently within a plan.
    pub max_concurrent_tasks: usize,
    /// Maximum number of gate evaluations that may run concurrently within a
    /// single run. Defaults to `max_concurrent_tasks` so gates can keep up
    /// with parallel task dispatch.
    pub gate_concurrency: usize,
```

#### Change 5: Initialize `gate_concurrency` in `from_roko_config`

**Find this code** (line 1347, inside `Self { ... }` in `from_roko_config`):
```rust
            max_concurrent_tasks,
            approval: false,
```

**Replace with:**
```rust
            max_concurrent_tasks,
            gate_concurrency: max_concurrent_tasks,
            approval: false,
```

#### Change 6: Initialize `gate_concurrency` in `Default` impl

**Find this code** (line 1395, inside `Default` impl):
```rust
            max_concurrent_tasks: 4,
            approval: false,
```

**Replace with:**
```rust
            max_concurrent_tasks: 4,
            gate_concurrency: 4,
            approval: false,
```

### File 3: `crates/roko-cli/src/commands/plan.rs`

#### Change 7: Initialize `gate_concurrency` in `cmd_plan` RunConfig construction

**Find this code** (line 456):
```rust
                max_concurrent_tasks,
                approval,
```

**Replace with:**
```rust
                max_concurrent_tasks,
                gate_concurrency: max_concurrent_tasks,
                approval,
```

### File 4: `crates/roko-cli/src/serve_runtime.rs`

#### Change 8: Add `gate_concurrency` to `serve_runtime.rs` RunConfig construction

There is a second `RunConfig` construction site in `serve_runtime.rs` (the `roko serve` code path).
Without this change, the code will not compile because the struct literal would be missing a field.

**Find this code** (line 553):
```rust
        max_concurrent_tasks,
        approval: false,
```

**Replace with:**
```rust
        max_concurrent_tasks,
        gate_concurrency: max_concurrent_tasks,
        approval: false,
```

### File 5: `crates/roko-cli/src/runner/event_loop.rs`

#### Change 9: Add `gate_sem` field to `RunContext` struct

**Find this code** (line 95, inside `RunContext` struct):
```rust
    agent_handle: &'a mut Option<AgentHandle>,
    agent_tx: &'a mpsc::Sender<AgentEvent>,
```

**Replace with:**
```rust
    agent_handle: &'a mut Option<AgentHandle>,
    gate_sem: Arc<tokio::sync::Semaphore>,
    agent_tx: &'a mpsc::Sender<AgentEvent>,
```

Note: `Arc` is already in scope from `use std::sync::Arc;` at line 7.

#### Change 10: Create per-run gate semaphore in `run()`

After the `let mut config = config.clone();` line (line 124), add:

```rust
    let gate_sem = Arc::new(tokio::sync::Semaphore::new(config.gate_concurrency.max(1)));
    tracing::debug!(permits = config.gate_concurrency.max(1), "created per-run gate semaphore");
```

#### Change 11: Wire `gate_sem` into RunContext construction

**Find this code** (line 981, inside the `RunContext` construction in the main loop):
```rust
                        agent_handle: &mut agent_handle,
                        agent_tx: &agent_tx,
```

**Replace with:**
```rust
                        agent_handle: &mut agent_handle,
                        gate_sem: gate_sem.clone(),
                        agent_tx: &agent_tx,
```

#### Change 12: Pass `gate_sem` to `spawn_gate` call

**Find this code** (line 2558):
```rust
                gate_dispatch::spawn_gate(
                    plan_id.clone(),
                    task_id,
                    *rung,
                    ctx.config.workdir.clone(),
                    verify_steps,
                    ctx.config.timeout_secs,
                    ctx.gate_tx.clone(),
                );
```

**Replace with:**
```rust
                gate_dispatch::spawn_gate(
                    plan_id.clone(),
                    task_id,
                    *rung,
                    ctx.config.workdir.clone(),
                    verify_steps,
                    ctx.config.timeout_secs,
                    ctx.gate_tx.clone(),
                    ctx.gate_sem.clone(),
                );
```

#### Change 13: Pass `gate_sem` to `spawn_plan_verify` call

**Find this code** (line 2627):
```rust
            gate_dispatch::spawn_plan_verify(
                plan_id.clone(),
                ctx.config.workdir.clone(),
                verify_steps,
                ctx.config.timeout_secs,
                ctx.gate_tx.clone(),
            );
```

**Replace with:**
```rust
            gate_dispatch::spawn_plan_verify(
                plan_id.clone(),
                ctx.config.workdir.clone(),
                verify_steps,
                ctx.config.timeout_secs,
                ctx.gate_tx.clone(),
                ctx.gate_sem.clone(),
            );
```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Build check:
cargo check -p roko-cli 2>&1 | head -30

# Verify no remaining references to old global:
grep -rn 'GATE_SEMAPHORE\|gate_semaphore()' crates/roko-cli/src/runner/

# Verify gate_sem is used:
grep -n 'gate_sem' crates/roko-cli/src/runner/event_loop.rs
grep -n 'gate_sem' crates/roko-cli/src/runner/gate_dispatch.rs
```

## Agent Prompt

```
Replace the global gate semaphore singleton with a per-run semaphore in the roko runner.

## Context

Files to modify:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/gate_dispatch.rs` -- has global
  `static GATE_SEMAPHORE: OnceLock<Arc<Semaphore>>` at line 20 and `fn gate_semaphore()` at line 22
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs` -- `RunConfig` struct at line 1234
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/plan.rs` -- `RunConfig` construction at line ~448
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/serve_runtime.rs` -- second `RunConfig` construction at line ~545
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` -- `run()` function, `RunContext`, and call sites

## Changes

### 1. gate_dispatch.rs
- Change `use std::sync::{Arc, OnceLock};` to `use std::sync::Arc;`
- Remove `static GATE_SEMAPHORE` and `fn gate_semaphore()` entirely (lines 20-26)
- Add `gate_sem: Arc<Semaphore>` as last parameter to `spawn_gate` (line 29) and
  `spawn_plan_verify` (line 132)
- Replace `gate_semaphore().acquire_owned()` with `gate_sem.acquire_owned()` in both functions

### 2. types.rs
- Add `pub gate_concurrency: usize` field to `RunConfig` after `max_concurrent_tasks` (line 1250)
- Initialize to `max_concurrent_tasks` in `from_roko_config` (line ~1347)
- Initialize to `4` in the `Default` impl (line ~1395)

### 3. commands/plan.rs
- Add `gate_concurrency: max_concurrent_tasks,` after `max_concurrent_tasks,` in the RunConfig
  construction (line ~456)

### 4. serve_runtime.rs
- Add `gate_concurrency: max_concurrent_tasks,` after `max_concurrent_tasks,` at line ~553
  (second `RunConfig` construction site used by `roko serve`)

### 5. event_loop.rs
- Add `gate_sem: Arc<tokio::sync::Semaphore>` field to `RunContext` struct (line ~95)
- Create `let gate_sem = Arc::new(tokio::sync::Semaphore::new(config.gate_concurrency.max(1)));`
  in `run()` after line ~124
- Wire `gate_sem: gate_sem.clone()` at the `RunContext` construction site (line ~981)
- Pass `ctx.gate_sem.clone()` as last arg to `gate_dispatch::spawn_gate` (line ~2558)
- Pass `ctx.gate_sem.clone()` as last arg to `gate_dispatch::spawn_plan_verify` (line ~2627)

Run `cargo check -p roko-cli` to verify.
```

## Commit

This batch is committed with all Wave 12 batches together. Do not commit individually.

## Checklist

- [ ] `OnceLock` import removed from `gate_dispatch.rs`
- [ ] `static GATE_SEMAPHORE` and `fn gate_semaphore()` removed
- [ ] `spawn_gate` accepts `gate_sem: Arc<Semaphore>` parameter
- [ ] `spawn_plan_verify` accepts `gate_sem: Arc<Semaphore>` parameter
- [ ] `gate_concurrency` field added to `RunConfig`
- [ ] `gate_concurrency` initialized in `from_roko_config`
- [ ] `gate_concurrency` initialized in `Default` impl
- [ ] `gate_concurrency` set in `cmd_plan` RunConfig construction
- [ ] `gate_concurrency` set in `serve_runtime.rs` RunConfig construction
- [ ] `gate_sem` field added to `RunContext`
- [ ] Per-run semaphore created in `event_loop::run()`
- [ ] `gate_sem` wired at `RunContext` construction
- [ ] `spawn_gate` call passes `ctx.gate_sem.clone()`
- [ ] `spawn_plan_verify` call passes `ctx.gate_sem.clone()`
- [ ] No remaining references to `GATE_SEMAPHORE` or `gate_semaphore()`
- [ ] `cargo check -p roko-cli` passes

## Audit Status

Audited: 2026-05-05. 1 issue fixed: added cross-batch overlap warning header documenting interactions with W12-B on RunContext struct/construction. All code snippets verified against source -- exact matches confirmed. Line numbers accurate. All construction sites covered. No compilation issues expected in isolation.
