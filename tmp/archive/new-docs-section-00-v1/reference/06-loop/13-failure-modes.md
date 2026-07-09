# Loop Failure Modes

> Stuck detection, timeouts, partial failure recovery, and observability signals.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [loop\_tick()](09-loop-tick-code.md), [Invariants](12-invariants.md)
**Used by**: Operators, monitoring systems
**Last reviewed**: 2026-04-19

---

## TL;DR

The cognitive loop fails in three broad categories: **partial failures** (one stage
fails, the tick completes with degraded output), **abort failures** (the tick cannot
complete at all), and **stuck loops** (the agent cycles on the same stimulus without
making progress). Each category has distinct detection signals, recovery paths, and
observability markers.

---

## Category 1: Partial Failures

A partial failure means one stage produced an error or suboptimal result, but the tick
continued and produced a (possibly degraded) outcome.

| Stage | Failure | Loop response | Outcome Engram? |
|---|---|---|---|
| QUERY | Timeout | Continue with empty candidates | Yes |
| QUERY | Substrate down | Continue with empty candidates; publish `substrate.unavailable` | Yes |
| SCORE | NaN score | Treat as zero; continue | Yes |
| ROUTE | Low confidence | Escalate to T1; if T1 also low → defer | Depends |
| COMPOSE | Token budget too small | Truncate stimulus; continue | Yes |
| ACT | Model API error | Continue to VERIFY with null output; VERIFY fails | No |
| ACT | Timeout | Continue to VERIFY with null output | No |
| VERIFY | SoftFail | Continue to PERSIST with `verified = false` | Yes (unverified) |
| REACT | Bus unavailable | Cache Pulses; log; tick completes | Yes |

---

## Category 2: Abort Failures

An abort failure means `loop_tick()` returns early without completing all stages.

| Cause | Return type | Engrams written |
|---|---|---|
| ROUTE error (not timeout) | `TickResult::aborted` | Provenance only |
| COMPOSE error | `TickResult::aborted` | Provenance only |
| VERIFY HardFail | Normal return | Provenance + Failure |
| PERSIST critical error | `TickResult::persist_failed` | None (substrate down) |
| Tick budget exceeded | `TickResult::aborted` | Provenance only |

The `TickResult::persist_failed` case is the most severe — if PERSIST cannot write,
the runtime cannot guarantee its memory. The Harness layer escalates this to the
orchestrator, which may restart the agent or trigger a failover.

---

## Category 3: Stuck Loops

A stuck loop occurs when the agent repeatedly processes the same stimulus, produces
low-quality or failed output, and does not make progress. This is distinct from
intentional retry (which is bounded).

### Detection

The `StuckDetector` in `roko-agent` monitors the last N ticks for:

1. **Repeated stimulus**: same stimulus fingerprint appearing in > 3 of the last 5 ticks.
2. **Repeated failure**: `TickResult::aborted` or `HardFail` in > 3 of the last 5 ticks.
3. **Zero novelty**: all QUERY results are the same set as the prior tick (Novelty axis ≈ 0).
4. **Free energy plateau**: `predict.error.total_free_energy` not decreasing over 20 ticks.

```rust
// source: crates/roko-agent/src/stuck_detector.rs
pub struct StuckDetector {
    window:          VecDeque<TickSummary>,
    window_size:     usize,   // default: 10
    repeat_threshold: usize,  // default: 3
}

impl StuckDetector {
    pub fn check(&self) -> Option<StuckReason> {
        if self.repeated_stimulus_count() >= self.repeat_threshold {
            return Some(StuckReason::RepeatedStimulus);
        }
        if self.failure_count() >= self.repeat_threshold {
            return Some(StuckReason::RepeatedFailure);
        }
        // … etc.
        None
    }
}
```

### Recovery

When `StuckDetector` fires, the Harness layer applies the stuck-recovery ladder:

1. **Inject novelty** — temporarily lower `QuerySpec.kind_filter` to surface a broader
   candidate set. Often resolves zero-novelty stuck loops.
2. **Escalate tier** — force the next tick to T1 or T2, assembling a richer context
   that may break the repetition pattern.
3. **Route to fallback** — route the stimulus to a designated fallback agent or model.
4. **Publish `agent.stuck` Pulse** — notify the orchestrator; may trigger human-in-the-loop.
5. **Suspend agent** — if all prior steps fail, suspend the agent and record a
   `agent.suspended` Engram. Manual intervention required.

---

## Timeout Hierarchy

Timeouts are set per stage and per tick. The per-tick budget is the hard outer limit.

```toml
# roko-agent config
[timeouts]
query_stage_ms    = 20
score_stage_ms    = 5     # computed; not I/O
route_stage_ms    = 20
compose_stage_ms  = 12
act_model_ms      = 30000
act_tool_ms       = 5000
verify_stage_ms   = 25
persist_stage_ms  = 25
react_stage_ms    = 18
tick_total_ms     = 60000  # hard outer limit
```

If the per-tick budget is exceeded before all stages complete, the current stage is
interrupted (if async), and `loop_tick()` returns `TickResult::aborted` with
`reason = "tick_budget_exceeded"`.

---

## Observability Signals

Every failure mode emits a named Pulse or metric:

| Signal | Type | When |
|---|---|---|
| `substrate.unavailable` | Pulse | QUERY finds substrate unreachable |
| `route.uncertain` | Pulse | Routing confidence < low threshold |
| `verify.failed` | Pulse | VERIFY returns HardFail |
| `act.blocked` | Pulse | Policy blocked execution |
| `act.timeout` | Pulse | ACT timed out |
| `persist.failed` | Pulse | PERSIST could not write |
| `agent.stuck` | Pulse | StuckDetector fired |
| `agent.suspended` | Pulse | Agent suspended after stuck recovery failed |
| `tick.aborted` | Metric | TickResult::aborted count |
| `tick.latency_p99` | Metric | 99th percentile tick wall time |
| `free_energy_avg` | Metric | Rolling average of predict.error.total_free_energy |

These signals feed into the operations dashboard and can be routed to alerting systems.

---

## Common Debugging Scenarios

### "Agent stopped responding"

1. Check `agent.stuck` and `agent.suspended` Pulses in the event log.
2. If neither: check `route.uncertain` — the agent may be deferring indefinitely.
3. Query the Substrate for `Kind::Failure` Engrams from the agent's last 10 ticks.
4. Check `substrate.unavailable` — the agent may have lost storage access.

### "Agent is producing wrong output"

1. Check `verify.failed` rate — if high, the Gate pipeline may be misconfigured.
2. Check `predict.error.total_free_energy` trend — if increasing, the world model is
   degrading. Trigger T2 consolidation.
3. Check the Provenance Engrams for the bad ticks — examine which candidates were
   composed and which route was taken.

### "Agent is repeating the same action"

1. Run `StuckDetector.check()` over the last 10 ticks directly.
2. Check Novelty axis values in recent SCORE outputs (via Provenance Engrams).
3. Check if the routing prior has converged to a single target (check `CascadeRouter`
   confidence EMA for the stuck stimulus type).

---

## See also

- [Invariants](12-invariants.md) — the invariants that failure modes must not violate
- [Performance](14-performance.md) — latency budgets that trigger timeout failures
- [loop\_tick() reference](09-loop-tick-code.md) — the error-handling structure
- [Operations / error-handling](../../operations/error-handling/README.md) — operational runbooks
