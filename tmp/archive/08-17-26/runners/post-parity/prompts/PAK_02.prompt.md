# PAK_02: Add unit tests for event_loop core dispatch mechanics

## Task
Add tests for `event_loop.rs` covering event dispatch, gate completion handling, and state persistence — currently 3,136 lines with zero tests.

## Runner Context
Runner PAK (Testing Gaps), batch 2 of 3. No dependencies.

## Problem
`event_loop.rs` (3,136 lines) is the heartbeat of the entire orchestration system. It has ZERO tests. Bugs here silently break the plan-execute-gate-persist loop that powers all of roko's self-hosting.

## Current Code

**event_loop.rs** — `crates/roko-cli/src/runner/event_loop.rs`:

Key public items:
- `pub async fn run()` (line 107) — main event loop orchestrator
- `pub struct RunReport` (line 58) — execution results summary
- `pub struct PlanReport` (line 72) — per-plan report
- `ExecutorAction::MergeBranch` handler (lines 2242-2282)
- Gate completion handler (around lines 1115-1122)
- State checkpoint/persistence logic

The event loop drives a `tokio::select!` over:
- Agent completion events
- Gate completion events
- Executor tick (next action)
- Periodic flush/checkpoint
- Cancellation signals

## Exact Changes

### Step 1: Add test module at bottom of event_loop.rs

```rust
#[cfg(test)]
mod tests {
    use super::*;
```

### Step 2: Test RunReport construction

```rust
    #[test]
    fn run_report_default_is_empty() {
        let report = RunReport::default();
        assert!(report.plans.is_empty());
        assert_eq!(report.total_tasks, 0);
        assert_eq!(report.passed_tasks, 0);
    }

    #[test]
    fn plan_report_tracks_task_counts() {
        let mut report = PlanReport::default();
        report.total_tasks = 5;
        report.passed_tasks = 3;
        report.failed_tasks = 2;
        assert_eq!(report.total_tasks, report.passed_tasks + report.failed_tasks);
    }
```

### Step 3: Test executor action dispatch (if testable without full runtime)

Read the actual code to determine which helper functions can be unit-tested in isolation. Common patterns:

```rust
    #[test]
    fn merge_branch_action_requires_branch_name() {
        // Test that ExecutorAction::MergeBranch carries required fields
        // Adapt to actual enum variant structure
    }
```

### Step 4: Test state checkpoint serialization

If the checkpoint/persistence logic uses a serializable struct:

```rust
    #[test]
    fn run_state_roundtrips_through_json() {
        let state = RunState {
            // ... fill with test data based on actual struct fields
        };
        let json = serde_json::to_string(&state).unwrap();
        let loaded: RunState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.plan_count(), state.plan_count());
    }
```

### Step 5: Adapt to actual types

Before writing tests, read the actual struct definitions:
- `RunReport` at line 58 — get exact fields
- `PlanReport` at line 72 — get exact fields
- Any `RunState` or checkpoint struct
- `ExecutorAction` enum variants

Write tests that exercise the real types. Focus on:
1. Data structure construction and field access
2. Serialization round-trips (if types are Serialize/Deserialize)
3. Helper functions that don't need a full tokio runtime
4. Edge cases: empty plans, zero tasks, cancelled runs

For async functions that need a runtime, use `#[tokio::test]`.

## Write Scope
- `crates/roko-cli/src/runner/event_loop.rs` (add `#[cfg(test)] mod tests`)

## Read-Only Context
- `crates/roko-cli/src/runner/event_loop.rs:58-107` (RunReport, PlanReport, run())
- `crates/roko-cli/src/runner/event_loop.rs:1115-1122` (gate completion handler)
- `crates/roko-cli/src/runner/event_loop.rs:2242-2282` (merge handler)

## Verify
```bash
cargo test -p roko-cli -- event_loop 2>&1 | tail -30
```

## Acceptance Criteria
- At least 4 test functions covering: report construction, state serialization, edge cases
- All tests pass with `cargo test -p roko-cli`
- Tests adapted to actual struct fields (not fabricated)
- No tests that require a running server or real agent process

## Do NOT
- Change the event_loop implementation
- Add integration tests that spawn real agents
- Mock the entire tokio runtime (use `#[tokio::test]` where needed)
- Add tests for functions that are purely internal to the select! loop (test via helpers instead)
