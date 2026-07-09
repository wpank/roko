# W11-A: Gate Channel Send Failure + Fatal Event Result Swallowed

**Priority**: P0 -- gate channel failure hangs tasks forever; swallowed Fatal leaves plans stuck
**Effort**: ~30 min
**Files to modify**: 2
**Dependencies**: None

## Problem

Two related bugs in `event_loop.rs` cause tasks or plans to hang indefinitely:

1. **Gate channel send failure**: When a read-only role auto-passes the gate, the completion is sent via `tokio::spawn`. If `gate_tx.send()` fails (buffer full, receiver dropped), the error is logged but the event loop never receives the gate completion. The task hangs forever.

2. **Fatal event result swallowed**: Five locations use `let _ = ctx.executor.apply_event(plan_id, &ExecutorEvent::Fatal(...))`. If the executor rejects the transition (e.g., plan already terminal), the `Err` is silently discarded. The plan never becomes terminal from the event loop's perspective and hangs until the plan timeout fires.

## Exact Code to Change

### File 1: `crates/roko-cli/src/runner/event_loop.rs`

#### Change 1: Add `fatal_tx` field to `RunContext`

**Find this code** (line 88):
```rust
/// Shared context for the dispatch loop, replacing 11 loose parameters.
struct RunContext<'a> {
    executor: &'a mut ParallelExecutor,
    task_index: &'a HashMap<String, HashMap<String, TaskDef>>,
    skip_enrichment: &'a HashMap<String, bool>,
    config: &'a RunConfig,
    tui: &'a TuiBridge,
    state: &'a mut RunState,
    agent_handle: &'a mut Option<AgentHandle>,
    agent_tx: &'a mpsc::Sender<AgentEvent>,
    gate_tx: &'a mpsc::Sender<GateCompletion>,
    paths: &'a PersistPaths,
    merge_queue: &'a MergeQueue,
    snapshot_writer: &'a SnapshotWriter,
    prompt_cache: &'a Arc<PromptCache>,
    factory: &'a SharedAgentFactory,
}
```

**Replace with:**
```rust
/// Shared context for the dispatch loop, replacing 11 loose parameters.
struct RunContext<'a> {
    executor: &'a mut ParallelExecutor,
    task_index: &'a HashMap<String, HashMap<String, TaskDef>>,
    skip_enrichment: &'a HashMap<String, bool>,
    config: &'a RunConfig,
    tui: &'a TuiBridge,
    state: &'a mut RunState,
    agent_handle: &'a mut Option<AgentHandle>,
    agent_tx: &'a mpsc::Sender<AgentEvent>,
    gate_tx: &'a mpsc::Sender<GateCompletion>,
    /// Clone of the agent event sender -- used as a fatal-event fallback
    /// when spawned gate completions fail to send on `gate_tx`.
    fatal_tx: mpsc::Sender<AgentEvent>,
    paths: &'a PersistPaths,
    merge_queue: &'a MergeQueue,
    snapshot_writer: &'a SnapshotWriter,
    prompt_cache: &'a Arc<PromptCache>,
    factory: &'a SharedAgentFactory,
}
```

#### Change 2: Wire `fatal_tx` at RunContext construction site

Search for every place `RunContext {` is constructed in the `run()` function body. There is one construction site inside `dispatch_action`. Find where `gate_tx:` is assigned and add `fatal_tx: ctx.agent_tx.clone(),` immediately after. The construction is inside `dispatch_action` which takes `ctx: &mut RunContext` so the field just needs to exist on the struct. The actual `RunContext` is constructed in `run()` -- search for `let mut ctx = RunContext {` or `RunContext {` in `run()`.

At every `RunContext {` construction, add after `gate_tx`:
```rust
    fatal_tx: agent_tx.clone(),
```

(where `agent_tx` is the local `mpsc::Sender<AgentEvent>` variable in `run()`).

#### Change 3: Fix gate auto-pass send failure with fatal fallback

**Find this code** (line 2548):
```rust
                let gate_tx = ctx.gate_tx.clone();
                tokio::spawn(async move {
                    if let Err(e) = gate_tx.send(completion).await {
                        error!(err = %e, "failed to send auto-pass gate completion");
                    }
                });
```

**Replace with:**
```rust
                let gate_tx = ctx.gate_tx.clone();
                let fatal_tx = ctx.fatal_tx.clone();
                let plan_id_fatal = plan_id.clone();
                tokio::spawn(async move {
                    if let Err(e) = gate_tx.send(completion).await {
                        error!(plan_id = %plan_id_fatal, err = %e,
                            "CRITICAL: failed to send auto-pass gate -- sending fatal");
                        let _ = fatal_tx.send(AgentEvent::Error {
                            message: format!(
                                "gate channel closed for plan {plan_id_fatal}: {e}"
                            ),
                        }).await;
                    }
                });
```

