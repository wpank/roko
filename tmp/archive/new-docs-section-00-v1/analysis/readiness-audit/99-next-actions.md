---
title: "Readiness Audit: Next Actions"
section: analysis
subsection: readiness-audit
id: ra-99
source: 31-implementation-readiness-audit.md (§Prioritized Gap List, §Recommended Execution Order)
tags: [gaps, next-actions, prioritized, G1-G33, execution-order]
---

# Readiness Audit: Next Actions

> **All 33 gaps from the readiness audit, organized by tier. Every gap is individually citable.**

## Tier 0 — Critical Path to Self-Hosting (~4 person-weeks)

| # | Gap | Section | Complexity | Impact | Blocking |
|---|---|---|---|---|---|
| **G1** | Wire ToolDispatcher into orchestrate.rs | 02, 11 | Medium | Safety coverage jumps 30%→100% | Safety layer is dead code |
| **G2** | Wire roko-index into ContextProvider | 15 | Medium | Code-aware context assembly | Agents lack code intelligence |
| **G3** | Register lang providers in detect_polyglot | 15 | Low | Multi-language symbol extraction | roko-index has no lang support |
| **G4** | Expand role prompts from ~20 to ~2,000 tokens | 02 | Medium | Largest single performance lever | Harness quality < model quality |
| **G5** | Implement somatic landscape + VCG affect bidding | 09 | Medium | Complete Daimon control loop | Affect lacks somatic lookup and full auction |

## Tier 1 — High-Leverage Improvements (~8 person-weeks)

| # | Gap | Section | Complexity | Impact |
|---|---|---|---|---|
| **G6** | Implement active inference EFE scorer | 03 | Medium-High | Replace static SectionScorer |
| **G7** | Wire 8 missing feedback loops | 05 | Medium (~495 LOC) | Close the learning loop |
| **G8** | Execute Signal→Engram rename | 00 | Low (find/replace) | Eliminate spec/code divergence |
| **G9** | Wire PAD persistence to `.roko/daimon/affect.json` | 03, 09 | Low-Medium | Cross-session emotional continuity |
| **G10** | Add tier field to `KnowledgeEntry` | 06 | Low | Enable tier-weighted decay |
| **G11** | Fix half-life constants (30d defaults → spec values) | 06 | Low | Correct knowledge decay rates |
| **G12** | Consolidate roko-daimon + roko-golem/daimon.rs | 09 | Medium | Prerequisite for daimon features |
| **G13** | Close safety critical integration gap | 11 | Medium | Activate all 6 safety guards |
| **G14** | Create Dockerfiles + fly.toml | 19 | Low (2-3 days) | Enable cloud deployment |
| **G15** | Implement Mattar-Daw utility scoring | 10 | Medium | Enable NREM replay prioritization |

## Tier 2 — Feature Enrichment (~20 person-weeks)

| # | Gap | Section | Complexity | Impact |
|---|---|---|---|---|
| **G16** | VCG Attention Auction | 03, 16 | High | Optimal context allocation |
| **G17** | MVT Predictive Foraging | 03 | Medium-High | Reduce unnecessary source queries |
| **G18** | Somatic landscape k-d tree | 09 | High | Affect-aware strategy retrieval |
| **G19** | Pheromone types in roko-core | 13 | Low | Foundation for coordination |
| **G20** | CodeIndex trait + search API | 15 | Medium | Unified code intelligence API |
| **G21** | Oracle trait + PredictionStore | 20 | Medium | Enable prediction loop |
| **G22** | MCP servers (GitHub, Slack) | 18 | Medium | Service integration |
| **G23** | Daemon mode (roko daemon install) | 19 | High | Background operation |
| **G24** | CorticalState + T0 probes | 16 | High | ~80% tick suppression |
| **G25** | Plugin SDK (EventSource, Integration) | 18 | High | Third-party extensibility |

## Tier 3 — Advanced / Phase 2+ (~50+ person-weeks)

