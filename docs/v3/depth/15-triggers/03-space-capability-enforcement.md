# Depth: Space and Capability Enforcement

> Parent: [15-TRIGGERS](../../15-TRIGGERS.md) -- Section 7

---

## Overview

When a trigger binding declares a `space` field, the coordinator resolves a
`TriggerExecutionScope` that restricts the resulting Flow to the intersection
of the Space's capability grants and the target Graph's allowed capabilities.
This enforces three invariants:

1. A trigger in `space:alpha` cannot observe Bus topics from `space:beta`.
2. A trigger in `space:alpha` cannot fire a Graph that is not visible in
   `space:alpha`.
3. The Flow runs with the minimum of both capability sets, never exceeding
   either boundary.

## Resolution: `resolve_trigger_execution_scope`

Located in `crates/roko-serve/src/trigger_runtime.rs`. The function takes
the `AppState`, the binding, and a mutable reference to the event (to stamp
the `space_id` field):

```rust
fn resolve_trigger_execution_scope(
    state: &AppState,
    binding: &TriggerBinding,
    event: &mut TriggerEvent,
) -> Result<TriggerExecutionScope>
```

### Unscoped bindings

If `binding.space` is `None`, the scope inherits any space from the event
payload and the capability set is `None`, meaning legacy unscoped execution
with no capability restriction.

### Scoped bindings

When `binding.space` is `Some(space_id)`:

1. **Space lookup**: Load the Space's `SpaceGrant` from the workspace
   configuration. If the Space is not declared, fail closed.

2. **Bus partition check**: For Bus-kind triggers, verify that the event's
   Bus topic belongs to the Space's partition. Events without a matching
   `space_id` tag are rejected.

3. **Graph visibility check**: Load the Space's `GraphAllowList`. If the
   allow-list is present and the binding's graph is not listed, fail closed
   with an error indicating the graph is not visible within the Space.

4. **Capability intersection**: Compute the intersection of the Space's
   `SpaceGrant` capabilities with the Graph's `CellCapabilities`. The
   resulting `CapabilitySet` is the effective permission boundary for the
   Flow.

5. **Non-empty check**: If the intersection is empty (the Space and Graph
   have no overlapping capabilities), fail closed.

6. **Event stamping**: Set `event.space_id = Some(space_id)` so downstream
   systems can attribute the Flow to its originating Space.

## `TriggerExecutionScope`

```rust
pub struct TriggerExecutionScope {
    pub space_id: Option<String>,
    pub capabilities: Option<CapabilitySet>,
}
```

The scope is passed to `Runtime::run_trigger_graph_scoped`, which applies
the capability restrictions during Graph execution. The `FlowStarted`
lifecycle event records the effective capabilities for audit.

## Fail-closed semantics

Every validation step fails closed:

| Check | Failure mode |
|---|---|
| Space not declared in config | `Err` -- trigger cannot fire |
| Bus event lacks Space partition tag | `Err` -- event rejected |
| Graph not in Space allow-list | `Err` -- graph not visible |
| Capability intersection is empty | `Err` -- no shared capabilities |

This prevents privilege escalation through trigger configuration: a binding
cannot bypass Space restrictions by referencing a Graph outside its boundary.

## Test coverage

The test `scope_enforcement_and_bus_partition` in `trigger_runtime.rs`
verifies:

- Scoped triggers resolve the correct capability intersection.
- The event's `space_id` is stamped by the resolver.
- Bus events without the Space's partition tag are rejected.
- Graphs not in the Space's allow-list are rejected.

**Source:** `crates/roko-serve/src/trigger_runtime.rs` (`resolve_trigger_execution_scope`),
`crates/roko-serve/src/runtime.rs` (`TriggerExecutionScope`)
