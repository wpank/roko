# W12-D: Runner Config Fixes -- MCP Wiring, Dream Logic, Permanent Retry, Budget Enforcement, Feedback Cap

**Priority**: P1 (MCP, Permanent retry) / P2 (dream, budget, feedback)
**Effort**: 1-2 hours
**Files to modify**: 3 files
**Dependencies**: W12-B (for agent_handles in budget enforcement)

## Cross-Batch Overlap Warning (W12 event_loop.rs)

All four W12 batches touch `event_loop.rs`. This batch (W12-D) overlaps with W12-B:

- **Line 415 (budget enforcement)**: This batch calls `stop_all_agents(&mut agent_handles, ...)` in its replacement code, but those names only exist if W12-B has been applied. If W12-B is NOT applied, use `stop_active_agent(&mut agent_handle, ...)` instead. The batch documents this with an IMPORTANT note in Change 3.
- **Line 1434/1465 (feedback cap)**: Only this batch touches the `emit_runner_event_with_facades` function. No conflict.
- **Line 3077 (dream consolidation)**: Only this batch touches this function. No conflict.

**Apply order**: Ideally apply W12-B first, then W12-D. If applying independently, use the `stop_active_agent` fallback variant documented in Change 3.

## Problem

Five configuration and control-flow bugs in the runner:

1. **MCP config hardcoded None**: `RunConfig` is constructed with `mcp_config: None` in `cmd_plan`, ignoring any MCP config discovered via the layered config system.

2. **Dream consolidation inverted logic**: The `else` branch of `let Some(roko_config) = ... else { ... }` runs dream consolidation when there is NO config.

3. **Permanent failures classified as retryable**: `is_retryable()` returns `true` for `Permanent`. Tasks that will never succeed are retried.

4. **Per-turn budget is warning-only**: The per-turn budget check logs a warning but does not stop the agent.

5. **Feedback facade spawns unbounded tasks**: Every runner event spawns a new `tokio::spawn` for the feedback facade with no back-pressure.

## Exact Code to Change

### File 1: `crates/roko-cli/src/commands/plan.rs`

#### Change 1: Wire MCP config into RunConfig

**Find this code** (line 460):
```rust
                mcp_config: None,
```

**Replace with:**
```rust
                mcp_config: {
                    // Resolve MCP config: explicit agent.mcp_config > .roko/mcp.json > auto-discovery
                    let mcp = roko_config.agent.mcp_config.as_ref()
                        .map(|p| wd.join(p))
                        .filter(|p| p.exists())
                        .or_else(|| {
                            let roko_local = wd.join(".roko").join("mcp.json");
                            roko_local.is_file().then_some(roko_local)
                        })
                        .or_else(|| {
                            roko_agent::mcp::find_mcp_config(&wd)
                                .and_then(|r| r.ok())
                                .map(|(p, _)| p)
                        });
                    if let Some(ref path) = mcp {
                        tracing::info!(path = ?path, "MCP config resolved for plan run");
                    } else {
                        tracing::debug!("no MCP config found for plan run");
                    }
                    mcp
                },
```

### File 2: `crates/roko-cli/src/runner/event_loop.rs`

#### Change 2: Fix dream consolidation inverted logic

**Find this code** (line 3077):
```rust
async fn run_dream_consolidation_if_enabled(config: &RunConfig) {
    let Some(roko_config) = config.roko_config.as_ref() else {
        debug!("running dream consolidation after plan completion");
        run_dream_consolidation(config).await;
        return;
    };

    if !roko_config.learning.dream_on_completion {
        debug!("dream consolidation after plan completion disabled");
        return;
    }

    debug!("running dream consolidation after plan completion");
    run_dream_consolidation(config).await;
}
```

