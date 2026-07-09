# Crash Recovery

> The ParallelExecutor's restart semantics: what is saved before a crash, how to resume,
> and what cannot be recovered.

**Status**: Shipping
**Crate**: `roko-orchestrator`
**Depends on**: [00-overview.md](00-overview.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko saves its execution state to `.roko/state/executor.json` periodically and on any
shutdown signal. After a crash, run `roko plan run plans/ --resume .roko/state/executor.json`
to resume. Completed tasks are not re-executed.

---

## What Gets Saved

The executor snapshot (`executor.json`) contains:

| Field | What it stores |
|-------|---------------|
| `plan_id` | Unique ID of the plan run |
| `task_states` | State of every task: `Pending`, `Running`, `Completed`, `Failed`, `Skipped` |
| `completed_task_ids` | Sorted list of all completed task IDs (used to skip on resume) |
| `failed_task_ids` | Sorted list of all permanently failed task IDs |
| `iteration_memory` | Per-task DO-NOT-REPEAT lists (retry memory) |
| `git_worktree_map` | Map from task ID to git worktree path |
| `agent_outputs` | Last agent output per task (for forensic analysis) |
| `gate_verdicts` | Last gate verdict per task (for forensic analysis) |
| `playbook_snapshot` | Point-in-time copy of active playbook rules |
| `timestamp` | When this snapshot was written |

The snapshot is a JSON file, human-readable, and diffable.

---

## Snapshot Frequency

The executor writes a snapshot:

1. **After every task completes** (success or failure).
2. **On `SIGTERM`** (graceful shutdown): flushes all in-progress state.
3. **On `SIGINT`** (Ctrl-C): same as SIGTERM.
4. **On panic** (via a `std::panic::set_hook` that writes the snapshot before printing
   the panic message).
5. **Every 5 minutes** (background snapshot task) — ensures recent progress is not lost
   even if no tasks have completed in a while.

**What is NOT saved during a SIGKILL (OOM kill, admin kill):**

If the process is killed with SIGKILL, the in-progress snapshot write is interrupted.
The previous snapshot (written after the last completed task) is used for recovery.
Any in-progress task at the time of SIGKILL must be re-executed.

---

## Resuming After a Crash

```bash
# Standard resume: skip all completed tasks, re-run pending and failed
roko plan run plans/ --resume .roko/state/executor.json

# Resume and retry tasks that previously failed
roko plan run plans/ --resume .roko/state/executor.json --retry-failed

# Resume but treat all "Running" tasks as failed (use if the process was killed mid-task)
roko plan run plans/ --resume .roko/state/executor.json --reset-running
```

**`--reset-running` is important after a SIGKILL.** If a task was `Running` when the
process was killed, it was interrupted mid-execution. The agent may have written
partial code. `--reset-running` marks those tasks as `Pending` so they are re-executed
cleanly.

---

## Resumability Invariants

The executor guarantees the following when resuming:

1. **Completed tasks are never re-executed.** A task marked `Completed` in the snapshot
   is unconditionally skipped, regardless of what the filesystem looks like.
2. **Failed tasks are skipped by default.** `--retry-failed` opt-in re-executes them.
3. **Downstream tasks of failed tasks remain Blocked.** They are not skipped; they
   stay `Blocked` until their dependency is resolved (by retrying or manually resolving
   the failed task).
4. **Git worktrees from the previous run are preserved.** The resume uses the same
   worktrees, so compiled artifacts (sccache) are available.

---

## Crash Report

On a panic, Roko writes a crash report to `.roko/logs/crash-<timestamp>.json`:

```json
{
  "timestamp": "2026-04-19T14:32:11Z",
  "panic_message": "called `Option::unwrap()` on a `None` value",
  "backtrace": "stack backtrace: ...",
  "executor_state": {
    "last_completed_task": "04-implement-gate",
    "in_progress_tasks": ["05-implement-learn", "06-implement-bus"]
  },
  "roko_version": "0.4.2",
  "os": "Linux 6.1.0-21-amd64"
}
```

The crash report is safe to share with the Roko maintainers for diagnosis.

---

## Worktree Recovery

After a crash, git worktrees from the interrupted run may be in a dirty state (partial
code changes). The resume handles this:

1. For `Completed` tasks: worktrees are left as-is (they contain good code).
2. For `Running` tasks (after `--reset-running`): the worktree is reset to the task's
   base branch before re-execution.
3. For `Failed` tasks: worktrees are preserved for forensic inspection (see
   [07-forensic-replay.md](07-forensic-replay.md)).

**Manually cleaning up worktrees:**

```bash
# List all roko worktrees
git worktree list | grep "roko/"

# Remove all roko worktrees from a previous run
git worktree prune
```

---

## Recovery When the Snapshot Is Missing or Corrupted

If `executor.json` is missing or corrupted, the run cannot be resumed automatically.
Options:

1. **Use event-log replay** (Built — see [03-event-log-replay.md](03-event-log-replay.md))
   to reconstruct the snapshot from the event log.
2. **Start fresh.** Remove the executor state and re-run the full plan. Completed tasks
   must be identified manually (or by inspecting git worktrees) and skipped via
   `--skip <task-id>`.

---

## Two Full Examples

**Example 1: Graceful shutdown (SIGTERM), clean resume:**

```bash
# Run was interrupted by Ctrl-C
$ roko plan run plans/
...
^C
Received SIGINT. Saving state to .roko/state/executor.json...
State saved. 7 of 12 tasks completed. 2 tasks were in progress (marked as Running).
Run `roko plan run plans/ --resume .roko/state/executor.json --reset-running` to continue.

# Resume
$ roko plan run plans/ --resume .roko/state/executor.json --reset-running
Resuming from .roko/state/executor.json
  Skipping 7 completed tasks
  Resetting 2 running tasks to Pending
  Remaining: 5 tasks
Starting execution...
```

**Example 2: OOM kill, partial snapshot recovery:**

```bash
# Process was killed by OOM killer
$ roko plan run plans/
... (runs for a while, OOM killed)

$ cat .roko/logs/crash-20260419-143211.json
# Shows last completed task was "08-implement-substrate"

# Resume from last saved state
$ roko plan run plans/ --resume .roko/state/executor.json --reset-running
Resuming from .roko/state/executor.json
  Snapshot timestamp: 2026-04-19 14:28:45 UTC
  Skipping 8 completed tasks
  Resetting 3 running tasks to Pending (these were in progress at crash)
  Remaining: 4 tasks
```

---

## See Also

- [03-event-log-replay.md](03-event-log-replay.md) — snapshot reconstruction from event log
- [05-partial-failure.md](05-partial-failure.md) — what happens when some (not all) agents fail
- [09-failure-drill-examples.md](09-failure-drill-examples.md) — full walkthrough of crash scenario

## Open Questions

- Background snapshot interval (5 minutes) is not yet configurable via `roko.toml`.
- `--reset-running` semantics for tasks that were mid-gate (partial output written) need more precise definition.
