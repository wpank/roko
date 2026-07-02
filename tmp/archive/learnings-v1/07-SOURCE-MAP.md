# Source File Map

> What lives where and what it contains. Reference for any session that needs to find specific content.

---

## Spec Layer (the authority)

| Path | Files | What's there | Status |
|---|---|---|---|
| `tmp/unified/` | 22 .md | **The protocol spec.** 3 fundamentals (Signal/Pulse, Cell, Graph), 9 protocols, 10 specializations. v2 rewritten from scratch with full research + source integration. Cell = renamed from Block. | **v2 complete** |
| `tmp/unified/UPDATE-PROMPT.md` | 1 | Prompt for rewriting the spec layer — lists all 60+ source docs, 33 updates across 7 categories, design principles, anti-principles | Reference |
| `tmp/unified/acp-integration-prd.md` | 1 | ACP (Agent Client Protocol) integration — maps ACP ↔ Roko, 5 session modes, slash commands, IDE distribution | Complete PRD |

## Research (7 rounds of deep research)

| Path | What it covers | Key findings |
|---|---|---|
| `tmp/research/research.md` | R1: Substrates | Field calculus (XC), parametric optics, CRDTs, event sourcing, QD + active inference, competitive landscape, empty quadrant |
| `tmp/research/research2.md` | R2: Algorithms | HGM CMP, AXIOM BMR, CaMeL IFC, HDC routing, scaling laws (ρ≈0.23), self-play (AZR/R-Zero), safety (Nasr 90%+ ASR), Verify-as-reward |
| `tmp/research/research3.md` | R3: Frontier integrations | ZK-HDC (Bionetta), MacNet 1000-agent scaling, AgentHER hindsight relabeling, PID c-factor, emergent communication, causal discovery, hardware co-design |
| `tmp/research/research4.md` | R4: Strategic positioning | Category creation (Sequoia/a16z/NFX), "Stripe for agents" framing, 5 compounding mechanisms, MCP playbook, marketplace economics (0%/$1M/12-15%), UX surfaces, failure patterns |
| `tmp/research/research5.md` | R5: Production reality | MCP adoption mechanics, real ARR (Cursor $2B, Claude Code $2.5B, Harvey $190M), HAL cost data ($32-59 naive, $1.42 optimized), 6-12 month window, regulatory (EU AI Act Aug 2), on-chain reality ($30-80K/day) |
| `tmp/research/research6.md` | R6: Series A intelligence | a16z partner map (Casado → Aubakirova → Dixon), 13-slide deck, 3-minute demo script, comps ($15-35M at $150-250M), 3 dangerous bear cases, last-14-days intel |
| `tmp/research/research7.md` | R7: Reality check | "Stripe for agents" taken by Stripe. 50ms blocks don't survive global scrutiny. Demurrage has no precedent. Repositioned to trust/identity. Named design partners (Cleric, Decagon, Harvey, Hebbia, Resolve.ai). ERC-8004 strategy. |

## Depth Layer (algorithms, research, implementation detail)

| Path | Files | What's there |
|---|---|---|
| `tmp/unified-depth/` | 22 dirs + 8 .md | Skeleton for depth docs. Each dir has INDEX.md mapping source docs. GUIDE.md explains structure. INGEST-PROMPT.md for feeding sources. |
| `tmp/unified-depth/RESEARCH-PROMPT-{1-7}.md` | 7 | Claude Desktop research prompts for rounds 1-8 (numbered off by one — prompt 1 was for R1-R2, etc.) |

## Learnings (self-contained briefings)

| Path | What it covers | Lines | Notes |
|---|---|---|---|
| `tmp/learnings/00-INDEX.md` | Reading order, key numbers, source map | ~110 | Updated through R7 |
| `tmp/learnings/01-ARCHITECTURE.md` | Full technical system from scratch | ~750 | Includes Nunchi blockchain section |
| `tmp/learnings/02-RESEARCH-SYNTHESIS.md` | 80+ papers, 15 topics | ~950 | Full citations |
| `tmp/learnings/03-STRATEGY-AND-PITCH.md` | Pitch, market, moat, GTM | ~1600 | R6+R7 intelligence, named partners, landing page as pitch |
| `tmp/learnings/04-IMPLEMENTATION-PRIORITIES.md` | Phase 0-4+ roadmap | ~800 | Includes blockchain phase, dashboard refocus |
| `tmp/learnings/05-RISKS-AND-ANTIPATTERNS.md` | Risks, failures, mitigations | ~750 | R7 reality checks included |
| `tmp/learnings/06-CONVERSATION-SUMMARY.md` | This session's full summary | ~200 | What happened, key decisions, key numbers |
| `tmp/learnings/07-SOURCE-MAP.md` | This file | ~200 | Where everything lives |
| `tmp/learnings/REWRITE-PROMPT.md` | Prompt for rewriting learnings from scratch | ~300 | Uses all sources |

