# run-anywhere/ — Design Specification Encyclopedia

**Directory**: `tmp/run-anywhere/`
**Status**: DONE — comprehensive reference, no action items
**Files**: 24 markdown docs (~600KB total)

## What This Is

NOT a deployment/distribution system. This is a **comprehensive design specification** of the complete Roko architecture — documenting what exists and what's planned for Phase 2+.

## Document Inventory

### Core Architecture (Implemented)

| File | Scope | Codebase Status |
|------|-------|-----------------|
| 01-agent-architecture.md | 1-noun-6-verb pattern, CoALA, 3-substrate memory | Wired |
| 05-runtime-tools-extensions.md | Agent runtime, 16 tools, MCP, 28 roles, safety | Wired |
| 08-inference-and-context.md | T0/T1/T2 gating, 8-layer context, CascadeRouter | Wired |
| 10-orchestration-and-build.md | Plan-execute-gate-persist, enrichment, DAG | Wired |
| 13-orchestration-parallel-execution.md | File-conflict DAG, worktrees, merge queue | Wired |
| 14-orchestration-context-engine.md | 9-layer context engine | Wired |
| 15-orchestration-quality-gates.md | 7-rung gates, AutoFixer, Reflexion | Wired |
| 17-connection-backends.md | LLM provider abstraction | Wired |
| 19-self-improvement-systems.md | Learning loops, efficiency tracking | Wired |

### Strategy & Positioning (Reference)

| File | Scope |
|------|-------|
| 02-whats-novel.md | Competitive differentiation vs LangChain/Mem0/CrewAI/Cursor |
| 03-use-cases-and-niches.md | 6 tiers, 6 market niches |
| 04-research-foundations.md | 60+ academic papers mapped to mechanisms |
| 16-benchmarks-and-evals.md | Testing framework design |
| 20-architectural-theory.md | High-level principles |

### Phase 2+ Specifications (Not Yet Implemented)

| File | Scope | Status |
|------|-------|--------|
| 06-blockchain-intelligence.md | Korai Ledger, custom EVM, PredictionEngine | Phase 2 |
| 07-cognitive-engine.md | Heartbeat, affect engine, dreams, daimon | Partially implemented |
| 09-skills-and-evolution.md | Pi Skills, population learning, Baldwin effect | Partially implemented |
| 11-continuous-chain-intelligence.md | Active inference, triage, Viable Systems Model | Phase 2 |
| 12-oneirography-and-art.md | NFT art from cognitive state, dream journals | Phase 2 |
| 18-service-integrations.md | External service integrations | Partial |
| wasm-and-vision.md | WASM compilation, visual integration | Phase 2 |
| ide.md | ACP (Agent Client Protocol) IDE integration | Phase 2 |
| 10-hdc-technical-analysis.md | HDC/VSA technical details | Research |

## No Remaining Action

These are reference specifications. No checklist items — the documents accurately describe what's wired vs. what's Phase 2+.

**Source files**: `tmp/run-anywhere/*.md`
