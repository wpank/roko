# Roko + Korai: Unified Architecture PRDs

## Document Set

### PRD Documents (Architecture & Design)

| # | Document | Lines | Topic |
|---|---|---|---|
| 1 | [PRD-01-OVERVIEW.md](PRD-01-OVERVIEW.md) | 1,047 | What is Roko/Korai, thesis, current state audit, 3 workstreams, glossary, 49 citations |
| 2 | [PRD-02-AGENT-RUNTIME.md](PRD-02-AGENT-RUNTIME.md) | 2,514 | Heartbeat, extensions, type-state, unified narrative (end-to-end flow), inference gateway, perf targets |
| 3 | [PRD-03-COGNITIVE-ENGINE.md](PRD-03-COGNITIVE-ENGINE.md) | 1,323 | T0/T1/T2 gating, native harness vs Claude CLI, 5 blue ocean features, inference gateway, triage |
| 4 | [PRD-04-CONTEXT-ENGINEERING.md](PRD-04-CONTEXT-ENGINEERING.md) | 1,144 | CognitiveWorkspace, VCG (9 bidders inc. WorldGraph), InsightStore integration, 3 feedback loops |
| 5 | [PRD-05-KNOWLEDGE-AND-STIGMERGY.md](PRD-05-KNOWLEDGE-AND-STIGMERGY.md) | 1,893 | Neuro, HDC 6 levels, 7 Korai gaps, 7-layer publishing defense, geometric privacy, measurement |
| 6 | [PRD-06-DOMAINS-AND-ARENAS.md](PRD-06-DOMAINS-AND-ARENAS.md) | 2,639 | 28-arena catalog, HuggingFace 5 layers, SWE-bench native crate, Knowledge Futures, work markets |
| 7 | [PRD-07-ISFR-AND-INSTRUMENTS.md](PRD-07-ISFR-AND-INSTRUMENTS.md) | 1,866 | ISFR, yield perps, clearing, Korai gaps, EventFabric integration, multi-chain sources, agent roles |
| 8 | [PRD-08-DEPLOYMENT-AND-UX.md](PRD-08-DEPLOYMENT-AND-UX.md) | 2,069 | CLI (25 DX improvements), Pi package CLI, persistent chat (WorldGraph), Agent/AI Studio, OpenClaw |
| 9 | [PRD-09-EXTENSIBILITY-AND-MULTICHAIN.md](PRD-09-EXTENSIBILITY-AND-MULTICHAIN.md) | 4,693 | Pi compat, packages, multi-chain, foraging, WorldGraph, HF integration, synergistic scaling |
| 10 | [PRD-10-DASHBOARD-AND-TUI.md](PRD-10-DASHBOARD-AND-TUI.md) | 3,386 | Dashboard + TUI unified surfaces, Nexus relay, auth, page catalog (8 sections ×30 pages with wireframes), jobs/bounties, roko stabilization requirements, demo plan |

### Implementation Plans (Granular Checklists)

