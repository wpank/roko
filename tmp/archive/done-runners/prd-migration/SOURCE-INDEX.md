# Source File Index

Every source file that feeds into the new Roko PRD docs at `/Users/will/dev/nunchi/roko/roko/docs/`,
grouped by target doc. Each target doc lists five source categories in priority order:

1. **Refactoring-PRD** — the canonical new-architecture spec at `/Users/will/dev/nunchi/roko/refactoring-prd/`. **Read these first.**
2. **Legacy PRD** — legacy architecture docs at `/Users/will/dev/nunchi/roko/bardo-backup/prd/`. Extract content, reframe through Synapse lens.
3. **Legacy research/tmp** — research docs at `/Users/will/dev/nunchi/roko/bardo-backup/tmp/`. Academic foundations, prior design rationale.
4. **Implementation plans** — active work at `/Users/will/dev/nunchi/roko/roko/tmp/implementation-plans/`. Concrete task breakdowns, status, verification criteria.
5. **Reference code** — current crate source at `/Users/will/dev/nunchi/roko/roko/crates/`. For alignment with shipping code.

**Always-include reference docs** (add to every prompt):
- `/Users/will/dev/nunchi/roko/refactoring-prd/00-overview.md` — Synapse Architecture, naming map, crate map
- `/Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md` — Rename & reframe rules, incompatibility flags
- `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/README.md` — Authoritative naming, reframe rules

---

## 00-architecture.md

**Covers**: Roko vision, Synapse Architecture, Engrams, 6 traits, universal cognitive loop, 5-layer taxonomy, crate map, C-Factor, autocatalytic improvement, Ashby's Law / VSM / Good Regulator, provenance & attestation.

### Refactoring-PRD (canonical)
- `refactoring-prd/00-overview.md`
- `refactoring-prd/01-synapse-architecture.md` — Engram, 6 traits, cognitive loop, cybernetic loops, composability
- `refactoring-prd/02-five-layers.md` — trait × layer map, dependency rules
- `refactoring-prd/07-implementation-priorities.md` — current state, Tier 0–6 roadmap, dropped/kept items
- `refactoring-prd/09-innovations.md` — Blue Ocean summary (§XVIII), Integration Map (§XVII), Autocatalytic loops
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/00-vision/00-bardo.md`
- `bardo-backup/prd/00-vision/01-thesis.md`
- `bardo-backup/prd/00-vision/02-architecture.md`
- `bardo-backup/prd/00-vision/03-philosophy.md`
- `bardo-backup/prd/00-vision/04-trust.md`
- `bardo-backup/prd/00-vision/05-manifesto.md`
- `bardo-backup/prd/00-narrative-strategy.md`
- `bardo-backup/prd/13-runtime/00-interaction-model.md`
- `bardo-backup/prd/13-runtime/11-state-model.md`
- `bardo-backup/prd/17-monorepo/00-packages.md`
- `bardo-backup/prd/17-monorepo/01-rust-workspace.md`
- `bardo-backup/prd/shared/glossary.md`
- `bardo-backup/prd/shared/dependencies.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/00-overview.md` — "scaffold IS the product" thesis
- `bardo-backup/tmp/mori-refactor/01-taxonomy.md` — layer taxonomy with citations
- `bardo-backup/tmp/mori-refactor/03-runtime.md`
- `bardo-backup/tmp/mori-refactor/04-framework.md`
- `bardo-backup/tmp/mori-refactor/12-cognitive-architecture.md` — CoALA, ACT-R, SOAR, dual-process
- `bardo-backup/tmp/mori-refactor/13-unified-theory.md` — unified theory + scaling laws
- `bardo-backup/tmp/mori-refactor/16-substrate.md`
- `bardo-backup/tmp/mori-refactor/21-generalization.md`
- `bardo-backup/tmp/roko-progress/12-unified-primitives.md` — Signal + 6 traits genesis
- `bardo-backup/tmp/roko-progress/13-dual-nature-agents.md` — coding + chain composition
- `bardo-backup/tmp/mori-refactor-plan/08-design-principles.md`
- `bardo-backup/tmp/mori-refactor-plan/15-deep-refactor.md`

### Implementation plans
- `roko/tmp/implementation-plans/00-INDEX.md` — current tier structure
- `roko/tmp/implementation-plans/11-inconsistencies.md` — reality check vs. docs

### Reference code
- `roko/crates/roko-core/src/lib.rs`
- `roko/crates/roko-core/src/traits.rs`
- `roko/crates/roko-core/src/signal.rs` (to be renamed `engram.rs`)
- `roko/README.md`, `roko/CLAUDE.md`

---

## 01-orchestration.md

**Covers**: Plan DAG, parallel executor, merge queue, worktree management, snapshot/recovery, niche construction, stigmergic coordination, Yerkes-Dodson dynamics, cross-domain orchestration.

### Refactoring-PRD (canonical)
- `refactoring-prd/02-five-layers.md` §Layer 4 Orchestration, §Stigmergy
- `refactoring-prd/05-agent-types.md` §7 Multi-Agent Orchestration
- `refactoring-prd/07-implementation-priorities.md` §Tier 1
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/25-mori/mori-overview.md`
- `bardo-backup/prd/25-mori/mori-parallel-execution.md`
- `bardo-backup/prd/25-mori/mori-unified-dag.md`
- `bardo-backup/prd/25-mori/mori-quality-gates.md`
- `bardo-backup/prd/25-mori/mori-resilience.md`
- `bardo-backup/prd/25-mori/mori-project-operations.md`
- `bardo-backup/prd/25-mori/mori-document-pipeline.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/07-orchestration.md` — full orchestration layer spec
- `bardo-backup/tmp/mori-refactor/05-scaffold.md`
- `bardo-backup/tmp/mori-refactor/18-agent-ecology.md` — niche construction, affordances
- `bardo-backup/tmp/death/04-orchestration.md`
- `bardo-backup/tmp/death/10-task-routing.md`
- `bardo-backup/tmp/death/11-queue-management.md`
- `bardo-backup/tmp/mori-agents/09-multi-agent-orchestration.md`
- `bardo-backup/tmp/mori-agents/16-prd-to-execution-pipeline.md`
- `bardo-backup/tmp/mori-refactor-plan/27-medium-files-and-orchestrator.md`

### Implementation plans
- `roko/tmp/implementation-plans/04-orchestrator-pipeline.md`
- `roko/tmp/implementation-plans/11-agent-dogfooding.md` §Phases 3-4 (scheduler, dispatch)
- `roko/tmp/implementation-plans/11-sections/phase-3-4.md` (cron, file watcher, event source wiring)

### Reference code
- `roko/crates/roko-orchestrator/src/` (all submodules)
- `roko/crates/roko-cli/src/orchestrate.rs` — the runtime harness (766 lines)

---

## 02-agents.md

**Covers**: LLM backends, agent trait, roles, pools, MCP, tool loop, harness engineering (6× gap), temperament profiling, dual-process tier routing, provider adapters.

