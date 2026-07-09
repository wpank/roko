# Cascade Failure

> How Roko detects and contains failures that propagate across subsystem boundaries,
> and what the operator does when one failing component takes down its dependants.

**Status**: Built (circuit breaker, error context propagation) / Specified (cross-agent cascade isolation)
**Crate**: `roko-orchestrator`, `roko-gate`, `roko-agent`
**Depends on**: [Error Taxonomy](01-error-taxonomy.md), [Recovery Strategies](02-recovery-strategies.md),
[Partial Failure](05-partial-failure.md)
**Used by**: [Observability](08-observability.md), [Failure Drill Examples](09-failure-drill-examples.md)
**Last reviewed**: 2026-04-19

---

## What Is a Cascade Failure?

A **cascade failure** occurs when a fault in component A causes component B to fail, and
B's failure causes C to fail — even though B and C have no intrinsic defect. The causal
chain is:

```
A fails → B loses its input/dependency → B fails → C loses B's output → C fails
```

In Roko the most common cascade paths are:

| Trigger | Propagation path | Ultimate symptom |
|---|---|---|
| LLM provider outage | Agent stalls → Gate never fires → Orchestrator times out | Task stuck/cancelled |
| Substrate write failure | Event log corrupt → Resume impossible → Must reset | Data loss risk |
| Rate limit on API key | T2 model unavailable → CascadeRouter degrades to T1 → Quality drop | Silent quality regression |
| MCP tool returns error | Agent retries → Exhausts max_turns → Gate never reached | Incomplete task |
| Bad system_prompt | Agent loops → max_turns hit → Gate compile skipped | Misleading output |

---

## Cascade Anatomy in Roko

### Example: LLM outage cascade

```
t=0:00  roko-agent        POST /chat/completions → timeout (30s)
t=0:30  roko-agent        RETRY 1/3 → timeout
t=1:00  roko-agent        RETRY 2/3 → timeout
t=1:30  roko-agent        RETRY 3/3 → timeout
        roko-agent        CIRCUIT OPEN: llm/openai
t=1:30  roko-orchestrator Agent returned error ROKO-L-001
        roko-gate         Gate not triggered (no diff produced)
        roko-orchestrator Task FAILED after 0/N subtasks completed
```

The gate pipeline never ran — not because of a gate bug, but because the agent upstream
of it could not produce a diff. The operator must not investigate gate configuration
for this failure.

### Example: substrate cascade

```
t=0:00  roko-fs           disk write → ENOSPC (no space left)
        roko-fs           ROKO-I-003: substrate write failed
        roko-orchestrator Cannot append to event log
        roko-orchestrator ROKO-I-003: event log unavailable
        roko-agent        Cannot commit episode → learning disabled
        roko-learn        Episode queue stalled (0 new episodes)
```

One disk-full error silently disables the learning subsystem. Nothing is visibly broken
to the user unless they monitor `roko learn status`.

---

## Detection Signals

### Log-level signals (structured JSON fields)

```json
{ "level": "ERROR", "target": "roko_agent", "error_code": "ROKO-L-001",
  "cascade_depth": 0, "caused_by": null }

{ "level": "ERROR", "target": "roko_orchestrator", "error_code": "ROKO-O-001",
  "cascade_depth": 1, "caused_by": "ROKO-L-001" }

{ "level": "ERROR", "target": "roko_gate", "error_code": "ROKO-G-004",
  "cascade_depth": 2, "caused_by": "ROKO-O-001" }
```

Filter for `cascade_depth > 0` to identify secondary/tertiary failures:

```bash
roko logs --last 1h | jq 'select(.cascade_depth > 0)'
```

### Metric signals

| Metric | Normal | Cascade indicator |
|---|---|---|
| `gate.runs_total` | > 0 per task | 0 when agent cascade prevents gate |
| `agent.turns_total` / `agent.turns_max` | < 0.8 | ≈ 1.0 means agent hit turn limit |
| `learn.episodes_queued` | Draining | Monotonically increasing = substrate cascade |
| `llm.circuit_open` | false | true = upstream API problem |
| `substrate.write_errors_total` | 0 | > 0 = disk/FS cascade source |

---

## Containment Strategies

### 1. Circuit breaker (Built)

The circuit breaker prevents a failing external service from receiving endless retries
that block the executor thread. When a breaker trips, the affected path is marked
**OPEN** and callers immediately receive `CircuitOpen` without network I/O.

