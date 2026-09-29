# Flow Lifecycle

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 7.
> Documents Flow states, transitions, and the `GuaranteedFinallyController`.

---

## Overview

A flow is a single execution of a graph -- one plan run, one workflow
invocation, or one authored graph execution. The flow lifecycle defines the
states a flow passes through, the events that drive transitions, and the
guarantees that hold regardless of outcome (success, failure, panic, or
cancellation).

The lifecycle is managed by the controller *outside* the graph engine.
The engine executes cells; the controller manages the surrounding state
machine: resource acquisition, lifecycle events, cleanup, and terminal
receipts.

**Source:** `crates/roko-graph/src/finally.rs`, `crates/roko-graph/src/engine.rs`

---

## Flow States

```
Created --> Running --> Completed
              |
              +--> Failed
              |
              +--> Cancelled
              |
              +--> Paused --> Running (resume)
```

Every state transition publishes a lifecycle event through the observation
bundle. These events are the sole source of execution observability for
dashboards and the TUI.

### State definitions

| State | Description | Terminal |
|---|---|---|
| `Created` | Flow has been constructed but not started | No |
| `Running` | Actively executing graph nodes | No |
| `Paused` | Execution suspended; state preserved on disk | No |
| `Completed` | All nodes finished successfully | Yes |
| `Failed` | One or more nodes failed terminally | Yes |
| `Cancelled` | Operator or timeout cancelled execution | Yes |

Terminal states are absorbing -- once reached, no further transitions occur.

### Transition events

| From | Event | To | Notes |
|---|---|---|---|
| Created | Start | Running | Resource acquisition begins |
| Running | AllNodesTerminal | Completed | Happy path |
| Running | UnrecoverableFailure | Failed | Failure strategy exhausted |
| Running | CancelRequested | Cancelled | CancellationToken triggered |
| Running | PauseRequested | Paused | Operator intervention |
| Paused | ResumeRequested | Running | Snapshot restore + continue |
| Running | Panic | Failed | Panic caught by finally controller |

---

## GuaranteedFinallyController

The `GuaranteedFinallyController` wraps graph execution with an absolute
guarantee that cleanup runs even on failure, panic, or cancellation. It is
NOT a graph node -- it is a controller hook that runs outside the DAG.

**Source:** `crates/roko-graph/src/finally.rs`

### Terminal outcomes

```rust
pub enum TerminalOutcome {
    Success,     // all tasks completed and passed gates
    Failure,     // one or more tasks failed or gates rejected
    Cancelled,   // execution was cancelled by operator or timeout
}
```

### TerminalReceipt

```rust
pub struct TerminalReceipt {
    pub run_id: String,
    pub plan_id: String,
    pub outcome: TerminalOutcome,
    pub started_at_ms: u64,
    pub finished_at_ms: u64,
    pub total_nodes: usize,
    pub completed_nodes: usize,
    pub failed_nodes: usize,
    pub skipped_nodes: usize,
    pub budget_spent_micro_usd: u64,
}
```

Exactly one `TerminalReceipt` is emitted per execution. This is the
controller's primary output; downstream consumers use it to drive
delivery, notification, and reporting.

### Guarantees

Regardless of how execution ends:

1. **Exactly one terminal receipt.** The controller emits precisely one
   `TerminalReceipt`. No double-emission, no missing receipt.

2. **Workspace lease release.** All workspace leases acquired during
   execution are released via the workspace provider.

3. **Agent process shutdown.** All tracked agent processes are stopped
   through the `ProcessSupervisor`. The shutdown sequence is: cancel
   token, SIGTERM, 10-second grace period, SIGKILL.

4. **Final snapshot flush.** The last snapshot is written to disk so the
   run can be inspected or resumed. The atomic write-fsync-rename pattern
   ensures no partial writes.

### FinallyGuard

The controller uses an explicit `FinallyGuard` that tracks whether cleanup
has been performed. If the guard is dropped without explicit cleanup (due
to a panic), it logs a diagnostic warning. The actual cleanup must be
called by the async controller since `Drop` cannot run async code.

```rust
struct FinallyGuard {
    cleaned_up: bool,
    run_id: String,
}

impl Drop for FinallyGuard {
    fn drop(&mut self) {
        if !self.cleaned_up {
            tracing::error!(
                run_id = %self.run_id,
                "FinallyGuard dropped without cleanup"
            );
        }
    }
}
```

