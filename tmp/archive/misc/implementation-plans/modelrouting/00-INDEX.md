# Model Routing — Extensible Multi-Provider Agent Backends

> **Priority**: 🔴 P0 — Unlocks cost reduction, model diversity, and self-optimizing routing
> **Status**: Not started
> **Branch**: TBD
> **Depends on**: None (core infrastructure, no prerequisites)
> **Blocks**: GLM-5.1 integration, Kimi-K2.5 integration, OpenRouter routing, self-hosted vLLM

## Document Index

| Doc | Title | Scope | Tasks |
|-----|-------|-------|-------|
| [01-architecture.md](01-architecture.md) | Architecture & Design | Three-layer provider system, traits, config schema | Reference only |
| [02-provider-registry.md](02-provider-registry.md) | Provider Registry & Config | `[providers.*]` + `[models.*]` TOML, ProviderConfig types | 2A.01–2A.12 |
| [03-provider-adapters.md](03-provider-adapters.md) | Provider Adapter Trait & Impls | ProviderAdapter trait, 4 protocol families, dispatch refactor | 2B.01–2B.18 |
| [04-translator-extensions.md](04-translator-extensions.md) | Translator Extensions | Thinking support, extended tools, response metadata | 2C.01–2C.10 |
| [05-glm-integration.md](05-glm-integration.md) | GLM-5.1 First-Class Backend | Z.AI adapter, thinking mode, MCP tools, web search | 2D.01–2D.14 |
| [06-kimi-integration.md](06-kimi-integration.md) | Kimi-K2.5 First-Class Backend | Moonshot adapter, thinking, vision, partial continuation | 2E.01–2E.14 |
| [07-openrouter-universal.md](07-openrouter-universal.md) | OpenRouter Universal Backend | Single-endpoint multi-model, provider routing params | 2F.01–2F.08 |
| [08-learning-loops.md](08-learning-loops.md) | Learning Loops & Cybernetic Feedback | Provider health, latency tracking, Pareto pruning, anomaly detection | 2G.01–2G.20 |
| [09-cost-normalization.md](09-cost-normalization.md) | Cost Normalization & Budget Guardrails | CostTable, standard tokenizer, budget enforcement | 2H.01–2H.10 |
| [10-model-experiments.md](10-model-experiments.md) | Model A/B Experiments | Model-level experiments extending ExperimentStore | 2I.01–2I.08 |
| [11-research-context.md](11-research-context.md) | Research & Academic Context | RouteLLM, MixLLM, FrugalGPT, GVU, GEPA, SAGE, ABC, 23 sections | Reference only |
| [12-advanced-patterns.md](12-advanced-patterns.md) | Advanced Patterns | Thompson Sampling, PF, gate feedback, skills, contracts, drift | 2J.01–2J.16 |
| [13-architectural-gaps.md](13-architectural-gaps.md) | Architectural Gaps | Chat types, cache layers, streaming, events, sessions, conductor, TaskRunner | 2K.01–2K.33 |
| [14-integration-refinements.md](14-integration-refinements.md) | Integration Refinements | Wire EXISTING ToolLoop, token counting, rate limits, MCP bridge, fallback chains | 2L.01–2L.16 |
| [15-operational-surface.md](15-operational-surface.md) | Operational Surface | CLI commands, testing, validation, dashboard, routing log, config migration | 2M.01–2M.16 |
| [16-production-hardening.md](16-production-hardening.md) | Production Hardening | Timeouts, retry jitter, concurrency, overflow, shutdown, serve API, hedging | 2N.01–2N.18 |
| [17-meta-learning-and-corrections.md](17-meta-learning-and-corrections.md) | Meta-Learning & Corrections | Missing feedback wires, stability, compound optimization, final audit | 2O.01–2O.13 |
| [18-structural-cleanup.md](18-structural-cleanup.md) | Structural Cleanup | ToolDef extension, dual config, hot reload, plugins, model-aware prompts | 2P.01–2P.12 |
| [19-implementation-guide.md](19-implementation-guide.md) | Implementation Guide | Exact wiring locations, Phase 1 sequence, what NOT to change | Reference only |
| [20-perplexity-integration.md](20-perplexity-integration.md) | Perplexity Sonar Backend | Search-grounded research, citations, deep research, embeddings, search API | 2Q.01–2Q.24 |
| [21-gemini-integration.md](21-gemini-integration.md) | Gemini First-Class Backend | 1M context, grounding, code execution, thinking, caching, free tier | 2R.01–2R.24 |
| [22-research-apis-backlog.md](22-research-apis-backlog.md) | Research API Backlog | Semantic Scholar, Exa, Jina Reader, Brave, Firecrawl, Tavily | Backlog |

## Dependency Graph