| # | Gap | Section | Complexity | Impact |
|---|---|---|---|---|
| **G26** | Agent Mesh transport (P2P) | 13 | Very High | Multi-agent coordination |
| **G27** | REM counterfactual generation (Pearl SCM) | 10 | Very High | Causal reasoning in dreams |
| **G28** | Causal discovery + mirage-rs integration | 20 | Very High | Intervention-based prediction |
| **G29** | Sheaf/tropical geometry | 20 | Very High | Algebraic foundation for robustness |
| **G30** | Korai chain deployment (L3 on Base) | 08, 14 | Very High | On-chain agent economy |
| **G31** | Spectre creature visualization | 12 | Very High | Embodied agent visualization |
| **G32** | 4-level conductor federation | 07 | Very High | Distributed safety oversight |
| **G33** | Witness DAG with ZK proofs | 11 | Very High | Verifiable execution history |

---

## Recommended Execution Order

```
Week 1-2:  G1 (ToolDispatcher wiring) + G2/G3 (code intelligence wiring) + G5 (Daimon→CascadeRouter)
Week 2-3:  G4 (role prompt expansion) + G8 (Signal→Engram rename) + G10/G11 (knowledge entry fixes)
Week 3-4:  G7 (8 feedback loops) + G9 (PAD persistence) + G14 (Docker/Fly)
Week 4-6:  G6 (EFE scorer) + G12 (daimon consolidation) + G13 (safety gap closure)
Week 6-8:  G15 (Mattar-Daw) + G19 (pheromone types) + G20 (CodeIndex trait)
Week 8-12: G16 (VCG auction) + G21 (Oracle trait) + G22 (MCP servers) + G24 (T0 probes)
```

After weeks 1-4: **full self-hosting with safety** (agents have code-aware context, safety guards active, knowledge decay correct, learning loop closed)

After weeks 4-8: **intelligent self-hosting** (active inference scores context, daimon modulates behavior, dreams prioritize by utility, coordination primitives exist)

After weeks 8-12: **optimal self-hosting** (attention auctioned, predictions feed routing, probes suppress ~80% computation)

---

## Gap × Integration-Map Cross-Reference

Gaps that unblock integration-map M-items:

| Gap | Unblocks M-Item | Integration Pair |
|---|---|---|
| G1 | G1 itself (safety) | [safety-x-agents](../integration-map/safety-x-agents.md) |
| G2, G3 | M8 | [code-intel-x-composition](../integration-map/code-intel-x-composition.md) |
| G5, G12 | M1, M2 (partial) | [daimon-x-orchestration](../integration-map/daimon-x-orchestration.md), [daimon-x-composition](../integration-map/daimon-x-composition.md) |
| G7 | M3, M4, M6 (loops 4, 5, 6) | [verification-x-orchestration](../integration-map/verification-x-orchestration.md), [learning-x-composition](../integration-map/learning-x-composition.md), [learning-x-routing](../integration-map/learning-x-routing.md) |
| G15 | M7 (Mattar-Daw is core of M7) | [dreams-x-neuro](../integration-map/dreams-x-neuro.md) |
| G19 | M12 (foundation) | [coordination-x-orchestration](../integration-map/coordination-x-orchestration.md) |
| G20 | M16 | [code-intel-x-verification](../integration-map/code-intel-x-verification.md) |
| G21 | M17 | [tech-analysis-x-heartbeat](../integration-map/tech-analysis-x-heartbeat.md) |
| G26 | M12, M19 (full) | [coordination-x-orchestration](../integration-map/coordination-x-orchestration.md), [coordination-x-dreams](../integration-map/coordination-x-dreams.md) |

---

## Cross-References

- [00-overview.md](./00-overview.md) — Scorecard and crate status
- [01-audit-summary.md](./01-audit-summary.md) — Systemic patterns
- [../integration-map/99-master-lattice.md](../integration-map/99-master-lattice.md) — All 20 missing integrations
