# Source File Locations — Where to Find Everything

> This file tells you where every source file lives on disk. Use the absolute paths here
> to read source material. Every reference to a source file in your prompt will use one
> of these base paths.

## Absolute path root layout

```
/Users/will/dev/nunchi/roko/
├── refactoring-prd/                 ← canonical new-architecture spec (SOURCE OF TRUTH)
│   ├── 00-overview.md
│   ├── 01-synapse-architecture.md
│   ├── 02-five-layers.md
│   ├── 03-cognitive-subsystems.md
│   ├── 04-knowledge-and-mesh.md
│   ├── 05-agent-types.md
│   ├── 06-interfaces.md
│   ├── 07-implementation-priorities.md
│   ├── 08-translation-guide.md    ← reframe rules (always read)
│   ├── 09-innovations.md          ← frontier features
│   ├── 10-developer-guide.md
│   └── MIGRATION-CHECKLIST.md
├── bardo-backup/                    ← legacy source material (READ-ONLY)
│   ├── prd/                         ← 359 legacy PRD files
│   ├── tmp/                         ← research docs, design notes
│   ├── crates/                      ← reference code (golem-*, mori-*)
│   └── apps/                        ← reference apps (mori, bardo-*, mirage-rs)
└── roko/                            ← the active Rust workspace
    ├── crates/                      ← the 18+ shipping crates
    │   ├── roko-core/
    │   ├── roko-agent/
    │   ├── roko-compose/
    │   ├── roko-gate/
    │   ├── roko-orchestrator/
    │   ├── roko-conductor/
    │   ├── roko-learn/
    │   ├── roko-neuro/
    │   ├── roko-daimon/
    │   ├── roko-dreams/
    │   ├── roko-chain/
    │   ├── roko-fs/
    │   ├── roko-std/
    │   ├── roko-index/
    │   ├── roko-lang-rust/
    │   ├── roko-lang-typescript/
    │   ├── roko-lang-go/
    │   ├── roko-cli/
    │   ├── bardo-primitives/       ← to be renamed `roko-primitives`
    │   ├── bardo-runtime/          ← to be renamed `roko-runtime`
    │   └── roko-golem/             ← TO BE DISSOLVED
    ├── apps/
    │   └── mirage-rs/
    ├── docs/                        ← ★ OUTPUT LIVES HERE ★
    └── tmp/
        ├── prd-migration/           ← this directory
        │   ├── README.md
        │   ├── SOURCE-INDEX.md      ← full source list per target doc
        │   ├── CHECKLIST.md
        │   ├── context-pack/        ← the files you're reading now
        │   └── prompts/             ← per-topic prompt files
        └── implementation-plans/    ← active work items
            ├── 00-INDEX.md
            ├── 11-agent-dogfooding.md
            ├── 12a-cognitive-layer.md
            ├── 12b-chain-layer.md
            ├── 11-sections/         ← 5 phase files
            └── modelrouting/        ← 23 model routing files
```

## Where to find things

### Canonical new-architecture spec (ALWAYS the source of truth)

`/Users/will/dev/nunchi/roko/refactoring-prd/` — 12 files. Read these first for any topic.

### Legacy PRD docs (body content, reframe through refactoring-prd lens)

`/Users/will/dev/nunchi/roko/bardo-backup/prd/` — 359 files organized by topic section:

