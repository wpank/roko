# Partial Failure

> How Roko handles the case where a task completes some work but not all of it — and how
> the operator recovers without replaying steps that already succeeded.

**Status**: Built (ParallelExecutor subtask tracking) / Specified (partial-success verdicts)
**Crate**: `roko-orchestrator`
**Depends on**: [Error Taxonomy](01-error-taxonomy.md), [Crash Recovery](04-crash-recovery.md),
[Event Log Replay](03-event-log-replay.md)
**Used by**: [Cascade Failure](06-cascade-failure.md), [Failure Drill Examples](09-failure-drill-examples.md)
**Last reviewed**: 2026-04-19

---

## What "Partial Failure" Means

A **partial failure** is any execution that produces *some* correct output but cannot
complete the full task graph. The task is not fully successful and not cleanly failed —
it sits in between:

```
         ┌────────┐   ┌────────┐   ┌────────┐
input ──►│step A  │──►│step B  │──►│step C  │──► output
         └────────┘   └────────┘   └────────┘
              ✓            ✓            ✗  ← partial failure here
```

This is distinct from:
- **Full failure** — nothing succeeded; the executor bails on step A.
- **Cascade failure** — step C's failure causes step B to retroactively fail too (see
  [06-cascade-failure.md](06-cascade-failure.md)).

Partial failures are common in multi-step gate pipelines (compile passes, test fails) and
in parallelised agent tasks (3 of 5 sub-agents finish before one hits a rate-limit error).

---

## Partial Failure in Gate Pipelines

The gate pipeline is the most frequent partial-failure site. Gates run sequentially by
rung, so an earlier rung's success is never undone by a later rung's failure.

### State after partial gate failure

```
Rung 1 — compile:    PASS
Rung 2 — test:       FAIL  ← gate pipeline stops here
Rung 3 — clippy:     (not reached)
Rung 4 — format:     (not reached)
Rung 5 — diff:       (not reached)
Rung 6 — semantic:   (not reached)
```

The gate verdict is **`Fail`** for the overall pipeline, but the compile rung recorded its
result (no recompile needed if the same diff is retried within the diff-hash cache window).

### What the operator sees

```
$ roko run --task "fix bug #42"

[14:03:01] rung/compile        PASS  (3.2s)
[14:03:09] rung/test           FAIL  (8.1s)
  ROKO-G-002: test gate failed — 3 failures in src/scoring/mod.rs
  Recovery: RETRY (attempt 1/3, back-off 2s)
[14:03:11] rung/test           FAIL  (7.9s)
  Recovery: RETRY (attempt 2/3, back-off 4s)
[14:03:17] rung/test           FAIL  (7.8s)
  Recovery: ESCALATE — returning test failure context to agent
```

The agent receives the test failure output as structured context and may attempt code
edits. Compile is **not** rerun unless the agent produces a new diff.

---

## Partial Failure in Parallel Task Execution

`ParallelExecutor` tracks each sub-task independently. A failure in one sub-task does not
automatically cancel siblings unless `fail_fast = true` (default: `false`).

### Subtask states

| State | Meaning |
|---|---|
| `Pending` | Not yet started |
| `Running` | Active; PID recorded in `executor.json` |
| `Succeeded` | Finished, output committed |
| `Failed(reason)` | Terminal failure, reason stored |
| `Cancelled` | Sibling triggered fail-fast |

### executor.json after a partial failure

```json
{
  "task_id": "task-7f3a",
  "created_at": "2026-04-19T14:00:00Z",
  "subtasks": [
    { "id": "sub-0", "status": "Succeeded", "output_ref": "events/sub-0.jsonl" },
    { "id": "sub-1", "status": "Succeeded", "output_ref": "events/sub-1.jsonl" },
    { "id": "sub-2", "status": "Failed",    "error": "ROKO-L-001: rate_limit", "attempts": 3 },
    { "id": "sub-3", "status": "Pending" },
    { "id": "sub-4", "status": "Pending" }
  ]
}
```

### Resume from partial failure