#### Change 4: Fix Fatal result swallowed -- budget exceeded

**Find this code** (line 2051):
```rust
                let _ = ctx.executor.apply_event(
                    plan_id,
                    &ExecutorEvent::Fatal(format!(
                        "budget exceeded: ${plan_spent:.2} >= ${max_plan_usd:.2}"
                    )),
                );
```

**Replace with:**
```rust
                if let Err(e) = ctx.executor.apply_event(
                    plan_id,
                    &ExecutorEvent::Fatal(format!(
                        "budget exceeded: ${plan_spent:.2} >= ${max_plan_usd:.2}"
                    )),
                ) {
                    error!(plan_id = %plan_id, error = %e,
                        "failed to apply Fatal event -- forcing plan terminal");
                    ctx.state.force_plan_terminal(plan_id);
                }
```

#### Change 5: Fix Fatal result swallowed -- task not found

**Find this code** (line 2068):
```rust
                    let _ = ctx.executor.apply_event(
                        plan_id,
                        &ExecutorEvent::Fatal(format!("task {task_id} not found")),
                    );
```

**Replace with:**
```rust
                    if let Err(e) = ctx.executor.apply_event(
                        plan_id,
                        &ExecutorEvent::Fatal(format!("task {task_id} not found")),
                    ) {
                        error!(plan_id = %plan_id, error = %e,
                            "failed to apply Fatal event -- forcing plan terminal");
                        ctx.state.force_plan_terminal(plan_id);
                    }
```

#### Change 6: Fix Fatal result swallowed -- dispatch planning failed

**Find this code** (line 2167):
```rust
                    let _ = ctx
                        .executor
                        .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()));
```

**Replace with:**
```rust
                    if let Err(e) = ctx
                        .executor
                        .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()))
                    {
                        error!(plan_id = %plan_id, error = %e,
                            "failed to apply Fatal event -- forcing plan terminal");
                        ctx.state.force_plan_terminal(plan_id);
                    }
```

#### Change 7: Fix Fatal result swallowed -- model resolution failed

**Find this code** (line 2248):
```rust
                            let _ = ctx
                                .executor
                                .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()));
```

**Replace with:**
```rust
                            if let Err(e) = ctx
                                .executor
                                .apply_event(plan_id, &ExecutorEvent::Fatal(message.clone()))
                            {
                                error!(plan_id = %plan_id, error = %e,
                                    "failed to apply Fatal event -- forcing plan terminal");
                                ctx.state.force_plan_terminal(plan_id);
                            }
```

#### Change 8: Fix Fatal result swallowed -- spawn failed

**Find this code** (line 2389):
```rust
                            let _ = ctx.executor.apply_event(
                                plan_id,
                                &ExecutorEvent::Fatal(format!("spawn failed: {e}")),
                            );
```

**Replace with:**
```rust
                            if let Err(e2) = ctx.executor.apply_event(
                                plan_id,
                                &ExecutorEvent::Fatal(format!("spawn failed: {e}")),
                            ) {
                                error!(plan_id = %plan_id, error = %e2,
                                    "failed to apply Fatal event -- forcing plan terminal");
                                ctx.state.force_plan_terminal(plan_id);
                            }
```

### File 2: `crates/roko-cli/src/runner/state.rs`

#### Change 9: Add `force_plan_terminal` method to `RunState`

**Find this code** (line 453):
```rust
    /// Cost accumulated for a specific plan.
    pub fn plan_cost(&self, plan_id: &str) -> f64 {
```

**Insert BEFORE it:**
```rust
    /// Last-resort escape hatch: mark a plan as terminal in `RunState`
    /// when `executor.apply_event(Fatal)` itself fails (e.g. plan already
    /// terminal or not found). Without this, the event loop hangs forever
    /// waiting for a terminal transition that will never happen.
    pub fn force_plan_terminal(&mut self, plan_id: &str) {
        tracing::warn!(plan_id = %plan_id, "force_plan_terminal: marking plan as dead in RunState");
        self.tasks_failed += 1;
        self.failure_reasons
            .entry(format!("{plan_id}:_forced"))
            .or_insert_with(|| "plan forced terminal after apply_event(Fatal) rejection".into());
    }

```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# 1. Ensure it compiles
cargo check -p roko-cli

# 2. Run existing tests
cargo test -p roko-cli

