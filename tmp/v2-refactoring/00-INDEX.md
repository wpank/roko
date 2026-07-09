# V2 Refactoring — Master Index

## The Problem

Roko has ~177K LOC across 38 crates. ~15K LOC is "built but never wired." The v2 spec
defines a cleaner architecture (Cell, Graph, Engine, Bus, Feed), and much of it is
**already partially built** in v1. The risk isn't building — it's building more things
that never get wired.

## Strategy: Build New, Wire Immediately, Delete Old

Every item in this plan follows one rule: **nothing gets built without a CLI command
that exercises it.** No trait without a caller. No struct without an instantiation site.

The existing `orchestrate.rs` (23K lines) is already dead — feature-gated behind
`legacy-orchestrate`, not enabled by default. The active paths are:

- `roko run` → WorkflowEngine (roko-runtime)
- `roko plan run` → Runner v2 (runner/event_loop.rs)
- `roko serve` → HTTP control plane (roko-serve)

New v2 code should target these active paths, not the dead orchestrate.rs.

## Documents

| Doc | What | Quick wins? |
|-----|------|-------------|
| [01-CURRENT-STATE.md](01-CURRENT-STATE.md) | What's wired, what's dead, what's floating | — |
| [02-WIRING-STRATEGY.md](02-WIRING-STRATEGY.md) | The anti-pattern and how to avoid it | — |
| [03-QUICK-WINS.md](03-QUICK-WINS.md) | Things that can be done in hours, not weeks | Yes |
| [04-CELL-EXECUTE.md](04-CELL-EXECUTE.md) | Add execute() to Cell, CellContext | Phase 1 |
| [05-SIGNAL-RENAME.md](05-SIGNAL-RENAME.md) | Engram → Signal migration | Phase 1 |
| [06-NEW-PROTOCOLS.md](06-NEW-PROTOCOLS.md) | Observe, Connect, Trigger traits | Phase 1 |
| [07-GRAPH-ENGINE.md](07-GRAPH-ENGINE.md) | New Graph + Engine from scratch | Phase 2 |
| [08-FEEDS.md](08-FEEDS.md) | Feed abstraction | Phase 3 |
| [09-GRADUATION.md](09-GRADUATION.md) | Pulse → Signal graduation policies | Phase 3 |
| [10-DEAD-CODE-AUDIT.md](10-DEAD-CODE-AUDIT.md) | What to delete vs what to wire | Cleanup |
| [CHECKLIST.md](CHECKLIST.md) | Master checklist of all items | Tracking |

## Phases

| Phase | What | Effort | Risk | Prerequisite |
|-------|------|--------|------|-------------|
| **0** | Quick wins + dead code cleanup | 2-3 days | None | — |
| **1** | Cell execute() + Signal rename + new protocols | 1-2 weeks | Low | — |
| **2** | Graph + Engine (new crates, build alongside existing) | 4-6 weeks | Medium | Phase 1 |
| **3** | Feeds + Graduation + Predict-Publish-Correct | 2-3 weeks | Low | Phase 2 |
| **4** | Migrate Runner v2 → Engine | 2-4 weeks | Medium-High | Phase 2 |