### Refactoring-PRD (canonical)
- `refactoring-prd/01-synapse-architecture.md` §2 Six Synapse Traits (composability)
- `refactoring-prd/02-five-layers.md` §Layer 1 Framework, §Temperament Profiling, §Dual-Process Tier Router
- `refactoring-prd/05-agent-types.md` §2–6 (Coding, Chain, Research, Ops, Cross-Domain, Extensibility)
- `refactoring-prd/10-developer-guide.md` §2 Implementing Custom Traits, §6 Plugin System
- `refactoring-prd/07-implementation-priorities.md` §Tier 1 (provider registry, adapters)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/19-agents-skills/00-agents-overview.md`
- `bardo-backup/prd/19-agents-skills/01-agent-categories.md`
- `bardo-backup/prd/19-agents-skills/02-agent-definitions.md`
- `bardo-backup/prd/19-agents-skills/03-delegation.md`
- `bardo-backup/prd/19-agents-skills/04-skills-overview.md`
- `bardo-backup/prd/19-agents-skills/05-skill-categories.md`
- `bardo-backup/prd/19-agents-skills/06-skill-definitions.md`
- `bardo-backup/prd/19-agents-skills/08-mcp-integration.md`
- `bardo-backup/prd/19-agents-skills/09-golem-agents.md`
- `bardo-backup/prd/19-agents-skills/11-composition.md`
- `bardo-backup/prd/19-agents-skills/12-observer-agents.md`
- `bardo-backup/prd/19-agents-skills/13-hermes-hierarchy.md`
- `bardo-backup/prd/25-mori/mori-provider-architecture.md`
- `bardo-backup/prd/25-mori/mori-agent-architecture.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-agents/01-architecture.md`
- `bardo-backup/tmp/mori-agents/02-connection-backends.md`
- `bardo-backup/tmp/mori-agents/03-agent-roles.md`
- `bardo-backup/tmp/mori-agents/07-self-improvement.md`
- `bardo-backup/tmp/mori-agents/08-harness-engineering.md` — Meta-Harness 6× gap
- `bardo-backup/tmp/mori-agents/19-practical-self-learning.md`
- `bardo-backup/tmp/mori-agents/26-agent-code-quality.md`
- `bardo-backup/tmp/mori-agents/27-agent-extensibility.md`
- `bardo-backup/tmp/mori-agents/28-sdk-and-ecosystem.md`
- `bardo-backup/tmp/mori-agents/10-extraction-plan.md`
- `bardo-backup/tmp/mori-refactor/06-harness.md`
- `bardo-backup/tmp/mori-refactor/18-agent-ecology.md`
- `bardo-backup/tmp/mori-refactor-plan/23-real-agent-extraction.md`
- `bardo-backup/tmp/death/03-providers.md`
- `bardo-backup/tmp/death/tools/05-agent-foundations.md`
- `bardo-backup/tmp/agent-chain/harness-engineering.md`

### Implementation plans
- `roko/tmp/implementation-plans/01-agent-wiring.md` — existing wiring tasks
- `roko/tmp/implementation-plans/02-system-prompt-integration.md`
- `roko/tmp/implementation-plans/11-agent-dogfooding.md` — 9 phases, 16 templates, 5 new crates
- `roko/tmp/implementation-plans/11-sections/phase-0-1.md` — roko-serve, roko-plugin extraction
- `roko/tmp/implementation-plans/11-sections/phase-3-4.md` — 16 agent template definitions
- `roko/tmp/implementation-plans/modelrouting/00-INDEX.md` — 23-doc breakdown
- `roko/tmp/implementation-plans/modelrouting/01-architecture.md` — three-layer provider system
- `roko/tmp/implementation-plans/modelrouting/02-provider-registry.md` — ProviderKind, ProviderConfig, ModelProfile
- `roko/tmp/implementation-plans/modelrouting/03-provider-adapters.md` — ProviderAdapter trait + 4 impls
- `roko/tmp/implementation-plans/modelrouting/04-translator-extensions.md` — thinking, reasoning, cached tokens
- `roko/tmp/implementation-plans/modelrouting/05-glm-integration.md`
- `roko/tmp/implementation-plans/modelrouting/06-kimi-integration.md`
- `roko/tmp/implementation-plans/modelrouting/07-openrouter-universal.md`
- `roko/tmp/implementation-plans/modelrouting/13-architectural-gaps.md` — chat types, ToolLoop wiring
- `roko/tmp/implementation-plans/modelrouting/14-integration-refinements.md` — wire existing ToolLoop
- `roko/tmp/implementation-plans/modelrouting/19-implementation-guide.md` — exact wiring locations
- `roko/tmp/implementation-plans/modelrouting/20-perplexity-integration.md`
- `roko/tmp/implementation-plans/modelrouting/21-gemini-integration.md`

### Reference code
- `roko/crates/roko-agent/src/` (all submodules)
- `roko/crates/roko-agent/src/tool_loop/mod.rs` — ALREADY EXISTS, do not rebuild
- `roko/crates/roko-agent/src/safety/`
- `roko/crates/roko-agent/src/dispatcher/mod.rs`
- `roko/crates/roko-agent/src/provider/`

---

## 03-composition.md

**Covers**: Composer trait, PromptComposer, SystemPromptBuilder (6-layer), enrichment pipeline, context engineering (ACE, CSO, ACON, Lost in the Middle), token budgets, active inference for context selection, VCG attention auction, predictive foraging, distributed context engineering.

### Refactoring-PRD (canonical)
- `refactoring-prd/02-five-layers.md` §Layer 2 Scaffold, §Context as Active Inference, §Predictive Foraging, §Three Levels of Context Engineering
- `refactoring-prd/09-innovations.md` §II VCG Attention Auction, §XIX.B EFE for Context Selection, §XIX.C Context Foraging Stopping Rule (MVT), §XIX.E VCG Bid Computation, §XV Distributed Context Engineering
- `refactoring-prd/01-synapse-architecture.md` §Composer trait signature
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/12-inference/00-overview.md`
- `bardo-backup/prd/12-inference/01-deployment-modes.md`
- `bardo-backup/prd/12-inference/01a-routing.md`
- `bardo-backup/prd/12-inference/02-caching.md`
- `bardo-backup/prd/12-inference/04-context-engineering.md`
- `bardo-backup/prd/12-inference/05-sessions.md`
- `bardo-backup/prd/12-inference/06-memory.md`
- `bardo-backup/prd/12-inference/13-reasoning.md`
- `bardo-backup/prd/12-inference/14-rust-implementation.md`
- `bardo-backup/prd/12-inference/15-inference-profiles.md`
- `bardo-backup/prd/12-inference/16-structured-outputs.md`
- `bardo-backup/prd/12-inference/19-multi-model-orchestration.md`
- `bardo-backup/prd/25-mori/mori-context-engineering.md`
- `bardo-backup/prd/25-mori/mori-context-service.md`
- `bardo-backup/prd/25-mori/mori-cost-efficiency.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/08-inference-optimization.md`
- `bardo-backup/tmp/mori-refactor/22-cost-optimization-architecture.md`
- `bardo-backup/tmp/mori-agents/04-context-engineering.md`
- `bardo-backup/tmp/mori-agents/05-prompt-engineering.md`
- `bardo-backup/tmp/mori-agents/17-dynamic-prompt-generation.md`
- `bardo-backup/tmp/mori-agents/24-prompt-budget-engineering.md`
- `bardo-backup/tmp/mori-agents/mori-context-optimization.md`
- `bardo-backup/tmp/death/17-context-engine.md`
- `bardo-backup/tmp/death/18-context-as-service.md`
- `bardo-backup/tmp/death/09-inference-gateway.md`
- `bardo-backup/tmp/death/tools/01-context-engineering.md`
- `bardo-backup/tmp/death/tools/03-context-assembly.md`
- `bardo-backup/tmp/death/tools/04-inference-routing.md`
- `bardo-backup/tmp/death/tools/07-prompt-assembly.md`
- `bardo-backup/tmp/mori-refactor-plan/12-context-data-optimization.md`
- `bardo-backup/tmp/mori-refactor-plan/14-optimization-playbook.md`
- `bardo-backup/tmp/agent-chain/15-dynamic-context-assembly.md`
- `bardo-backup/tmp/agent-chain-new/07-context-assembly.md`
- `bardo-backup/tmp/agent-chain/context-quality-science.md`
- `bardo-backup/tmp/agent-chain/harness-engineering.md`

### Implementation plans
- `roko/tmp/implementation-plans/12-context-provider.md`
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §E.1 — 5-stage pipeline (Query→Score→Dedupe→Budget→Format), active inference scoring (`score = track_record × belief_change / uncertainty`), attention-curve U-shape positioning, affect-modulated retrieval (E1–E6)
- `roko/tmp/implementation-plans/modelrouting/13-architectural-gaps.md` §B Cache Layers

### Reference code
- `roko/crates/roko-compose/src/`
- `roko/crates/roko-compose/src/system_prompt_builder.rs`
- `roko/crates/roko-compose/src/context_provider.rs`
- `roko/crates/roko-compose/src/enrichment/`
- `roko/crates/roko-compose/src/scorer.rs`
- `roko/crates/roko-compose/src/role_prompts.rs`

---

## 04-verification.md

**Covers**: Gate trait, 11+ gate implementations, 6-rung selector, GatePipeline, VerifyChainGate, artifact store, ratcheting, adaptive thresholds, process reward models (AgentPRM), agent feedback from gate results, evaluation lifecycle, autonomous eval generation, generation-verification gap.

### Refactoring-PRD (canonical)
- `refactoring-prd/02-five-layers.md` §Layer 3 Harness, §Process Reward Models, §Conductor as Meta-Cognition
- `refactoring-prd/01-synapse-architecture.md` §Gate trait, §Cybernetic loops (Outcome → Scaffold)
- `refactoring-prd/09-innovations.md` §IX Forensic AI / Causal Replay, §X EvoSkills adversarial verification
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/16-testing/00-thesis-validation.md`
- `bardo-backup/prd/16-testing/01-gauntlet.md`
- `bardo-backup/prd/16-testing/05-evaluation-lifecycle.md`
- `bardo-backup/prd/16-testing/07-fast-feedback-loops.md`
- `bardo-backup/prd/16-testing/08-slow-feedback-loops.md`
- `bardo-backup/prd/16-testing/09-evaluation-map.md`
- `bardo-backup/prd/25-mori/mori-quality-gates.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/11-safety-observability-learning.md`
- `bardo-backup/tmp/mori-refactor/06-harness.md`
- `bardo-backup/tmp/death/16-autonomous-verification.md`
- `bardo-backup/tmp/mori-agents/06-eval-and-scoring.md`
- `bardo-backup/tmp/mori-agents/11-benchmarks-and-evals.md`
- `bardo-backup/tmp/mori-agents/20-verification-first-architecture.md`
- `bardo-backup/tmp/mori-refactor-plan/18-testing-and-ci.md`
- `bardo-backup/tmp/mori-refactor-plan/28-integration-testing-and-ci.md`
- `bardo-backup/tmp/agent-chain/17-autonomous-eval-generation.md`
- `bardo-backup/tmp/agent-chain-new/09-autonomous-evaluation.md`

### Implementation plans
- `roko/tmp/implementation-plans/modelrouting/12-advanced-patterns.md` — gate-to-scaffold feedback, section effectiveness tracking, process rewards
- `roko/tmp/implementation-plans/modelrouting/13-architectural-gaps.md` §H — generated test gates (GVU verification)
- `roko/tmp/implementation-plans/11-sections/phase-7-8.md` — PRD-driven workflow gate verification

### Reference code
- `roko/crates/roko-gate/src/`
- `roko/crates/roko-gate/src/pipeline.rs`
- `roko/crates/roko-gate/src/selector.rs`

---

## 05-learning.md

**Covers**: Episodes, playbooks, skill library (Voyager), bandits (UCB1, Thompson, LinUCB), model routing (CascadeRouter, LinUCB contextual), pattern discovery (trigram miner), task metrics, baseline comparison, regression detection, cost database, provider health, 8 cybernetic feedback loops, autocatalytic thesis, compound improvement.

### Refactoring-PRD (canonical)
- `refactoring-prd/03-cognitive-subsystems.md` §5 Cybernetic Self-Learning Architecture, §6 conceptual feedback loops
- `refactoring-prd/09-innovations.md` §VI Collective Calibration (31.6×), §VII Predictive Foraging, §X EvoSkills, §XI ADAS
- `refactoring-prd/07-implementation-priorities.md` §Tier 1M — 8 missing feedback loops
- `refactoring-prd/00-overview.md` §Autocatalytic Improvement (compound math)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/04-memory/10-research.md`
- `bardo-backup/prd/04-memory/11-roadmap.md`
- `bardo-backup/prd/12-inference/06-memory.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/09-memory-and-knowledge.md`
- `bardo-backup/tmp/mori-agents/07-self-improvement.md`
- `bardo-backup/tmp/mori-agents/19-practical-self-learning.md`
- `bardo-backup/tmp/mori-agents/22-efficiency-monitoring.md`
- `bardo-backup/tmp/mori-agents/23-model-routing-optimization.md`
- `bardo-backup/tmp/death/22-cybernetic-learning.md`
- `bardo-backup/tmp/mori-refactor-plan/06-phase-5-cybernetic.md`
- `bardo-backup/tmp/mori-refactor-plan/09-exponential-roadmap.md`
- `bardo-backup/tmp/mori-refactor-plan/11-cybernetic-learning-dashboard.md`
- `bardo-backup/tmp/agent-chain/self-improvement-frameworks.md`
- `bardo-backup/tmp/agent-chain-new/10-self-improvement.md`
- `bardo-backup/tmp/agent-chain/09-exponential-flywheels.md`

