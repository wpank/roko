# Speed Triggers

> What causes each speed tier to activate, escalate, or de-escalate.

**Status**: Shipping
**Crate**: `roko-agent`
**Last reviewed**: 2026-04-19

---

## TL;DR

Gamma activates by default for any incoming Pulse. Theta activates when Gamma's
routing confidence is insufficient or when a prior Gamma tick failed. Delta activates
on schedule, on free-energy threshold, or on manual command. Each transition is
tracked by the speed scheduler and observable via metrics.

---

## Gamma Triggers

| Trigger | Condition |
|---|---|
| Incoming Pulse | Any Pulse arrives in the queue |
| Routing confidence ≥ 0.85 | CascadeRouter returns confidence above threshold |
| Route hint present | Pulse has `route_hint` tag (static route; confidence = 1.0) |
| Prior Theta tick succeeds | Next stimulus defaults back to Gamma |

Gamma is the baseline. If none of the de-escalation conditions apply, the agent runs
at Gamma.

---

## Theta Triggers

| Trigger | Condition |
|---|---|
| Routing confidence 0.60–0.85 | CascadeRouter confidence in the Theta band |
| Gamma VERIFY SoftFail | Same stimulus retried at Theta |
| Gamma VERIFY HardFail | Same stimulus retried at Theta |
| StuckDetector escalation | 3+ failures on same stimulus → force T1 |
| Operator override | `roko agent set-speed --tier theta --agent-id <id>` |

---

## Delta Triggers

| Trigger | Condition | Priority |
|---|---|---|
| Scheduled interval | Every `delta.interval_hours` (default: 4 h) | Low |
| Free energy threshold | Rolling `free_energy_avg > consolidation_threshold` | Medium |
| Emergency consolidation | `free_energy_avg > emergency_threshold` (2× normal) | High |
| StuckDetector: no improvement after T1 | 3+ T1 failures with high free energy | Medium |
| Manual command | `roko agent consolidate` | Immediate |
| Post-major-event | Orchestrator signals "session ended" or "task complete" | Low |

The "emergency consolidation" trigger bypasses the normal scheduling queue and starts
immediately, even if a regular Delta was recently completed.

---

## De-escalation

De-escalation (returning to a lower speed tier after an escalation) happens
automatically:

| De-escalation | Condition |
|---|---|
| Theta → Gamma | Next stimulus of same type has clean routing confidence |
| Theta → Gamma | Prior Theta tick produced a clean VERIFY pass |
| Delta ends | Real-time loop resumes Gamma processing automatically |

De-escalation is not instant. After a Theta escalation, the agent's routing prior for
that stimulus type is updated (confidence EMA decreased slightly). The next few
stimuli of the same type will still start at Theta. Only after the EMA recovers to
≥ 0.85 will the agent return to Gamma for that stimulus type.

---

## Trigger Interaction

Triggers can interact:

- A free-energy threshold trigger during a scheduled Delta window merges into the
  scheduled pass (they do not run separately).
- A manual consolidation command during an active Delta pass is queued for after the
  current pass completes.
- A StuckDetector T1 escalation during active Theta processing on the same stimulus
  is a no-op (already in T1).

---

## Observability

| Signal | Type | Meaning |
|---|---|---|
| `tier.gamma.start` | Event | Gamma tick started |
| `tier.theta.start` | Event | Theta tick started |
| `tier.delta.start` | Event | Delta consolidation started |
| `tier.escalation` | Event | Gamma → Theta escalation with reason |
| `tier.deferral` | Event | Theta → Delta deferral with reason |
| `speed.current` | Gauge | Current primary speed (gamma/theta/delta) |
| `escalation_rate_5m` | Metric | Fraction of recent ticks that escalated |

---

## See also

- [Gamma](01-gamma-reactive.md), [Theta](02-theta-reflective.md), [Delta](03-delta-consolidation.md)
- [Speed Coordination](04-speed-coordination.md) — how tiers interact concurrently
- [Dual-Process](../06-loop/10-dual-process.md) — the confidence-based selection rule
- [Active Inference](../06-loop/11-active-inference.md) — free energy threshold trigger