```bash
# Resume from where it stopped — sub-0 and sub-1 are NOT re-run
roko run --resume task-7f3a

# Force a specific subtask to re-run even if it previously succeeded
roko run --resume task-7f3a --force-subtask sub-1

# Inspect what was already completed
roko events show task-7f3a --subtask sub-0
```

`--resume` reads `executor.json`, marks `Succeeded` subtasks as skipped, and re-queues
`Failed` and `Pending` subtasks.

---

## Partial Failure in the Learning Subsystem

The `roko-learn` CascadeRouter processes episodes asynchronously. If an episode's T1 or T2
inference fails mid-routing, the raw event log is preserved and the episode is marked
`routing_failed`.

### Episode states

```
┌──────────────────────────────────────────────────────────┐
│ Episode lifecycle                                         │
│                                                          │
│  captured → queued → routing → distilling → complete     │
│                          ↓                               │
│                   routing_failed (event log preserved)   │
└──────────────────────────────────────────────────────────┘
```

Episodes in `routing_failed` are retried on the next `roko learn --process` run. The raw
event log is never discarded as long as `substrate.data_dir` is intact.

### Operator action

```bash
# Check for episodes with routing failures
roko learn status --show-failed

# Re-process failed episodes
roko learn --process --retry-failed

# Inspect a specific failed episode's raw events
roko events show <episode-id>
```

---

## Partial Failure Detection Checklist

Use this checklist when a run exits with a non-zero code but the output directory is
non-empty:

1. **Check `executor.json`** — which subtasks succeeded, which failed.
   ```bash
   cat .roko/state/executor.json | jq '.subtasks[] | {id, status}'
   ```

2. **Check gate rung results** — which rungs passed before failure.
   ```bash
   roko run status --last | grep -E 'PASS|FAIL'
   ```

3. **Check event log integrity** — confirm completed subtasks' logs are valid.
   ```bash
   roko events verify --task <task-id>
   ```

4. **Identify safe resume point** — only retry `Failed` and `Pending` subtasks.
   ```bash
   roko run --resume <task-id> --dry-run
   ```

5. **Resume or abandon** — resume if the completed work is valid; reset if state is
   inconsistent (hash mismatch in step 3).
   ```bash
   # Resume
   roko run --resume <task-id>

   # Abandon and reset
   roko run --reset-running <task-id>
   ```

---

## Idempotency Contracts

For `--resume` to be safe, each step must be idempotent with respect to its prior output:

| Step type | Idempotent? | Notes |
|---|---|---|
| Gate compile | Yes | Cargo cache; same inputs → same artefact |
| Gate test | Yes | Tests are read-only |
| Gate clippy | Yes | Lint is read-only |
| Gate diff | Yes | Diff computed from working tree snapshot |
| Gate semantic | Mostly | LLM re-inference may produce slightly different output |
| LLM call (T0 rules) | Yes | Deterministic |
| LLM call (T1/T2) | No | Non-deterministic; temperature > 0 |
| Substrate write | Yes | JSONL append is idempotent if event ID is unique |
| MCP tool call | **No** — side effects | Treat as non-idempotent; log before call |

**Rule**: only skip a step on resume if its output was committed to the event log and the
log hash was verified. LLM inference steps that were in-flight at crash time must be
re-run.

---

## Configuration

No dedicated `[partial_failure]` section. Relevant knobs live in other tables:

```toml
[gate]
max_retries    = 3       # per-gate retry limit before ESCALATE
retry_backoff  = "2s"    # base back-off; doubles each attempt

[agent]
max_turns      = 20      # caps total agent turns across resume cycles

[learn]
retry_failed_episodes = true   # auto-retry routing_failed on next roko learn --process
```

---

## See also

- [06-cascade-failure.md](06-cascade-failure.md) — when partial failure propagates
- [03-event-log-replay.md](03-event-log-replay.md) — how the event log enables safe resume
- [04-crash-recovery.md](04-crash-recovery.md) — crash vs partial failure distinction
- [09-failure-drill-examples.md](09-failure-drill-examples.md) — runbook walkthroughs