**Replace with:**
```rust
async fn run_dream_consolidation_if_enabled(config: &RunConfig) {
    let Some(roko_config) = config.roko_config.as_ref() else {
        debug!("no roko config -- skipping dream consolidation");
        return;
    };

    if !roko_config.learning.dream_on_completion {
        debug!("dream consolidation after plan completion disabled");
        return;
    }

    debug!("running dream consolidation after plan completion");
    run_dream_consolidation(config).await;
}
```

#### Change 3: Enforce per-turn budget

**Find this code** (line 415):
```rust
                // Per-turn budget check.
                if is_turn_done {
                    let max_turn = config.max_turn_usd;
                    if max_turn > 0.0 && state.cost_usd > max_turn {
                        warn!(
                            task = %state.current_task,
                            turn_cost = state.cost_usd,
                            limit = max_turn,
                            "single turn exceeded per-turn budget limit"
                        );
                    }
                }
```

**Replace with:**
```rust
                // Per-turn budget enforcement.
                if is_turn_done {
                    let max_turn = config.max_turn_usd;
                    if max_turn > 0.0 && state.cost_usd > max_turn {
                        warn!(
                            task = %state.current_task,
                            turn_cost = state.cost_usd,
                            limit = max_turn,
                            "single turn exceeded per-turn budget limit -- stopping agent"
                        );
                        stop_all_agents(&mut agent_handles, &mut state, Duration::from_secs(3)).await;
                        let plan_id = state.plan_id.clone();
                        if !plan_id.is_empty() {
                            let _ = executor.apply_event(
                                &plan_id,
                                &ExecutorEvent::Fatal(format!(
                                    "turn cost ${:.2} exceeded per-turn limit ${:.2}",
                                    state.cost_usd, max_turn,
                                )),
                            );
                        }
                    }
                }
```

**IMPORTANT**: If W12-B has NOT been applied yet, use `stop_active_agent(&mut agent_handle, ...)` instead
of `stop_all_agents(&mut agent_handles, ...)`. The function names and variable names depend on
whether W12-B's per-plan agent handle refactor has been applied.

If W12-B is not applied:
```rust
                        stop_active_agent(&mut agent_handle, &mut state, Duration::from_secs(3)).await;
```

#### Change 4: Replace unbounded feedback spawns with JoinSet

Add a `JoinSet` declaration before the main `loop {` in `run()`, near the other variable declarations:

```rust
    let mut feedback_tasks: tokio::task::JoinSet<()> = tokio::task::JoinSet::new();
```

Then modify `emit_runner_event_with_facades` to accept and use it.

**Find this code** (line 1434):
```rust
fn emit_runner_event_with_facades(
    paths: &PersistPaths,
    state: &mut RunState,
    tui: &TuiBridge,
    projection: Option<&Arc<super::projection::Projection>>,
    feedback_facade: Option<&Arc<crate::runtime_feedback::FeedbackFacade>>,
    event: RunnerEvent,
) {
```

**Replace with:**
```rust
fn emit_runner_event_with_facades(
    paths: &PersistPaths,
    state: &mut RunState,
    tui: &TuiBridge,
    projection: Option<&Arc<super::projection::Projection>>,
    feedback_facade: Option<&Arc<crate::runtime_feedback::FeedbackFacade>>,
    event: RunnerEvent,
    feedback_tasks: Option<&mut tokio::task::JoinSet<()>>,
) {
```

**Find this code** (line 1465):
```rust
    // ── Translate to FeedbackEvent and fan out (fire-and-forget) ────────
    if let Some(facade) = feedback_facade {
        if let Some(feedback) = runner_event_to_feedback(&event, &state.routing_context) {
            let facade = Arc::clone(facade);
            tokio::spawn(async move {
                if let Err(err) = facade.on_event(&feedback).await {
                    warn!(
                        event_type = feedback.label(),
                        %err,
                        "feedback facade returned terminal error",
                    );
                }
            });
        }
    }
```