| Section | Content |
|---|---|
| `00-vision/` | Original vision, thesis, architecture, philosophy, trust, manifesto |
| `00-narrative-strategy.md` | Narrative framing |
| `01-golem/` | Agent overview, cognition, mind, heartbeat, mortality (skip death files), creation, provisioning, funding, inheritance, replication, lifecycle, teardown, context governor, attention auction, sleepwalker, risk engine, prediction engine, cortical state, config |
| `02-mortality/` | **MOSTLY SKIP** — death clocks, thanatopsis, necrocracy, etc. Keep 14-research-foundations.md + 15-references.md for citations. Extract non-death concepts from 02, 05, 07. |
| `03-daimon/` | PAD affect engine, appraisal, emotion memory, behavior, dream daimon, runtime daimon, infrastructure, evaluation. **SKIP 04-mortality-daimon and 05-death-daimon entirely.** |
| `04-memory/` | Grimoire (→Neuro), memetic, HDC, emotional memory, economy, safety, Library of Babel, research |
| `05-dreams/` | Dream overview, architecture, evolution, replay, imagination, consolidation, threats, integration, Venice dreaming |
| `06-hypnagogia/` | Hypnagogia neuroscience, architecture, divergence-alpha, homunculus, hauntology, xenocognition, inner worlds |
| `07-tools/` | Tool overview, architecture, 20+ tool definitions, config, profiles, wallets, distribution, testing |
| `09-economy/` | Identity, reputation, clade (→collective), marketplace, coordination, agent economy, commerce bazaar |
| `10-safety/` | Defense, custody, policy, ingestion, prompt security, threat model, adaptive risk, temporal logic, witness DAG, formal verification, MEV protection |
| `11-compute/` | Compute overview, architecture, provisioning |
| `12-inference/` | Inference overview, deployment modes, routing, caching, context engineering, sessions, memory, safety, observability, API, providers, reasoning, rust implementation, inference profiles, structured outputs, streaming, golem-config, multi-model orchestration, parameters, performance, sheaf observation |
| `13-runtime/` | Runtime overview, activities, state model, collective intelligence, packaging, cybernetic loops |
| `14-chain/` | Chain architecture, witness, triage, protocol state, chain scope, heartbeat integration, events/signals, generative views, stream API, anomaly detection |
| `15-dev/` | mirage-rs specs, deployment, debug UI, tooling, indexer |
| `16-testing/` | Thesis validation, gauntlet, mechanism testing, evaluation lifecycle, fast/slow feedback loops, evaluation map |
| `17-monorepo/` | Packages, rust workspace |
| `18-interfaces/` | Portal, CLI, UI system, TUI, spatial grammar, bardo-terminal foundation, creature system, perspective/, protocol/, rendering/, screens/ |
| `19-agents-skills/` | Agent overview, categories, definitions, delegation, skills overview, categories, definitions, MCP integration, golem agents, vault agents, composition, observer agents, hermes hierarchy |
| `20-styx/` (→Mesh) | Architecture, clade sync, marketplace, p2p transport, transport config |
| `21-integrations/` | Overview, MetaMask, Venice, Bankr, AgentCash, Uniswap |
| `22-oneirography/` | Overview, dream journals, self-appraisal, auctions, extended forms. **SKIP 02-death-masks.** |
| `23-ta/` | Witness-as-TA, hyperdimensional TA, spectral liquidity manifolds, adaptive signal metabolism, causal microstructure discovery, predictive geometry, resonant pattern ecosystem, DeFi-native TA, adversarial signal robustness, somatic TA, emergent multiscale intelligence |
| `24-sonification/` | Musical language, preset catalog — keep for music theory, remap presets to behavioral states |
| `25-mori/` (→Roko Orchestrator) | Overview, parallel execution, unified DAG, quality gates, resilience, project operations, document pipeline, provider architecture, agent architecture, context engineering, context service, cost efficiency, deployment, interfaces |
| `shared/` | Glossary, dependencies, branding, chains, citations, config-reference, data-privacy, doc-standards, emergent-capabilities, evaluation, event-catalog, hdc-vsa, hdc-applications, hdc-fingerprints, integrated-information, port-allocation, research, timeline, x402-protocol, eip-analysis |
| `appendices/` | Various appendix files |

### Legacy research/tmp docs

`/Users/will/dev/nunchi/roko/bardo-backup/tmp/` — key subdirectories:

