# Cross-Layer Protocols

> How layers communicate across boundaries without violating the downward-only rule.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## TL;DR

Layers communicate through three patterns: trait-object injection (downward), Pulse
events on the Bus (sideways and upward), and Substrate reads (downward). Direct
function calls upward in the layer stack are prohibited.

---

## Pattern 1: Trait-Object Injection (Downward)

The primary communication pattern. Higher layers pass implementations to lower layers
via the `TickContext`:

```
L3 (Harness) builds TickContext {
  scorer: Arc<dyn Scorer>,   // L2 implementation
  router: Arc<dyn Router>,   // L2 implementation
  …
}
→ passes it to L2's loop_tick()
→ L2 calls scorer.score() without knowing the concrete type
```

This is strictly downward — L3 controls what L2 executes, but L2 never calls L3.

---

## Pattern 2: Pulse Events (Sideways / Upward via Bus)

Pulses on the Bus are the only approved upward communication channel. A lower layer
may not call a higher-layer function, but it may publish a Pulse that a higher layer
listens to.

```
L2 (roko-agent): publishes tick.failed Pulse
L3 (roko-orchestrator): listens for tick.failed → triggers recovery

L2 (roko-agent): publishes agent.stuck Pulse
L4 (roko-serve): listens for agent.stuck → notifies operator via API
```

The Bus (`Arc<dyn Bus>`) is injected into L2 by L3. L2 publishes to it; L3/L4
subscribe to it. The Pulse is the data; the Bus is the channel. Neither layer
knows about the other's implementation.

---

## Pattern 3: Substrate Reads (Downward)

All layers may read from the Substrate. The Substrate is injected at L3 and flows
down to L2. Higher layers may query it directly (e.g., L4 querying Engrams for
`roko-serve`'s API).

---

## What Is Prohibited

| Pattern | Why prohibited |
|---|---|
| L2 calling a function defined in L3 | Circular dependency |
| L1 holding a reference to an L2 struct | L1 would become coupled to implementations |
| L0 publishing a Pulse to the Bus | L0 is below L1; `Bus` trait is at L1 |
| Shared mutable global state across layers | Breaks testability and substitutability |
| L3 importing a concrete type from L2 and downcasting it | Defeats the abstraction |

---

## Approved Upward Signals

The complete list of events that flow upward (via Pulse):

| Pulse | Publisher | Consumer |
|---|---|---|
| `tick.completed` | L2 | L3, L4 |
| `tick.failed` | L2 | L3 |
| `agent.stuck` | L2 | L3, L4 |
| `agent.suspended` | L2 | L4 |
| `substrate.unavailable` | L2 | L3 |
| `budget.exceeded` | L2 | L3, L4 |
| `verify.failed` | L2 | L3, L4 |
| `delta.start` / `delta.complete` | L2 | L3 (multi-agent coord) |

---

## See also

- [Dependency Rules](06-dependency-rules.md) — the rule that motivates these patterns
- [Bus / transport fabric](../04-bus/README.md) — the Pulse channel
- [Overview](00-overview.md) — the five-layer structure