## Source Documents (read during this session)

### Refinements (architectural redesign proposals)
| Path | Files | Key content |
|---|---|---|
| `tmp/refinements/` | 36 | "2 mediums + 2 fabrics" kernel redesign, predict-publish-correct, demurrage, heuristics with falsifiers, c-factor, 7 compounding loops, plugin SPI, StateHub projections, moat analysis |

### Visual-gate2 (verification redesign)
| Path | Files | Key content |
|---|---|---|
| `tmp/visual-gate2/` | 10 | EvidenceCollector ≠ Criterion, conjunctive hard + Pareto soft (never weighted-sum), pairwise Bradley-Terry judges, 6 anti-Goodhart safeguards, 7-step flywheel, DAW marketplace |

### Run-anywhere (deployment ubiquity)
| Path | Files | Key content |
|---|---|---|
| `tmp/run-anywhere/` | 22 | WASM compilation, Merkle-CRDT distributed learning, brain export (~100KB-1MB), ACP protocol, progressive enhancement (Tier 0-3), 3-tier edge/regional/central deployment |

### 04-21-26 (operational specs)
| Path | Files | Key content |
|---|---|---|
| `tmp/04-21-26/` | 117 | PRDs (10 + impl), generalizations (Golem vision, agent runtime, domain specialization, extension model, native harness), arenas (8 concrete + meta-arena), HDC deep integration (6 levels), knowledge publishing (7-layer defense), geometric sharing |

### DeFi gaps
| Path | Files | Key content |
|---|---|---|
| `tmp/defi/gap/` | 14 (574KB) | Pre-action verification, continuous rewards (P&L → f64), tick-driven heartbeat (2-5s gamma), multi-slot state, regime conditioning, VenueAdapter trait, prospect theory affect, DeFi dream triggers |

### Workflow specs
| Path | Files | Key content |
|---|---|---|
| `tmp/workflow/` | 12 | Module/Workflow/Artifact/Macro/Slot primitives (became Cell/Graph/Signal/Macro/Slot), execution engine, trigger system, visual config wizard, marketplace |

### Architecture specs
| Path | Files | Key content |
|---|---|---|
| `tmp/architecture/` | 21 | Agent runtime, extensions (22 hooks/8 layers), connectivity/relay, feeds, knowledge/pheromones, meta-agents, registries, deployment |

### Nunchi blockchain
| Path | Files | Key content |
|---|---|---|
| `docs/08-chain/` | 25 (8K lines) | Chain spec (Simplex consensus, 400ms blocks), HDC precompile (~400 gas), ERC-8004 agent identities, token economics, ERC-8004 registries, 4-tier gossip, ERC-8183 job market, 7-domain reputation, x402 micropayments, ISFR clearing, mirage-rs EVM simulator |

### Core docs (deep system specs)
| Path | Files | Key content |
|---|---|---|
| `docs/` | 422 (8.8MB) | 22 sections: architecture, orchestration, agents, composition, verification, learning, neuro, conductor, chain, daimon, dreams, safety, interfaces, coordination, identity-economy, code-intelligence, heartbeat, lifecycle, tools, deployment, technical-analysis, references |

### Dashboard
| Path | What |
|---|---|
| `/Users/will/dev/nunchi/nunchi-dashboard/` | Landing page (7 narrative sections, ROSEDUST design) + app dashboard (27+ pages, 7 sidebar sections) |
| `nunchi-dashboard/tmp/prds/` | 5 PRD bundles (360-item spec, strategy) |
| `nunchi-dashboard/tmp/ux-refresh-context/` | 6 docs (current state audit, optimal redesign, UX research: dopamine, foraging, composability) |

### Demo resources
| Path | What |
|---|---|
| `demo/demo-resources/` | Demo scripts: agent-workflows, chain-coordination, benchmark-flow, coding-agent-benchmarks, provider-routing, full-self-hosting, agent-matchmaking |
