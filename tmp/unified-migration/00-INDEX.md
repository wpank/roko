# Unified Spec Migration Checklist

> Granular checklist for migrating the Roko codebase (~523 KLOC, 30 crates) from the current architecture (Engram + 6 traits) to the unified spec (Signal/Pulse + 9 protocols + Graph engine). Every item is self-contained with linked sources so an agent with zero context can implement it.

**Spec source**: `tmp/unified/00-INDEX.md` through `tmp/unified/21-ROADMAP.md`
**Depth docs** (in progress): `tmp/unified-depth/` — algorithmic detail, theory, domain-specific knowledge
**Current codebase**: `crates/` — 30 crates, key integration hubs at `crates/roko-cli/src/orchestrate.rs` and `crates/roko-serve/`
**Audit of dead code**: `tmp/roko-trustworthy/AUDIT.md`

---

## Naming Convention

All renames are **literal** — Rust types, files, modules, and public API all change to match spec names.

| Old Name | New Name | When |
|---|---|---|
| `Engram` | `Signal` | Phase 1 |
| `Envelope<E>` / ad-hoc events | `Pulse` | Phase 1 |
| `EventBus` | `Bus` (trait) / `BroadcastBus` (impl) | Phase 1 |
| `Substrate` | `Store` | Phase 1 |
| `Scorer` | `Score` | Phase 1 |
| `Gate` | `Verify` | Phase 1 |
| `Router` | `Route` | Phase 1 |
| `Composer` | `Compose` | Phase 1 |
| `Policy` | `React` (breaking: now takes Pulses) | Phase 1 |
| Module/trait impl | `Cell` (new universal trait) | Phase 1 |
| Plan/tasks.toml | `Graph` (TOML-defined composition) | Phase 2 |
| — | `Observe` (new protocol) | Phase 1 |
| — | `Connect` (new protocol) | Phase 1 |
| — | `Trigger` (new protocol) | Phase 1 |

---

## Phases

| Phase | Focus | File | Items |
|---|---|---|---|
| **0** | Prep & cleanup | [01-PHASE-0-PREP.md](./01-PHASE-0-PREP.md) | Pre-migration cleanup, dead code wiring, rename scaffolding |
| **1** | Kernel upgrade | [02-PHASE-1-KERNEL.md](./02-PHASE-1-KERNEL.md) | Pulse/Bus, predict-publish-correct, demurrage, heuristics, EFE, Observe/Trigger/Connect, renames |
| **2** | Graph engine + Agent runtime | [03-PHASE-2-ENGINE.md](./03-PHASE-2-ENGINE.md) | Hot Graph, type-state Agent, CognitiveWorkspace, StateHub, Surfaces, Rack, SPI, Marketplace |
| **3** | Autonomy, safety, economy | [04-PHASE-3-ECONOMY.md](./04-PHASE-3-ECONOMY.md) | L4 self-evolution, CaMeL IFC, 5-head corrigibility, on-chain, arenas, brain export |

---

## How to Use This Checklist

1. **Each checkbox is one feature** — implementable by an agent in one session (typically 3-10 files changed)
2. **Sources linked** — every item links to the spec doc section AND the current code location
3. **Dependencies explicit** — items note what must be done first
4. **Verification** — every item ends with how to verify it works
5. **Mark complete** — change `- [ ]` to `- [x]` when implemented, tested, and passing `cargo test --workspace`

## Rules

- `cargo test --workspace` must pass after every item
- `cargo clippy --workspace --no-deps -- -D warnings` must pass after every item
- No dead code: if you replace something, delete the old version in the same PR
- No type aliases for backward compat — rename directly
