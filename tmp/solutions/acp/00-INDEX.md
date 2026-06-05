# ACP Solutions — Index

**Created:** 2026-04-28
**Purpose:** Audit findings + implementation plan for full-fledged ACP integration

> **2026-05-01 update:** Most items here have shipped. For the **live tracker** of what's still open, see [`REMAINING.md`](REMAINING.md). The documents below are kept as historical reference; do not use them as a live to-do list.

## Documents

| Doc | What |
|-----|------|
| [01-CURRENT-STATE.md](01-CURRENT-STATE.md) | What exists, what works, what's missing |
| [02-GAP-ANALYSIS.md](02-GAP-ANALYSIS.md) | Detailed gap matrix vs other runtimes |
| [03-ARCHITECTURE-PLAN.md](03-ARCHITECTURE-PLAN.md) | How to integrate properly (shared, extensible) |
| [04-MEGA-PARITY-OVERLAP.md](04-MEGA-PARITY-OVERLAP.md) | What mega-parity already covers vs ACP needs |
| [05-TASK-BATCHES.md](05-TASK-BATCHES.md) | Concrete tasks to bundle into mega-parity run |
| [06-MEGA-PARITY-INTEGRATION.md](06-MEGA-PARITY-INTEGRATION.md) | TOML entries, DAG, anti-patterns for mega-parity |
| [07-UX-GAP-ANALYSIS.md](07-UX-GAP-ANALYSIS.md) | Element-by-element screenshot parity analysis |
| [08-NOVEL-WORKFLOWS.md](08-NOVEL-WORKFLOWS.md) | 10 novel workflows that make roko best-in-class |
| [09-NOVEL-BATCHES.md](09-NOVEL-BATCHES.md) | Concrete batches for novel workflows (20 total) |

## TL;DR

**ACP is 100% implemented at the protocol level** (7,168 LOC, 41 tests, 49 slash commands,
multi-model dispatch, 3 workflow templates). The acp-runner successfully generated batches
ACP01–ACP08 in its final run. The architecture runner (wp-arch2) then wired it to WorkflowEngine.

**What's missing is subsystem integration + UX richness.** ACP operates as an isolated silo:
- Learning (no episodes, no cascade router feedback)
- Safety (no contracts, no role auth, no permission dialogs)
- Knowledge (no neuro queries, static system prompts, no knowledge cards)
- Tool dispatch (direct CLI subprocess, no MCP routing)
- Budget (cost fields always 0, UsageUpdate never emitted)
- UX (no phase badges, no narrative text, no iteration tracking, no context resolution)

**The fix is wiring, not building.** All the subsystems exist. ACP just needs connectors.
This is the same "built but never connected" pattern the CLAUDE.md warns about.

## Design Principles for Integration

1. **Share, don't duplicate** — Use existing `roko-compose`, `roko-learn`, `roko-agent` instead of ACP-local implementations
2. **Trait-based extension** — ACP adapter already implements `EventConsumer`; add `FeedbackSink`, `SafetyAware`
3. **Incremental wiring** — Each batch connects one subsystem; no big-bang refactor
4. **Test at boundary** — Integration tests verify cross-crate connections
