# Unified Spec Migration Checklist

> **Last updated: 2026-08-13**

## What is this?

This directory tracks the migration from **mori** (the original 108K LOC orchestrator at
`/Users/will/dev/uniswap/bardo/apps/mori/`) to **roko** (the replacement toolkit at
`/Users/will/dev/nunchi/roko/roko/`). The migration involves renaming core types,
upgrading the kernel to a unified Signal/Pulse/Cell architecture, and rebuilding the
execution engine around composable Graph pipelines.

The companion directory `tmp/unified-migration-runner/` contains the concrete
implementation plans and a parallel-agent runner that executes them.

---

> Granular checklist for migrating the Roko codebase (~177K LOC, 18 crates) from the current architecture (Signal (formerly Engram, renamed 2026-08-12) + 6 traits) to the unified spec (Signal/Pulse + 9 protocols + Graph engine). Every item is self-contained with linked sources so an agent with zero context can implement it.

**Spec source**: `tmp/unified/00-INDEX.md` through `tmp/unified/21-ROADMAP.md`
**Depth docs** (in progress): `tmp/unified-depth/` — algorithmic detail, theory, domain-specific knowledge
**Current codebase**: `crates/` — 18 crates, key integration hubs at `crates/roko-cli/src/runner/event_loop.rs` (runner v2) and `crates/roko-serve/`
**Audit of dead code**: `tmp/roko-trustworthy/AUDIT.md`

---

## Naming Convention

All renames are **literal** — Rust types, files, modules, and public API all change to match spec names.

| Old Name | New Name | When | Status |
|---|---|---|---|
| `Engram` | `Signal` | Phase 1 | **DONE** (2026-08-12) — struct renamed to `Signal` in `roko-core`; `pub type Engram = Signal` alias retained for downstream compat; all new code uses `Signal` |
| `Envelope<E>` / ad-hoc events | `Pulse` | Phase 1 | DONE — `Pulse` struct exists in `roko-core/src/pulse.rs` |
| `EventBus` | `Bus` (trait) / `BroadcastBus` (impl) | Phase 1 | Pending |
| `Substrate` | `Store` | Phase 1 | Pending |
| `Scorer` | `Score` | Phase 1 | Pending |
| `Gate` | `Verify` | Phase 1 | Pending |
| `Router` | `Route` | Phase 1 | Pending |
| `Composer` | `Compose` | Phase 1 | Pending |
| `Policy` | `React` (breaking: now takes Pulses) | Phase 1 | Pending |
| Module/trait impl | `Cell` (new universal trait) | Phase 1 | Pending |
| Plan/tasks.toml | `Graph` (TOML-defined composition) | Phase 2 | Pending |
| — | `Observe` (new protocol) | Phase 1 | Pending |
| — | `Connect` (new protocol) | Phase 1 | Pending |
| — | `Trigger` (new protocol) | Phase 1 | Pending |

---

## Phase Status Summary

| Phase | Focus | File | Status |
|---|---|---|---|
| **0** | Prep & cleanup | [01-PHASE-0-PREP.md](./01-PHASE-0-PREP.md) | **Mostly DONE** — runner v2 replaced orchestrate.rs; main.rs decomposed; serve routes split. Remaining: dead code wiring (ExtensionChain, KnowledgeAdmission, BanditPolicy, gateway fixes). |
| **1** | Kernel upgrade | [02-PHASE-1-KERNEL.md](./02-PHASE-1-KERNEL.md) | **Partially DONE** — Signal rename DONE (2026-08-12). `Pulse` struct exists. Remaining: other 6 trait renames, Bus kernel, Cell trait, demurrage, EFE, Observe/Trigger/Connect. |
| **2** | Graph engine + Agent runtime | [03-PHASE-2-ENGINE.md](./03-PHASE-2-ENGINE.md) | Pending — depends on Phase 1 completion |
| **3** | Autonomy, safety, economy | [04-PHASE-3-ECONOMY.md](./04-PHASE-3-ECONOMY.md) | Pending — depends on Phases 1-2 |

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