| Dir | Content |
|---|---|
| `mori-refactor/` | 27 docs: layer taxonomy, cognitive architecture, unified theory, runtime, framework, scaffold, harness, orchestration, inference optimization, memory/knowledge, code intelligence, safety/obs/learning, cognitive architecture, current state, gaps/frontier, migration plan, substrate, human-agent interface, agent ecology, developmental trajectory, information architecture, generalization, cost optimization, crate consolidation, developer experience, config/server polish, module docs |
| `mori-refactor-plan/` | 31 docs: issues catalog (21 production failures), design principles, deep refactor, failure prevention, cybernetic learning dashboard, exponential roadmap (65% compound improvement), phase plans, testing/CI, medium files/orchestrator, optimization playbook, context data optimization, TUI/support cleanup, real agent extraction, etc. |
| `mori-agents/` | 34 docs: architecture, connection backends, agent roles, context engineering, prompt engineering, eval/scoring, self-improvement, harness engineering (6× gap paper), multi-agent orchestration, extraction plan, benchmarks/evals, references, CLI/deployment, service integrations, automation workflows, PRD-to-execution pipeline, dynamic prompt generation, code intelligence, practical self-learning, verification-first architecture, efficiency monitoring, model routing optimization, prompt budget engineering, SDK/ecosystem, agent code quality, agent extensibility, observer agents, etc. |
| `death/` | Original "death" research tree. **CAUTION**: contains many mortality-framed docs. Extract non-mortality content from: project-structure, orchestration, providers, interfaces, server-and-remote, task-routing, queue-management, agent-foundations, autonomous-verification, context-engine, context-as-service, inference-gateway, cost-tracking, cybernetic-learning, agent-optimization, proposals-and-billing, project-deployment, fly-deploy, dependency-architecture, payments/*. **SKIP** mortality-specific files. |
| `agent-chain/` | 27 docs: overview, chain architecture, stigmergy, HDC, knowledge layer, tokenomics, implementation, references, exponential flywheels, predictive foraging, adversarial defense and value, golem orchestrators, orchestration as a service, academic foundations, dynamic context assembly, mirage-rs PoC, autonomous eval generation, harness engineering, context-quality science, eval research, exponential mechanisms research, proving collective intelligence, agent-chain-research2, agent-research2, self-improvement frameworks, README |
| `agent-chain-new/` | 14 docs: vision, coordination theory, chain architecture, knowledge layer, token economics, golem architecture, context assembly, chain architecture, autonomous evaluation, self-improvement, adversarial defense, agent economy, exponential growth, implementation |
| `hyperliquid/` | HyperEVM research |
| `production/` | 7 docs: overview, dependency refactor, packaging/distribution, config/state, deployment, migration plan, playground architecture |
| `roko-progress/` | 140+ docs: unified primitives, dual-nature agents, MORI-PARITY-CHECKLIST (1,253 items), MISTAKES-LEARNED, CONFIG-REDESIGN, CURRENT-STATE, CLI-compatibility, language-agnostic design, COMPONENTS/ (140+ per-component specs), 12-unified-primitives.md, 13-dual-nature-agents.md, 09-refactor-gaps.md, 08-gap-inventory.md |

### Implementation plans (active work items)

`/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/`:

| File | Content |
|---|---|
| `00-INDEX.md` | Master index of plans |
| `01-agent-wiring.md` | **Completed** — agent wiring tasks |
| `02-system-prompt-integration.md` | **Completed** — system prompt wiring |
| `03-safety-hooks.md` | Safety hooks tasks |
| `04-orchestrator-pipeline.md` | Orchestrator pipeline tasks |
| `05-learning-wiring.md` | **Completed** — learning wiring |
| `06-process-management.md` | Process management |
| `07-mcp-tool-wiring.md` | **Superseded** by MASTER-PLAN |
| `08-observability-wiring.md` | **Superseded** by MASTER-PLAN |
| `09-tui-dashboard.md` | **Superseded** by MASTER-PLAN |
| `10-golem-integration.md` | **Superseded** by 12b-chain-layer |
| `11-agent-dogfooding.md` | 9 phases, 16 agent templates, 5 new crates, 235+ items |
| `11-sections/phase-0-1.md` | roko-serve extraction, roko-plugin SDK, kind constants, webhook routes, subscriptions, dispatch loop |
| `11-sections/phase-2.md` | roko-mcp-github (17 tools), roko-mcp-slack (8 tools), roko-mcp-scripts |
| `11-sections/phase-3-4.md` | 16 agent template full definitions, cron scheduler, file watcher |
| `11-sections/phase-5-6.md` | Daemon lifecycle, launchd, systemd, Fly.io, remote orchestrator, multi-repo, secrets |
| `11-sections/phase-7-8.md` | Learning loops, feedback, HDC integration, metrics, PRD workflow |
| `11-inconsistencies.md` | **Critical**: documents the dispatcher-not-called gap + other drift between docs and code |
| `12-context-provider.md` | Context provider |
| `12-nunchi-integration.md` | **Superseded** — split into 12a+12b |
| `12a-cognitive-layer.md` | 72 items: Neuro, Daimon, Dreams, Context assembly, Operating frequencies, C-Factor. Sections D, E, F, G, I, J, R1-R3. |
| `12b-chain-layer.md` | 76 items, 11 sections: Identity, Gossip, Job Market, ChainWitness, Reputation, Payments, Safety, ISFR, Clearing, Privacy, Mirage, Crate arch. |
| `modelrouting/00-INDEX.md` | 23-doc index |
| `modelrouting/01-architecture.md` | Three-layer provider system, traits, config schema |
| `modelrouting/02-provider-registry.md` | ProviderKind, ProviderConfig, ModelProfile |
| `modelrouting/03-provider-adapters.md` | ProviderAdapter trait + 4 impls (OpenAiCompat, ClaudeCli, AnthropicApi, CursorAcp) |
| `modelrouting/04-translator-extensions.md` | Thinking, reasoning, cached tokens |
| `modelrouting/05-glm-integration.md` | GLM-5.1 Z.AI backend |
| `modelrouting/06-kimi-integration.md` | Kimi-K2.5 Moonshot backend |
| `modelrouting/07-openrouter-universal.md` | OpenRouter universal backend |
| `modelrouting/08-learning-loops.md` | Provider health, latency, Pareto pruning, anomaly detection (20 tasks) |
| `modelrouting/09-cost-normalization.md` | CostTable, budgets, guardrails |
| `modelrouting/10-model-experiments.md` | Thompson Sampling, discount factor, UCB1 |
| `modelrouting/11-research-context.md` | RouteLLM, MixLLM, FrugalGPT, GVU, GEPA, SAGE, ABC — 23 sections |
| `modelrouting/12-advanced-patterns.md` | Thompson, predictive foraging, gate feedback, skills, contracts, drift |
| `modelrouting/13-architectural-gaps.md` | Chat types (must be in roko-core), cache layers, streaming, events, sessions, conductor, TaskRunner, generated test gates (33 gaps) |
| `modelrouting/14-integration-refinements.md` | **Wire EXISTING ToolLoop**, don't rebuild. Token counting, rate limits, MCP bridge, fallback chains |
| `modelrouting/15-operational-surface.md` | CLI commands, testing, validation, dashboard, routing log, config migration |
| `modelrouting/16-production-hardening.md` | Timeouts (p95×2), retries (full-jitter), concurrency, shutdown (3-phase drain), serve API, hedging |
| `modelrouting/17-meta-learning-and-corrections.md` | **8 missing cybernetic feedback loops**, stability (hysteresis, frequency separation), compound optimization |
| `modelrouting/18-structural-cleanup.md` | ToolDef extension, dual config, hot reload, plugins |
| `modelrouting/19-implementation-guide.md` | Exact wiring locations, Phase 1 sequence, what NOT to change |
| `modelrouting/20-perplexity-integration.md` | Perplexity Sonar: search-grounded research, citations, deep research, embeddings |
| `modelrouting/21-gemini-integration.md` | Gemini: 1M context, grounding, code execution, thinking, caching, free tier |
| `modelrouting/22-research-apis-backlog.md` | Semantic Scholar, Exa, Jina Reader, Brave, Firecrawl, Tavily |

### Reference code (active codebase)

`/Users/will/dev/nunchi/roko/roko/crates/` — read actual source for current state:

- `roko-core/src/lib.rs`, `traits.rs`, `signal.rs` (→engram), `kind.rs`, `score.rs`
- `roko-agent/src/` — backends, dispatcher, safety, tool_loop, provider
- `roko-compose/src/` — system_prompt_builder, context_provider, enrichment, role_prompts, scorer
- `roko-gate/src/` — gate pipeline, selector, ratcheting, artifact store
- `roko-orchestrator/src/` — executor, merge queue, worktrees, DAG, safety
- `roko-conductor/src/` — watchers, circuit breakers, diagnosis
- `roko-learn/src/` — episode_logger, playbook, cascade_router, bandits, efficiency, baseline, regression, skill_library, hdc_clustering
- `roko-fs/src/` — FileSubstrate JSONL
- `roko-std/src/` — defaults, 19 built-in tools, mock dispatcher
- `roko-index/src/` — parser, graph, HDC fingerprints
- `roko-cli/src/` — orchestrate.rs, run.rs, all subcommands
- `bardo-primitives/src/hdc.rs`, `tier.rs` (→roko-primitives)
- `bardo-runtime/src/process.rs` (→roko-runtime)
- `roko-golem/src/daimon.rs`, `dreams.rs`, `grimoire.rs`, `hypnagogia.rs`, `chain_witness.rs`, `mortality.rs` (all to be redistributed)

### Reference-only (DO NOT MODIFY)

- `/Users/will/dev/uniswap/bardo/` — the original Mori codebase (stale mirror). Only `bardo-backup/` is the source of truth for legacy content.
- `/Users/will/dev/nunchi/roko/bardo-backup/crates/` — legacy reference crates (golem-*, mori-*).
- `/Users/will/dev/nunchi/roko/bardo-backup/apps/` — legacy reference apps.

## How to find source files for a specific topic

The master mapping is in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/SOURCE-INDEX.md`.
That file lists, for each of the 22 target docs, all the legacy + refactoring-prd + implementation-plan sources.

Your prompt file will point you to the correct section of SOURCE-INDEX.md.
