# W10-B: Episode Data Wiring

**Priority**: P1 -- episodes and gate verdicts show all zeros, breaking learning feedback loop
**Effort**: 2-3 hours
**Files to modify**: 1-2
**Dependencies**: None

## Problem

Two related data pipeline bugs cause the learning subsystem to receive empty data:

1. **14.12 / 14.27**: `runner_event_to_feedback()` constructs `AgentOutcome` with hardcoded zeros for `tokens_in`, `tokens_out`, `cost_usd`, and `duration_ms`. The comment says "Per-attempt usage is not stored on RunnerEvent". But the data IS available in `RunState` -- it just is not threaded through. The efficiency pipeline (`efficiency.jsonl`) gets real data from `RunState` directly, so efficiency data is correct while episodes show zeros.

2. **14.13**: Gate verdicts are stored in-memory and in `gate-thresholds.json`, but never written as `Kind::GateVerdict` engrams to the substrate. When `cmd_status` queries `substrate.query(Query::of_kind(Kind::GateVerdict))`, it finds nothing and reports 0/0 gates.

## Root Cause

### 14.12 / 14.27
The `RunnerEvent::TaskAttemptCompleted` variant does not carry per-task token/cost data. The `runner_event_to_feedback` function receives only the `RunnerEvent` and a `RoutingContext`, but the real token/cost data lives in `RunState` which is not passed to the function.

### 14.13
The runner v2 event loop handles `GateCompletion` events to update in-memory state and write `gate-thresholds.json`, but there is no code path that writes a `Kind::GateVerdict` engram to the substrate. The substrate write was in the orchestrate.rs v1 path but was never ported to the runner v2 event loop.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

#### Change 1 (14.12): Add TaskUsageSnapshot struct

Add this struct definition near line 1484, just before the `runner_event_to_feedback` function.

**Find this code** (lines 1482-1492):
```rust
/// Translate a [`RunnerEvent`] into a [`FeedbackEvent`] when the runner
/// has enough information for one. Returns `None` for variants that do
/// not map to the feedback layer (e.g. `RunStarted`, `ResumeMarker`).
///
/// `routing_ctx` is the dispatch-time routing context stored on
/// [`RunState`] — threaded here so `TaskCompleted` events carry the
/// real feature vector for the CascadeRouter's bandit.
fn runner_event_to_feedback(
    event: &RunnerEvent,
    routing_ctx: &Option<roko_learn::model_router::RoutingContext>,
) -> Option<crate::runtime_feedback::FeedbackEvent> {
```

**Replace with:**
```rust
/// Per-task usage data extracted from `RunState` at event emission time.
/// Passed to `runner_event_to_feedback` to populate `AgentOutcome` with
/// real values instead of zeros.
#[derive(Debug, Clone, Default)]
struct TaskUsageSnapshot {
    tokens_in: u64,
    tokens_out: u64,
    cost_usd: f64,
    duration_ms: u64,
}

/// Translate a [`RunnerEvent`] into a [`FeedbackEvent`] when the runner
/// has enough information for one. Returns `None` for variants that do
/// not map to the feedback layer (e.g. `RunStarted`, `ResumeMarker`).
///
/// `routing_ctx` is the dispatch-time routing context stored on
/// [`RunState`] -- threaded here so `TaskCompleted` events carry the
/// real feature vector for the CascadeRouter's bandit.
fn runner_event_to_feedback(
    event: &RunnerEvent,
    routing_ctx: &Option<roko_learn::model_router::RoutingContext>,
    usage: &TaskUsageSnapshot,
) -> Option<crate::runtime_feedback::FeedbackEvent> {
```

#### Change 2 (14.12): Populate AgentOutcome from usage snapshot instead of zeros

**Find this code** (lines 1504-1520):
```rust
            let succeeded = matches!(outcome, TaskAttemptOutcome::Passed);
            // Per-attempt usage is not stored on `RunnerEvent`; tokens /
            // cost stay at zero here — attribution uses `model` /
            // `provider` from dispatch (`RunState` at completion).
            let agent_outcome = AgentOutcome {
                task_id: attempt.task_id.clone(),
                plan_id: attempt.plan_id.clone(),
                model: model.clone(),
                provider: provider.clone(),
                output: String::new(),
                tokens_in: 0,
                tokens_out: 0,
                cost_usd: 0.0,
                duration_ms: 0,
                exit_code: None,
                is_error: !succeeded,
            };
```

