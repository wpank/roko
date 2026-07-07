# Success Metrics

> How we know a phase succeeded: per-phase checkpoint criteria and the quantitative metric table that tracks before/after state.

**Last reviewed**: 2026-04-19

---

## Phase A — Docs Alignment

### Checkpoint criteria

- [ ] The architecture chapter set consistently uses the two-medium / two-fabric framing (`Engram`, `Pulse`, `Substrate`, `Bus`).
- [ ] `GLOSSARY.md` has entries for `Engram`, `Pulse`, `Bus`, `Topic`, and `TopicFilter`.
- [ ] The loop chapter (`reference/06-loop/`) reflects the seven-step loop with co-equal `PERSIST` and `BROADCAST`.
- [ ] The layer chapter (`reference/08-layers/`) places `Bus` at Layer 0 alongside `Substrate`.
- [ ] No canonical chapter still relies on the old equivalence disclaimer as its primary explanation.
- [ ] All target-state pages carry `Status: Specified` frontmatter (not inline disclaimers).

---

## Phase B — Kernel Addition

### Checkpoint criteria

- [ ] `Pulse`, `Bus`, `Topic`, `TopicFilter`, `Datum<'_>`, and the graduation path are exported from `roko-core`.
- [ ] The initial in-process Bus implementations exist in `roko-std` and are testable in isolation.
- [ ] All existing callers still compile (`cargo build --workspace` passes).
- [ ] At least one integration test exercises the full round-trip: `Bus.publish(Pulse)` → subscription → handler.
- [ ] Compatibility shims are in place for all call sites that will migrate in Phase C.

---

## Phase C — Subsystem Migration

### Checkpoint criteria

- [ ] Subsystem-specific transport enums are gone from all migrated call paths.
- [ ] All publication uses `Bus.publish(Pulse { topic, payload })`.
- [ ] TUI polling loops are eliminated; replaced with Bus subscriptions.
- [ ] Conductor, Learning, Orchestration, and Agent paths all consume the shared Bus model.
- [ ] All Phase B compatibility shims have been removed.
- [ ] No cross-crate subsystem transport imports remain on migrated paths.

---

## Phase D — Chain & Mesh Buses

### Checkpoint criteria

- [ ] `ChainBus` and `MeshBus` backends have parity with the core `Bus` surface.
- [ ] Replay behavior is defined and tested for each backend.
- [ ] `MultiBus` composition works without forcing backend-specific details into callers.
- [ ] Existing in-process deployments can add a distributed backend without changing their publication or subscription code.

---

## Quantitative Metrics (Baseline vs. Target)

The following table tracks the concrete, grep-able signals that confirm migration progress. Baseline is the state before Phase A. Target is the state after Phase C completes.

| Metric | Baseline | Target after Phase C | How to measure |
|---|---|---|---|
| Cross-crate subsystem transport imports | Non-zero | 0 | `cargo check` + `grep -r` for subsystem transport type imports |
| Polling loops in the TUI | Present | 0 | `grep -r "loop\|poll\|recv" crates/roko-tui/src` |
| Transport-specific coupling in Conductor and Learning | Present | Reduced to shared Bus abstractions | Diff of import graphs before/after |
| Direct subsystem broadcast assumptions | Present | Eliminated on migrated paths | `grep -r "broadcast\|local_send"` in migrated crates |
| Workspace-level transport surface clarity | Mixed | Topic-driven and documented | Architecture doc review |
| `Pulse` exported from `roko-core` | Absent | Present | `cargo doc --open` + `roko-core` API surface |
| `Bus` trait exported from `roko-core` | Absent | Present | Same |
| `Topic`, `TopicFilter` exported from `roko-core` | Absent | Present | Same |

---

## Qualitative Target

The qualitative signal is simple: **the architecture should look like one transport model with named topics, not several incompatible transport surfaces**.

A reviewer who opens any subsystem crate after Phase C should see:
- Imports from `roko-core::{Bus, Pulse, Topic}` (and nothing subsystem-specific for transport).
- `bus.publish(pulse)` calls where broadcast calls used to be.
- `bus.subscribe(TopicFilter::...)` calls where polling or queue pops used to be.

---

## See Also

- [00-overview.md](00-overview.md)
- [current-status.md](current-status.md) — current values of these metrics
- [dependencies.md](dependencies.md) — phase ordering
