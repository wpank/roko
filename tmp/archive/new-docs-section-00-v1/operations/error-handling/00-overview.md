# Error Handling Overview

> Roko treats failures as verdicts, not as exceptions. Every failure has a type, a
> recovery strategy, and an observable signal. The system is designed to recover
> automatically from transient failures and to surface durable failures with enough
> context for a human to diagnose and fix.

**Status**: Shipping
**Crate**: `roko-orchestrator`, `roko-agent`, `roko-gate`
**Depends on**: [operations/error-handling/README.md](README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Roko does not treat errors as exceptional events. Every failure mode is anticipated,
classified, and routed to an appropriate recovery strategy. An unhandled error is a bug,
not an expected outcome.

---

## The Philosophy: Verdicts, Not Errors

Classical software raises exceptions for unexpected failures and handles them at the
call site. This leads to inconsistent handling, missed cases, and opaque error messages.

Roko uses a different model: every output from a gate, agent, or subsystem is a
**verdict** — a typed, first-class value that can be `Pass`, `Fail(reason)`, `Retry`,
`Escalate`, or `Fatal`. The calling code does not guess — it pattern-matches on a
well-defined enum.

This means:

1. **Every failure is typed.** You can tell from the error value what class of failure
   it is, not just what exception was thrown.
2. **Recovery is data, not code.** The recovery strategy for a failure class is a
   configuration and policy concern, not an inline `catch` block.
3. **Failures are observable.** Every verdict is persisted as an Engram and emitted as a
   Pulse. There is no silent failure.
4. **Unhandled failures are bugs.** If the system receives a failure verdict that it
   does not know how to handle, that is a bug in the error taxonomy or the recovery
   logic — not an acceptable unknown.

---

## Recoverable vs Fatal

Every error in Roko is either **recoverable** or **fatal**:

| Class | Recovery behaviour |
|-------|-------------------|
| **Recoverable** | Automatic retry, escalation, or circuit break. Operator does not need to intervene. |
| **Fatal** | The current operation is aborted. The system persists state and surfaces the error for human review. No automatic retry. |

The key distinction: a recoverable error may eventually succeed with the same or a
different approach. A fatal error cannot succeed without a human changing something
(the code, the config, or the external system).

---

## Error Classes at a Glance

Full taxonomy: [01-error-taxonomy.md](01-error-taxonomy.md).

| Class | Examples | Recoverable? |
|-------|---------|--------------|
| Gate verdict | Compile failed, test failed, diff too large | Yes (up to max_retries) |
| Infrastructure | Network timeout, disk full, process crash | Varies: transient = yes, durable = no |
| User | Bad TOML config, missing API key, invalid plan | No (requires human fix) |
| LLM | Model rate limit, context window exceeded, malformed response | Partially (rate limit: yes; context exceeded: escalate) |
| Safety | Policy gate rejection, role auth failure, taint violation | No (requires human review) |

---

## The Recovery Pipeline

When a task fails:

```
Task fails
  │
  ├─ Is it fatal? ──yes──► Mark task Failed. Persist verdict. Surface to operator.
  │
  └─ Is it recoverable?
       │
       ├─ Gate verdict: retry with iteration memory (up to max_retries)
       │
       ├─ LLM rate limit: wait + rotate key + retry
       │
       ├─ LLM context exceeded: escalate to longer-context model
       │
       ├─ Network timeout: retry with exponential backoff
       │
       ├─ Process crash: restart via ParallelExecutor recovery
       │
       └─ All retries exhausted: downgrade to fatal path
```

The recovery pipeline is implemented in `roko-orchestrator::recovery`. Policy operators
can intercept verdicts and apply custom recovery strategies.

---

## State Persistence and Resumability

Roko's executor is designed to be **resumed after any failure**. The executor writes a
state snapshot to `.roko/state/executor.json` on:

- Successful plan completion.
- Task failure (after all retries exhausted).
- Process shutdown (graceful — SIGTERM).
- Process crash (via a background writer on a separate Tokio task).

On resume, the executor reads the snapshot and continues from the last committed plan.
Completed tasks are not re-executed. Failed tasks are optionally retried (with `--retry-failed`).

This means a Roko run can be interrupted at any time — power cut, OOM, SIGKILL — and
resume without re-executing work that was already completed.

---

## What Operators Need to Know

**In normal operation:** You do not need to think about error handling. The system
handles transient failures automatically.

**When you need to intervene:**

1. **A task is stuck in retry loops.** Check `roko status` for tasks in `Retrying` state.
   Look at the gate failure reason. Fix the root cause (e.g. a compile error that keeps
   recurring because the model has the wrong context).

2. **A task has permanently failed.** Check `.roko/state/executor.json` or `roko status`
   for `Failed` tasks. The failure reason tells you what class of error it was.

3. **The process crashed.** Check `.roko/logs/roko.log` for the panic message. Run
   `roko plan run plans/ --resume .roko/state/executor.json` to resume.

4. **A gate is circuit-broken.** Check the `CircuitBreakerOpen` warning in the logs.
   The circuit will auto-reset after the configured cooldown. If you need to reset
   immediately: `roko gate reset-circuit <gate-name>`.

---

## See Also

- [01-error-taxonomy.md](01-error-taxonomy.md) — full error class definitions
- [02-recovery-strategies.md](02-recovery-strategies.md) — retry, circuit break, escalate, fail
- [04-crash-recovery.md](04-crash-recovery.md) — crash recovery and resume
- [09-failure-drill-examples.md](09-failure-drill-examples.md) — concrete failure scenarios

## Open Questions

- A `roko errors` subcommand (shows all failures from the current and recent runs) is planned but not yet implemented.
- Error budget tracking (SLO-style, e.g. "gate pass rate must stay > 85%") is a planned Policy operator.