```
02-provider-registry ─────────────────────┐
                                          │
03-provider-adapters ─────────────────────┤
    (depends on 02)                       │
                                          │
04-translator-extensions ─────────────────┤
    (depends on 03)                       │
                                          │
13-architectural-gaps A (Tool Loop) ──────┤  ← CRITICAL: HTTP backends need this
    (depends on 02, 03, 04)               │
                                          │
13-gaps B (Cache Layers) ─────────────────┤  ← Highest cost ROI
    (depends on 04)                       │
                                          ├── 05-glm-integration
                                          │    (depends on 02, 03, 04, 13-A)
                                          │
                                          ├── 06-kimi-integration
                                          │    (depends on 02, 03, 04, 13-A)
                                          │
                                          ├── 07-openrouter-universal
                                          │    (depends on 02, 03, 13-A)
                                          │
                                          ├── 20-perplexity-integration
                                          │    (depends on 02, 03)
                                          │
                                          ├── 21-gemini-integration
                                          │    (depends on 02, 03)
                                          │
08-learning-loops ────────────────────────┤
    (independent, can start in parallel)  │
                                          │
13-gaps D (Event Fabric) ─────────────────┤  ← Can start with 08
    (independent)                         │
                                          │
09-cost-normalization ────────────────────┤
    (depends on 02)                       │
                                          │
10-model-experiments ─────────────────────┤
    (depends on 08)                       │
                                          │
12-advanced-patterns ─────────────────────┤
    (depends on 08, 03)                   │
                                          │
13-gaps C,E,F,G,H ───────────────────────┘
    (depends on 13-A, 13-D)
```

## Execution Order

**Phase 1** (foundation — do first, sequential):
1. `02-provider-registry` — config types and TOML parsing
2. `03-provider-adapters` — trait + 4 adapter impls
3. `04-translator-extensions` — thinking + extended response types
4. `13-gaps A` (2K.01–2K.04) — ChatMessage/ChatRequest/ChatResponse canonical types
5. `14-refinements A` (2L.01–2L.05) — **Wire EXISTING ToolLoop** via LlmBackend impl (**REPLACES 2K.05–2K.09**)

> **IMPORTANT**: `ToolLoop` already exists at `crates/roko-agent/src/tool_loop/mod.rs` with full
> multi-turn iteration, ToolDispatcher integration, context pruning, and checkpointing.
> Do NOT rebuild it. Only implement `LlmBackend` for HTTP endpoints and wire it in.

**Phase 2** (model backends + cache — parallelizable after Phase 1):
5. `13-gaps B` (2K.11–2K.15) — Cache layer alignment (highest cost ROI)
6. `05-glm-integration` — GLM-5.1 first-class support
7. `06-kimi-integration` — Kimi-K2.5 first-class support
8. `07-openrouter-universal` — OpenRouter as universal backend
9. `20-perplexity-integration` — Perplexity Sonar (search-grounded research, citations, deep research)
10. `21-gemini-integration` — Gemini (1M context, grounding, code execution, free tier)

**Phase 3** (learning & events — parallelizable, can start during Phase 2):
9. `08-learning-loops` — provider health, latency, Pareto, anomaly detection
10. `13-gaps D` (2K.20–2K.23) — Event fabric (decouples learning loops)
11. `09-cost-normalization` — CostTable, budgets
12. `10-model-experiments` — model-level A/B testing

**Phase 4** (advanced — after Phase 3):
13. `12-advanced-patterns` — Thompson Sampling, PF, skills, contracts, drift
14. `13-gaps C,E,F,G` (2K.16–2K.19, 2K.24–2K.30) — Streaming, sessions, conductor, TaskRunner
15. `13-gaps H` (2K.31–2K.33) — Generated test gates (GVU verification)
16. `14-refinements B-H` (2L.06–2L.16) — Token counting, rate limits, MCP bridge, fallback chains, concurrency
17. `15-operational-surface` (2M.01–2M.16) — CLI commands, testing, dashboard, validation, config migration
18. `16-production-hardening` (2N.01–2N.18) — Timeouts, retries, concurrency, shutdown, serve API
19. `17-meta-learning` (2O.01–2O.13) — Wire missing feedback loops, stability, compound optimization
20. `18-structural-cleanup` (2P.01–2P.10) — ToolDef extension, dual config, hot reload, extension docs

## Task ID Convention

All tasks use the format `2X.NN` where:
- `2` = Tier 2 (Model Routing)
- `X` = Section letter (A=registry, B=adapters, C=translators, D=GLM, E=Kimi, F=OpenRouter, G=learning, H=costs, I=experiments, J=advanced, K=gaps, L=refinements, M=ops, N=hardening, O=meta, P=cleanup, Q=Perplexity, R=Gemini)
- `NN` = Sequential number within section

Commit messages: `parity(2A.03): Add ProviderConfig deserialization from roko.toml`