### Execution flow

```
1. Create FinallyGuard (cleaned_up = false)
2. Start graph execution
3. Wait for completion, failure, or cancellation
4. Run cleanup:
   a. Determine terminal outcome
   b. Release workspace leases
   c. Stop agent processes
   d. Flush snapshot
   e. Emit TerminalReceipt
   f. Set cleaned_up = true
5. Drop FinallyGuard (no-op because cleaned_up = true)
```

If step 2 panics, the guard's `Drop` fires at step 5 with
`cleaned_up = false`, producing the diagnostic warning. The caller's
panic handler can then perform synchronous cleanup.

---

## Plan Phase Sequence

For plans executing through the full pipeline, the controller drives a
higher-level phase sequence on top of the graph execution:

```
Queued -> Enriching -> Implementing -> Gating -> Verifying
       -> Reviewing -> DocRevision -> Merging -> Complete
```

### Retry loops

The phase sequence includes bounded retry loops:

- **Auto-fix loop:** `Gating -> AutoFixing -> Gating` (up to 5 iterations).
  When a gate fails, an auto-fixer agent attempts to fix the issues. After
  5 failed cycles, the plan transitions to `Failed(AutoFixExhausted)`.

- **Verify regeneration:** `Verifying -> RegeneratingVerify -> Verifying`.
  When verification fails, a regeneration agent rewrites the verification
  code.

- **Review rejection:** `Reviewing -> Implementing`. When the auditor
  rejects the implementation, it returns to the implementation phase
  with the review feedback attached.

### Terminal states

| State | Cause |
|---|---|
| `Complete` | Plan merged successfully |
| `Failed(AutoFixExhausted)` | 5 gate-fix cycles without passing |
| `Failed(Deadlock)` | 3 merge attempts without success |
| `Failed(Other(reason))` | Arbitrary failure |
| `Skipped` | Operator skip |

---

## Pause and Resume

Execution can be paused and resumed without losing state:

### Pause

1. The cancellation token is triggered for the current wave.
2. Running nodes complete or are interrupted (depending on cell
   implementation -- most cells check the token at natural breakpoints).
3. The snapshot is flushed to disk with the write-fsync-rename pattern.
4. The flow transitions to `Paused`.

### Resume

1. The snapshot is loaded from disk.
2. The graph fingerprint is validated against the current graph definition.
   A mismatch (e.g., `tasks.toml` changed during pause) rejects the resume.
3. Node statuses are restored:
   - `Complete` nodes are skipped (outputs loaded from activity recording).
   - `Running` nodes are handled by `ReconcileAction` (default: reset
     to `Pending`).
   - `Pending` nodes execute normally.
4. Budget state is restored (spent + reserved microdollars).
5. The `ActivityReplayer` loads recorded outputs for completed Activity
   nodes.
6. Execution continues from the first non-complete node in topological
   order.

---

## Lifecycle Events

Every state transition emits a lifecycle event through the observation
bundle:

| Transition | Event |
|---|---|
| `Created -> Running` | `flow.started` |
| `Running -> Paused` | `flow.paused` |
| `Paused -> Running` | `flow.resumed` |
| `Running -> Completed` | `flow.completed` |
| `Running -> Failed` | `flow.failed` |
| `Running -> Cancelled` | `flow.cancelled` |

These events feed into the TUI dashboard, the HTTP control plane's SSE
endpoint, and the telemetry Lens runtime. They are also the source data
for the `roko plan status` command.

---

## Scope Boundary

The `GuaranteedFinallyController` owns the finally-guarantee lifecycle.
It does NOT own:

- Graph construction (see `topology.rs`, `convert.rs`)
- Graph execution (see `engine.rs`)
- Approval/control commands (see `control.rs`)
- Delivery state machine (see `delivery.rs`)
- Plan phase transitions (see `plan_phase.rs`)

---

## Verification Commands

```bash
# Start a plan flow
cargo run -p roko-cli -- plan run plans/<dir>

# Pause a running plan
cargo run -p roko-cli -- plan pause plans/<dir>

# Resume a paused plan
cargo run -p roko-cli -- plan resume plans/<dir>

# Cancel a running plan
cargo run -p roko-cli -- plan cancel plans/<dir>

# Check flow status
cargo run -p roko-cli -- plan status plans/<dir>
```