**Replace with:**
```rust
    // ── Translate to FeedbackEvent and fan out ──────────────────────────
    if let Some(facade) = feedback_facade {
        if let Some(feedback) = runner_event_to_feedback(&event, &state.routing_context) {
            if let Some(tasks) = feedback_tasks {
                // Reap completed tasks (non-blocking) to prevent unbounded growth.
                while tasks.try_join_next().is_some() {}

                if tasks.len() >= 32 {
                    debug!("feedback task backlog full ({} tasks), dropping event", tasks.len());
                } else {
                    let facade = Arc::clone(facade);
                    tasks.spawn(async move {
                        if let Err(err) = facade.on_event(&feedback).await {
                            warn!(
                                event_type = feedback.label(),
                                %err,
                                "feedback facade returned terminal error",
                            );
                        }
                    });
                }
            } else {
                // Fallback for callers that don't provide a JoinSet.
                let facade = Arc::clone(facade);
                tokio::spawn(async move {
                    if let Err(err) = facade.on_event(&feedback).await {
                        warn!(
                            event_type = feedback.label(),
                            %err,
                            "feedback facade returned terminal error",
                        );
                    }
                });
            }
        }
    }
```

Now update the wrapper functions.

**Find this code** (line 1402):
```rust
fn emit_runner_event(
    paths: &PersistPaths,
    state: &mut RunState,
    tui: &TuiBridge,
    config: &RunConfig,
    event: RunnerEvent,
) {
    emit_runner_event_with_facades(
        paths,
        state,
        tui,
        config.projection.as_ref(),
        config.feedback_facade.as_ref(),
        event,
    );
}
```

This function is called from inside the main loop where `feedback_tasks` is in scope. However,
the simplest approach is to pass `None` from the wrapper and handle the `JoinSet` at direct call
sites in `run()`.

Actually, the cleanest approach: keep `emit_runner_event` as-is but pass `None` for the new parameter:

**Replace the wrapper with:**
```rust
fn emit_runner_event(
    paths: &PersistPaths,
    state: &mut RunState,
    tui: &TuiBridge,
    config: &RunConfig,
    event: RunnerEvent,
) {
    emit_runner_event_with_facades(
        paths,
        state,
        tui,
        config.projection.as_ref(),
        config.feedback_facade.as_ref(),
        event,
        None,
    );
}
```

**And the facadeless wrapper:**
```rust
fn emit_runner_event_facadeless(
    paths: &PersistPaths,
    state: &mut RunState,
    tui: &TuiBridge,
    event: RunnerEvent,
) {
    emit_runner_event_with_facades(paths, state, tui, None, None, event, None);
}
```

This keeps the behavior correct (existing calls use unbounded spawn as fallback), while allowing
the direct call sites in `run()` to be migrated to pass `Some(&mut feedback_tasks)` in a follow-up.

After the main loop exits, drain the JoinSet:
```rust
    // Drain any pending feedback tasks.
    while feedback_tasks.try_join_next().is_some() {}
```

### File 3: `crates/roko-cli/src/runner/types.rs`

#### Change 5: Remove Permanent from is_retryable

**Find this code** (line 45):
```rust
    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::Transient | Self::Permanent | Self::Structural | Self::Unknown
        )
    }
```

**Replace with:**
```rust
    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::Transient | Self::Structural | Self::Unknown
        )
    }
```

#### Change 6: Fix the test that asserts Permanent is retryable

**Find this code** (line 1583):
```rust
        let permanent = RunnerFailureKind::from_output("error[E0308]: mismatched types");
        assert_eq!(permanent, RunnerFailureKind::Permanent);
        assert!(permanent.is_retryable());
```

**Replace with:**
```rust
        let permanent = RunnerFailureKind::from_output("error[E0308]: mismatched types");
        assert_eq!(permanent, RunnerFailureKind::Permanent);
        assert!(!permanent.is_retryable());
```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Build check:
cargo check -p roko-cli 2>&1 | head -30

# Run the is_retryable test:
cargo test -p roko-cli -- failure_kind 2>&1