### Implementation plans
- `roko/tmp/implementation-plans/05-learning-wiring.md` — completed wiring reference (efficiency, cascade, experiments, adaptive gates)
- `roko/tmp/implementation-plans/modelrouting/08-learning-loops.md` — provider health, latency tracking, Pareto pruning, anomaly detection (2G.01–2G.20)
- `roko/tmp/implementation-plans/modelrouting/09-cost-normalization.md` — CostTable, blended cost (3:1), budget guardrails
- `roko/tmp/implementation-plans/modelrouting/10-model-experiments.md` — Thompson Sampling (Beta), discount factor, UCB1 fallback
- `roko/tmp/implementation-plans/modelrouting/11-research-context.md` — RouteLLM, MixLLM, FrugalGPT, GVU, GEPA, SAGE, ABC
- `roko/tmp/implementation-plans/modelrouting/12-advanced-patterns.md` — Thompson Sampling, PF, gate feedback, skills, contracts, drift
- `roko/tmp/implementation-plans/modelrouting/17-meta-learning-and-corrections.md` — **8 missing feedback loops**, stability (hysteresis, frequency separation), compound optimization
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §D (distillation pipeline, half-life values)

### Reference code
- `roko/crates/roko-learn/src/`
- `roko/crates/roko-learn/src/episode_logger.rs`
- `roko/crates/roko-learn/src/playbook.rs`
- `roko/crates/roko-learn/src/skill_library.rs`
- `roko/crates/roko-learn/src/cascade_router.rs`
- `roko/crates/roko-learn/src/efficiency.rs`
- `roko/crates/roko-learn/src/baseline.rs`
- `roko/crates/roko-learn/src/regression.rs`

---

## 06-neuro.md

**Covers**: Neuro subsystem (formerly Grimoire), 6 knowledge types (Insight/Heuristic/Warning/CausalLink/StrategyFragment/AntiKnowledge), 4-tier validation (Transient/Working/Consolidated/Persistent), HDC encoding, Ebbinghaus decay, tier × type half-life multiplication, cross-domain HDC transfer, Library of Babel, knowledge backup/restore, Ebbinghaus × tier model.

### Refactoring-PRD (canonical)
- `refactoring-prd/03-cognitive-subsystems.md` §1 Neuro — all sections (6 types, tiers, HDC encoding, cross-domain transfer)
- `refactoring-prd/04-knowledge-and-mesh.md` §1 Knowledge Architecture, §5 Knowledge Backup & Restore
- `refactoring-prd/01-synapse-architecture.md` §Decay enum (Ebbinghaus variant, `Ttl`, `HalfLife`)
- `refactoring-prd/09-innovations.md` §III Somatic Landscape (integration with Neuro), §XIII Cross-Domain Insight Resonance (false-positive thresholds)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/04-memory/00-overview.md`
- `bardo-backup/prd/04-memory/01-grimoire.md`
- `bardo-backup/prd/04-memory/01b-grimoire-memetic.md`
- `bardo-backup/prd/04-memory/01c-grimoire-hdc.md`
- `bardo-backup/prd/04-memory/02-emotional-memory.md`
- `bardo-backup/prd/04-memory/06-economy.md`
- `bardo-backup/prd/04-memory/09-safety.md`
- `bardo-backup/prd/04-memory/13-library-of-babel.md`
- `bardo-backup/prd/shared/hdc-vsa.md`
- `bardo-backup/prd/shared/hdc-applications.md`
- `bardo-backup/prd/shared/hdc-fingerprints.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/09-memory-and-knowledge.md`
- `bardo-backup/tmp/mori-refactor/12-cognitive-architecture.md`
- `bardo-backup/tmp/agent-chain/04-hdc.md` — HDC math from first principles
- `bardo-backup/tmp/agent-chain/05-knowledge-layer.md` — 6 knowledge types origin
- `bardo-backup/tmp/agent-chain/15-dynamic-context-assembly.md`
- `bardo-backup/tmp/death/tools/c01-retrieval.md`
- `bardo-backup/tmp/death/tools/c02-structure.md`
- `bardo-backup/tmp/death/tools/c03-token-economics.md`
- `bardo-backup/tmp/death/tools/c04-pre-computation.md`

### Implementation plans
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §D Knowledge & Memory (D1–D18: 4-tier distillation pipeline, knowledge types + storage, HDC integration with half-lives Insight 30d / Heuristic 90d / Fact 365d; confirmation boost ×1.5)
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §R1 roko-neuro crate creation

### Reference code
- `roko/crates/roko-neuro/src/` (to be created; see Tier 0C dissolution)
- `roko/crates/bardo-primitives/src/hdc.rs` — to rename `roko-primitives`
- `roko/crates/roko-index/src/hdc.rs`
- `roko/crates/roko-learn/src/hdc_clustering.rs`
- `roko/crates/roko-golem/src/grimoire.rs` — scaffold to delete after R1

---

## 07-conductor.md

**Covers**: Reactive intelligence layer, 10 watchers, circuit breaker, graduated interventions, diagnosis engine (34 error patterns), stuck detection, health monitors, cybernetic loop, meta-cognition, Good Regulator Theorem, precision-weighted prediction errors, Yerkes-Dodson curves.

### Refactoring-PRD (canonical)
- `refactoring-prd/02-five-layers.md` §Conductor as Meta-Cognition (in Layer 3 section)
- `refactoring-prd/03-cognitive-subsystems.md` §5 cybernetic self-learning, §Self-Model (Good Regulator), §Ashby's Law
- `refactoring-prd/09-innovations.md` §XII.2 Cognitive Signals (Pause/Resume/Reprioritize/InjectContext/Escalate/Cooldown/Explore/Shutdown)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/25-mori/mori-resilience.md`
- `bardo-backup/prd/13-runtime/21-cybernetic-loops.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/07-orchestration.md` (conductor sections)
- `bardo-backup/tmp/death/21-agent-optimization.md`
- `bardo-backup/tmp/mori-refactor-plan/10-failure-prevention.md`
- `bardo-backup/tmp/mori-refactor-plan/08-design-principles.md`
- `bardo-backup/tmp/mori-refactor-plan/00-issues-catalog.md` — 21 production failures

### Implementation plans
- `roko/tmp/implementation-plans/modelrouting/08-learning-loops.md` — circuit breaker (3-state), anomaly detection
- `roko/tmp/implementation-plans/modelrouting/16-production-hardening.md` — adaptive timeouts (p95×2), full-jitter backoff, per-provider semaphores, graceful shutdown (3-phase drain), content-addressed dedup cache
- `roko/tmp/implementation-plans/06-process-management.md`

### Reference code
- `roko/crates/roko-conductor/src/`
- `roko/crates/bardo-runtime/src/process.rs` — to rename `roko-runtime`

---

## 08-chain.md

**Covers** (**LARGEST DOC**): Korai chain vision and architecture, HDC on-chain precompile, KORAI/DAEJI tokenomics with demurrage, stigmergy theory, ChainClient/ChainWallet traits, mirage-rs in-process EVM simulator, chain intelligence (block ingestion, witness, ABI decoder), triage (curiosity scoring, MIDAS), the 6 Solidity contracts, exponential flywheels, agent marketplace (Spore/Sparrow), Korai Passport (ERC-721 soulbound), 4-tier gossip, job market (3 hiring models), reputation framework, clearing/settlement, Valhalla privacy.

