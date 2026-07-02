# Conversation Summary — April 25-26, 2026

> What was accomplished in this Claude Code session and what context a new session needs.

---

## What happened in this session

### Phase 1: Unified Spec Creation (22 docs from scratch)

Started with three overlapping spec sets:
- `tmp/workflow/` (12 PRDs, ~223KB) — workflow subsystem
- `tmp/architecture/` (21 docs, ~70K lines) — architecture specs
- `docs/` (422 files, 8.8MB) — deep per-system specs

**Created** `tmp/unified/` — 22 spec documents reducing 36 overlapping concepts to 12 core (3 fundamentals + 9 protocols). Used a team of 5 parallel writing agents. Total: 572KB.

### Phase 2: Depth Layer Structure

**Created** `tmp/unified-depth/` — skeleton for depth docs organized by spec doc number. 22 directories with INDEX.md files mapping 372 source docs from `docs/` to their depth directories. Includes GUIDE.md, INGEST-PROMPT.md, and multiple research prompts.

### Phase 3: Research Integration (7 rounds)

Read and synthesized 7 research documents (now at `tmp/research/research{1-7}.md`):
- **R1**: Substrates (field calculus, parametric optics, CRDTs, event sourcing, QD/active inference)
- **R2**: Algorithms (HGM CMP, AXIOM BMR, CaMeL IFC, HDC routing, scaling laws, self-play, safety)
- **R3**: Frontier integrations (ZK-HDC, MacNet 1000-agent scaling, AgentHER hindsight, Verify-as-reward, PID c-factor)
- **R4**: Strategic positioning (category creation, VC theses, market sizing, competitive moat, MCP playbook, 5 compounding mechanisms, 5 UX surfaces, marketplace economics, failure patterns)
- **R5**: Production reality (MCP adoption playbook, real ARR data, 6-12 month window, regulatory cliffs, HAL cost benchmarks, 10K-agent reality check)
- **R6**: Series A intelligence (a16z partner map — Casado/Aubakirova/Dixon, 13-slide deck, 3-minute demo, bear cases, comps $15-35M at $150-250M, last-14-days intel)
- **R7**: Reality check (Stripe owns "Stripe for agents," 50ms blocks don't survive global scrutiny, demurrage has no precedent, repositioned pitch to trust/identity layer, named design partners, ERC-8004 strategy)

### Phase 4: Additional Source Ingestion

Read and analyzed these additional source sets:
- `tmp/refinements/` (36 docs) — proposed "2 mediums + 2 fabrics + 6 operators" kernel redesign, demurrage, heuristics with falsifiers, c-factor, 7 compounding loops, predict-publish-correct
- `tmp/visual-gate2/` (10 docs) — verification redesign: evidence typing, conjunctive hard + Pareto soft, Bradley-Terry judges, 7-step flywheel
- `tmp/run-anywhere/` (22 docs) — WASM compilation, Merkle-CRDT sync, brain export, ACP protocol, progressive enhancement
- `tmp/04-21-26/` (117 docs) — PRDs, generalizations (Golem vision), arenas, HDC deep integration, knowledge publishing, geometric sharing
- `tmp/defi/gap/` (14 docs, 574KB) — DeFi-specific requirements, real-time patterns, pre-action verification, continuous rewards
- `docs/08-chain/` (25 docs) — Nunchi blockchain: HDC precompile, ERC-8004 agent identities, token economics, ERC-8183 job market, 7-domain reputation, gossip architecture
- `tmp/workflow/` (12 docs) — workspace subsystem, visual config wizard, execution engine, doc-ingest example
- `tmp/unified/acp-integration-prd.md` — ACP (Agent Client Protocol) for IDE integration

### Phase 5: Unified Spec v2 Redesign

Created comprehensive UPDATE-PROMPT.md (now REWRITE-PROMPT) with 33 updates across 7 categories:
- **A-G**: 7 fundamental kernel upgrades (two mediums Signal+Pulse, Bus as kernel fabric, predict-publish-correct, demurrage, heuristics with falsifiers, c-factor, Verify redesign per visual-gate2)
- **1-12**: 12 major additions (mortality/vitality, type-state lifecycle, CorticalState atomics, learnable context CognitiveWorkspace, hot Graph, EFE routing, regime conditioning, hindsight relabeling, L4 self-evolution with CMP, CaMeL security, arenas, domain profiles)
- **13-20**: 8 structural additions (multi-slot state, multi-chain temporal, package ecosystem, workspace scoping, StateHub projections, WASM/brain export, marketplace composability, somatic markers)
- **H-N**: 7 strategic upgrades (protocol-first framing, cost as principle, 5 compounding mechanisms, exoskeleton protocols, 5 UX surfaces, marketplace economics, spec-as-runtime-artifact)

The unified docs were then rewritten from scratch using this prompt. "Block" was renamed to "Cell" as a primitive.

### Phase 6: Learnings Creation

Created `tmp/learnings/` with 5 self-contained documents:
- `01-ARCHITECTURE.md` (~700 lines) — full technical system
- `02-RESEARCH-SYNTHESIS.md` (~950 lines) — 80+ papers across 15 topics
- `03-STRATEGY-AND-PITCH.md` (~1500+ lines) — pitch narrative, market, moat, go-to-market
- `04-IMPLEMENTATION-PRIORITIES.md` (~800 lines) — Phase 0-4+ roadmap
- `05-RISKS-AND-ANTIPATTERNS.md` (~700 lines) — risks with citations, mitigations

These were iteratively updated with research6 (a16z intelligence), research7 (reality check/repositioning), Nunchi blockchain details, dashboard/product context, and named design partners.

### Phase 7: Dashboard Assessment

Reviewed the Nunchi dashboard (landing page + app):
- **Landing page**: 7 narrative sections (Loop, Scaffold, Anatomy, Memory, Collective, Chain, Proof) with distinctive ROSEDUST dark aesthetic. Functions as pitch deck.
- **App dashboard**: 27+ pages across 7 sections (PULSE, FLEET, FORGE, KNOWLEDGE, ARENA, MEASUREMENTS, TREASURY). Too busy for a pitch demo — needs refocusing.
- Key finding: the side-by-side cost comparison and live chain view already exist. Need sharpening with HAL numbers.
- Dashboard UX docs and PRDs read from `nunchi-dashboard/tmp/`

### Key Decisions Made

1. **"Nunchi"** is the canonical name (project + blockchain). "Roko" is the agent runtime.
2. **"Cell"** is the renamed primitive (was "Block" — renamed to avoid confusion with blockchain blocks)
3. **50ms blocks** via Hyperliquid-style validator clustering (honest about geography)
4. **Demurrage dropped** from the token (no successful precedent at scale)
5. **"Stripe for agents" retired** (Stripe literally built it). New positioning: "the trust layer for the agent economy"
6. **Pitch thesis**: "The model is the same. The system is the variable." (from landing page)
7. **ACP** added as 5th exoskeleton protocol (alongside MCP, A2A, ERC-8004, x402)
8. **Roko runtime is open-source**. Business model TBD (OSS + chain + managed cloud hybrid)

---

## Key numbers to carry forward

| Metric | Value | Source |
|---|---|---|
| Coordination failure rate | 41-86% | MAST, NeurIPS 2025 |
| Failures from coordination | 79% | MAST |
| HAL naive agent cost | $32-59/task | SWE-bench Verified Mini |
| HAL optimized cost | ~$1.42/task | With caching + routing |
| Cost reduction claim | 10-30× | Cache 5× + Route 3× + Gate 2× |
| ERC-8004 active agents | ~80-150K | 3 months post-mainnet |
| MCP downloads | 97M/month | Linux Foundation |
| Window before lock-in | 6-12 months | MCP/A2A/ERC-8004 convergence |
| EU AI Act enforcement | August 2, 2026 | ~97 days away |
| Series A target | $20-30M at $200-400M | Research6 comp analysis |
| Closest funded analog | Nava $8.3M | Polychain + Archetype |
| Machine:human identity | 82:1 to 144:1 | CyberArk / Entro 2025 |
| ZK-HDC proving | <1 second | Laptop, Circom + Groth16 |
| Cursor ARR | ~$2B | Founder disclosure |
| Claude Code run rate | ~$2.5B | Anthropic |

---

## Pitch positioning (final)

**Retired**: "Stripe for the agent economy" (Stripe literally built Stripe for agents)

**Current**: "Nunchi is the identity, reputation, and verifiable-similarity layer for the agent economy. First production ZK-HDC primitive. First ERC-8004 agent identity system. The trust layer that EU AI Act Article 50 mandates."

**Thesis statement** (from landing page): "The model is the same. The system is the variable."

**Network effect**: "The thousandth agent joins smarter than the first."
