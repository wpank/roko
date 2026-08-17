# V2 Refactoring — Master Index

> **What is this?** These documents track the refactoring effort to align the roko
> codebase with the v2 spec (`docs/v2/`, 29 chapters + 5 guides). The v2 spec defines
> a cleaner architecture based on five primitives (Signal, Cell, Graph, Bus, Store) and
> nine protocols. This directory contains the analysis, phased plan, and master checklist
> for migrating incrementally from the current codebase toward that target architecture.
>
> **For first-time readers:** Roko already works end-to-end for self-hosting (read PRDs,
> generate plans, execute via agents, validate with gates, persist results). This
> refactoring is about aligning the *internals* with the v2 spec -- replacing procedural
> event loops with declarative Graph + Engine composition, and ensuring every abstraction
> is wired into a runtime path rather than sitting unused. You do not need to understand
> the v2 spec to use roko today; these docs are for contributors working on the
> architectural migration.
>
> **Last updated: 2026-08-13**

## The Problem

Roko has ~177K LOC across 38 crates. ~15K LOC is "built but never wired." The v2 spec
defines a cleaner architecture (Cell, Graph, Engine, Bus, Feed), and much of it is
**already partially built** in v1. The risk isn't building — it's building more things
that never get wired.

## Strategy: Build New, Wire Immediately, Delete Old

Every item in this plan follows one rule: **nothing gets built without a CLI command
that exercises it.** No trait without a caller. No struct without an instantiation site.

The legacy `orchestrate.rs` (23K lines) has been **deleted** (E12-T07). The active
paths are:

- `roko run` → WorkflowEngine (roko-runtime)
- `roko plan run` → Runner v2 (runner/event_loop.rs) -- **sole production engine**
- `roko serve` → HTTP control plane (roko-serve)

New v2 code should target these active paths.

## Current status (2026-08-13)

A previous audit (2026-07) estimated ~45% of Phase 0-1 was done. Since then, significant
progress on Phases 0-1:

### Completed milestones

- [x] **Signal rename**: DONE (2026-08-12). Struct renamed from `Engram` to `Signal` in
  `engram.rs`; `pub type Engram = Signal` backward-compat alias retained. `Signal`
  is the canonical public API name. New code uses `Signal` everywhere.
- [x] **Runner v2**: DONE. Sole production engine for `roko plan run`. `orchestrate.rs`
  deleted entirely (E12-T07), not just feature-gated.
- [x] **main.rs decomposition**: DONE. Commands extracted to `commands/` module with 20+
  subcommand files.
- [x] **docs/v2 spec chapters**: DONE. 29 chapters (00-28) + 5 guides exist in `docs/v2/`
  with implementation status markers. See `28-ROADMAP.md` for per-chapter status.
- [x] **`balance` field on Signal**: DONE. Added with `serde(default)`, `touch()`,
  and tests (QW-3).
- [x] **`roko-calc` skeleton**: DONE (deleted, QW-5).
- [x] **`orchestrate.rs`**: DONE (deleted, QW-4).

### Still outstanding

Phase 2+ (Graph, Engine, Feeds, Graduation) remains **not started**. Within Phase 0-1,
the following items are still open:
- Cell `execute()` method (P1-1 through P1-5)
- Full struct rename of Engram to Signal (P1-6 through P1-8; alias is in place)
- New protocol traits: Observe, Connect, Trigger (P1-9 through P1-14)
- Floating code wiring: calibration_policy, demurrage_consumer, run_ledger,
  error_enrichment, jsonl_rotation, post_gate_reflection, section_outcome (DCA-1 through DCA-6)
- TopicFilter combinators And/Or/Not (QW-2)
- STATUS comment tagging on floating modules (QW-7)

## Documents

| Doc | What | Status |
|-----|------|--------|
| [01-CURRENT-STATE.md](01-CURRENT-STATE.md) | What's wired, what's dead, what's floating | Updated 2026-08-13; dead code sections current |
| [02-WIRING-STRATEGY.md](02-WIRING-STRATEGY.md) | The anti-pattern and how to avoid it | Still relevant (core philosophy) |
| [03-QUICK-WINS.md](03-QUICK-WINS.md) | Things that can be done in hours, not weeks | QW-1/3/4/5 done; QW-2/6/7/8 outstanding |
| [04-CELL-EXECUTE.md](04-CELL-EXECUTE.md) | Add execute() to Cell, CellContext | Not started |
| [05-SIGNAL-RENAME.md](05-SIGNAL-RENAME.md) | Signal rename (from Engram) | DONE (2026-08-12); struct is `Signal`, backward-compat alias retained |
| [06-NEW-PROTOCOLS.md](06-NEW-PROTOCOLS.md) | Observe, Connect, Trigger traits | Not started |
| [07-GRAPH-ENGINE.md](07-GRAPH-ENGINE.md) | New Graph + Engine from scratch | Not started (Phase 2) |
| [08-FEEDS.md](08-FEEDS.md) | Feed abstraction | Not started (Phase 3) |
| [09-GRADUATION.md](09-GRADUATION.md) | Pulse → Signal graduation policies | Not started (Phase 3) |
| [10-DEAD-CODE-AUDIT.md](10-DEAD-CODE-AUDIT.md) | What to delete vs what to wire | orchestrate.rs + roko-calc deletions done; 10 WIRE-NOW items outstanding |
| [CHECKLIST.md](CHECKLIST.md) | Master checklist of all items | Updated 2026-08-13 |

## Phases

| Phase | What | Effort | Risk | Prerequisite | Status |
|-------|------|--------|------|-------------|--------|
| **0** | Quick wins + dead code cleanup | 2-3 days | None | -- | ~70% done (QW-1/3/4/5 + deletions done; QW-2/6/7/8 + DCA wiring outstanding) |
| **1** | Cell execute() + Signal rename + new protocols | 1-2 weeks | Low | -- | Signal rename alias done; Cell execute() + full rename + protocols not started |
| **2** | Graph + Engine (new crates, build alongside existing) | 4-6 weeks | Medium | Phase 1 | Not started |
| **3** | Feeds + Graduation + Predict-Publish-Correct | 2-3 weeks | Low | Phase 2 | Not started |
| **4** | Migrate Runner v2 → Engine | 2-4 weeks | Medium-High | Phase 2 | Not started |