### Refactoring-PRD (canonical)
- `refactoring-prd/04-knowledge-and-mesh.md` — full chain spec: Korai, HDC on-chain, KORAI economics, ERC-8004, mesh, stigmergy, C-Factor, mirage-rs
- `refactoring-prd/09-innovations.md` §VI Collective Calibration, §VIII x402 Micropayments, §XVI Knowledge Futures Market, §XIII Cross-Domain Resonance
- `refactoring-prd/07-implementation-priorities.md` §Tier 6 — Korai chain roadmap (deferred)
- `refactoring-prd/05-agent-types.md` §3 Chain Agent (9-step heartbeat mapping to universal loop)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/01-golem/00-overview.md`
- `bardo-backup/prd/14-chain/00-architecture.md`
- `bardo-backup/prd/14-chain/01-witness.md`
- `bardo-backup/prd/14-chain/02-triage.md`
- `bardo-backup/prd/14-chain/03-protocol-state.md`
- `bardo-backup/prd/14-chain/04-chain-scope.md`
- `bardo-backup/prd/14-chain/05-heartbeat-integration.md`
- `bardo-backup/prd/14-chain/06-events-signals.md`
- `bardo-backup/prd/14-chain/07-generative-views.md`
- `bardo-backup/prd/14-chain/08-stream-api.md`
- `bardo-backup/prd/14-chain/09-anomaly-detection.md`
- `bardo-backup/prd/15-dev/01-mirage-rs.md`
- `bardo-backup/prd/15-dev/01b-mirage-rpc.md`
- `bardo-backup/prd/15-dev/01c-mirage-scenarios.md`
- `bardo-backup/prd/15-dev/01d-mirage-integration.md`
- `bardo-backup/prd/15-dev/01e-mirage-tx-compatibility.md`
- `bardo-backup/prd/shared/chains.md`

### Legacy research/tmp
- `bardo-backup/tmp/agent-chain/01-overview.md` — CoALA + chain
- `bardo-backup/tmp/agent-chain/02-chain-architecture.md`
- `bardo-backup/tmp/agent-chain/03-stigmergy.md`
- `bardo-backup/tmp/agent-chain/04-hdc.md`
- `bardo-backup/tmp/agent-chain/05-knowledge-layer.md`
- `bardo-backup/tmp/agent-chain/06-tokenomics.md` — rename GNOS→KORAI/DAEJI
- `bardo-backup/tmp/agent-chain/07-implementation.md`
- `bardo-backup/tmp/agent-chain/08-references.md`
- `bardo-backup/tmp/agent-chain/09-exponential-flywheels.md`
- `bardo-backup/tmp/agent-chain/10-predictive-foraging.md`
- `bardo-backup/tmp/agent-chain/11-adversarial-defense-and-value.md`
- `bardo-backup/tmp/agent-chain/12-golem-orchestrators.md`
- `bardo-backup/tmp/agent-chain/13-orchestration-as-a-service.md`
- `bardo-backup/tmp/agent-chain/14-academic-foundations.md`
- `bardo-backup/tmp/agent-chain/15-dynamic-context-assembly.md`
- `bardo-backup/tmp/agent-chain/16-mirage-rs-poc.md`
- `bardo-backup/tmp/agent-chain/17-autonomous-eval-generation.md`
- `bardo-backup/tmp/agent-chain/README.md`
- `bardo-backup/tmp/agent-chain/agent-chain-research2.md`
- `bardo-backup/tmp/agent-chain/agent-research2.md`
- `bardo-backup/tmp/agent-chain/context-quality-science.md`
- `bardo-backup/tmp/agent-chain/eval-research.md`
- `bardo-backup/tmp/agent-chain/exponential-mechanisms-research.md`
- `bardo-backup/tmp/agent-chain/proving-collective-intelligence.md`
- `bardo-backup/tmp/agent-chain-new/01-vision.md` through `14-implementation.md` (all 14 files)
- `bardo-backup/tmp/hyperliquid/` (all files — HyperEVM research)

### Implementation plans
- `roko/tmp/implementation-plans/12b-chain-layer.md` — 76 items, 11 sections: Identity (Korai Passport, tiers, ventriloquist defense), Gossip (4-tier, 8 topics, GossipSub v1.1), Job Market (Spore/Sparrow, 3 hiring models, power-of-two-choices), ChainWitness, Reputation (7-domain EMA, tiers, disputes), Payments (DAEJI token, x402 micropayments), Safety, ISFR (collective price discovery), Clearing (QP solver, bisection), Privacy (Valhalla TEE, PSI, ZK proofs), Mirage, Crate cleanup
- `roko/tmp/implementation-plans/12-nunchi-integration.md` — historical split context
- `roko/tmp/implementation-plans/10-golem-integration.md` — superseded but has context

### Reference code
- `roko/crates/roko-chain/src/`
- `roko/apps/mirage-rs/src/`
- `bardo-backup/crates/golem-chain/src/` (reference only, read-only)
- `bardo-backup/crates/golem-chain-intelligence/src/` (reference only)
- `bardo-backup/crates/golem-triage/src/` (reference only)
- `bardo-backup/crates/golem-uniswap/src/` (reference only)

---

## 09-daimon.md

**Covers**: PAD (Pleasure/Arousal/Dominance) affect engine, ALMA three-layer temporal model (emotion/mood/personality), OCC/Scherer appraisal pipeline, somatic markers (Damasio), mood-congruent memory retrieval, collective emotional contagion with exponential decay, dream-daimon and runtime-daimon, behavioral influence on decisions, 8D somatic landscape, 6 behavioral states (NO mortality).

### Refactoring-PRD (canonical)
- `refactoring-prd/03-cognitive-subsystems.md` §2 Daimon — all sections (PAD vector, 6 behavioral states, somatic markers, coding integration, integration points)
- `refactoring-prd/09-innovations.md` §III Somatic Landscape (k-d tree, 15% contrarian retrieval, concrete struct), §XIX.F 8D Somatic Strategy Space (default coding dims, chain dims alternative), §XIX.E VCG Bid (affect weight)
- `refactoring-prd/08-translation-guide.md` §**INCOMPATIBLE: Emotion Mapped to Mortality**
- `refactoring-prd/07-implementation-priorities.md` §Tier 2D–2E (Daimon wiring)

### Legacy PRD
- `bardo-backup/prd/03-daimon/00-overview.md`
- `bardo-backup/prd/03-daimon/01-appraisal.md`
- `bardo-backup/prd/03-daimon/02-emotion-memory.md`
- `bardo-backup/prd/03-daimon/03-behavior.md`
- `bardo-backup/prd/03-daimon/06-dream-daimon.md`
- `bardo-backup/prd/03-daimon/07-runtime-daimon.md`
- `bardo-backup/prd/03-daimon/08-infrastructure.md`
- `bardo-backup/prd/03-daimon/09-evaluation.md`

### Legacy PRD — SKIP ENTIRELY (death-related)
- `bardo-backup/prd/03-daimon/04-mortality-daimon.md` — extract non-death concepts only (somatic citations, ALMA citations)
- `bardo-backup/prd/03-daimon/05-death-daimon.md` — SKIP entirely

### Legacy research/tmp
- (Cross-reference via mori-refactor, agent-chain for affect citations)

### Implementation plans
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §F Daimon (F1–F9: PadVector, 8 octant states, appraisal triggers, decay, behavior modulation table, affect signatures on episodes, affect → SystemPromptBuilder, affect → CascadeRouter, persistence)
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §R2 roko-daimon crate creation

### Reference code
- `roko/crates/roko-daimon/src/` (to be created)
- `roko/crates/roko-golem/src/daimon.rs` — scaffold (972 lines, to move after dissolution)

---

## 10-dreams.md

**Covers**: 3-phase dream cycle (NREM replay + REM imagination + integration staging), Mattar-Daw prioritized replay, Boden 3 creativity modes, Pearl structural causal models, emotional depotentiation, HDC counterfactual synthesis, SQLite staging buffer, sleep-time compute (Lin 2025), WSCL forgetting reduction, hypnagogia engine (Thalamic Gate + Executive Loosener + Dali Interrupt + Homuncular Observer), Oneirography, Venice dreaming. NO death proximity triggers.

### Refactoring-PRD (canonical)
- `refactoring-prd/03-cognitive-subsystems.md` §3 Dreams — all sections
- `refactoring-prd/09-innovations.md` §IV Alpha Convergence Problem & Hypnagogia (concrete LLM recipe), §V Dream Engine 3-phase (NREM Mattar-Daw, REM Boden+Pearl, integration), §XIX.G Dream Scheduling (idle-time, cheap model, concurrent)
- `refactoring-prd/08-translation-guide.md` §**INCOMPATIBLE: Dream as Approaching Death**
- `refactoring-prd/07-implementation-priorities.md` §Tier 2F–2G (Dreams wiring, counterfactual simulation)

### Legacy PRD
- `bardo-backup/prd/05-dreams/00-overview.md`
- `bardo-backup/prd/05-dreams/01-architecture.md`
- `bardo-backup/prd/05-dreams/01b-dream-evolution.md`
- `bardo-backup/prd/05-dreams/02-replay.md`
- `bardo-backup/prd/05-dreams/03-imagination.md`
- `bardo-backup/prd/05-dreams/04-consolidation.md`
- `bardo-backup/prd/05-dreams/05-threats.md`
- `bardo-backup/prd/05-dreams/06-integration.md`
- `bardo-backup/prd/05-dreams/07-venice-dreaming.md`
- `bardo-backup/prd/06-hypnagogia/00-overview.md`
- `bardo-backup/prd/06-hypnagogia/01-neuroscience.md` — full neuroscience basis
- `bardo-backup/prd/06-hypnagogia/02-architecture.md`
- `bardo-backup/prd/06-hypnagogia/03-divergence-alpha.md`
- `bardo-backup/prd/06-hypnagogia/04-homunculus.md`
- `bardo-backup/prd/06-hypnagogia/05-hauntology.md` — Derrida, traces
- `bardo-backup/prd/06-hypnagogia/06-xenocognition.md`
- `bardo-backup/prd/06-hypnagogia/07-inner-worlds.md`
- `bardo-backup/prd/22-oneirography/00-overview.md`
- `bardo-backup/prd/22-oneirography/01-dream-journals.md`
- `bardo-backup/prd/22-oneirography/03-self-appraisal.md`
- `bardo-backup/prd/22-oneirography/04-auctions.md`
- `bardo-backup/prd/22-oneirography/05-extended-forms.md`

### Legacy PRD — SKIP
- `bardo-backup/prd/22-oneirography/02-death-masks.md`

### Implementation plans
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §G Dreams (G1–G8: replay scheduler, mistake ID, heuristic strengthen/weaken, counterfactual via HDC permutation, cross-episode consolidation, novel strategy gen)
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §R3 roko-dreams crate creation

### Reference code
- `roko/crates/roko-dreams/src/` (scaffold to be expanded)
- `roko/crates/roko-golem/src/dreams.rs` — placeholder (43 lines, to delete after R3)
- `roko/crates/roko-golem/src/hypnagogia.rs` — placeholder (42 lines, to move to roko-dreams)

---

## 11-safety.md

**Covers**: Defense-in-depth, capability tokens, content-addressed audit chain, taint-aware ingestion and propagation, permits and allowlists, loop detection, sandboxing (validation-only), prompt security and injection prevention, threat model, adaptive risk management, MEV protection, temporal logic verification, witness DAG, formal verification pipeline, Engram Syscalls (Cognitive Namespaces). **CRITICAL**: safety policies exist but dispatcher never calls them — #1 integration gap.

### Refactoring-PRD (canonical)
- `refactoring-prd/01-synapse-architecture.md` §Provenance & Attestation, §Taint tracking, §Decay (memory management not mortality)
- `refactoring-prd/09-innovations.md` §IX Forensic AI / Causal Replay Engine (regulatory compliance table), §XII Cognitive Kernel Primitives (Cognitive Namespaces with ACL, Engram Syscalls single-enforcement-point)
- `refactoring-prd/07-implementation-priorities.md` §Tier 1G Production Hardening
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/10-safety/00-defense.md`
- `bardo-backup/prd/10-safety/01-custody.md`
- `bardo-backup/prd/10-safety/02-policy.md`
- `bardo-backup/prd/10-safety/03-ingestion.md`
- `bardo-backup/prd/10-safety/04-prompt-security.md`
- `bardo-backup/prd/10-safety/05-threat-model.md`
- `bardo-backup/prd/10-safety/06-adaptive-risk.md`
- `bardo-backup/prd/10-safety/07-temporal-logic-verification.md`
- `bardo-backup/prd/10-safety/08-witness-dag.md`
- `bardo-backup/prd/10-safety/09-formal-verification-pipeline.md`
- `bardo-backup/prd/10-safety/10-mev-protection.md`
- `bardo-backup/prd/04-memory/09-safety.md`
- `bardo-backup/prd/12-inference/07-safety.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/11-safety-observability-learning.md`
- `bardo-backup/tmp/roko-progress/09-refactor-gaps.md` — safety gaps
- `bardo-backup/tmp/mori-agents/11-benchmarks-and-evals.md`
- `bardo-backup/tmp/agent-chain/11-adversarial-defense-and-value.md`