| # | Document | Tasks | Topic |
|---|---|---|---|
| 1 | [IMPL-01-RUNTIME.md](IMPL-01-RUNTIME.md) | 30 | Agent runtime extraction from orchestrate.rs |
| 2 | [IMPL-02-COGNITIVE-ENGINE.md](IMPL-02-COGNITIVE-ENGINE.md) | 21 | Cognitive gating, prediction error, somatic integration |
| 3 | [IMPL-03-CONTEXT.md](IMPL-03-CONTEXT.md) | 18 | CognitiveWorkspace, VCG wiring, feedback loops |
| 4 | [IMPL-04-KNOWLEDGE.md](IMPL-04-KNOWLEDGE.md) | 19 | HDC wiring, PP-HDC, dreams, InsightStore queries |
| 5 | [IMPL-05-DOMAINS.md](IMPL-05-DOMAINS.md) | 20 | Domain profiles, arenas, blockchain/research extensions |
| 6 | [IMPL-06-ISFR.md](IMPL-06-ISFR.md) | 28 | ISFR oracle, yield perps, cooperative clearing |
| 7 | [IMPL-07-CHAIN.md](IMPL-07-CHAIN.md) | 21 | Kauri BFT, SpecPool EVM, precompiles, InsightStore |
| 8 | [IMPL-08-SURFACES.md](IMPL-08-SURFACES.md) | 30 | CLI, chat, TUI, Agent/AI Studio, OpenClaw, MCP |
| 9 | [IMPL-09-EXTENSIBILITY-AND-MULTICHAIN.md](IMPL-09-EXTENSIBILITY-AND-MULTICHAIN.md) | 66 | Package system, Pi compat, multi-chain, foraging, WorldGraph |
| 10 | [IMPL-10-DASHBOARD-AND-TUI.md](IMPL-10-DASHBOARD-AND-TUI.md) | 43 tasks | 3 workstreams (roko stabilization, dashboard rewrite, TUI enhancements), 6 phases, Nexus relay, jobs backend, auth, bardo widget ports |
| 10D | [IMPL-10-DEMO.md](IMPL-10-DEMO.md) | 25 tasks | 3-day sprint: Stream A (dashboard full rewrite, 9 tasks), Stream B (backend stabilization + jobs, 10 tasks), Stream C (TUI enhancements, 6 tasks) |

**Totals: 10 PRDs + 11 IMPLs = 21 documents, ~45,400 lines, 400+ implementation tasks**

## Reading Order

1. **PRD-01** — Orientation (what, why, glossary)
2. **PRD-02** — Agent runtime (the core of everything)
3. **PRD-03** — Cognitive gating (the cost innovation)
4. **PRD-04** — Context engineering (the quality innovation)
5. **PRD-05** — Knowledge/stigmergy (the collective intelligence innovation)
6. **PRD-06** — Domains/arenas (how agents specialize)
7. **PRD-07** — ISFR/instruments (the financial primitives)
8. **PRD-08** — Deployment/UX (what users see)
9. **PRD-09** — Extensibility/multi-chain (ecosystem and chain architecture)
10. **PRD-10** — Dashboard & TUI (unified surfaces, Nexus relay, jobs, demo)

Then IMPL-01 through IMPL-10 for implementation details. IMPL-10-DEMO is the fast-track for Thursday's demo.

## Key Architectural Decisions

1. **Universal runtime, domain profiles** — One AgentRuntime handles all domains. Profiles control frequency, extensions, gates, tools.
2. **Extension-based composition** — 22 hooks across 8 layers. Behavior comes from extensions, not monolithic code.
3. **Cognitive gating** — 80% of ticks cost $0 (T0). Only novel situations escalate to LLM.
4. **Learnable context** — VCG auction, section effect tracking, 3 feedback loops. Prompt quality improves autonomously.
5. **Type-state lifecycle** — Invalid agent states are compile errors.
6. **Pi-compatible package system** — `roko install npm:pi-package` works. Roko adds cognitive-native packages.
7. **Actor-per-chain** — Each blockchain gets its own actor, feeding a canonical event bus. Hierarchical temporal windows.
8. **Predictive foraging** — Gittins index attention allocation. Dynamic contract discovery. WorldGraph evolves through perception.
9. **Stigmergic intelligence** — InsightStore on Korai. Knowledge compounds across agents. PP-HDC privacy.
10. **ISFR as generalized benchmark pattern** — Multi-source, dual-median, validator-computed. Extensible to non-financial indices.

## Key Sources

### Internal
- Blue ocean paper: `papers/new/blue-ocean/` (28 chapters)
- Korai litepaper: `papers/new/litepaper/` (16 chapters)
- ISFR spec: `papers/new/isfr-rewrite/isfr-index-spec.md`
- Bardo PRDs: `bardo-backup/prd/` (359 files)
- Mori agent docs: `bardo-backup/tmp/mori-agents/` (19 docs)

### Academic (49 citations in PRD-01, additional in PRD-09)
See PRD-01 §10 for full bibliography.
