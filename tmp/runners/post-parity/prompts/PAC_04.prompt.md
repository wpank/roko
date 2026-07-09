# PAC_04: Wire task-level recovery actions from contract violations

## Task
Extend recovery actions from per-tool-call to per-task scope, so repeated contract violations trigger task-level recovery (abort, replan, or escalate).

## Runner Context
Runner PAC (Safety Completeness), batch 4 of 4. Depends on PAC_03.

## Problem
ISS-6 safety gap: `RecoveryAction` (contract.rs:225-234) is per-tool-call only. If a task accumulates multiple warnings or minor violations across many tool calls, there's no task-level escalation. Additionally, there are TWO disjoint `RecoveryAction` types (`roko-agent::safety::contract::RecoveryAction` and `roko-core::extension::RecoveryAction`), neither fully wired for task-level recovery.

## Current Code

**Agent RecoveryAction** — `crates/roko-agent/src/safety/contract.rs:225-234`:
```rust
pub fn applicable_recovery(&self, result: &ToolResult) -> Option<RecoveryAction>
```
Per-tool-call scope. Returns `Retry`, `Skip`, `Abort`, or `Escalate`.

**Core RecoveryAction** — `crates/roko-core/src/extension.rs:211`:
Separate enum with `Propagate` variant. Used in orchestrate.rs extension chain. Per-task scope but only logs.

## Exact Changes

### Step 1: Add task-level violation accumulator

```rust
// In event_loop.rs or a shared task context:
struct TaskViolationTracker {
    violations: Vec<PostCheckViolation>,
    warning_count: u32,
    max_warnings_before_abort: u32,  // default: 10
}

impl TaskViolationTracker {
    fn new() -> Self {
        Self { violations: Vec::new(), warning_count: 0, max_warnings_before_abort: 10 }
    }

    fn record(&mut self, violation: PostCheckViolation) -> TaskRecovery {
        self.violations.push(violation);
        self.warning_count += 1;
        if self.warning_count >= self.max_warnings_before_abort {
            TaskRecovery::Abort
        } else {
            TaskRecovery::Continue
        }
    }
}

enum TaskRecovery {
    Continue,
    Abort,
}
```

### Step 2: Wire into the event loop's task dispatch

```rust
// Create tracker per task:
let mut violation_tracker = TaskViolationTracker::new();

// After each tool call's post-check (from PAC_03):
if let PostCheckResult::Violations(vs) = post_result {
    for v in vs {
        match violation_tracker.record(v) {
            TaskRecovery::Continue => {} // log already happened in PAC_03
            TaskRecovery::Abort => {
                warn!(
                    task = %task.id,
                    total_violations = violation_tracker.warning_count,
                    "task aborted due to repeated contract violations"
                );
                // Kill the agent and mark task as failed
                agent_handle.kill(Duration::from_secs(5)).await;
                break;
            }
        }
    }
}
```

### Step 3: Report violations in task outcome

When recording the task result:
```rust
if !violation_tracker.violations.is_empty() {
    debug!(
        task = %task.id,
        violations = violation_tracker.violations.len(),
        "task completed with {} contract violations",
        violation_tracker.violations.len()
    );
}
```

## Write Scope
- `crates/roko-cli/src/runner/event_loop.rs` (TaskViolationTracker, wire into task dispatch)

## Read-Only Context
- `crates/roko-agent/src/safety/contract.rs` (RecoveryAction, PostCheckResult from PAC_03)
- `crates/roko-core/src/extension.rs` (RecoveryAction — different type, don't unify yet)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Task-level violation tracker counts warnings per task
- 10+ violations in a single task → task abort
- Agent killed gracefully on abort (5s grace)
- Violation count included in task outcome debug log
- Config override for max_warnings_before_abort

## Do NOT
- Unify the two RecoveryAction types (they serve different scopes)
- Add automatic replan on abort (that's a separate concern)
- Change per-tool-call recovery behavior
