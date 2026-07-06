# Phase B — Kernel Addition

> Add `Pulse`, `Topic`, `TopicFilter`, `Bus`, and `Datum` to the kernel without changing existing behavior.

**Status**: Planned
**Phase index**: 02 of 04
**Duration estimate**: 2 weeks
**Risk**: Low to moderate (additive — no existing callers break)
**Merge shape**: New types, new modules, compatibility shims
**Depends on**: [Phase A — Docs Alignment](01-phase-docs-alignment.md)
**Unlocks**: [Phase C — Subsystem Migration](03-phase-subsystem-migration.md)
**Last reviewed**: 2026-04-19

---

## Goal

Add the transport vocabulary to the kernel so that Phase C's migration from ad hoc transport enums to Bus-backed Pulses becomes mechanical rather than speculative. By the end of this phase, existing callers still compile and the new path is testable in isolation.

---

## Scope

Phase B is **additive only**. It does not change any existing behavior.

### Core additions

| Item | Description |
|---|---|
| `Pulse` | The ephemeral medium; the counterpart to the durable `Engram` |
| `Topic` | A routing primitive that names a stream of Pulses |
| `TopicFilter` | A subscription predicate over Topics |
| `Bus` | A kernel trait alongside `Substrate`; the transport fabric |
| `Datum<'_>` | A generalized operator input type that accepts either `Engram` or `Pulse` |
| Graduation path | A first-class mechanism to promote a `Pulse` to an `Engram` when lineage matters |

### Crate placement

| Crate | What it gains in Phase B |
|---|---|
| `roko-core` | Exports all new kernel types listed above |
| `roko-std` | Provides the initial in-process Bus implementations |
| Existing crates | Receive compatibility shims; continue to compile unchanged |

---

## Prerequisites

- Phase A must be complete so that the docs and the code match from the moment the new types land.
- The glossary must define `Pulse`, `Bus`, `Topic`, `TopicFilter`, and `Datum` before the crate exports them.

---

## Deliverables

1. `Pulse` type in `roko-core`, with associated `PulseSource` and graduation path.
2. `Topic` and `TopicFilter` types in `roko-core`.
3. `Bus` trait in `roko-core`, at the same level as `Substrate`.
4. `Datum<'_>` generalization of operator input types in `roko-core`.
5. In-process Bus implementation in `roko-std`, testable in isolation.
6. Compatibility shims that keep existing callers compiling.
7. Tests confirming the new path works end-to-end in isolation (without touching migrated callers).

---

## Exit Criteria

- [ ] `Pulse`, `Bus`, `Topic`, `TopicFilter`, `Datum`, and the graduation path are exported from `roko-core`.
- [ ] The initial Bus implementations exist in `roko-std` and are testable in isolation.
- [ ] All existing callers still compile (verified by `cargo build --workspace`).
- [ ] At least one integration test exercises the `Bus.publish(Pulse)` → subscription → handler round trip.

---

## Current Status

Not started. Depends on Phase A reaching exit criteria.

---

## Roadmap Alignment

Phase B is the kernel-addition portion of the **Q1 Foundation** milestone. See [`strategy/roadmap/milestone-q1-foundation.md`](../roadmap/milestone-q1-foundation.md).

---

## Risks

1. **Bus buffer sizing** may be too small for high-chatter topics or too large for memory-sensitive deployments. Mitigation: make buffer sizes configurable from the start; do not hard-code them in the trait.
2. **Graduation policy regressions** may let a Pulse remain ephemeral when lineage should have been preserved. Mitigation: explicit graduation policy with tests for boundary cases.
3. **Compatibility shim rot** — shims intended as temporary may calcify. Mitigation: track shim removals as explicit Phase C milestones.

---

## See Also

- [Phase A — Docs Alignment](01-phase-docs-alignment.md)
- [Phase C — Subsystem Migration](03-phase-subsystem-migration.md)
- [dependencies.md](dependencies.md)
- [success-metrics.md](success-metrics.md)
- [`reference/04-bus/`](../../reference/04-bus/README.md) — Bus specification
- [`reference/02-pulse/`](../../reference/02-pulse/README.md) — Pulse specification
- [`reference/01-engram/`](../../reference/01-engram/README.md) — Engram (the durable sibling)