```
         ┌─────────────────────────────────┐
         │ Circuit state machine           │
         │                                 │
  CLOSED ──[failure_count >= threshold]──► OPEN
         │                                 │
         │  ◄──[reset_timeout elapsed]──   │
         │             HALF-OPEN           │
         │  ──[probe succeeds]──►  CLOSED  │
         │  ──[probe fails]────►  OPEN     │
         └─────────────────────────────────┘
```

Default thresholds (configurable in `[gate]`):

| Parameter | Default | Meaning |
|---|---|---|
| `circuit_failure_threshold` | 5 | Consecutive failures before OPEN |
| `circuit_reset_timeout` | `"60s"` | Time in OPEN before probe attempt |

When a breaker is open, the gate pipeline immediately returns `ROKO-G-003: circuit open`
rather than waiting for `gate.timeout`.

### 2. Cascade depth limit (Built)

Error propagation is tagged with `cascade_depth`. If depth exceeds 3, the orchestrator
treats it as a fatal cascade and aborts the task:

```
cascade_depth 0 — root cause, normal recovery
cascade_depth 1 — secondary; log with warning
cascade_depth 2 — tertiary; escalate to operator
cascade_depth 3 — fatal cascade; abort task, write post-mortem to events
```

### 3. Subtask isolation (Shipping)

`ParallelExecutor` runs subtasks in separate Tokio tasks. A panic in one subtask is
caught via `JoinHandle::catch_unwind`. It does **not** propagate to other subtasks unless
`fail_fast = true`.

```toml
# roko.toml — prevent cascade across parallel subtasks
[orchestrator]
fail_fast = false    # default; one subtask failure does not cancel siblings
```

### 4. Learn subsystem isolation (Shipping)

Episode logging and learning are asynchronous. A substrate failure disables learning
but does not block the agent from completing its task:

```
Agent ──► commits diff ──► gate passes ──► task SUCCESS
                │
                └──► tries to log episode ──► ENOSPC ──► logs warning, continues
```

The task result is not degraded. The operator will see `learn.episodes_dropped_total`
increment and should investigate the substrate.

---

## Recovery Playbook

### Cascade triggered by LLM provider outage

1. Confirm circuit is open:
   ```bash
   roko status --circuits
   # output: llm/openai  OPEN  (opened 14:03:01, resets 14:04:01)
   ```

2. Check provider status page (external).

3. Wait for auto-reset (60s default), or force-close if provider is confirmed healthy:
   ```bash
   roko circuit reset llm/openai
   ```

4. Resume any stalled tasks:
   ```bash
   roko run --resume <task-id>
   ```

### Cascade triggered by ENOSPC

1. Identify disk pressure:
   ```bash
   df -h $(roko config get substrate.data_dir)
   ```

2. Free space or expand volume.

3. Verify substrate can write:
   ```bash
   roko substrate health
   ```

4. Re-process any dropped episodes:
   ```bash
   roko learn --process --retry-failed
   ```

5. Confirm `learn.episodes_queued` is draining:
   ```bash
   roko learn status
   ```

### Cascade triggered by bad system_prompt (agent loop)

1. Identify the stuck task:
   ```bash
   roko run status --running
   ```

2. Check turn utilisation:
   ```bash
   roko events show <task-id> | jq '.turns_used / .turns_max'
   ```

3. Kill the stuck run and reset:
   ```bash
   roko run --reset-running <task-id>
   ```

4. Edit `system_prompt` in `roko.toml` to remove the loop-inducing instruction.

5. Reduce `max_turns` temporarily as a safety stop while debugging:
   ```toml
   [agent]
   max_turns = 5    # tighter leash during debugging
   ```

---

## Configuration Reference

```toml
[orchestrator]
fail_fast                  = false    # abort siblings on subtask failure
cascade_depth_limit        = 3        # abort task when cascade exceeds this depth

[gate]
circuit_failure_threshold  = 5        # consecutive failures before circuit opens
circuit_reset_timeout      = "60s"    # time in OPEN state before probe

[agent]
max_turns                  = 20       # hard cap; prevents infinite agent loops
```

---

## See also

- [02-recovery-strategies.md](02-recovery-strategies.md) — circuit breaker detail
- [05-partial-failure.md](05-partial-failure.md) — partial vs cascade distinction
- [08-observability.md](08-observability.md) — metrics and alerting for cascade detection
- [09-failure-drill-examples.md](09-failure-drill-examples.md) — step-by-step cascade drills
