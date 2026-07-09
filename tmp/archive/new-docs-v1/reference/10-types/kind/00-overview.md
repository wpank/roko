# Kind — Overview

> The enum that declares what cognitive role an Engram plays in the agent's knowledge graph.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Engram](../../01-engram/00-overview.md)  
**Used by**: [ContentHash](../content-hash/00-overview.md), [Decay Tier Matrix](../decay/08-tier-matrix.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

`Kind` is an enum with 15 concrete variants plus `Custom(String)`. It is part of the
Engram's `ContentHash` — two Engrams with identical bodies but different Kinds have
different identities. Kind determines the default decay policy, influences Gate filtering,
and informs the Substrate's indexing strategy.

---

## The Idea

Knowledge isn't monolithic. A tool invocation trace, a high-level plan, a raw observation,
and a reflective insight are all knowledge — but they have different lifetimes, different
trust assumptions, and different roles in the cognitive loop. Kind is the vocabulary for
these distinctions.

Kind enables the system to:
- Apply appropriate default decay (tool traces decay fast; reflections decay slowly).
- Route Engrams to the right consumers (e.g., only `GateVerdict` Engrams are read by
  the gate audit trail).
- Provide meaningful introspection ("what does the agent know and what kind is it?").

---

## Specification

```rust
<!-- source: crates/roko-core/src/kind.rs -->

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Kind {
    /// Output produced by a Roko agent instance.
    AgentOutput,
    /// A pass/fail verdict from the gate pipeline.
    GateVerdict,
    /// A record of a tool invocation and its result.
    ToolTrace,
    /// A durable piece of domain knowledge.
    KnowledgeEntry,
    /// A belief about a future state.
    Prediction,
    /// A raw input from the environment.
    Observation,
    /// A sequence of intended actions.
    Plan,
    /// A record of a completed task or session.
    Episode,
    /// Introspective self-assessment by an agent.
    Reflection,
    /// A stigmergic signal deposited for other agents.
    Pheromone,
    /// A numeric measurement or health indicator.
    Metric,
    /// A window of assembled context for inference.
    ContextAssembly,
    /// A record of which model was selected for a task.
    ModelSelection,
    /// A structured error event with context.
    ErrorRecord,
    /// Application-specific kind, identified by name.
    Custom(String),
}
```

---

## Canonical Encoding

`Kind` contributes its canonical bytes to the `ContentHash`. See
[canonical encoding](../content-hash/01-canonical-encoding.md) for the full encoding.
The byte representation is the snake_case name of the variant:

```
AgentOutput     → b"agent_output"
GateVerdict     → b"gate_verdict"
...
Custom("foo")   → b"foo"
```

---

## Kind Count

There are 15 concrete variants plus `Custom`. The `Custom` variant allows application-level
extension without modifying `roko-core`.

---

## Open Questions

- Should `Custom(String)` be namespaced (e.g., `"crate/kind_name"`) to prevent conflicts
  between applications? Not yet required.
- Should `Kind` implement `Display` for human-readable output?

## See Also

- [`01-variant-reference.md`](01-variant-reference.md) — full descriptions of each variant
- [`02-kind-and-decay.md`](02-kind-and-decay.md) — how Kind selects default decay
- [`03-api-reference.md`](03-api-reference.md) — API and invariants
