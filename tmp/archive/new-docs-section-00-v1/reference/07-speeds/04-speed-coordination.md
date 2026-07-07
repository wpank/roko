# Speed Coordination

> How Gamma, Theta, and Delta interact; the adaptive clock; multi-agent synchronization.

**Status**: Shipping
**Crate**: `roko-agent`
**Depends on**: [Gamma](01-gamma-reactive.md), [Theta](02-theta-reflective.md),
[Delta](03-delta-consolidation.md), [Scheduler](../08-layers/03-L3-harness.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

The three speed tiers run concurrently within a single agent. Gamma and Theta share the
real-time loop; Delta runs in the background. The adaptive clock adjusts tick periods
based on load, prediction error, and idle state. Multi-agent speed coordination uses
the Bus to synchronize consolidation passes across agents.

---

## Concurrent Execution Model

An agent does not "switch" between speeds in a mutually exclusive way. Instead:

- **Gamma/Theta** operate on the real-time stimulus queue. Each incoming Pulse starts
  a tick; the ROUTE stage determines whether it runs at Gamma or Theta parameters.
- **Delta** runs as a background task, triggered by schedule or free energy threshold.
  It shares the Substrate with the real-time loop but runs at lower I/O priority.

```
┌─────────────────────────────────────────────────────┐
│  Agent runtime                                      │
│                                                     │
│  ┌───────────────────────┐   ┌───────────────────┐ │
│  │ Real-time loop        │   │ Delta background  │ │
│  │ (Gamma/Theta ticks)   │   │ (consolidation)   │ │
│  │                       │   │                   │ │
│  │  Pulse queue → loop_tick()  │   │  Scheduled / triggered  │ │
│  └───────────────────────┘   └───────────────────┘ │
│           │                           │             │
│           └───────────────────────────┘             │
│                    Substrate                        │
└─────────────────────────────────────────────────────┘
```

The two loops share the Substrate but do not share a mutex. Delta reads Engrams and
writes updated versions; the real-time loop reads and writes Engrams as part of normal
tick processing. Substrate-level write ordering is managed by the substrate
implementation (sled's MVCC, Postgres transactions).

---

## The Adaptive Clock

The Gamma tick period is not fixed. The `AdaptiveClock` in `roko-agent` adjusts it
based on:

### Load adaptation

If the Pulse queue depth exceeds `load_high_watermark`, the clock shortens toward
`min_period_secs` to process stimuli faster. If the queue is empty for more than
`idle_timeout`, the clock lengthens toward `max_period_secs`.

```rust
// source: crates/roko-agent/src/clock.rs
fn next_period(&self) -> Duration {
    let queue_depth = self.queue.len();
    let load_factor = (queue_depth as f32 / self.load_high_watermark as f32).min(1.0);
    let period_ms = lerp(
        self.max_period_ms as f32,
        self.min_period_ms as f32,
        load_factor,
    );
    Duration::from_millis(period_ms as u64)
}
```

### Outcome adaptation

The `outcome_modifier` from REACT adjusts the next tick's period:

- Clean pass, high confidence → modifier 1.0 (normal cadence)
- SoftFail or low confidence → modifier 0.5 (faster retry)
- HardFail or error → modifier 0.25 (fast recovery)
- Deferred → modifier 5.0 (back off; wait)

These modifiers interact multiplicatively with the load-adjusted period.

### Jitter

A small random jitter (±10% of the period) is added to prevent agents from
synchronizing their tick times when deployed in a fleet. Without jitter, N agents
started simultaneously would submit N model requests at the same instant, causing
burst load on model APIs.

---

## Multi-Agent Speed Coordination

When multiple agents share a deployment, their Delta consolidation passes may benefit
from coordination — particularly when agents share knowledge domains.

The Bus carries two speed-coordination Pulse types:

| Pulse | Emitted by | Consumed by | Effect |
|---|---|---|---|
| `delta.start` | Agent A beginning Delta | Peer agents | Peers delay their own Delta start to avoid concurrent substrate writes |
| `delta.complete` | Agent A finishing Delta | Peer agents | Peers may start their own Delta; they inherit updated routing priors |

This is a best-effort protocol. An agent that does not receive `delta.start` within
its coordination window will proceed with its own Delta regardless.

---

## Gamma / Theta Interaction

Gamma and Theta share the real-time loop queue. They do not run simultaneously on the
same stimulus — a Theta tick on stimulus S blocks Gamma ticks on the same stimulus
(deduplicated by stimulus fingerprint).

When the ROUTE stage decides T1 (Theta) for a stimulus, it does not interrupt the
current Gamma cycle. It places the stimulus in a "pending Theta" queue and processes it
in the next available Theta slot. The Gamma loop continues processing other stimuli
meanwhile.

This means an agent can answer simple Gamma questions while a complex Theta question is
being processed — they do not block each other.

---

## Configuration

```toml
[adaptive_clock]
min_period_ms               = 5000    # 5 s
max_period_ms               = 15000   # 15 s
load_high_watermark         = 10      # queue depth that triggers max speed
idle_timeout_secs           = 30      # idle period before extending to max
jitter_fraction             = 0.10    # ±10% random jitter

[multi_agent_coordination]
delta_coordination_enabled  = true
delta_coordination_window_s = 30      # wait up to 30 s for peers to finish
```

---

## See also

- [Gamma](01-gamma-reactive.md), [Theta](02-theta-reflective.md), [Delta](03-delta-consolidation.md)
- [Triggers](05-triggers.md) — what causes each tier to activate
- [Resource Budgets](06-resource-budgets.md) — compute allocation across tiers
- [Bus / transport fabric](../04-bus/README.md) — the channel for coordination Pulses