### Implementation plans
- `roko/tmp/implementation-plans/03-safety-hooks.md`
- `roko/tmp/implementation-plans/11-inconsistencies.md` — documents the dispatcher-never-calls-safety gap
- `roko/tmp/implementation-plans/12b-chain-layer.md` §P Privacy (Valhalla TEE, PSI, ZK range proofs)

### Reference code
- `roko/crates/roko-agent/src/safety/` — SafetyLayer (256 lines) wired to ToolDispatcher via `.with_safety(layer)`
- `roko/crates/roko-orchestrator/src/safety/`
- `roko/crates/roko-agent/src/dispatcher/mod.rs`
- `bardo-backup/crates/golem-safety/src/` (reference only)

---

## 12-interfaces.md

**Covers**: CLI commands, TUI design (29 screens, ROSEDUST color system, Spectre creature viewport), Web Portal, MCP server for code intelligence, Portal concept, ROSEDUST design language, Spectre procedural generation, Generative UI (A2UI), port allocation, agent onboarding flow.

### Refactoring-PRD (canonical)
- `refactoring-prd/06-interfaces.md` — full interface spec: CLI commands, HTTP API, TUI 29 screens, Spectre, Web Portal, ROSEDUST palette
- `refactoring-prd/09-innovations.md` §XIV Generative Interfaces (A2UI) — agents create their own UI
- `refactoring-prd/08-translation-guide.md` §INCOMPATIBLE: Death Phases as UX (replace vitality phases with cognitive states)
- `refactoring-prd/10-developer-guide.md` §1 Quick Start, §11 CLI UX (progressive help, status, error-as-teacher)
- `refactoring-prd/07-implementation-priorities.md` §Tier 4 Interfaces

### Legacy PRD
- `bardo-backup/prd/18-interfaces/00-portal.md`
- `bardo-backup/prd/18-interfaces/01-cli.md`
- `bardo-backup/prd/18-interfaces/02-ui-system.md` — original ROSEDUST component system
- `bardo-backup/prd/18-interfaces/03-tui.md`
- `bardo-backup/prd/18-interfaces/19-spatial-grammar.md`
- `bardo-backup/prd/18-interfaces/26-bardo-terminal-foundation.md`
- `bardo-backup/prd/18-interfaces/28-creature-system.md` — Spectre origins
- `bardo-backup/prd/18-interfaces/perspective/` (all files)
- `bardo-backup/prd/18-interfaces/protocol/` (all files)
- `bardo-backup/prd/18-interfaces/rendering/` (all files)
- `bardo-backup/prd/18-interfaces/screens/` (all files)
- `bardo-backup/prd/25-mori/mori-interfaces.md`
- `bardo-backup/prd/15-dev/03-debug-ui.md`
- `bardo-backup/prd/shared/branding.md` — brand guidelines
- `bardo-backup/prd/shared/port-allocation.md`

### Legacy research/tmp
- `bardo-backup/tmp/death/05-interfaces.md`
- `bardo-backup/tmp/death/06-server-and-remote.md`
- `bardo-backup/tmp/mori-refactor/17-human-agent-interface.md`
- `bardo-backup/tmp/mori-refactor-plan/16-tui-and-support-cleanup.md`
- `bardo-backup/tmp/mori-agents/13-cli-and-deployment.md`
- `bardo-backup/tmp/death/tools/06-mcp-context-server.md`

### Implementation plans
- `roko/tmp/implementation-plans/09-tui-dashboard.md`
- `roko/tmp/implementation-plans/11-agent-dogfooding.md` §Phase 0-1 (roko-serve extraction, HTTP API, WebSocket)
- `roko/tmp/implementation-plans/11-sections/phase-0-1.md` — roko-serve creation, webhook routes, dispatch loop

### Reference code
- `roko/crates/roko-cli/src/` — CLI entry point (38 tests)
- `roko/crates/roko-serve/src/` (scaffold)
- `bardo-backup/apps/mori/src/tui/` (50+ files, reference only)
- `bardo-backup/apps/bardo-terminal/src/` (reference only)
- `bardo-backup/crates/mori-mcp/src/` (reference only)

---

## 13-coordination.md