**Replace with:**
```rust
            let succeeded = matches!(outcome, TaskAttemptOutcome::Passed);
            tracing::debug!(
                task_id = %attempt.task_id,
                tokens_in = usage.tokens_in,
                tokens_out = usage.tokens_out,
                cost_usd = usage.cost_usd,
                duration_ms = usage.duration_ms,
                "runner_event_to_feedback: populating AgentOutcome from RunState usage"
            );
            let agent_outcome = AgentOutcome {
                task_id: attempt.task_id.clone(),
                plan_id: attempt.plan_id.clone(),
                model: model.clone(),
                provider: provider.clone(),
                output: String::new(),
                tokens_in: usage.tokens_in,
                tokens_out: usage.tokens_out,
                cost_usd: usage.cost_usd,
                duration_ms: usage.duration_ms,
                exit_code: None,
                is_error: !succeeded,
            };
```

#### Change 3 (14.12): Update the call site to pass RunState data

**Find this code** (lines 1466-1467):
```rust
    if let Some(facade) = feedback_facade {
        if let Some(feedback) = runner_event_to_feedback(&event, &state.routing_context) {
```

**Replace with:**
```rust
    if let Some(facade) = feedback_facade {
        let usage = TaskUsageSnapshot {
            tokens_in: state.tokens_in,
            tokens_out: state.tokens_out,
            cost_usd: state.cost_usd,
            duration_ms: state.task_elapsed_ms(),
        };
        if let Some(feedback) = runner_event_to_feedback(&event, &state.routing_context, &usage) {
```

Note on RunState fields: `state.tokens_in` (u64), `state.tokens_out` (u64), `state.cost_usd` (f64) are public fields defined at lines 41-49 of `state.rs`. `state.task_elapsed_ms()` is a public method at line 501 of `state.rs` returning `u64`.

#### Change 4 (14.13): Write gate verdicts to signals.jsonl

After the `update_gate_thresholds` call in the gate completion handler (Branch 2), add a signals.jsonl write.

**Find this code** (lines 646-652):
```rust
                update_gate_thresholds(
                    &mut gate_thresholds,
                    &paths.gate_thresholds_json,
                    completion.rung,
                    completion.passed,
                );
                emit_gate_thresholds_event(&gate_thresholds, &tui);
```

**Replace with:**
```rust
                update_gate_thresholds(
                    &mut gate_thresholds,
                    &paths.gate_thresholds_json,
                    completion.rung,
                    completion.passed,
                );
                emit_gate_thresholds_event(&gate_thresholds, &tui);

                // Write gate verdict to signals.jsonl for `roko status` queries.
                // No substrate handle in the event loop -- append directly.
                {
                    let verdict_json = serde_json::json!({
                        "kind": "GateVerdict",
                        "plan_id": completion.plan_id,
                        "task_id": completion.task_id,
                        "rung": completion.rung,
                        "passed": completion.passed,
                        "gate_kind": format!("{:?}", completion.kind),
                        "duration_ms": completion.duration_ms,
                        "timestamp": chrono::Utc::now().to_rfc3339(),
                    });
                    let signals_path = config.workdir.join(".roko/signals.jsonl");
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&signals_path)
                    {
                        use std::io::Write;
                        let _ = writeln!(f, "{}", verdict_json);
                        tracing::debug!(
                            plan_id = %completion.plan_id,
                            task_id = %completion.task_id,
                            rung = completion.rung,
                            passed = completion.passed,
                            "wrote gate verdict to signals.jsonl"
                        );
                    }
                }
```

Note: `chrono` is already a dependency of roko-cli (check `Cargo.toml`). If it is not imported in this file, add `use chrono::Utc;` to the imports, or use `chrono::Utc::now()` fully qualified. The `serde_json` crate is already used throughout the event loop. If `std::io::Write` conflicts with an existing import, scope it inside the block as shown above.

