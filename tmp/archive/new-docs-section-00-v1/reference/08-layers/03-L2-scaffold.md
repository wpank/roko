# L2 — Scaffold Layer

> Default implementations of L1 traits; `loop_tick()`; the standard agent.

**Status**: Shipping
**Crates**: `roko-std`, `roko-agent`, `roko-compose`
**Depends on**: [L1 Framework](02-L1-framework.md), [L0 Runtime](01-L0-runtime.md)
**Used by**: [L3 Harness](04-L3-harness.md), [L4 Orchestration](05-L4-orchestration.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

L2 is where "Roko" actually exists as runnable code. The three L2 crates provide:
`roko-std` (default operator implementations), `roko-agent` (`loop_tick()` and the
`Agent` struct), and `roko-compose` (multi-agent composition utilities). Most users
building on Roko work primarily with L2.

---

## `roko-std` — Default Implementations

`roko-std` provides batteries-included implementations of all L1 operator traits.

| L1 trait | `roko-std` implementation |
|---|---|
| `Substrate` | `SledSubstrate` (sled-backed), `InMemorySubstrate` (tests) |
| `Scorer` | `WeightedScorer` (configurable linear combination) |
| `Router` | `CascadeRouter` (static → Wilson CI → LinUCB) |
| `Composer` | `GreedyComposer` (token-budget greedy fill) |
| `Gate` | `SafetyGate`, `FormatGate`, `PolicyGate`, `SchemaGate` |
| `Policy` | `AllowAllPolicy`, `DenyListPolicy`, `RateLimitPolicy` |
| `Bus` | `InProcessBus` (local), `TokioChannelBus` (async channels) |

All of these are "good enough for most uses." The architecture exists so that any one
of them can be replaced by a custom implementation.

---

## `roko-agent` — The Cognitive Loop

`roko-agent` provides:

- **`loop_tick()`** — the canonical eight-stage cognitive loop
  (see [loop_tick() reference](../06-loop/09-loop-tick-code.md))
- **`Agent`** — a struct that owns a `TickContext` and runs ticks on a schedule
- **`AdaptiveClock`** — the three-speed scheduler
- **`StuckDetector`** — stuck-loop detection and recovery
- **`PredictionEngine`** — the active inference predict/update cycle
- **`TickBudget`** — per-tick budget tracking

```rust
// source: crates/roko-agent/src/agent.rs
pub struct Agent {
    ctx:      TickContext,
    clock:    AdaptiveClock,
    detector: StuckDetector,
    predict:  PredictionEngine,
}

impl Agent {
    pub async fn run_forever(&mut self) {
        loop {
            let stimulus = self.ctx.bus.next_pulse().await;
            self.ctx.stimulus = stimulus;
            let result = loop_tick(&self.ctx).await;
            self.detector.record(&result);
            self.clock.advance(&result);
            // StuckDetector may trigger tier escalation here
        }
    }
}
```

---

## `roko-compose` — Multi-Agent Composition

`roko-compose` provides utilities for building multi-agent systems:

- **`AgentGraph`** — a DAG of agents with declared input/output Pulse types
- **`PipelineAgent`** — chains agents sequentially: A → B → C
- **`FanOutAgent`** — broadcasts a stimulus to N agents in parallel
- **`FanInAgent`** — collects results from N agents and merges them

These compositions are themselves `Agent`-compatible — they implement the same
`run_forever()` interface and can be nested.

---

## L2 and the Cross-Cuts

The cross-cuts (Neuro, Daimon, Dreams) are injected at L2 through `TickContext`. They
are L1 traits, and their default implementations (`roko-neuro`, `roko-daimon`,
`roko-dreams`) implement those traits at L2. The `TickContextBuilder` at L3 wires
everything together.

---

## See also

- [L1 Framework](02-L1-framework.md) — the traits L2 implements
- [L3 Harness](04-L3-harness.md) — the layer that wires L2 into running agents
- [loop\_tick() reference](../06-loop/09-loop-tick-code.md) — L2's most important function
- [Cross-Cuts](../09-cross-cuts/README.md) — how Neuro/Daimon/Dreams inject into L2
