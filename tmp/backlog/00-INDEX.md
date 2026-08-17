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
| 21 | [Landing Page Fake Metrics](21-landing-page-fake-metrics.md) | S (½d) |
| 33 | [CLI Gist Scrubbing](33-cli-gist-scrubbing.md) | S (½d) |
| 45 | [ACP Tool Permission Gate](45-acp-tool-permission-gate.md) | M (1-2d) |
| 56 | [ACP Single-Agent Chat: Tools Require Client Capability Declaration](56-acp-single-agent-tools.md) | M (1-2d) |
| 48 | [Serve Auth Default Posture](48-serve-auth-default.md) | S (½d) |
| 49 | [Serve CORS Restrictive](49-serve-cors-restrictive.md) | S (½d) |
| 50 | [Serve Rate and Body Limits](50-serve-rate-body-limits.md) | S (1d) |
| 51 | [Serve Agent Name Validation](51-serve-path-traversal.md) | S (½d) |

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
| 20 | [Event Loop Decomposition](20-event-loop-decomposition.md) | XL (2-3w) |
| 22 | [Chat Inline Decomposition](22-chat-inline-decomposition.md) | M (2-3d) |
| 34 | [PRD Cascade Learning](34-prd-cascade-learning.md) | S (1d) |
| 35 | [CLI Output Redesign](35-cli-output-redesign.md) | M (2-3d) |
| 37 | [Multi-Process Locking](37-multi-process-locking.md) | S (1d) |
| 38 | [Provider Error UX](38-provider-error-ux.md) | S (1d) |
| 39 | [ACP Learning-Pipeline Parity](39-learning-pipeline-acp-parity.md) | M (1-2d) |
| 40 | [Gate Rung Input Completion](40-gate-rung-input-completion.md) | S (1d) |
| 43 | [Clippy Suppression Removal](43-clippy-suppression-removal.md) | M (1-2d) |
| 44 | [Calibration Feedback Loop](44-calibration-feedback-loop.md) | M (2d) |
| 46 | [ACP Test Coverage](46-acp-test-coverage.md) | S (1d) |
| 47 | [ConfigLayer Elimination](47-configlayer-elimination.md) | L (3-5d) |
| 52 | [MCP Stderr Capture & CostTable Gaps](52-mcp-stderr-costtable.md) | S (1d) |
| 53 | [Immune System Adaptive Screening](53-immune-adaptive-screening.md) | L (5-7d) |
| 54 | [Graph Engine Runner-v2 Parity](54-graph-engine-runner-parity.md) | XL (3-4w) |
| 55 | [AgentPool Runtime Integration](55-agent-pool-runtime-integration.md) | M (2-3d) |
| 57 | [Plan Generation Escalation](57-plan-generation-escalation.md) | S (1d) |
| 58 | [Performance Hot-Path Fixes](58-perf-hot-path-fixes.md) | M (2-3d) |

### P3 — Low (nice-to-have / Phase 2+)

| # | Title | Size |
|---|---|---|
| 09 | [Recursive Safety Patterns](09-recursive-safety-patterns.md) | L (4-5d) |
| 11 | [Justfile](11-justfile.md) | XS (½d) |
| 16 | [Warm Agent Spawning](16-warm-agent-spawning.md) | M (2-3d) |
| 19 | [Contextual Bandit Dead Code](19-contextual-bandit-dead-code.md) | XS (½d) |
| 41 | [TUI Push-Mode Panel Data](41-tui-push-mode-panel-data.md) | S (1d) |
| 42 | [Duplicate Type Consolidation](42-duplicate-type-consolidation.md) | S (1d) |
| 59 | [HuggingFace Provider](59-huggingface-provider.md) | S (½d) |

---

## Removed Items (already implemented)

The following items from the original backlog are fully implemented and have been
removed from the active index:

| # | Title | Status |
|---|---|---|
| 06 | Output Budgeting | Implemented in `crates/roko-gateway/src/output_budget.rs` |
| 07 | Inference Cache L1/L2 | Implemented in `crates/roko-gateway/src/cache.rs` |
| 08 | Key Rotation | Implemented in `crates/roko-gateway/src/provider.rs` |
| 36 | Atomic File I/O | Implemented in `crates/roko-fs/src/atomic.rs`; all runner/learn persistence paths use `atomic_write` |

---

## Status Notes

Items 04 (Compile Auto-Fix) and 05 (Express Mode) have partial implementations —
types exist in `roko-gate/` but the final wiring pieces are incomplete. The specs
detail exactly what exists vs. what's missing.

Item 15 (Post-Gate Reflection) has scaffolding (store, dedup, injection) but the
actual LLM call is replaced by deterministic pattern synthesis.

Item 16 (Warm Agent Spawning) has the `WarmPool` container and integration points
but inserts placeholder structs instead of real pre-spawned processes.

Item 46 (ACP Test Coverage) is partial — Gap 1 (stdin-EOF clean exit) is resolved;
Gaps 2-3 (MCP crash surfacing, cross-provider tool matrix) remain open.
