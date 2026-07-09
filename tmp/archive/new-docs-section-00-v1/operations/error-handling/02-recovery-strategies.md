# Recovery Strategies

> The four recovery strategies Roko applies to failures: retry, circuit-break, escalate,
> and fail. Each error class maps to one or more strategies. This page defines what each
> strategy does and when it is applied.

**Status**: Shipping (retry, fail) / Built (circuit-break, escalate)
**Crate**: `roko-orchestrator`
**Depends on**: [01-error-taxonomy.md](01-error-taxonomy.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

Most failures are retried automatically. Rate limits escalate with backoff. A gate
that fails too many times opens a circuit breaker. Anything that cannot be recovered
automatically is marked Failed and surfaces for human review.

---

## Strategy 1: Retry

**When applied**: Gate verdicts (up to `gate.max_retries`), transient infrastructure
errors (network timeout, 503), LLM malformed response (1 retry).

**How it works:**

1. The failure reason is appended to the task's iteration memory as a structured entry.
2. The agent is re-invoked with:
   - The original task context.
   - All iteration memory entries (accumulated DO-NOT-REPEAT list).
   - The current retry count (so the agent knows how many attempts remain).
3. The agent produces new output.
4. The gate pipeline runs again on the new output.

**Iteration memory format (appended to agent context):**

```
--- ITERATION MEMORY (attempt 3 of 4) ---
Attempt 1 failed: ROKO-G-001 (CompileFail)
  error[E0308]: mismatched types
  --> src/orchestrator.rs:145:18

Attempt 2 failed: ROKO-G-001 (CompileFail)
  error[E0507]: cannot move out of `self.state` which is behind a mutable reference
  --> src/orchestrator.rs:148:22

DO NOT repeat the type mismatch from attempt 1.
DO NOT repeat the borrow issue from attempt 2.
---
```

**Retry with exponential backoff** (for infrastructure transient errors):

| Attempt | Wait |
|---------|------|
| 1 | 1 s |
| 2 | 4 s |
| 3 | 16 s |
| 4 | 60 s (cap) |

The backoff cap is 60 seconds. If all 4 attempts fail, the error is promoted to a
durable infrastructure failure.

---

## Strategy 2: Circuit Break

**When applied**: A single gate or infrastructure component fails above a threshold
of attempts within a time window.

**Status: Built** — implemented in `roko-orchestrator::circuit_breaker`. Not yet
configurable via `roko.toml`.

**How it works:**

The circuit breaker maintains a rolling window of the last N results for each gate.
If the failure rate in the window exceeds the threshold, the circuit opens:

| Parameter | Default | Description |
|-----------|---------|-------------|
| Window size | 10 tasks | How many recent results to consider |
| Failure threshold | 80% | Open the circuit if > 80% failed |
| Cooldown | 300 s | Time the circuit stays open before attempting to close |
| Half-open probe | 1 task | Run one task to test if the circuit should close |

**Circuit states:**

```
CLOSED ──(failure rate > 80%)──► OPEN ──(after 300s)──► HALF-OPEN
   ▲                                                         │
   └───────(probe task passes)──────────────────────────────┘
           (probe task fails) ──► OPEN (reset cooldown)
```

**What operators see when a circuit is open:**

```
Warning: gate circuit breaker open for "test"
  Opened at: 2026-04-19 14:32:11 UTC
  Reason: 9/10 recent tasks failed (90% failure rate)
  Cooldown: 285s remaining
  Action: investigate the test suite; use `roko gate reset-circuit test` to force-close

New tasks will NOT run the "test" gate until the circuit closes.
```

When the circuit is open, new tasks skip the broken gate and continue with the rest
of the pipeline. This prevents a broken external tool (e.g. a flaky test runner) from
blocking all progress.

---

## Strategy 3: Escalate

**When applied**: LLM context window exceeded; LLM T1 tier failure in CascadeRouter.

**How it works:**

Escalation routes the task to a more capable model or configuration without human
intervention.

**Context window escalation:**

```
Task requires 150K tokens
  Agent.model = claude-sonnet-4-5 (200K context)
  → Context assembled: 195K tokens — approaching limit
  → Escalate: use claude-sonnet-3-7-20250101 (500K context) or truncate context
```

Context escalation is triggered when the assembled context exceeds 90% of the model's
context window. The orchestrator attempts to:
1. Truncate lower-priority context sections.
2. If still too large, switch to a longer-context model (if `agent.fallback_model` is set).
3. If no fallback model is configured, fail with `ROKO-L-002`.

**CascadeRouter escalation:**

```
T0 (rules) → no match
T1 (haiku)  → gate fails
T2 (opus)   → ← escalated here
```

T1 → T2 escalation is automatic within the CascadeRouter. There is no T3. If T2 fails,
the task goes to normal retry logic.

---

## Strategy 4: Fail

**When applied**: All retries exhausted; fatal infrastructure errors; user errors;
safety errors; LLM safety refusals.

**How it works:**

1. The task state is set to `Failed(<reason>)`.
2. A failure Engram is written to the Substrate with kind `TaskFailed`.
3. A `TaskFailed` Pulse is emitted on the event bus.
4. The executor logs the failure at `error` level with the full error chain.
5. Downstream tasks that depend on the failed task are marked `Blocked(depends_on_failed_task)`.

**Fail does not stop the entire plan run.** The executor continues executing tasks
that are not blocked by the failed task. Only the subgraph of the DAG downstream of the
failed task is blocked.

**After a failed run:**

```bash
# See all failed tasks
roko status --failed

# Retry only failed tasks (after fixing the root cause)
roko plan run plans/ --resume .roko/state/executor.json --retry-failed

# Skip a specific failed task and continue
roko plan run plans/ --resume .roko/state/executor.json --skip 05-my-task
```

---

## Strategy Selection Decision Tree

```
Error received
  │
  ├─ Safety error? ──────────────────────────────────► Fail immediately (no retry)
  │
  ├─ User error? ─────────────────────────────────────► Fail immediately (no retry)
  │
  ├─ LLM safety refusal? ─────────────────────────────► Fail immediately (no retry)
  │
  ├─ Gate verdict?
  │   ├─ retries_used < max_retries? ─────────────────► Retry with iteration memory
  │   └─ retries_used >= max_retries? ────────────────► Fail
  │
  ├─ LLM rate limit?
  │   ├─ keys available? ──────────────────────────────► Retry with key rotation
  │   └─ all keys rate-limited? ──────────────────────► Wait + retry
  │
  ├─ LLM context exceeded?
  │   ├─ fallback model available? ───────────────────► Escalate
  │   └─ no fallback? ────────────────────────────────► Truncate context → retry
  │
  ├─ Transient infra (network, 503)?
  │   ├─ retries_used < 4? ───────────────────────────► Retry with exponential backoff
  │   └─ all retries failed? ─────────────────────────► Fail
  │
  └─ Durable infra (disk, corruption)?
      └────────────────────────────────────────────────► Fail (requires human)
```

---

## Configuring Recovery

The main recovery knobs in `roko.toml`:

```toml
[gate]
max_retries         = 3     # Gate verdict retry limit
timeout_seconds     = 120   # Per-gate timeout (timeouts are retried)

[agent]
timeout_seconds     = 600   # Per-task timeout (on expiry: mark TimedOut, save snapshot)
```

Circuit breaker and escalation parameters are not yet configurable via `roko.toml`.

---

## See Also

- [01-error-taxonomy.md](01-error-taxonomy.md) — error class definitions
- [04-crash-recovery.md](04-crash-recovery.md) — recovery after process crash
- [06-cascade-failure.md](06-cascade-failure.md) — circuit breaker details

## Open Questions

- Circuit breaker parameters (`window_size`, `failure_threshold`, `cooldown_seconds`) are not yet in `roko.toml`.
- `agent.fallback_model` key (for context window escalation) is not yet in the schema.
- The `--skip <task>` flag is planned but not yet implemented.
