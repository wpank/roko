# Backlog — Implementation Specs

> **What is this?** Self-contained backlog items for the roko workspace. Each doc
> specifies a problem, what already exists, what needs building, where to put it,
> and how to verify it.
>
> **How to use:** Pick an item, read the spec, implement it. Update `.roko/GAPS.md` when done.
>
> Last reviewed: 2026-08-17

---

## Priority Index

### P0 — Critical (crashes / data loss)

| # | Title | Size |
|---|---|---|
| 17 | [ACP Stability Hardening](17-acp-stability-hardening.md) | L (5-7d) |

### P1 — High (direct cost/quality impact)

| # | Title | Size |
|---|---|---|
| 03 | [Context Injection Scoping](03-context-injection-scoping.md) | M (2-3d) |
| 04 | [Compile Auto-Fix Path](04-compile-autofix-path.md) | S (1-2d) |
| 18 | [ACP Spec Upgrade & Refactor](18-acp-spec-upgrade-and-refactor.md) | XL (2-3w) |

### P2 — Medium (efficiency / UX / maintainability)

| # | Title | Size |
|---|---|---|
| 01 | [T0 Reflex Store](01-t0-reflex-store.md) | M (2-3d) |
| 02 | [Reactive Agent Mode](02-reactive-agent-mode.md) | L (3-5d) |
| 05 | [Express Mode](05-express-mode.md) | M (2-3d) |
| 10 | [Daimon TUI View](10-daimon-tui-view.md) | S (1-2d) |
| 12 | [E2E Test Harness](12-e2e-test-harness.md) | M (2d) |
| 13 | [Historical Cost Calibration](13-historical-cost-calibration.md) | S (1d) |
| 14 | [Plan Mutation Protocol](14-plan-mutation-protocol.md) | M (2-3d) |
| 15 | [Post-Gate Reflection](15-post-gate-reflection.md) | M (2-3d) |

### P3 — Low (nice-to-have / Phase 2+)

| # | Title | Size |
|---|---|---|
| 09 | [Recursive Safety Patterns](09-recursive-safety-patterns.md) | L (4-5d) |
| 11 | [Justfile](11-justfile.md) | XS (½d) |
| 16 | [Warm Agent Spawning](16-warm-agent-spawning.md) | M (2-3d) |

---

## Removed Items (already implemented)

The following items from the original backlog are fully implemented and have been
removed from the active index:

| # | Title | Status |
|---|---|---|
| 06 | Output Budgeting | Implemented in `crates/roko-gateway/src/output_budget.rs` |
| 07 | Inference Cache L1/L2 | Implemented in `crates/roko-gateway/src/cache.rs` |
| 08 | Key Rotation | Implemented in `crates/roko-gateway/src/provider.rs` |

---

## Status Notes

Items 04 (Compile Auto-Fix) and 05 (Express Mode) have partial implementations —
types exist in `roko-gate/` but the final wiring pieces are incomplete. The specs
detail exactly what exists vs. what's missing.

Item 15 (Post-Gate Reflection) has scaffolding (store, dedup, injection) but the
actual LLM call is replaced by deterministic pattern synthesis.

Item 16 (Warm Agent Spawning) has the `WarmPool` container and integration points
but inserts placeholder structs instead of real pre-spawned processes.