**Covers**: Stigmergy theory (Grassé 1959), digital pheromones (Parunak 2002), pheromone field semantics (Threat/Opportunity/Wisdom + Alpha/Pattern/Anomaly/Consensus + Custom), PheromoneScope (Local/Mesh/Global), agent mesh sync (WS/Iroh), morphogenetic specialization, knowledge exchange, P2P transport, collective intelligence emergence, exponential flywheels (Reed's Law), generalized stigmergy beyond blockchain (git commits, code patterns).

### Refactoring-PRD (canonical)
- `refactoring-prd/04-knowledge-and-mesh.md` §3 Agent Mesh P2P Connectivity, §4 Stigmergy — Generalized Indirect Coordination
- `refactoring-prd/02-five-layers.md` §Stigmergy (git as stigmergy, knowledge as stigmergy, pheromone types, cross-domain)
- `refactoring-prd/09-innovations.md` §VI Network Flywheel, §XIII Cross-Domain Insight Resonance
- `refactoring-prd/05-agent-types.md` §2 Pheromones (coding agent PATTERN traces)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/02-mortality/10-clade-ecology.md` — rename clade → Collective/Mesh
- `bardo-backup/prd/02-mortality/10b-morphogenetic-specialization.md`
- `bardo-backup/prd/09-economy/04-coordination.md`
- `bardo-backup/prd/13-runtime/06-collective-intelligence.md`
- `bardo-backup/prd/20-styx/00-architecture.md`
- `bardo-backup/prd/20-styx/03-clade-sync.md` — rename clade → Collective/Mesh
- `bardo-backup/prd/20-styx/04-marketplace.md`
- `bardo-backup/prd/20-styx/07-p2p-transport.md`
- `bardo-backup/prd/20-styx/08-transport-config.md`

### Legacy research/tmp
- `bardo-backup/tmp/agent-chain/03-stigmergy.md` — full stigmergy spec
- `bardo-backup/tmp/agent-chain/proving-collective-intelligence.md`
- `bardo-backup/tmp/agent-chain/09-exponential-flywheels.md`
- `bardo-backup/tmp/agent-chain-new/02-coordination-theory.md`
- `bardo-backup/tmp/mori-refactor/18-agent-ecology.md`
- `bardo-backup/tmp/death/tools/c05-multi-agent.md`

### Implementation plans
- `roko/tmp/implementation-plans/12b-chain-layer.md` §B Gossip (4-tier, GossipSub v1.1, 8 topics, 3-layer peer scoring)
- `roko/tmp/implementation-plans/12b-chain-layer.md` §N ISFR (collective price discovery)

### Reference code
- `roko/crates/roko-chain/src/`
- `bardo-backup/apps/bardo-styx/src/` (reference only)
- `bardo-backup/crates/golem-coordination/src/` (reference only)

---

## 14-identity-economy.md

**Covers**: ERC-8004 agent identity (ERC-721 soulbound, Agent Card, endpoints), reputation system (EMA decay, 7-domain framework, halving, tiers, disputes), knowledge marketplace, commerce bazaar, Machine Payment Protocol (MPP) / x402 HTTP 402 / ERC-3009 signatures, agent economy, KORAI/DAEJI tokenomics (1% demurrage, Vickrey reputation-adjusted auction), Knowledge Futures Market, a16z Series A framing.

### Refactoring-PRD (canonical)
- `refactoring-prd/04-knowledge-and-mesh.md` §2 Korai The Agent Chain, §2 KORAI Token Economics (demurrage), §3 ERC-8004
- `refactoring-prd/09-innovations.md` §VIII x402 Micropayments, §XVI Knowledge Futures Market, §IX Forensic AI (regulatory compliance moat), §XVIII Blue Ocean Summary
- `refactoring-prd/07-implementation-priorities.md` §Tier 6E/6G (Agent registry, reputation system), §What Makes This a Series A Story
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/09-economy/00-identity.md`
- `bardo-backup/prd/09-economy/01-reputation.md`
- `bardo-backup/prd/09-economy/02-clade.md` — rename clade → Collective
- `bardo-backup/prd/09-economy/03-marketplace.md`
- `bardo-backup/prd/09-economy/05-agent-economy.md`
- `bardo-backup/prd/09-economy/06-commerce-bazaar.md`
- `bardo-backup/prd/shared/x402-protocol.md`
- `bardo-backup/prd/shared/eip-analysis.md`

### Legacy research/tmp
- `bardo-backup/tmp/agent-chain/06-tokenomics.md` — rename GNOS → KORAI/DAEJI
- `bardo-backup/tmp/agent-chain/12-golem-orchestrators.md` — OaaS
- `bardo-backup/tmp/agent-chain/13-orchestration-as-a-service.md`
- `bardo-backup/tmp/agent-chain-new/05-token-economics.md`
- `bardo-backup/tmp/agent-chain-new/12-agent-economy.md`
- `bardo-backup/tmp/agent-chain-new/11-adversarial-defense.md`
- `bardo-backup/tmp/death/14-proposals-and-billing.md`
- `bardo-backup/tmp/death/15-cost-tracking.md`
- `bardo-backup/tmp/death/payments/` (all 10 files)

### Implementation plans
- `roko/tmp/implementation-plans/12b-chain-layer.md` §A Agent Identity (Korai Passport, 4 tiers, ventriloquist defense), §C Job Market (Spore/Sparrow, 3 hiring models, Vickrey reputation-adjusted), §K Reputation (7-domain EMA, disputes), §L Payments (DAEJI token, x402, escrow), §N ISFR, §O Clearing (QP solver, bisection)

### Reference code
- `bardo-backup/crates/golem-identity/src/` (reference only)
- `bardo-backup/crates/golem-economy/src/` (reference only)
- `bardo-backup/crates/mpp/src/` (reference only)
- `bardo-backup/apps/mori-service/src/` (reference only)

---

## 15-code-intelligence.md

**Covers**: Incremental code indexing (tree-sitter), symbol extraction and directed dependency graph, PageRank for symbol importance, HDC fingerprints for structural similarity, context assembly from code search, MCP context server design, index.db scaling, snapshot optimization.

### Refactoring-PRD (canonical)
- `refactoring-prd/05-agent-types.md` §2 Coding Agent (indexing role), §2 Niche Construction (affordance assessment)
- `refactoring-prd/00-overview.md` §Crate Map (roko-index status)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/15-dev/06-indexer.md`
- `bardo-backup/prd/07-tools/13-tools-intelligence.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/10-code-intelligence.md`
- `bardo-backup/tmp/mori-agents/18-code-intelligence-and-gateway.md`
- `bardo-backup/tmp/death/tools/02-code-index.md`
- `bardo-backup/tmp/death/docs/30-index-performance.md`

### Implementation plans
- (None directly — indexing is already in roko-index, referenced by cognitive layer)

### Reference code
- `roko/crates/roko-index/src/`
- `roko/crates/roko-index/src/hdc.rs`
- `roko/crates/roko-lang-rust/`, `roko-lang-typescript/`, `roko-lang-go/`
- `bardo-backup/crates/mori-index/src/` (reference only)
- `bardo-backup/crates/mori-context/src/` (reference only)

---

## 16-heartbeat.md

**Covers**: CoALA 9-step cognitive pipeline, mapping to universal Synapse loop, 3-speed cognition (Gamma ~5-15s / Theta ~75s / Delta hours), adaptive clock, gating (when to suppress/escalate ticks), context governor, attention auctions (VCG), sleepwalker 3-step variant, CorticalState, dual-process (System 1 / System 2), active inference for compute allocation.

### Refactoring-PRD (canonical)
- `refactoring-prd/01-synapse-architecture.md` §3 Universal Cognitive Loop (9 steps), §Three Cognitive Speeds, §Dual-Process Cognition, §4 Active Inference Integration
- `refactoring-prd/03-cognitive-subsystems.md` §1–5 (all — these subsystems drive the heartbeat)
- `refactoring-prd/02-five-layers.md` §Adaptive Clock (in Layer 0 Runtime)
- `refactoring-prd/09-innovations.md` §I 16 Zero-Cost Cognitive Probes (T0 Layer), §II VCG Attention Auction
- `refactoring-prd/05-agent-types.md` §3 CoALA Heartbeat mapping
- `refactoring-prd/08-translation-guide.md` §7 Golem Heartbeat → Universal Loop

### Legacy PRD
- `bardo-backup/prd/01-golem/02-heartbeat.md`
- `bardo-backup/prd/01-golem/01-cognition.md`
- `bardo-backup/prd/01-golem/03-mind.md`
- `bardo-backup/prd/01-golem/03b-cognitive-mechanisms.md`
- `bardo-backup/prd/01-golem/03c-state-management.md`
- `bardo-backup/prd/01-golem/14-context-governor.md`
- `bardo-backup/prd/01-golem/14b-attention-auction.md`
- `bardo-backup/prd/01-golem/15-sleepwalker.md`
- `bardo-backup/prd/01-golem/16-risk-engine.md`
- `bardo-backup/prd/01-golem/17-prediction-engine.md`
- `bardo-backup/prd/01-golem/18-cortical-state.md`
- `bardo-backup/prd/12-inference/01-deployment-modes.md`
- `bardo-backup/prd/12-inference/01a-routing.md`
- `bardo-backup/prd/12-inference/15-inference-profiles.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-refactor/12-cognitive-architecture.md`
- `bardo-backup/tmp/mori-refactor/03-runtime.md` — adaptive clock spec
- `bardo-backup/tmp/agent-chain/01-overview.md` — CoALA mapping

### Implementation plans
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §I Operating Frequencies (I1–I5: Gamma/Theta/Delta loops, frequency scheduler, meta-cognition hook — "Am I stuck? Should I escalate?")

### Reference code
- `roko/crates/bardo-runtime/src/` — to rename `roko-runtime`
- `roko/crates/bardo-primitives/src/tier.rs` — InferenceTier, TierRouter
- `roko/crates/roko-learn/src/cascade_router.rs`
- `bardo-backup/crates/golem-heartbeat/src/` (reference only)
- `bardo-backup/crates/golem-runtime/src/` (reference only)

---

## 17-lifecycle.md

**Covers** (**REPLACES mortality**): Agent creation and provisioning, configuration and operator model, funding and compute provisioning, knowledge transfer via backup/restore (replaces "succession"), agent deletion/teardown (user-initiated only), knowledge staleness via epistemic decay (Ebbinghaus, not mortality), knowledge demurrage, replication (spawn from existing). NO death, no Thanatopsis, no stochastic death, no necrocracy.

### Refactoring-PRD (canonical)
- `refactoring-prd/04-knowledge-and-mesh.md` §5 Knowledge Backup & Restore (4-step: BACKUP → DELETE → CREATE → RESTORE)
- `refactoring-prd/08-translation-guide.md` §**ALL incompatibility sections** (Mortality → Resource Management, Succession → Backup/Restore, etc.)
- `refactoring-prd/07-implementation-priorities.md` §Dropped Items (full removal list)
- `refactoring-prd/03-cognitive-subsystems.md` §1 Tier Progression (Ebbinghaus decay × tier multiplier)
- `refactoring-prd/01-synapse-architecture.md` §Decay enum (Ebbinghaus variant — memory management not mortality)

### Legacy PRD (extract non-death concepts, reframe as lifecycle)
- `bardo-backup/prd/01-golem/06-creation.md`
- `bardo-backup/prd/01-golem/07-provisioning.md`
- `bardo-backup/prd/01-golem/08-funding.md`
- `bardo-backup/prd/01-golem/09-inheritance.md`
- `bardo-backup/prd/01-golem/10-replication.md`
- `bardo-backup/prd/01-golem/11-lifecycle.md`
- `bardo-backup/prd/01-golem/12-teardown.md`
- `bardo-backup/prd/01-golem/19-config-and-operator-model.md`
- `bardo-backup/prd/02-mortality/02-epistemic-decay.md` — concept of knowledge staleness is still relevant
- `bardo-backup/prd/02-mortality/05-knowledge-demurrage.md` — knowledge decay over time
- `bardo-backup/prd/02-mortality/07-succession.md` — knowledge transfer mechanism (reframe)
- `bardo-backup/prd/02-mortality/14-research-foundations.md` — 130+ papers, keep citations
- `bardo-backup/prd/02-mortality/15-references.md` — 162 citations
- `bardo-backup/prd/04-memory/03-mortal-memory.md` — reframe as lifecycle memory
- `bardo-backup/prd/11-compute/00-overview.md`
- `bardo-backup/prd/11-compute/01-architecture.md`
- `bardo-backup/prd/11-compute/02-provisioning.md`

### Legacy PRD — SKIP ENTIRELY
- `bardo-backup/prd/02-mortality/00-thesis.md`
- `bardo-backup/prd/02-mortality/01-architecture.md`
- `bardo-backup/prd/02-mortality/03-stochastic-mortality.md`
- `bardo-backup/prd/02-mortality/04-economic-mortality.md` — extract budget math only
- `bardo-backup/prd/02-mortality/06-thanatopsis.md`
- `bardo-backup/prd/02-mortality/08-mortality-affect.md` — extract somatic marker citations only
- `bardo-backup/prd/02-mortality/09-fractal-mortality.md`
- `bardo-backup/prd/02-mortality/11-immortal-control.md`
- `bardo-backup/prd/02-mortality/16-necrocracy.md`
- `bardo-backup/prd/02-mortality/18-antifragile-mortality.md`
- `bardo-backup/prd/01-golem/04-mortality.md`
- `bardo-backup/prd/01-golem/05-death.md`

### Implementation plans
- (None directly — lifecycle is user-initiated CLI workflow + config)

### Reference code
- `bardo-backup/crates/golem-mortality/src/` — extract non-death parts only (reference)
- `bardo-backup/crates/golem-identity/src/` — replication/succession (reference)
- `roko/crates/roko-cli/src/` — init, delete, backup, restore commands

---

## 18-tools.md

**Covers**: Tool architecture (ToolDef, ToolContext, ToolResult, ToolExecutor registry), 19 built-in tools, 423+ DeFi tools (chain domain plugin), tool categories (data, trading, LP, vault, lending, staking, restaking, derivatives, yield, safety, intelligence, memory, identity, wallet, streaming), tool profiles and configuration, wallet management, testing strategy, service integrations (MetaMask, Venice, Bankr, AgentCash, Uniswap, Slack, GitHub, Linear), MCP servers (GitHub 17 tools, Slack 8 tools, Scripts wrapper).

### Refactoring-PRD (canonical)
- `refactoring-prd/05-agent-types.md` §2 Coding Agent tools, §5 Operations Agent (MCP servers), §8 Adding a new LLM Backend, §Adding a New Domain
- `refactoring-prd/06-interfaces.md` §1 CLI `roko new` scaffolders for tools
- `refactoring-prd/10-developer-guide.md` §6 Plugin System (EventSource, MCP tool plugin, feedback collector, plugin loading & discovery)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/07-tools/00-overview.md`
- `bardo-backup/prd/07-tools/01-architecture.md`
- `bardo-backup/prd/07-tools/02-tools-data.md` through `19-tools-streaming.md` (all tool definitions)
- `bardo-backup/prd/07-tools/20-config.md`
- `bardo-backup/prd/07-tools/21-profiles.md`
- `bardo-backup/prd/07-tools/22-wallets.md`
- `bardo-backup/prd/07-tools/23-distribution.md`
- `bardo-backup/prd/07-tools/24-testing.md`
- `bardo-backup/prd/07-tools/IMPLEMENTATION-PLAN.md`
- `bardo-backup/prd/21-integrations/00-overview.md`
- `bardo-backup/prd/21-integrations/01-metamask.md`
- `bardo-backup/prd/21-integrations/02-venice.md`
- `bardo-backup/prd/21-integrations/03-bankr.md`
- `bardo-backup/prd/21-integrations/04-agentcash.md`
- `bardo-backup/prd/21-integrations/05-uniswap.md`

### Legacy research/tmp
- `bardo-backup/tmp/mori-agents/14-service-integrations.md`
- `bardo-backup/tmp/mori-agents/15-automation-workflows.md`

### Implementation plans
- `roko/tmp/implementation-plans/07-mcp-tool-wiring.md`
- `roko/tmp/implementation-plans/11-agent-dogfooding.md` §Phase 2 — MCP servers, 16 agent templates, 21+ subscriptions
- `roko/tmp/implementation-plans/11-sections/phase-2.md` — roko-mcp-github (17 tools), roko-mcp-slack (8 tools), roko-mcp-scripts (script wrapper, any language)
- `roko/tmp/implementation-plans/11-sections/phase-3-4.md` — 16 agent template definitions (doc-lifecycle, digest, meeting-sync, pr-review, triage, auto-plan, code-implementer, gate-fixer, action-tracker, pm-board, slack-notify, freshness, conflict-detector, enrich, pm-health, prd-ingestion)

### Reference code
- `roko/crates/roko-std/src/tool/`
- `roko/crates/roko-plugin/` (to be created per plan 11)
- `roko/crates/roko-mcp-*/` (to be created)
- `bardo-backup/crates/golem-tools/src/` (reference only)

---

## 19-deployment.md

**Covers**: Packaging and distribution strategy, configuration and state management, Fly.io deployment, Docker deployment (partial in roko), WASM compilation, daemon mode (launchd/systemd), edge/embedded (~500KB binary), playground architecture, multi-repo subscription config, secret management, cloud deployment, cross-repo coordination, remote orchestrator.

### Refactoring-PRD (canonical)
- `refactoring-prd/10-developer-guide.md` §5 Deployment Targets (Native/WASM/Docker/Daemon/Cloud/Edge)
- `refactoring-prd/05-agent-types.md` §8 Deployment Flexibility (native/WASM/Docker/daemon/edge table)
- `refactoring-prd/06-interfaces.md` §7 Port Allocation
- `refactoring-prd/07-implementation-priorities.md` §Tier 3H (daemon, cloud deploy)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/13-runtime/10-packaging-deployment.md`
- `bardo-backup/prd/15-dev/02-deployment.md`
- `bardo-backup/prd/15-dev/05-tooling.md`
- `bardo-backup/prd/25-mori/mori-deployment.md`

### Legacy research/tmp
- `bardo-backup/tmp/death/07-project-deployment.md`
- `bardo-backup/tmp/death/29-fly-deploy.md`
- `bardo-backup/tmp/production/00-overview.md`
- `bardo-backup/tmp/production/01-dependency-refactor.md`
- `bardo-backup/tmp/production/02-packaging-distribution.md`
- `bardo-backup/tmp/production/03-config-and-state.md`
- `bardo-backup/tmp/production/04-deployment.md`
- `bardo-backup/tmp/production/05-migration-plan.md`
- `bardo-backup/tmp/production/06-playground-architecture.md`
- `bardo-backup/tmp/mori-agents/13-cli-and-deployment.md`

### Implementation plans
- `roko/tmp/implementation-plans/11-agent-dogfooding.md` §Phase 5-6 (daemon lifecycle, multi-repo)
- `roko/tmp/implementation-plans/11-sections/phase-5-6.md` — daemon wiring, launchd plist (macOS), systemd unit (Linux), cloud deploy (Fly.io), remote orchestrator, subscription format, secret management
- `roko/tmp/implementation-plans/modelrouting/15-operational-surface.md` — CLI commands, testing, validation, dashboard, routing log, config migration
- `roko/tmp/implementation-plans/modelrouting/16-production-hardening.md` — timeouts, retries, concurrency, shutdown, serve API, hedging

### Reference code
- `bardo-backup/apps/mori/src/deploy/` (reference only)
- `roko/crates/roko-cli/src/` — daemon, init, config commands

---

## 20-technical-analysis.md

**Covers**: Generalized oracles and predictive systems across domains. Witness-as-TA, hyperdimensional technical analysis, spectral liquidity manifolds, adaptive signal metabolism, causal microstructure discovery, predictive geometry, resonant pattern ecosystem, DeFi-native TA, adversarial signal robustness, somatic TA, emergent multiscale intelligence. **Coding oracles** as TA equivalents (build time prediction, test failure probability, complexity drift, dependency risk, performance regression).

### Refactoring-PRD (canonical)
- `refactoring-prd/03-cognitive-subsystems.md` §4 Oracles & Predictive Systems (generalized trait, domain-specific oracles for Chain/Coding/Research, predictive foraging integration)
- `refactoring-prd/09-innovations.md` §VII Predictive Foraging (CalibrationTracker), §XIX.A Active Inference State Space (factorized POMDP)
- `refactoring-prd/01-synapse-architecture.md` §4 Active Inference Integration (EFE decomposition)
- `refactoring-prd/08-translation-guide.md`

### Legacy PRD
- `bardo-backup/prd/23-ta/00-witness-as-technical-analyst.md`
- `bardo-backup/prd/23-ta/01-hyperdimensional-technical-analysis.md`
- `bardo-backup/prd/23-ta/02-spectral-liquidity-manifolds.md`
- `bardo-backup/prd/23-ta/03-adaptive-signal-metabolism.md`
- `bardo-backup/prd/23-ta/04-causal-microstructure-discovery.md`
- `bardo-backup/prd/23-ta/05-predictive-geometry.md`
- `bardo-backup/prd/23-ta/06-resonant-pattern-ecosystem.md`
- `bardo-backup/prd/23-ta/07-defi-native-technical-analysis.md`
- `bardo-backup/prd/23-ta/08-adversarial-signal-robustness.md`
- `bardo-backup/prd/23-ta/09-somatic-technical-analysis.md`
- `bardo-backup/prd/23-ta/10-emergent-multiscale-intelligence.md`

### Legacy research/tmp
- `bardo-backup/tmp/agent-chain/10-predictive-foraging.md` — full predictive foraging spec, falsifiable prediction
- `bardo-backup/tmp/agent-chain/14-academic-foundations.md` — 15 research traditions

### Implementation plans
- `roko/tmp/implementation-plans/modelrouting/12-advanced-patterns.md` — predictive foraging calibration, residual aggregation
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §J C-Factor Metrics (J1–J12: information flow rate, turn-taking equality, social sensitivity proxy, knowledge integration, task diversity, convergence velocity)

### Reference code
- `roko/crates/roko-learn/src/` — prediction store, calibration
- `bardo-backup/crates/golem-triage/src/` (reference only)

---

## references.md

**Covers**: Master academic citation list, 200+ papers grouped by domain. Group by: (1) Lifecycle & Finite Agency, (2) Memory Consolidation, (3) Affective Computing, (4) Dreams & Offline Learning, (5) Coordination & Multi-Agent, (6) Biological Analogues, (7) Self-Learning Systems, (8) Context Engineering, (9) Security & Provenance, (10) HDC/VSA, (11) Market Microstructure, (12) Streaming Algorithms, (13) Signal Processing, (14) Philosophy, (15) Generational Learning, (16) Agent Harnesses & Tool Use, (17) Cybernetics & VSM, (18) Active Inference, (19) Process Reward Models, (20) Collective Intelligence, (21) Regulatory Compliance / C2PA / AI Act.

### Refactoring-PRD (must scan all for new citations)
- `refactoring-prd/01-synapse-architecture.md` — Scherer 2001, Kahneman, CLARION, Friston, Conant & Ashby, Kauffman, Bower 1981
- `refactoring-prd/02-five-layers.md` — Hewitt 1973, Agha 1986, Armstrong 2003, Hoare 1978, Milner 1999, Liu et al. "Lost in the Middle", Grassé 1959, Charnov 1976, Pirolli & Card 1999
- `refactoring-prd/03-cognitive-subsystems.md` — Ebbinghaus 1885, Kanerva, Plate, Frady, Kleyko 2022 ACM, Damasio 1994, Bower 1981, McClelland 1995, Lacaux 2021, Sumers et al. 2023 CoALA, Beer VSM, Woolley et al. 2010
- `refactoring-prd/04-knowledge-and-mesh.md` — Parunak et al. 2002, Reed's Law, Metcalfe, ERC-8004
- `refactoring-prd/05-agent-types.md` — Odling-Smee et al. 2003 (niche construction)
- `refactoring-prd/07-implementation-priorities.md` — FrugalGPT, DSPy, SWE-bench, CoALA, Meta-Harness Lee et al. 2026
- `refactoring-prd/09-innovations.md` — VCG (Vickrey 1961, Clarke 1971, Groves 1973), Damasio, Lacaux 2021, Dormio Haar Horowitz, Derrida 1993, Mattar & Daw 2018, Walker & van der Helm 2009, Boden, Pearl SCM, Hu et al. ICLR 2025 (ADAS), EvoSkills 2026, Kanerva 2009, Karpathy 2025, Meta-Harness arXiv:2603.28052, Chen et al. FrugalGPT arXiv:2305.05176, Parr et al. arXiv:2402.14460, Koudahl et al. arXiv:2412.10425, Johnson-Lindenstrauss 1984, Sumers 2023 arXiv:2309.02427, Park 2023 arXiv:2304.03442, Liu 2023 arXiv:2307.03172, Lewis 2020 RAG arXiv:2005.11401, Mehrabian 1996
- `refactoring-prd/10-developer-guide.md` — referenced works

### Legacy primary citation sources
- `bardo-backup/prd/shared/citations.md`
- `bardo-backup/prd/shared/research.md`
- `bardo-backup/prd/02-mortality/14-research-foundations.md` — 130+ papers
- `bardo-backup/prd/02-mortality/15-references.md` — 162 citations
- `bardo-backup/prd/04-memory/10-research.md`
- `bardo-backup/prd/shared/hdc-vsa.md`
- `bardo-backup/tmp/hyperliquid/new/shared/citations.md`
- `bardo-backup/tmp/agent-chain/08-references.md`
- `bardo-backup/tmp/agent-chain/14-academic-foundations.md`
- `bardo-backup/tmp/mori-agents/12-references.md`

### Implementation plans
- `roko/tmp/implementation-plans/modelrouting/11-research-context.md` — RouteLLM, MixLLM, FrugalGPT, GVU, GEPA, SAGE, ABC (23 sections)
- `roko/tmp/implementation-plans/12a-cognitive-layer.md` §Source Documents & Academic Foundations table (14 core references)

### Reference code
- (None — this is a pure citation consolidation)

---

## Unmapped supplementary files (cross-cutting, scan selectively)

These contain useful context but don't map to a single target doc:

### Legacy PRD overviews
- `bardo-backup/prd/SUMMARY.md` — 2141-line summary of all PRDs
- `bardo-backup/prd/prd-summary.md` — 2167-line summary
- `bardo-backup/prd/appendices/` (all)
- `bardo-backup/prd/shared/branding.md`
- `bardo-backup/prd/shared/config-reference.md`
- `bardo-backup/prd/shared/data-privacy.md`
- `bardo-backup/prd/shared/doc-standards.md`
- `bardo-backup/prd/shared/emergent-capabilities.md`
- `bardo-backup/prd/shared/evaluation.md`
- `bardo-backup/prd/shared/event-catalog.md`
- `bardo-backup/prd/shared/integrated-information.md`
- `bardo-backup/prd/shared/timeline.md`
- `bardo-backup/prd/12-inference/03-economics.md`
- `bardo-backup/prd/12-inference/08-observability.md`
- `bardo-backup/prd/12-inference/09-api.md`
- `bardo-backup/prd/12-inference/10-roadmap.md`
- `bardo-backup/prd/12-inference/11-privacy-trust.md`
- `bardo-backup/prd/12-inference/12-providers.md`
- `bardo-backup/prd/12-inference/17-streaming.md`
- `bardo-backup/prd/12-inference/18-golem-config.md`
- `bardo-backup/prd/12-inference/20-inference-parameters.md`
- `bardo-backup/prd/12-inference/21-inference-performance.md`
- `bardo-backup/prd/12-inference/sheaf-observation.md`
- `bardo-backup/prd/13-runtime/01-defi-activities.md` through `22-first-fifteen-minutes.md`
- `bardo-backup/prd/19-agents-skills/10-vault-agents.md`

### Legacy tmp overviews
- `bardo-backup/tmp/mori-refactor/02-current-state.md`
- `bardo-backup/tmp/mori-refactor/14-gaps-and-frontier.md`
- `bardo-backup/tmp/mori-refactor/15-migration-plan.md`
- `bardo-backup/tmp/mori-refactor/19-developmental-trajectory.md`
- `bardo-backup/tmp/mori-refactor/20-information-architecture.md`
- `bardo-backup/tmp/mori-refactor/23-crate-consolidation.md`
- `bardo-backup/tmp/mori-refactor/24-developer-experience.md`
- `bardo-backup/tmp/mori-refactor/25-config-and-server-polish.md`
- `bardo-backup/tmp/mori-refactor/26-module-docs-and-113-missing.md`
- `bardo-backup/tmp/mori-refactor-plan/` (all files not already mapped)
- `bardo-backup/tmp/mori-agents/` (remaining unmapped files)
- `bardo-backup/tmp/death/` (remaining unmapped files)
- `bardo-backup/tmp/roko-progress/` — unified primitives, dual-nature agents, config redesign, CLI compatibility mapping, language-agnostic design, MORI-PARITY-CHECKLIST (1,253 items), MISTAKES-LEARNED (30+ mistakes), COMPONENTS/ (140+ specs)

### Implementation plans — cross-cutting or superseded
- `roko/tmp/MASTER-PLAN.md` — single source of truth that supersedes MASTER-REMAINING-WORK.md, PROMPT-EXECUTOR-PARITY.md, and plans 07-10
- `roko/tmp/MASTER-REMAINING-WORK.md` (superseded by MASTER-PLAN)
- `roko/tmp/PROMPT-EXECUTOR-PARITY.md` (superseded by MASTER-PLAN)
- `roko/tmp/implementation-plans/07-mcp-tool-wiring.md` (superseded, content absorbed into MASTER-PLAN Tier 1C+2D)
- `roko/tmp/implementation-plans/08-observability-wiring.md` (superseded, Tier 1D)
- `roko/tmp/implementation-plans/09-tui-dashboard.md` (superseded, Tier 1H)
- `roko/tmp/implementation-plans/10-golem-integration.md` (superseded, see 12b-chain-layer.md)
- `roko/tmp/implementation-plans/12-nunchi-integration.md` (split into 12a+12b)
- `roko/tmp/DESIGN-PRD-COMMANDS.md`, `DESIGN-TASK-GENERATION.md`, `PROMPT-WIRING.md` — design notes
- `roko/tmp/MORI-VS-ROKO-COMPARISON.md`, `AUDIT-RESULTS.md`, `DEPTH-PASS-GUIDELINES.md`
- `roko/tmp/implementation-plans/modelrouting/13-architectural-gaps.md` — catalogs 33 gaps
- `roko/tmp/implementation-plans/modelrouting/14-integration-refinements.md`
- `roko/tmp/implementation-plans/modelrouting/18-structural-cleanup.md`
- `roko/tmp/implementation-plans/modelrouting/22-research-apis-backlog.md` — Semantic Scholar, Exa, Jina, Brave, Firecrawl, Tavily

### Parallel checklist (for cross-reference)
- `refactoring-prd/MIGRATION-CHECKLIST.md` — the refactoring-prd's own checklist targeting `bardo-backup/prd-updated/`. Has 24-doc topology including `12b-sonification.md`, `21-references.md`, `22-glossary.md`, `23-config-reference.md` as separate docs. Use as supplementary reference — our 22-doc checklist does not include those as standalone docs (sonification folds into 12-interfaces; glossary and config-reference are optional future expansions).
