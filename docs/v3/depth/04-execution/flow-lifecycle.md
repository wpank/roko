# Flow Lifecycle

> Depth file for [04-EXECUTION.md](../../04-EXECUTION.md) section 7.
> Documents Flow states, transitions, and where a plan run's cleanup happens.

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

**Source:** `crates/roko-cli/src/graph_execution/plan_runner.rs` (`run_one_plan`),
`crates/roko-graph/src/engine.rs`

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

---

## Cleanup on exit

A `GuaranteedFinallyController` was drafted in `crates/roko-graph/src/finally.rs`, but
roko-graph never compiled it (its `lib.rs` declared no `mod finally`), and it was deleted
on 2026-10-01 (gap-ff6e83).

`run_one_plan` (`crates/roko-cli/src/graph_execution/plan_runner.rs`) does a plan run's
cleanup:

- On an interrupt it cancels the graph and sends SIGTERM to in-flight agents. Attempts
  still running after a drain timeout are stopped, and agents that ignored SIGTERM are
  killed.
- It then writes the checkpoint's terminal status and the tasks the run did not complete,
  and closes the run manifest.

A forced exit or SIGHUP ends the run without that terminal write (bug-4641e3).

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

`run_one_plan` owns a plan run's cleanup. It does NOT own:

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