# 3. Grep to confirm no remaining `let _ = ctx.executor.apply_event(.*Fatal` in dispatch_action
grep -n 'let _ = ctx.executor.apply_event' crates/roko-cli/src/runner/event_loop.rs
# Should return 0 results for Fatal sites (lines 2051, 2068, 2167, 2248, 2389 are all fixed)

# 4. Verify force_plan_terminal exists
grep -n 'force_plan_terminal' crates/roko-cli/src/runner/state.rs
```

## Agent Prompt

```
Fix two related bugs in the roko runner that cause tasks/plans to hang forever.

## Context

The runner event loop is in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`.
RunState is in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/state.rs`.

## Bug 1: Gate channel send failure (line ~2548)

When a read-only role auto-passes the gate, completion is sent via `tokio::spawn` on `gate_tx`.
If `gate_tx.send()` fails, only `error!()` is logged. The task hangs forever because the event
loop never gets the completion.

### Fix

1. Add a `fatal_tx: mpsc::Sender<AgentEvent>` field to the `RunContext` struct (line ~88).
   This is a clone of the existing `agent_tx` sender. It does NOT need a lifetime -- use
   `mpsc::Sender<AgentEvent>` (owned, not `&'a`).

2. Wire `fatal_tx: agent_tx.clone()` at every `RunContext { ... }` construction site in `run()`.

3. At line ~2548, clone both `gate_tx` and `fatal_tx` before the `tokio::spawn`. On send
   failure, send an `AgentEvent::Error { message }` on `fatal_tx` as a fallback so the event
   loop can handle the failure:
   ```rust
   let gate_tx = ctx.gate_tx.clone();
   let fatal_tx = ctx.fatal_tx.clone();
   let plan_id_fatal = plan_id.clone();
   tokio::spawn(async move {
       if let Err(e) = gate_tx.send(completion).await {
           error!(plan_id = %plan_id_fatal, err = %e,
               "CRITICAL: failed to send auto-pass gate -- sending fatal");
           let _ = fatal_tx.send(AgentEvent::Error {
               message: format!("gate channel closed for plan {plan_id_fatal}: {e}"),
           }).await;
       }
   });
   ```

## Bug 2: Fatal event result swallowed (lines ~2051, ~2068, ~2167, ~2248, ~2389)

Five `let _ = ctx.executor.apply_event(plan_id, &ExecutorEvent::Fatal(...))` calls discard
the Result. If the executor rejects the transition, the plan hangs forever.

### Fix

1. Replace each `let _ =` with `if let Err(e) =` and call `ctx.state.force_plan_terminal(plan_id)`.
   The five sites are:
   - Line ~2051: `"budget exceeded: ..."` -- use `if let Err(e)`, variable is `e`
   - Line ~2068: `"task {task_id} not found"` -- use `if let Err(e)`, variable is `e`
   - Line ~2167: `message.clone()` -- use `if let Err(e)`, variable is `e`
   - Line ~2248: `message.clone()` (model resolution failed) -- use `if let Err(e)`, variable is `e`
   - Line ~2389: `"spawn failed: {e}"` -- use `if let Err(e2)`, variable is `e2` (outer `e` already in scope)

2. Add `force_plan_terminal(&mut self, plan_id: &str)` to `RunState` in `state.rs`:
   ```rust
   pub fn force_plan_terminal(&mut self, plan_id: &str) {
       tracing::warn!(plan_id = %plan_id, "force_plan_terminal: marking plan as dead in RunState");
       self.tasks_failed += 1;
       self.failure_reasons
           .entry(format!("{plan_id}:_forced"))
           .or_insert_with(|| "plan forced terminal after apply_event(Fatal) rejection".into());
   }
   ```
   Insert it just before `pub fn plan_cost(...)` (line ~453).

Run `cargo check -p roko-cli` and `cargo test -p roko-cli` to verify.
```

## Commit

This batch is committed with Wave 11. Do not commit individually.

## Checklist

- [ ] `fatal_tx` field added to `RunContext` (owned `mpsc::Sender<AgentEvent>`, not `&'a`)
- [ ] `fatal_tx` wired at all `RunContext` construction sites in `run()`
- [ ] Gate auto-pass spawn sends `AgentEvent::Error` fallback on send failure
- [ ] Five `let _ = ctx.executor.apply_event(Fatal)` replaced with `if let Err` (lines ~2051, ~2068, ~2167, ~2248, ~2389)
- [ ] `force_plan_terminal` added to `RunState` in `state.rs`
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo test -p roko-cli` passes

## Audit Status

Audited: 2026-05-05. PASS no changes needed.
