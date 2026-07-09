# Pulse — Pulse Sources

> PulseSource is a lightweight origin attribution for Pulses — who emitted this event.

**Status**: Specified  
**Crate**: `roko-core` (planned)  
**Depends on**: [Specification](01-specification.md)  
**Last reviewed**: 2026-04-19

> **Target state — no code yet.**

---

## TL;DR

`PulseSource` is a small enum that identifies the origin of a Pulse: a local agent, an
external tool, a chain node, the heartbeat, or a user action. It is lighter than
`Provenance` (which is for Engrams) and carries no trust level or taint. Trust
evaluation happens at graduation time when the Pulse becomes an Engram.

---

## Specification

```rust
<!-- source: crates/roko-core/src/pulse.rs (target state) -->

/// Origin attribution for a Pulse.
/// Intentionally lighter than `Provenance` — Pulses don't need trust tiers or taint.
#[derive(Clone, Debug, PartialEq)]
pub enum PulseSource {
    /// Emitted by a local agent process.
    Agent { agent_id: String },

    /// Emitted by an external tool execution.
    Tool { tool_name: String },

    /// Emitted by a chain node (on-chain event, oracle, etc.).
    Chain { chain_id: String, node: String },

    /// Emitted by the cognitive heartbeat loop.
    Heartbeat { tier: CognitiveTier },

    /// Emitted by a user action (CLI, API, UI).
    User { session_id: String },

    /// Emitted by a subsystem (orchestrator, gate pipeline, substrate GC, etc.).
    Subsystem { name: String },
}

impl PulseSource {
    /// Human-readable name for logging and tags.
    pub fn name(&self) -> String { /* ... */ }
}

/// The cognitive tier that emitted this Pulse.
#[derive(Clone, Debug, PartialEq)]
pub enum CognitiveTier {
    T0,  // Sub-millisecond reactive
    T1,  // Millisecond-to-second deliberative
    T2,  // Second-to-minute reflective
}
```

---

## Conversion to Provenance Author

When a Pulse is graduated to an Engram, the `PulseSource` maps to the Engram's
`provenance.author`:

| PulseSource variant | provenance.author |
|--------------------|------------------|
| `Agent { agent_id }` | `agent_id` |
| `Tool { tool_name }` | `"tool::{tool_name}"` |
| `Chain { chain_id, node }` | `"chain::{chain_id}::{node}"` |
| `Heartbeat { tier }` | `"heartbeat::T0"` / `"T1"` / `"T2"` |
| `User { session_id }` | `"user::{session_id}"` |
| `Subsystem { name }` | `"subsystem::{name}"` |

The `trust` level at graduation is determined by the graduation subscriber, not by the
PulseSource.

---

## Open Questions

- Should `PulseSource` carry a cryptographic identity for chain sources?
- Should `Heartbeat { tier }` include a tick counter?

---

## See Also

- [`01-specification.md`](01-specification.md) — Pulse struct that carries PulseSource
- [`../10-types/provenance/01-author.md`](../10-types/provenance/01-author.md) — Engram author (for durable records)
