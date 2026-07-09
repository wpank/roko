# Implementation Plans — Master Index

> **Updated**: 2026-04-08
>
> **Single source of truth**: [`../MASTER-PLAN.md`](../MASTER-PLAN.md) — the new master plan
> that supersedes MASTER-REMAINING-WORK.md, PROMPT-EXECUTOR-PARITY.md, and plans 07-10.
>
> This index lists all plan files for reference. The MASTER-PLAN.md defines the
> tier/section structure and tracks all items.

---

## Active Plans

| # | Plan | Priority | Master Plan Tier | Status |
|---|------|----------|-----------------|--------|
| MR | [Model Routing](modelrouting/00-INDEX.md) | 🔴 P0 | Tier 2 | Not started |
| 11 | [Agent Dogfooding](11-agent-dogfooding.md) | 🔴 P0 | Tiers 2-4 | Not started |
| 11-phases | [Phase 0-1](11-sections/phase-0-1.md), [Phase 2](11-sections/phase-2.md), [Phase 3-4](11-sections/phase-3-4.md), [Phase 5-6](11-sections/phase-5-6.md), [Phase 7-8](11-sections/phase-7-8.md) | — | Detailed specs | Not started |
| 12a | [Cognitive Layer](12a-cognitive-layer.md) | 🟢 P3 | Tier 5 | Not started |
| 12b | [Chain Layer](12b-chain-layer.md) | 🟢 P3 | Tier 6 | Deferred |

## Completed Plans (files retained for reference)

| # | Plan | Completed |
|---|------|-----------|
| 01 | Agent Wiring | ✅ Agent dispatch wired, safety layer integrated |
| 02 | Session Persistence | ✅ Snapshot/resume, state machine |
| 03 | PRD Workflow | ✅ idea→draft→plan→execute pipeline |
| 04 | System Prompts | ✅ 6-layer builder, role templates |
| 05 | [Learning Wiring](05-learning-wiring.md) | ✅ Efficiency, cascade, experiments, adaptive gates |
| 06 | Research Agent | ✅ topic, enhance-prd, enhance-plan, enhance-tasks, analyze |

## Superseded Plans (content absorbed into MASTER-PLAN.md)

| # | Plan | Superseded By |
|---|------|--------------|
| 07 | [MCP & Tool Registry](07-mcp-tool-wiring.md) | MASTER-PLAN.md Tier 1C + Tier 2D |
| 08 | [Observability](08-observability-wiring.md) | MASTER-PLAN.md Tier 1D |
| 09 | [TUI & Dashboard](09-tui-dashboard.md) | MASTER-PLAN.md Tier 1H |
| 10 | [Golem Integration](10-golem-integration.md) | 12b-chain-layer.md (Tier 6) |
| 12 | [Nunchi Integration](12-nunchi-integration.md) | Split into 12a + 12b |

## Other Superseded Documents

| File | Superseded By |
|------|--------------|
| [`../MASTER-REMAINING-WORK.md`](../MASTER-REMAINING-WORK.md) | MASTER-PLAN.md (all sections A-K) |
| [`../PROMPT-EXECUTOR-PARITY.md`](../PROMPT-EXECUTOR-PARITY.md) | MASTER-PLAN.md (all sections 1-11) |

## Still-Active Reference Documents (not superseded)

| File | Purpose |
|------|---------|
| [`../DESIGN-TASK-GENERATION.md`](../DESIGN-TASK-GENERATION.md) | Task decomposition philosophy |
| [`../v2163-effectiveness.md`](../v2163-effectiveness.md) | Agent harness effectiveness data |
| [`../run-parity.sh`](../run-parity.sh) | Orchestration script (rewritten for MASTER-PLAN.md) |