# Verify dream logic is correct:
grep -A5 'run_dream_consolidation_if_enabled' crates/roko-cli/src/runner/event_loop.rs | head -10

# Verify MCP config is wired:
grep -A10 'mcp_config' crates/roko-cli/src/commands/plan.rs | head -15

# Verify Permanent is not retryable:
grep -A3 'is_retryable' crates/roko-cli/src/runner/types.rs | head -6
```

## Agent Prompt

```
Fix five runner bugs in the roko plan executor.

## Context

Read the full batch spec at `/Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W12-D-runner-config-fixes.md` for exact find/replace blocks.

Files to modify:
- `crates/roko-cli/src/commands/plan.rs` -- MCP config wiring
- `crates/roko-cli/src/runner/event_loop.rs` -- dream logic, budget enforcement, feedback cap
- `crates/roko-cli/src/runner/types.rs` -- is_retryable fix + test

## Summary

### 1. MCP config (plan.rs line ~460)
Replace `mcp_config: None` with resolution logic that checks:
- `roko_config.agent.mcp_config` (explicit path from config)
- `.roko/mcp.json` (conventional location)
- `roko_agent::mcp::find_mcp_config(&wd)` (auto-discovery)

### 2. Dream consolidation (event_loop.rs line ~3077)
The `else` branch (no config) RUNS consolidation. It should SKIP it. Change the `else` branch to
`debug!("no roko config -- skipping dream consolidation"); return;`.

### 3. Permanent retry (types.rs line ~45)
Remove `Self::Permanent` from the `is_retryable()` match. Update the test at line ~1585 to
assert `!permanent.is_retryable()`.

### 4. Budget enforcement (event_loop.rs line ~415)
After the per-turn budget warning, call `stop_active_agent` (or `stop_all_agents` if W12-B
applied) and apply `ExecutorEvent::Fatal` to enforce the limit.

### 5. Feedback cap (event_loop.rs)
Add `feedback_tasks: Option<&mut tokio::task::JoinSet<()>>` parameter to
`emit_runner_event_with_facades`. Use the JoinSet when provided, with a cap of 32 tasks and
`try_join_next()` for non-blocking reap. Pass `None` from the wrapper functions for now.
Drain after the main loop.

Run `cargo check -p roko-cli` and `cargo test -p roko-cli -- failure_kind` to verify.
```

## Commit

This batch is committed with all Wave 12 batches together. Do not commit individually.

## Checklist

- [ ] MCP config resolved from `roko_config.agent.mcp_config` / `.roko/mcp.json` / auto-discovery
- [ ] `mcp_config: None` replaced with resolved value
- [ ] `tracing::info!` for resolved path, `tracing::debug!` for no config
- [ ] Dream consolidation `else` branch returns early (no config = skip)
- [ ] `Permanent` removed from `is_retryable()` match
- [ ] Test updated to assert `!permanent.is_retryable()`
- [ ] Per-turn budget enforcement: agent stopped and Fatal applied when exceeded
- [ ] `feedback_tasks` parameter added to `emit_runner_event_with_facades`
- [ ] JoinSet used for feedback dispatch with 32-task cap
- [ ] Wrapper functions pass `None` for backward compatibility
- [ ] JoinSet drained after main loop
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo test -p roko-cli -- failure_kind` passes

## Audit Status

Audited: 2026-05-05. 1 issue fixed: added cross-batch overlap warning header documenting dependency on W12-B for budget enforcement variable names. All code snippets verified against source -- exact matches confirmed. Line numbers accurate. `roko_agent::mcp::find_mcp_config` path verified (re-exported from `mcp/mod.rs` line 21). `wd` and `roko_config` confirmed in scope at plan.rs line 460. `executor` confirmed accessible in Branch 1 select! scope at line 415. All callers of `emit_runner_event_with_facades` covered (2 callers: lines 1409, 1430). serve_runtime.rs already wires MCP config from `cli_config` -- correctly not modified by this batch.