Also verify that `config.workdir` is accessible -- `config` is `&RunConfig` which has a `pub workdir: PathBuf` field. If `config.workdir` is not available, use `paths.workdir` or extract the workdir from the `paths` struct.

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Build check
cargo check -p roko-cli 2>&1 | tail -5

# Verify no hardcoded zeros remain in the feedback translation
grep -n 'tokens_in: 0' crates/roko-cli/src/runner/event_loop.rs
grep -n 'cost_usd: 0.0' crates/roko-cli/src/runner/event_loop.rs
grep -n 'duration_ms: 0,' crates/roko-cli/src/runner/event_loop.rs
# None of these should match inside runner_event_to_feedback

# Verify TaskUsageSnapshot struct exists
grep -n 'TaskUsageSnapshot' crates/roko-cli/src/runner/event_loop.rs
# Should show struct definition and usage at both definition and call site

# Verify gate verdict write exists
grep -n 'GateVerdict' crates/roko-cli/src/runner/event_loop.rs
# Should show the signals.jsonl write
```

## Agent Prompt

```
You are fixing two data pipeline bugs in the runner v2 event loop. This is a Rust project at /Users/will/dev/nunchi/roko/roko. The event loop is the core plan executor.

IMPORTANT: Read the source files FIRST before making changes. The batch file has exact find/replace pairs but line numbers may drift if other changes have been applied.

### Bug 1 (14.12/14.27): runner_event_to_feedback() hardcodes zeros for tokens/cost/duration

The function at line ~1489 of event_loop.rs constructs AgentOutcome with `tokens_in: 0`, `tokens_out: 0`, `cost_usd: 0.0`, `duration_ms: 0`. The real data IS available on the `state: RunState` at the call site (line ~1467).

Fix steps:
1. Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs lines 1460-1530
2. Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/state.rs lines 18-55 and 498-503

Apply these changes:
a) Add a `TaskUsageSnapshot` struct (tokens_in: u64, tokens_out: u64, cost_usd: f64, duration_ms: u64) just before the function definition.
b) Add `usage: &TaskUsageSnapshot` parameter to the function signature.
c) Replace the hardcoded zeros in AgentOutcome with `usage.tokens_in`, `usage.tokens_out`, `usage.cost_usd`, `usage.duration_ms`.
d) At the call site (line ~1467), construct a TaskUsageSnapshot from `state.tokens_in`, `state.tokens_out`, `state.cost_usd`, `state.task_elapsed_ms()` and pass it as the third argument.

### Bug 2 (14.13): Gate verdicts never written to substrate

The event loop processes GateCompletion events and writes gate-thresholds.json, but never writes GateVerdict records to signals.jsonl. The `roko status` command queries for GateVerdict records and finds nothing.

Fix steps:
1. Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs lines 646-655
2. After the `update_gate_thresholds` + `emit_gate_thresholds_event` calls, add a block that appends a JSON line to `.roko/signals.jsonl`.
3. There is NO substrate handle in the event loop -- use direct file append with `std::fs::OpenOptions`.
4. Include: kind, plan_id, task_id, rung, passed, gate_kind, duration_ms, timestamp.

After all changes, run:
```bash
cargo check -p roko-cli 2>&1 | tail -20
```
Then run the verification grep commands.
```

## Commit

This batch is committed with Wave 10. Do not commit individually.

## Checklist

- [ ] `TaskUsageSnapshot` struct defined with `tokens_in`, `tokens_out`, `cost_usd`, `duration_ms`
- [ ] `runner_event_to_feedback` signature updated to accept `&TaskUsageSnapshot`
- [ ] `AgentOutcome` populated from `usage` fields instead of zeros
- [ ] Call site populates `TaskUsageSnapshot` from `state.tokens_in`, etc.
- [ ] Gate verdict written to `.roko/signals.jsonl` after each GateCompletion
- [ ] `tracing::debug!` added at key instrumentation points
- [ ] `cargo check -p roko-cli` passes

## Audit Status

Audited: 2026-05-05. PASS no changes needed
