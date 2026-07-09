# REACT — Stage 8 of the Cognitive Loop

> Publish Pulses from the tick's outcome; schedule the next tick.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Bus / transport fabric](../04-bus/README.md),
[PersistResult](07-stage-persist.md), [Pulse](../02-pulse/README.md)
**Used by**: [loop\_tick()](09-loop-tick-code.md),
[Three Cognitive Speeds](../07-speeds/README.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

REACT is the final stage of every tick. It publishes `Pulse` events derived from the
tick's outcome to the Bus, notifying other agents, subsystems, and the speed scheduler
of what happened. It also schedules the next tick — either immediately (for reactive
stimuli) or after the inter-tick interval for the current speed tier.

---

## The Idea

A tick that does not emit any signal is a tick that the rest of the system cannot learn
from. REACT closes the feedback loop: what the agent learned this tick becomes a Pulse
that other agents can react to, and the active inference layer uses to update its
predictions.

REACT has two responsibilities:

1. **Publish** — emit Pulses encoding the tick's outcome.
2. **Schedule** — tell the speed scheduler when to run the next tick.

These are kept in one stage because scheduling depends on what happened: a successful
tick with high-confidence output may allow the agent to return to resting cadence, while
a failed or uncertain tick may trigger immediate re-planning.

---

## Pulses Published by REACT

| Pulse kind | When | Recipients |
|---|---|---|
| `tick.completed` | Every tick | Speed scheduler, monitoring |
| `tick.outcome` | Successful tick | Cross-cuts, other agents |
| `tick.failed` | HardFail in VERIFY or error in ACT | Speed scheduler, monitoring |
| `predict.error` | Always | Active inference layer |
| `budget.consumed` | Every tick | Budget controller |
| `route.uncertain` | RouteDecision.confidence < threshold | Orchestrator, human loop |
| `verify.failed` | HardFail in VERIFY | Safety monitoring |
| `persist.complete` | After PERSIST writes | Neuro cross-cut (for knowledge indexing) |

The `predict.error` Pulse is always emitted, regardless of outcome. It carries:
- The pre-tick prediction (from Active Inference)
- The actual outcome (from PersistResult)
- The prediction error magnitude

This pulse is the primary learning signal for online adaptation.

---

## Specification

```rust
// source: crates/roko-agent/src/loop/react.rs
pub struct ReactResult {
    pub pulses_published: Vec<PulseId>,
    pub next_tick_at:     Timestamp,
    pub speed_tier:       SpeedTier,
}

pub trait ReactStage: Send + Sync {
    fn react(
        &self,
        persist_result: &PersistResult,
        verify_result:  &VerifyResult,
        route_decision: &RouteDecision,
        bus:            &dyn Bus,
        scheduler:      &dyn Scheduler,
    ) -> Result<ReactResult, ReactError>;
}
```

---

## Next-Tick Scheduling

The next tick time is set based on the current speed tier and the tick's outcome:

```
next_tick_at = now + tier.base_interval × outcome_modifier
```

| Outcome | `outcome_modifier` | Effect |
|---|---|---|
| Clean pass, high confidence | 1.0 | Normal cadence |
| SoftFail or low confidence | 0.5 | Next tick sooner |
| HardFail or error | 0.25 | Very soon; possible re-planning |
| Deferred (route.uncertain) | 5.0 | Back off; wait for external signal |

The speed scheduler can override these defaults based on global load and inter-agent
coordination. See [Speed Coordination](../07-speeds/04-speed-coordination.md).

---

## Active Inference Integration

REACT is where the predict/publish/correct cycle closes:

1. Before QUERY (in the previous tick's REACT), a prediction Pulse was published.
2. REACT receives the actual outcome.
3. It computes `prediction_error = actual_outcome − predicted_outcome`.
4. It publishes `predict.error` with the error magnitude.
5. The Active Inference subsystem reads this Pulse to update its world model.

See [Active Inference](11-active-inference.md) for the full mechanism.

---

## Failure Modes

| Failure | Cause | Recovery |
|---|---|---|
| `ReactError::BusUnavailable` | Bus is down | Cache Pulses locally; retry on Bus recovery |
| `ReactError::SchedulerError` | Scheduler rejected next-tick time | Default to tier.base_interval; log warning |
| Missing `predict.error` Pulse | Active inference layer not initialized | Skip; log; not a fatal failure |

REACT errors are not fatal to the tick. A tick that produced a valid PERSIST result
has done its main work. Bus unavailability means other agents won't hear about this
tick, but the agent's own memory is intact.

---

## Performance

| Metric | Target | P99 budget |
|---|---|---|
| Pulse publishing (in-process bus) | < 0.5 ms | < 2 ms |
| Pulse publishing (networked bus) | < 5 ms | < 15 ms |
| Scheduling calculation | < 0.1 ms | < 0.5 ms |
| Total stage budget | < 6 ms | < 18 ms |

---

## Examples

### 1. Normal reactive tick

A Gamma tick completes cleanly. REACT publishes `tick.completed`, `tick.outcome`,
`predict.error` (small error: prediction was close), and `budget.consumed`. Schedules
next tick in 5 s (base interval for Gamma).

### 2. Failed tick with rapid retry

VERIFY returned HardFail. REACT publishes `tick.failed`, `verify.failed`,
`predict.error` (large error: outcome was unexpected), and `budget.consumed`.
Next tick scheduled in 1.25 s (0.25 × 5 s base) for rapid re-planning.

### 3. Uncertain route, back-off

Router returned confidence 0.45, resulting in `RouteTarget::Defer`. REACT publishes
`route.uncertain` and `tick.failed`. Schedules next tick in 25 s (5.0 × 5 s base) to
wait for an external signal (human approval or sub-agent result).

---

## See also

- [PERSIST](07-stage-persist.md) — the result that drives what Pulses are published
- [Active Inference](11-active-inference.md) — how predict.error Pulses are consumed
- [Three Cognitive Speeds](../07-speeds/README.md) — the scheduler that receives next-tick signals
- [Speed Triggers](../07-speeds/05-triggers.md) — what causes a speed tier to advance
- [Bus / transport fabric](../04-bus/README.md) — the channel Pulses travel on
