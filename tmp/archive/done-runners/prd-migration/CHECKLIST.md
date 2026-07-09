# PRD Migration Checklist

Each unchecked item = one target doc to generate at `/Users/will/dev/nunchi/roko/roko/docs/`.
The embedded prompt tells the agent exactly what to read, what to produce, and what to
rename/reframe. Check off when done.

**See also**:
- `README.md` — authoritative naming map, reframe rules, migration principles
- `SOURCE-INDEX.md` — full source file listing per target doc
- `/Users/will/dev/nunchi/roko/refactoring-prd/` — canonical new-architecture spec (source of truth)
- `/Users/will/dev/nunchi/roko/refactoring-prd/MIGRATION-CHECKLIST.md` — parallel 24-doc checklist targeting `bardo-backup/prd-updated/`

---

## How to use

For each unchecked item:

1. Open a new Claude session with access to the full Roko workspace.
2. Copy the **Prompt** block into the session.
3. The prompt references: (a) refactoring-prd files to read first, (b) SOURCE-INDEX.md entries
   for the legacy + implementation-plan sources, (c) specific rename/reframe rules, (d) sections
   to produce.
4. The agent reads everything, writes the target doc to
   `/Users/will/dev/nunchi/roko/roko/docs/<filename>`.
5. Check the item off and fill in the completion table at the bottom.

## Always-include reference docs

Every session must have access to these. Mention them explicitly at the top of the prompt:

```
ALWAYS READ FIRST:
  /Users/will/dev/nunchi/roko/refactoring-prd/00-overview.md           ← Synapse Architecture, naming map, crate map, C-Factor
  /Users/will/dev/nunchi/roko/refactoring-prd/08-translation-guide.md  ← Rename & reframe rules, incompatibility flags, citation rules
  /Users/will/dev/nunchi/roko/refactoring-prd/09-innovations.md        ← Frontier features, blue ocean summary, resolved algorithms
  /Users/will/dev/nunchi/roko/roko/tmp/prd-migration/README.md         ← Authoritative naming map, reframe rules, concepts kept/removed
  /Users/will/dev/nunchi/roko/roko/tmp/prd-migration/SOURCE-INDEX.md   ← Full source file listing for each target doc
```

## Naming conventions (authoritative summary)

See `README.md` for the full table. Key replacements, to apply to ALL docs:

| Old | New |
|---|---|
| Bardo / Mori | Roko / Roko Orchestrator |
| Golem(s) | Agent(s) |
| Grimoire | Neuro / NeuroStore |
| Styx | Agent Mesh / Mesh |
| GNOS | KORAI (mainnet) / DAEJI (testnet) |
| Clade | Collective / Mesh (**NOT "fleet"**) |
| Signal (architecture noun) | Engram |
| "1 noun, 6 verbs" | Synapse Architecture |
| bardo-primitives / bardo-runtime | roko-primitives / roko-runtime |
| roko-golem | **Dissolved** (subsystems to roko-daimon, roko-dreams, roko-neuro, roko-chain) |
| golem.toml | roko.toml |
| Death / mortality / thanatopsis | Lifecycle / budget / confidence tracking |
| Succession | Knowledge backup/restore + mesh sharing |

## Rules for ALL docs

1. **Keep ALL academic citations.** Every paper, every reference. Add new ones from refactoring-prd.
2. **Keep ALL research context and design rationale.** The "why" stays; only framing changes.
3. **Rewrite implementation details** to reference roko's crate structure (see 00-overview Crate Map).
4. **Apply the 5-layer taxonomy** (Runtime / Framework / Scaffold / Harness / Orchestration).
5. **Integrate Synapse Architecture language** (Engrams flowing through 6 traits: Substrate, Scorer, Gate, Router, Composer, Policy).
6. **Remove all death/mortality framing** — reframe per `08-translation-guide.md`.
7. **Keep ROSEDUST, Spectre, Daimon, Dreams, Neuro** — all reframed appropriately.
8. **Domain-agnostic core** — blockchain is one domain plugin, not the default framing.
9. **Cognitive subsystems are cross-cuts**, not layers — injected via trait objects.
10. **Implementation-plans content feeds "Status / Gaps" sections** of target docs, not conceptual sections.

---

## The Checklist

### Architecture & Core

- [ ] **00-architecture.md** — Synapse Architecture, Engrams, 6 traits, universal loop, crate map, 5-layer taxonomy, C-Factor, autocatalytic improvement

> **Prompt**:
>
> ALWAYS READ FIRST (the 5 reference docs listed at the top of CHECKLIST.md).
>
> Read all sources listed under **"00-architecture.md"** in `/Users/will/dev/nunchi/roko/roko/tmp/prd-migration/SOURCE-INDEX.md`. Refactoring-PRD files you must read:
> - `refactoring-prd/00-overview.md`
> - `refactoring-prd/01-synapse-architecture.md`
> - `refactoring-prd/02-five-layers.md`
> - `refactoring-prd/07-implementation-priorities.md`
> - `refactoring-prd/09-innovations.md` §XVII Integration Map, §XVIII Blue Ocean Summary
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/00-architecture.md` covering:
> (1) **Roko vision** — cognitive agent OS, "the scaffold IS the product" thesis (Meta-Harness Lee et al. 2026, FrugalGPT Chen et al. 2023, DSPy, SWE-bench, CoALA Sumers 2023).
> (2) **Naming map** — summarize the old→new renames as a readable table.
> (3) **The Engram** — struct definition, content addressing, 7-axis Score (confidence/novelty/utility/reputation + precision/salience/coherence), Kind enum (core + domain-extensible via Custom reverse-DNS), Decay variants (None/HalfLife/Ttl/Ebbinghaus), Provenance + Attestation + taint tracking.
> (4) **Synapse Architecture** — the 6 Synapse traits with full Rust trait signatures: Substrate (async), Scorer (sync), Gate (async, returns Verdict directly), Router (sync, Option<Selection>, feedback()), Composer (sync, takes &dyn Scorer), Policy (sync, batch stream input).
> (5) **Universal cognitive loop** — PERCEIVE → EVALUATE → ATTEND → INTEGRATE → ACT → VERIFY → PERSIST → ADAPT → META-COGNIZE. Mapping to CoALA's 9-step pipeline.
> (6) **Three cognitive speeds** — Gamma (~5-15s), Theta (~75s), Delta (hours). Adaptive clock in roko-runtime.
> (7) **Dual-process (System 1 / System 2)** — T0/T1/T2 cascade driven by prediction error. Active inference as principled decision framework (EFE = pragmatic + epistemic value).
> (8) **5-layer taxonomy** — L0 Runtime, L1 Framework, L2 Scaffold, L3 Harness, L4 Orchestration. Dependency rules (flow downward only). Crate map showing which crate lives at which layer. Trait × layer distribution table.
> (9) **Cognitive cross-cuts** — Neuro, Daimon, Dreams (+ inference optimization, safety/provenance, observability). Injected into multiple layers, never owned by one layer.
> (10) **C-Factor and collective intelligence** — Level 1 (ratio: Collective/Sum(Individual) for reporting) and Level 2 (C-Score composite: gate_pass×0.3 + cost_eff×0.2 + speed×0.15 + first_try×0.25 + knowledge_growth×0.1 for optimization). Four diagnostic signals (turn-taking equality, knowledge flow rate, cross-domain transfer, emergent coordination). Woolley et al. Science 2010.
> (11) **Provenance & verification** — ContentHash (BLAKE3), lineage DAG, Attestation (Ed25519 + optional ChainAttestation), taint propagation, causal replay.
> (12) **Autocatalytic improvement** — multiplicative compound math (0.9^4 = 0.656), 5 levels of self-improvement (Foundation → Autocatalytic Loops), Kauffman citation, VSM mapping (Beer's 5 systems), Ashby's Law, Good Regulator Theorem.
> (13) **Design principles** — composability, content-addressing, trait-driven, dependencies flow down, cross-cuts via trait objects, everything observable.
> (14) **Frontier differentiators summary** — brief list of 14 blue ocean innovations (from 09-innovations.md §XVIII); defer details to 09-innovations.md which is the canonical source.
>
> Include ALL citations from source material. Apply all naming conventions from README.md.

---

- [ ] **01-orchestration.md** — Plan DAG, parallel executor, merge queue, worktrees, stigmergy, niche construction

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"01-orchestration.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/02-five-layers.md` §Layer 4 Orchestration, §Stigmergy, §Cross-Domain Orchestration
> - `refactoring-prd/05-agent-types.md` §7 Multi-Agent Orchestration (worktrees, pools, HEFT, Conductor Yerkes-Dodson)
> - `refactoring-prd/07-implementation-priorities.md` §Tier 1 (production hardening)
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/01-orchestration.md` covering:
> (1) Plan discovery and UnifiedTaskDag. (2) Parallel executor state machine (14-phase). (3) Merge queue with file-conflict serialization. (4) Worktree management (create/remove/list/prune/health). (5) Snapshot/recovery and crash resilience. (6) Post-merge regression testing. (7) Document pipeline (PRD → plan → tasks → implementation). (8) **Stigmergy via git** — each commit is a pheromone, workspace state is shared environment, agents read/write without direct coordination. (9) **Niche construction theory** — agents construct the codebase they operate in; positive vs negative niche construction; affordance assessment with Marginal Value Theorem stopping rule (Charnov 1976). (10) **Yerkes-Dodson dynamics** in multi-agent cooperation (moderate pressure → max cooperation; extreme pressure → collapse in 5-12 turns). (11) Cross-domain orchestration (coding + chain + research tasks in a single plan). (12) **Current status / gaps** — pull from implementation-plans (11-agent-dogfooding phases 3-4, scheduler, file watcher).
>
> Include ALL citations: MetaGPT, ChatDev, AutoGen, CAMEL, HEFT, Graham 1966, Grassé 1959, Odling-Smee 2003, Charnov 1976, Yerkes-Dodson.
> Rename: mori → Roko Orchestrator.

---

### Agents & Composition

- [ ] **02-agents.md** — Backends, roles, pools, MCP, tool loop, harness engineering, temperaments, tier routing

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"02-agents.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/01-synapse-architecture.md` §2 Synapse Traits (composability)
> - `refactoring-prd/02-five-layers.md` §Layer 1 Framework (dual-process tier router, temperament profiling)
> - `refactoring-prd/05-agent-types.md` all sections
> - `refactoring-prd/10-developer-guide.md` §2 Implementing Custom Traits, §6 Plugin System, §7 Integration Patterns
> - `refactoring-prd/07-implementation-priorities.md` §Tier 1 (provider registry, adapters)
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/modelrouting/01-architecture.md` through `07-openrouter-universal.md`
> - `roko/tmp/implementation-plans/modelrouting/13-architectural-gaps.md` §A Chat Types, §H Generated Test Gates
> - `roko/tmp/implementation-plans/modelrouting/14-integration-refinements.md` — wire EXISTING ToolLoop, don't rebuild
> - `roko/tmp/implementation-plans/modelrouting/19-implementation-guide.md` — exact wiring locations
> - `roko/tmp/implementation-plans/modelrouting/20-perplexity-integration.md`, `21-gemini-integration.md`
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/02-agents.md` covering:
> (1) **Agent trait** and 5 backends (Claude CLI, HTTP API, Codex, Ollama, Cursor, Mock).
> (2) **Provider registry** — ProviderKind enum (Anthropic, ClaudeCli, OpenAiCompat, CursorAcp), ProviderConfig, ModelProfile, 4 ProviderAdapter implementations.
> (3) **Chat types** must live in `roko-core` (ChatMessage, ChatRequest, ChatResponse) because roko-compose needs them and can't depend on roko-agent.
> (4) **Agent roles** (Implementer, Reviewer, Scribe, Architect, Researcher, etc.) and role-specific behavior.
> (5) **AgentPool** (sequential) and **MultiAgentPool** (parallel with warm spawning).
> (6) **MCP integration** — JSON-RPC client, tool converter, dynamic registry, 3 MCP servers (github 17 tools, slack 8 tools, scripts wrapper).
> (7) **Tool loop** — multi-turn driver. **ALREADY EXISTS** at `roko-agent/src/tool_loop/mod.rs` with checkpoint, max-iter, prune, result messaging. Implement `LlmBackend` for HTTP providers; do NOT rebuild the loop.
> (8) **Harness engineering** — critical distinction between agent (LLM) and harness (everything else). Cite Meta-Harness Lee et al. 2026 and note the 6× performance gap. Clarify that the "6×" actually comes from ref [46] (SWE-bench mobile) and Meta-Harness's direct contribution is +7.7 on classification, +4.7 on IMO-level math at 4× fewer tokens.
> (9) **Format translation** (Claude, ReAct, OpenAI, thinking mode extraction, cached token parsing, reasoning extraction).
> (10) **Agent extensibility and SDK patterns** — custom backends via LlmBackend trait, A2A interop with LangChain/CrewAI.
> (11) **Temperament profiling** — Conservative / Balanced / Aggressive / Exploratory. One high-level dial replaces scattered heuristics. Drives verbosity, tool selection, gate strictness, review depth, and model routing simultaneously.
> (12) **Dual-process tier routing** — T0 (no LLM) / T1 (fast model, shallow) / T2 (full model, deep). Driven by Thompson sampling over weighted signals (epistemic fitness, prediction error, contextual novelty, computational load, domain-specific signals). System 1 = exploit, System 2 = deliberate.
> (13) **8 agent creation sites** — from modelrouting/19-implementation-guide.md, all must be refactored to use provider factory (orchestrate.rs L428/451/6718/6753, run.rs L311/333, agent_exec.rs L39, dispatch.rs in roko-serve).
> (14) **Current status / gaps** — from implementation-plans 11-inconsistencies.md and modelrouting/13-architectural-gaps.md. Note that ExecAgent is a legacy fallback, NOT a ProviderKind.
>
> Rename: golem → agent; mori → Roko Orchestrator; bardo → roko.

---

- [ ] **03-composition.md** — Prompt assembly, context engineering, active inference, VCG attention auction, predictive foraging

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"03-composition.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/02-five-layers.md` §Layer 2 Scaffold, §Context as Active Inference, §Predictive Foraging, §Three Levels of Context Engineering
> - `refactoring-prd/09-innovations.md` §II VCG Attention Auction, §XIX.B EFE for Context Selection, §XIX.C Context Foraging Stopping Rule (MVT), §XIX.E VCG Bid Computation, §XV Distributed Context Engineering
> - `refactoring-prd/01-synapse-architecture.md` §Composer trait
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/12a-cognitive-layer.md` §E (E1–E6: 5-stage pipeline with active inference scoring formula `track_record × belief_change / uncertainty`, U-shape attention positioning, affect-modulated retrieval)
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/03-composition.md` covering:
> (1) **Composer trait** and PromptComposer with priority-based dropping + U-shape placement (Liu et al. "Lost in the Middle" arXiv:2307.03172).
> (2) **SystemPromptBuilder** with 6 composable layers.
> (3) **9 role templates** (Implementer, Reviewer, Scribe, Architect, Researcher, ...) and their purposes.
> (4) **13-step enrichment pipeline** with batch/direct LLM clients.
> (5) **Token budget management** — budget_for(role), stanza constants, priority-based dropping under budget.
> (6) **Context engineering** — ACE framework (Zhang et al. 2025), CSO (Samsung), ACON (Kang et al.), Lost in the Middle (Liu et al.), RAGAS, ARES. Cite Karpathy 2025 blog on context engineering.
> (7) **Active inference for context selection** — practical EFE approximation. Formula from 09-innovations.md §XIX.B:
>     `G(section) = pragmatic_value + epistemic_value - ambiguity`
>     `pragmatic_value = cosine_similarity(embed(section), embed(task_goal))`
>     `epistemic_value = 1 - max_similarity_to_already_selected`
>     Selection: `softmax(gamma * G)` with gamma=8.0.
> (8) **5-stage assembly pipeline**: Query → Score → Deduplicate → Budget → Format. Active inference scoring from 12a-cognitive-layer: `score = track_record × belief_change / uncertainty`.
> (9) **Attention-curve positioning** — highest-value entries at beginning and end of context window (U-shape).
> (10) **Predictive foraging** — patch-leaving via MVT (Charnov 1976). Stop when marginal relevance ≤ average. Fit λ online from first retrievals. Hard floor relevance < 0.05. Exploration budget before committing to implementation. Scent following (descriptive names, docs, test names guide efficient exploration — Pirolli & Card 1999).
> (11) **VCG Attention Auction** — Vickrey-Clarke-Groves mechanism (Vickrey 1961, Clarke 1971, Groves 1973). Truthful bidding for limited context budget. Subsystems: Neuro, Daimon, iteration memory, code intelligence, playbook rules, research artifacts, task context, oracle predictions. Affect modulation on bids. Full bid formula from 09-innovations.md §XIX.E.
> (12) **Distributed context engineering** — Write / Select / Compress / Isolate strategies at network scale (not just single agent).
> (13) **Affect-modulated retrieval** — PAD state biases what knowledge is surfaced (high arousal → recent + action-oriented).
> (14) **Cost optimization strategies** — $1.01→$0.38/task from optimization playbook. Blended cost accounting (3:1 input:output per Artificial Analysis), tokenizer ratios, multi-level budget guardrails.
> (15) **Current status / gaps** — SystemPromptBuilder does 6-layer assembly; needs scoring/dedup stages (from 12a E1).
>
> Include all context-engineering citations. Rename: golem → agent; bardo-gateway → roko gateway; mori → Roko Orchestrator.

---

### Verification & Learning

- [ ] **04-verification.md** — Gates, rungs, ratcheting, process rewards, EvoSkills, causal replay

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"04-verification.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/02-five-layers.md` §Layer 3 Harness, §Process Reward Models, §Conductor as Meta-Cognition
> - `refactoring-prd/01-synapse-architecture.md` §Gate trait, §Cybernetic loops
> - `refactoring-prd/09-innovations.md` §IX Forensic AI / Causal Replay, §X EvoSkills
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/modelrouting/12-advanced-patterns.md` — gate-to-scaffold feedback, section effectiveness tracking
> - `roko/tmp/implementation-plans/modelrouting/13-architectural-gaps.md` §H Generated Test Gates (GVU)
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/04-verification.md` covering:
> (1) **Gate trait** (returns Verdict directly, not Result<Verdict>) and 11+ concrete implementations (CompileGate, TestGate, ClippyGate, DiffGate, SymbolGate, LlmJudgeGate, TxSimGate, WalletGate, VerifyChainGate, PropertyTestGate, GeneratedTestGate).
> (2) **6-rung selector system**.
> (3) **GatePipeline** and **VerifyChainGate** (short-circuit chains).
> (4) **Artifact store** (hash-addressed).
> (5) **Failure ratcheting** — how gates prevent regression (best-ever test count, results must match or exceed).
> (6) **Adaptive thresholds** — EMA per rung.
> (7) **Process reward models** — AgentPRM (Promise + Progress scoring intermediate steps, not just outcomes). Catches bad reasoning early. Cite "Let's Verify Step by Step" (Lightman et al.), generation-verification gap (Song et al. ICLR 2025).
> (8) **Agent feedback from gate results** — how Gate verdicts flow back to Scorer (source trust), Router (bandit arms), Composer (section weights), Daimon (PAD), Neuro (tier promotion), self-model.
> (9) **Evaluation lifecycle** — fast feedback (compile, test) to slow feedback (eval suites, regression detection).
> (10) **Autonomous eval generation** — EVMbench, DSPy Bayesian optimizers, Karpathy autoresearch loop.
> (11) **EvoSkills** — self-evolving skill libraries via adversarial surrogate verification. 5-round loop. Cross-model transfer (+35-44 pp gains across 6 models). Citation: EvoSkills April 2026.
> (12) **Forensic AI / Causal Replay Engine** — content-addressed lineage replay. Replay any agent action with: which Engrams were in the Substrate, which Scores, which Router decision, which Composer context, which Gate verdict, which Policy fired. Cryptographically verifiable (BLAKE3). Regulatory compliance mapping table (EU AI Act Art. 14 + FRIA, SEC/CFTC, HIPAA, SOX). Pre-certified agent templates.
> (13) **SWE-bench harness methodology** — benchmarks for verification.
> (14) **Current status / gaps** — from modelrouting/12-advanced-patterns.md.
>
> Rename: golem → agent; mori → Roko Orchestrator.

---

- [ ] **05-learning.md** — Episodes, playbooks, skills, bandits, cascade routing, 8 feedback loops, autocatalytic thesis

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"05-learning.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/03-cognitive-subsystems.md` §5 Cybernetic Self-Learning Architecture, §6 conceptual feedback loops
> - `refactoring-prd/09-innovations.md` §VI Collective Calibration, §VII Predictive Foraging, §X EvoSkills, §XI ADAS
> - `refactoring-prd/07-implementation-priorities.md` §Tier 1M — 8 missing feedback loops
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/05-learning-wiring.md` — completed reference
> - `roko/tmp/implementation-plans/modelrouting/08-learning-loops.md` — provider health, latency, Pareto pruning, anomaly detection
> - `roko/tmp/implementation-plans/modelrouting/09-cost-normalization.md` — CostTable, budgets, guardrails
> - `roko/tmp/implementation-plans/modelrouting/10-model-experiments.md` — Thompson Sampling (Beta), discount factor, UCB1
> - `roko/tmp/implementation-plans/modelrouting/11-research-context.md` — RouteLLM, MixLLM, FrugalGPT, GVU, GEPA, SAGE, ABC
> - `roko/tmp/implementation-plans/modelrouting/12-advanced-patterns.md` — Thompson Sampling, predictive foraging, gate feedback, skills, contracts, drift
> - `roko/tmp/implementation-plans/modelrouting/17-meta-learning-and-corrections.md` — **8 missing cybernetic feedback loops** + stability mechanisms + compound optimization
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/05-learning.md` covering:
> (1) **Episode logger** (JSONL, retention, compaction).
> (2) **Playbook system** with globset rule matching.
> (3) **Skill library** with Voyager-style growth (Wang et al. 2023).
> (4) **Bandits**: UCB1, BanditBank, TrackAndStop, Thompson Sampling (Beta distribution per arm with discount factor for concept drift).
> (5) **Model routing**: LinUCB contextual bandit + 3-stage cascade router.
> (6) **Pattern discovery** (trigram miner).
> (7) **Task metrics pipeline**, baseline comparison, regression detection, efficiency grading.
> (8) **Cost database** and provider health tracking. 3-state circuit breaker (Closed/Open/HalfOpen), error-type-specific cooldowns.
> (9) **Pareto frontier pruning** for provider selection.
> (10) **Cost normalization** — blended cost (3:1), tokenizer ratios, multi-level budget guardrails (per-task/session/day, 80%→downgrade, 95%→block, 100%→hard stop).
> (11) **Self-improvement frameworks** — Reflexion (Shinn et al.), ExpeL (Zhao et al.), DSPy (Khattab et al.), Meta-Harness (Lee et al. 2026, +7.7 pp).
> (12) **The 8 missing cybernetic feedback loops** (from 17-meta-learning-and-corrections.md):
>     1. Health → Routing (ProviderHealth → CascadeRouter)
>     2. Conductor → Routing (abort/restart → penalty)
>     3. Section → Scaffold (Gate results → SystemPromptBuilder, lift > 0.05)
>     4. Failure → Replanning (Gate failures → Plan generator)
>     5. Skills → Prompts (SkillLibrary → Prompt assembly, confidence-ordered)
>     6. Cost → Routing (Cost anomaly → downgrade tier)
>     7. Latency → Reward (Observed latency → reward)
>     8. Experiments → Static (Conclusions → router cold-start)
> (13) **Stability mechanisms** — hysteresis (10% score delta to switch), frequency separation (router: every episode, thresholds: every 5, patterns: every 20).
> (14) **Collective calibration (31.6× heuristic)** — 1/sqrt(N×t) scaling. WITH explicit caveats: CLT-inspired, not theorem; independence assumption; distribution shift; correlation across agents. From 09-innovations.md §VI.
> (15) **Predictive foraging** — falsifiable predictions as learning signal. PredictionClaim → ResidualAggregation → bias correction per (model, task_category). ~50ns per correction. CalibrationTracker.
> (16) **ADAS** — meta-agent architecture search (Hu et al. ICLR 2025). Agents that design agents. +14% on ARC, +13.6 F1 on reading comp, +14.4% on math.
> (17) **EvoSkills** — self-evolving skill libraries (see 04-verification.md for full treatment).
> (18) **Autocatalytic thesis** (Kauffman) — no single component improves itself; the network catalyzes mutual improvement. Compound math: 0.9^4 = 0.656.
> (19) **Cybernetic learning loop** — 6 conceptual feedback loops from 03-cognitive-subsystems §5.
> (20) **C-Factor as optimization metric** — brief reference; full treatment in 00-architecture.md.
> (21) **Current status / gaps** — from modelrouting/13-architectural-gaps.md (existing code vs. missing wiring).
>
> Rename: golem → agent; mori → Roko Orchestrator.

---

### Cognitive Subsystems

- [ ] **06-neuro.md** — Knowledge architecture (was Grimoire), tiers, HDC, retrieval, cross-domain transfer

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"06-neuro.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/03-cognitive-subsystems.md` §1 Neuro — all sections
> - `refactoring-prd/04-knowledge-and-mesh.md` §1 Knowledge Architecture, §5 Knowledge Backup & Restore
> - `refactoring-prd/01-synapse-architecture.md` §Decay enum (Ebbinghaus)
> - `refactoring-prd/09-innovations.md` §XIII Cross-Domain Insight Resonance (false-positive math), §III Somatic Landscape
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/12a-cognitive-layer.md` §D Knowledge & Memory (D1–D18 — 4-tier distillation pipeline, knowledge types + storage, HDC integration)
> - `roko/tmp/implementation-plans/12a-cognitive-layer.md` §R1 roko-neuro crate creation
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/06-neuro.md` covering:
> (1) **Neuro subsystem vision** — semantic wrapper around Substrate. Persistent, tiered, HDC-indexed knowledge.
> (2) **6 knowledge types** — Insight, Heuristic, Warning, CausalLink, StrategyFragment, AntiKnowledge. With coding + chain examples.
> (3) **4 validation tiers** × **type base half-life** multiplicative decay:
>     - Tiers: Transient 0.1× | Working 0.5× | Consolidated 1.0× | Persistent 5.0×
>     - Type half-lives: Insight 30d | Heuristic 90d | Fact 365d | Warning 7d | CausalLink 60d | StrategyFragment 14d | AntiKnowledge never (floor 0.3)
>     - Example: Persistent Insight = 5.0 × 30 = 150 days
> (4) **HDC/VSA foundations** — 10,240-bit BSC vectors. XOR binding, majority bundling, cyclic-shift permutation. Hamming similarity. ~13ns per comparison with AVX-512, ~170µs for 100K entries (ARM NEON ~2-3× slower). Johnson-Lindenstrauss 1984 (4604 dim minimum for N=100K, ε=0.1; 10,240 is generous).
> (5) **HDC encoding for knowledge** — text → BSC vector. 3-tier search: Bloom filter → approximate → exact top-K.
> (6) **Promotion/demotion** — automatic based on outcome feedback. Good outcome → promote tier. Bad → demote. Cross-agent validation → promote.
> (7) **Cross-domain knowledge transfer** via HDC structural analogy. `BIND(high_complexity, more_review)` ≈ `BIND(high_volatility, more_caution)` because both encode `BIND(high_uncertainty, more_verification)`.
> (8) **Cross-domain resonance detection** — threshold 0.526 for 10,240-bit vectors (Bonferroni corrected for 100K vocabulary, <1% false positive rate). Require confirmation by 2 independent agents.
> (9) **Ebbinghaus decay integration** — successful use increases strength → decay slows. Failed use decreases strength → decay accelerates. Tier progression emerges naturally.
> (10) **Emotional memory integration** — mood-congruent retrieval, somatic marker influence on ranking. 15% contrarian retrieval mandatory.
> (11) **Library of Babel** — cross-collective knowledge via public Korai.
> (12) **Knowledge backup/restore** — BACKUP → DELETE → CREATE → RESTORE lifecycle (4 steps). User-controlled. Restored entries start at Transient tier. Provenance tracks origin. REPLACES succession entirely — no biological metaphor.
> (13) **4-tier distillation pipeline** (from 12a D1–D6):
>     Raw Episodes → Insights (pattern detection) → Heuristics (3+ confirmations) → Playbook rules
>     Confirmation boost ×1.5. Temporal decay per type.
> (14) **NeuroStore API** — init, query (semantic + temporal relevance + affect filters), ingest, decay, gc.
> (15) **AntiKnowledge** — challenge mechanism. Locally: "this insight was wrong" with evidence.
> (16) **Current status / gaps** — roko-neuro crate scaffold only; bardo-primitives HDC exists; roko-index HDC exists; wiring gaps from 12a §D and §R1.
>
> Rename: grimoire → neuro; golem → agent; clade → collective/mesh. Include ALL HDC citations: Kanerva 2009, Plate, Frady 2021, Kleyko 2022 ACM Computing Surveys, Neubert 2022 VSA survey — full 14+ paper list.

---

- [ ] **07-conductor.md** — Watchers, circuit breaker, interventions, diagnosis, meta-cognition, cognitive signals

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"07-conductor.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/02-five-layers.md` §Conductor as Meta-Cognition
> - `refactoring-prd/03-cognitive-subsystems.md` §Self-Model (Good Regulator), §Ashby's Law, §VSM mapping
> - `refactoring-prd/09-innovations.md` §XII.2 Cognitive Signals (Pause/Resume/Reprioritize/InjectContext/Escalate/Cooldown/Explore/Shutdown)
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/modelrouting/08-learning-loops.md` — circuit breaker (3-state), anomaly detection
> - `roko/tmp/implementation-plans/modelrouting/16-production-hardening.md` — adaptive timeouts (p95×2), full-jitter backoff, graceful shutdown (3-phase drain)
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/07-conductor.md` covering:
> (1) The conductor's role as **reactive intelligence layer**, not just timeout manager. Theory-of-mind about its own pipeline.
> (2) All 10 watchers and what each detects.
> (3) **3-state circuit breaker** (Closed → Open → HalfOpen) with error-type-specific cooldowns (RateLimit 5s, Timeout 10s, ServerError 30s, Auth 5min).
> (4) Intervention types and graduated response (nudge, retry, replan, escalate, abort).
> (5) Diagnosis engine with 34 error patterns.
> (6) Stuck detection heuristics. Meta-cognition hook: "Am I stuck? Should I escalate?" (12a I5).
> (7) Health monitors.
> (8) **Cognitive signals** (typed interrupts — differ from OS signals: they change agent *behavior* without killing the process):
>     - Pause, Resume, Reprioritize(TaskId), InjectContext(Engram), Escalate, Cooldown, Explore, Shutdown
> (9) **Adaptive timeouts** — p95×2, measured per-role/per-model.
> (10) **Full-jitter backoff** for retries.
> (11) **Graceful shutdown** — 3-phase drain protocol.
> (12) **Cybernetic loop** — conductor creates closed-loop feedback.
> (13) **Good Regulator Theorem** (Conant & Ashby) — agents must model themselves to self-regulate. Self-model persists in Neuro as SelfModel Kind.
> (14) **Ashby's Law** — agent's internal variety must match environmental variety. Each new failure mode → new Policy. Each new domain → new Gate. Each new provider → new Backend.
> (15) **Precision-weighted prediction errors** — failure on familiar task = high-precision error (learn strongly). Failure on novel task = low-precision error (learn cautiously).
> (16) **Yerkes-Dodson dynamics** — moderate pressure maximizes cooperation; extreme pressure causes collapse in 5-12 turns. Conductor adjusts pressure parameters.
> (17) **VSM mapping** — Beer's 5 recursive systems. Conductor = System 3 (internal oversight) + System 3* (audit) via watchers.
> (18) **21 production failure catalog** (from `bardo-backup/tmp/mori-refactor-plan/00-issues-catalog.md`).
> (19) **Current status / gaps** — from roko-conductor source + implementation plans.
>
> Rename: golem → agent; mori → Roko Orchestrator.

---

### Chain, Identity & Coordination

- [ ] **08-chain.md** — Korai chain, mirage-rs, agent marketplace, identity, reputation, gossip, tokenomics, privacy

> **Prompt**: **LARGEST DOC**. Budget extra context and expect it to span many sections.
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"08-chain.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/04-knowledge-and-mesh.md` — full chain spec
> - `refactoring-prd/05-agent-types.md` §3 Chain Agent (9-step heartbeat → universal loop mapping)
> - `refactoring-prd/09-innovations.md` §VI Collective Calibration, §VIII x402 Micropayments, §XVI Knowledge Futures Market, §XIII Cross-Domain Resonance
> - `refactoring-prd/07-implementation-priorities.md` §Tier 6
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/12b-chain-layer.md` — **FULL FILE** (76 items, 11 sections: Identity, Gossip, Job Market, ChainWitness, Reputation, Payments, Safety, ISFR, Clearing, Privacy, Mirage, Crate cleanup)
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/08-chain.md` covering:
> (1) **Korai chain vision and architecture** — dedicated EVM chain for agent knowledge coordination. Mainnet = Korai (token: KORAI). Testnet = Daeji (token: DAEJI). 400ms block time. Agents as first-class citizens via ERC-8004.
> (2) **Three-level knowledge architecture**: Korai Chain (global public) → Agent Mesh (peer/private) → Local Neuro Store (private).
> (3) **What goes on-chain / what stays off-chain**.
> (4) **HDC on-chain precompile** — 10,240-bit vectors stored and queried via native EVM precompile. ~400 gas for top-k=20 similarity search. 3-tier search (Bloom → approximate → exact).
> (5) **KORAI tokenomics** — 1% annual **demurrage** (knowledge must be maintained, prevents garbage accumulation, mirrors Engram half-life). Earning (registration mint, validated knowledge posting, confirmation, heartbeat, challenge defense). Spending (posting/anti-spam, querying, challenging). Quality incentives (duplicate penalty, novelty bonus, curation bonds, cross-agent confirmation multiplier).
> (6) **Korai Passport** — ERC-721 soulbound NFT. Full struct from 12b §A: passportId, owner, capabilityList (u64 bitmask), domainStakes, reputationTracks, teeAttestation, systemPromptHash (ventriloquist defense), tier, slashHistory. 4 tiers: Protocol / Sovereign (25K KORAI) / Worker (5K KORAI) / Edge.
> (7) **ERC-8004** — 3 registries: Identity (ERC-721 Agent Card with capabilities, endpoints, payment address), Reputation (feedback authorization, off-chain scoring), Validation (verification requests, reputation/stake/zkML/TEE oracles).
> (8) **4-tier gossip architecture** (from 12b §B):
>     - Tier 0: GossipSub v1.1 (ms) — the mesh, mocked by mirage
>     - Tier 1: MiroFish simulation sandbox (sec–min)
>     - Tier 2: FABRIC TEE aggregation (epoch-level)
>     - Tier 3: Canonical Event Bus (block-finalized)
>     GossipSub v1.1 config: D=8, D_low=6, D_high=12, D_out=4, heartbeat=700ms, max msg=256KB.
>     8 topics: capabilities, reputation, spore/jobs, spore/deltas, spore/status, sparrow, isfr, txs.
>     3-layer peer scoring: 0.4 × behavioral + 0.4 × TraceRank + 0.2 × TEE.
> (9) **Job market (Spore + Sparrow)** — Spore = bounty spec (budget escrowed). Sparrow = dispatch via power-of-two-choices (Ousterhout 2013): probe 2 random capable agents → ask queue depth → least-loaded wins. O(log log N) max load.
> (10) **3 hiring models**: (a) Random VRF assignment for jobs < 50 DAEJI; (b) Blind auction (FPSB / Vickrey reputation-adjusted / Dutch); (c) Direct hire (1.5× fee premium, anti-centralization > 20% volume → 2× fee).
> (11) **Vickrey reputation-adjusted auction formula**: `s_i = p_i × (1 + (1 - R_i))`. Winner = argmin. Payment = `s_second / (1 + (1 - R_winner))`.
> (12) **Reputation system** — 7-domain EMA, tiers, discipline, disputes (from 12b §K, `korai-reputation-framework.md`).
> (13) **Agent Mesh** — P2P connectivity. WebSocket (co-located, low latency) + Iroh (NAT-traversing, encrypted, cross-network) + ERC-8004 (discovery).
> (14) **Permissioned subnets** — company collectives with private knowledge meshes (e.g., Boston Dynamics internal agents + MCP server, opt-in publishing to public Korai).
> (15) **Stigmergy beyond termites** — git for coding, Korai chain for blockchain, shared substrate for research, infra state for ops, HDC vector space for cross-domain. Table from 04-knowledge-and-mesh §4.
> (16) **Pheromone system** — PheromoneKind enum (Threat/Opportunity/Wisdom + Alpha/Pattern/Anomaly/Consensus + Custom). PheromoneScope (Local/Mesh/Global).
> (17) **ChainClient + ChainWallet traits** (`roko-chain`). 3 custody modes: Delegation (enclave keys), Embedded (ERC-4337 account abstraction), Local key (dev).
> (18) **mirage-rs in-process EVM simulator** — Korai proxy during development. JSON-RPC (full Ethereum RPC + custom `mirage_*`). Fork mode for mainnet Ethereum. Scenario engine. Copy-on-write branching. HDC precompile emulation.
> (19) **Chain intelligence** — block ingestion, witness engine, ABI decoder, event categorization.
> (20) **Triage** — curiosity scoring, MIDAS anomaly detection, Bayesian scoring.
> (21) **The 6 Solidity contracts** (Agent Registry, Reputation, Marketplace, Escrow, KORAI token, Validation).
> (22) **ISFR** (Intersubjective Fact Registry) — collective price discovery, rate aggregation, disputed claim resolution, 3-arbitrator voting.
> (23) **Clearing & settlement** — QP solver, bisection, certificates, fallback, batch clearing, DVP, settlement finality.
> (24) **x402 micropayments** — Coinbase protocol (Linux Foundation, AWS/Visa/Mastercard/Stripe). Per-API-call billing at < $0.001. Sub-second settlement (USDC on Base). Self-funding agent loop. Enables agent-as-a-business.
> (25) **Knowledge Futures Market** (P3, deferred) — on-chain escrow for committed knowledge production.
> (26) **Valhalla privacy layer** — 4 modes, TEE attestation, PSI protocol, ZK range proofs. From 12b §P.
> (27) **Exponential flywheels** — O(N) insights → O(N²) network value (Reed's Law / Metcalfe).
> (28) **Collective calibration 31.6× heuristic** — with explicit caveats.
> (29) **Chain agent 9-step heartbeat** mapping to universal Synapse loop (from 05-agent-types.md §3): OBSERVE → RETRIEVE → ANALYZE → GATE → SIMULATE → VALIDATE → EXECUTE → VERIFY → REFLECT.
> (30) **Framing**: Blockchain is ONE domain plugin (`roko-chain`), not the default. Coding is another domain (`roko-lang-rust` etc.). Cross-domain agents compose both.
> (31) **Current status / gaps** — deferred Tier 6 (blocked by Tier 5 agent mesh). Note that solo agents and event-driven agents do NOT need the chain layer.
>
> Include ALL chain/DeFi/coordination citations. Rename: golem → agent; mori → Roko Orchestrator; styx → Agent Mesh; GNOS → KORAI/DAEJI; clade → collective/mesh.

---

- [ ] **09-daimon.md** — Affect engine, PAD, somatic markers, behavioral states (NO mortality)

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs). **Pay special attention to** `refactoring-prd/08-translation-guide.md` §INCOMPATIBLE: Emotion Mapped to Mortality.
>
> Read all sources listed under **"09-daimon.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/03-cognitive-subsystems.md` §2 Daimon — all sections
> - `refactoring-prd/09-innovations.md` §III Somatic Landscape (full struct), §XIX.F 8D Somatic Strategy Space, §XIX.E VCG Bid (affect weight)
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/12a-cognitive-layer.md` §F Daimon (F1–F9)
> - `roko/tmp/implementation-plans/12a-cognitive-layer.md` §R2 roko-daimon crate creation
>
> **SKIP entirely**: `bardo-backup/prd/03-daimon/04-mortality-daimon.md` (extract somatic marker citations only), `bardo-backup/prd/03-daimon/05-death-daimon.md` (skip completely).
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/09-daimon.md` covering:
> (1) **Daimon subsystem** — affect engine driven by **PAD vector** (Pleasure-Arousal-Dominance, Mehrabian 1996, Current Psychology 14(4)). Each dimension in [-1, 1]. 8 octant states.
> (2) **What each dimension captures**:
>     - Pleasure: outcome quality trajectory (tasks succeeding vs. failing)
>     - Arousal: cognitive load & urgency (routine vs. high-stakes)
>     - Dominance: confidence in approach (uncertain exploring vs. confident exploiting)
> (3) **6 behavioral states** (replace vitality phases):
>     - Engaged (balanced) → standard T0/T1/T2 distribution, normal operation
>     - Struggling (low P, high A) → force T2, escalate model, re-plan, request help
>     - Coasting (high P, low A) → bias toward T0/T1, take more tasks, cheap models
>     - Exploring (low D) → T2 for research, T1 for breadth
>     - Focused (high D, high P) → T0/T1 exploit known patterns
>     - Resting (low A, low D) → T1 for Dreams, offline learning
>     States are **cyclical, not terminal**. NO Thriving→Terminal progression.
> (4) **Behavioral state → tier router bias** — concrete mechanism by which affect controls compute allocation.
> (5) **ALMA three-layer temporal model**:
>     - Emotion (seconds) — reactive to immediate events
>     - Mood (hours) — accumulated emotional trajectory
>     - Personality (lifetime) — stable traits
> (6) **OCC/Scherer appraisal pipeline** — appraisal triggers: gate pass/fail, task outcome, blockers, time pressure, prediction accuracy.
> (7) **Decay toward baseline** — 4h half-life default. Prevents permanent drift.
> (8) **Somatic markers** (Damasio 1994 — "Descartes' Error") — emotional fast-heuristics for decisions. Implemented as **k-d tree over 8D strategy space**. `SomaticLandscape` struct with `SomaticMarker` (strategy_coords, valence, intensity, episodes). Queried before acting (< 1ms). Nearest-neighbor emotional valence lookup.
> (9) **Mandatory 15% contrarian retrieval** — always retrieve at least 15% from markers with *opposite* valence. Prevents emotional echo chambers (Bower 1981).
> (10) **8D default coding dimensions** (configurable per domain): Complexity, Risk, Novelty, Confidence, Time pressure, Scope, Reversibility, Dependency depth. Chain dimensions: volatility, exposure, liquidity, correlation, leverage, time_horizon, slippage_risk, counterparty_risk.
> (11) **Integration points** — Daimon PAD drives: (a) behavioral state selection, (b) tier routing bias, (c) VCG auction bidding (urgency × arousal, extreme states → more affect_weight), (d) somatic landscape querying, (e) SystemPromptBuilder tone modulation, (f) CascadeRouter model selection, (g) mood-congruent memory retrieval, (h) episode affect signatures.
> (12) **Mood-congruent memory retrieval** (Bower) — current emotional state biases which knowledge is surfaced.
> (13) **Coding agent integration** — per-crate confidence tracking, error pattern sensitivity, fatigue detection.
> (14) **Collective emotional contagion** — exponential decay across mesh. Somatic field shared across agent collective.
> (15) **Dream-daimon and runtime-daimon** (keep both; reframe dream-daimon to not be about approaching death).
> (16) **Persistence** — `.roko/daimon/affect.json`. Agent "wakes up" with residual affect.
> (17) **Current status / gaps** — roko-daimon crate scaffold; existing roko-golem/daimon.rs (972 lines) to move; wiring gaps from 12a F1–F9.
>
> **SKIP entirely**: death-daimon, mortality-daimon (as sources). Extract ALL affect citations (Mehrabian 1996, Damasio 1994, Bechara, Bower 1981, Plutchik, Russell-Mehrabian, Zhang et al. SIGDIAL, Scherer 2001 appraisal).
>
> Rename: golem → agent; clade → collective/mesh.

---

- [ ] **10-dreams.md** — Offline learning, 3-phase dream cycle, hypnagogia, synthesis (NO death triggers)

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs). **Pay special attention to** `refactoring-prd/08-translation-guide.md` §INCOMPATIBLE: Dream as Approaching Death.
>
> Read all sources listed under **"10-dreams.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/03-cognitive-subsystems.md` §3 Dreams
> - `refactoring-prd/09-innovations.md` §IV Hypnagogia (concrete LLM recipe), §V Dream Engine 3-phase, §XIX.G Dream Scheduling
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/12a-cognitive-layer.md` §G Dreams (G1–G8)
> - `roko/tmp/implementation-plans/12a-cognitive-layer.md` §R3 roko-dreams crate creation
>
> **SKIP entirely**: `bardo-backup/prd/22-oneirography/02-death-masks.md`.
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/10-dreams.md` covering:
> (1) **Dreams subsystem** — Delta-frequency cognitive process. Runs during idle time or on schedule. **NOT triggered by death proximity**.
> (2) **6-step dream cycle**: REPLAY → CONSOLIDATE → PRUNE → SYNTHESIZE → VALIDATE → OPTIMIZE.
> (3) **Phase 1: NREM Replay** (8-15 min) — Mattar-Daw utility formula: `Utility(episode) = Gain × Need × (1/spacing_penalty)`. Cite Mattar & Daw 2018 "Prioritized memory access explains planning and hippocampal replay". 30% perturbed replays (2× slippage, 5× gas, correlation shifts for stress testing). PAD modulates selection (anxious → 2× warning episode weight).
> (4) **Phase 2: REM Imagination** (5-15 min) — counterfactual scenario generation via **Boden's three creativity modes**:
>     - Combinational (recombine knowledge)
>     - Exploratory (traverse boundaries of known strategy space)
>     - Transformational (break constraints, discover novel regions)
>     Implemented via **Pearl's structural causal models** — build causal graph, intervene on causal variables. **Emotional depotentiation** reduces arousal on charged memories by 0.3-0.5 per cycle (Walker & van der Helm 2009). **HDC counterfactual synthesis** via permutation (nanosecond-scale).
> (5) **Phase 3: Integration & Staging** (5-10 min) — SQLite staging buffer. Hypotheses enter at 0.20-0.30 confidence. Only those reaching 0.70 through live validation get promoted. Prevents hallucinated insights from corrupting knowledge base.
> (6) **What Dreams produce** — knowledge promotions, pattern discovery → Warnings, skill extraction → StrategyFragments, hypothesis generation via HDC bundling, routing updates, prompt optimization.
> (7) **Neuroscience basis** — Complementary Learning Systems (McClelland 1995) — fast episodic → slow semantic. Non-veridical replay aids generalization. Beneficial forgetting prevents overfitting. Hippocampal replay (Wilson & McNaughton 1994). DreamerV3 (Hafner 2025). World Models (Ha & Schmidhuber 2018).
> (8) **Sleep-time compute** (Lin et al. 2025) — 5× reduction in test-time compute via overnight consolidation.
> (9) **WSCL** (2024) — 38% reduction in catastrophic forgetting via sleep-based consolidation.
> (10) **Hypnagogia engine** — solves the Alpha Convergence Problem. All AI agents using the same foundation models → same analyses → alpha → 0 in competitive domains. Hypnagogia forces experiential divergence (each agent "differently haunted" — Derrida 1993).
> (11) **Hypnagogia 4 layers**:
>     - ThalalamicGate — progressively reduce external input, redirect attention inward
>     - ExecutiveLoosener — temperature annealing (T=1.3-1.5 ideation, T=0.3-0.5 evaluation) + min-p sampling
>     - DaliInterrupt — capture 3-5 partial completions at peak temp before LLM converges (Edison/Dali "bottle drop")
>     - HomuncularObserver — evaluate fragments at T=0.4 on novelty/relevance/coherence
> (12) **Concrete LLM implementation recipe** (from 09-innovations.md §IV):
>     Step 1 Thalamic Gate → HDC-encode recent 5 episodes → bundle → retrieve 3-5 entries with LOWEST similarity (anti-correlated)
>     Step 2 Executive Loosener → LLM call 1 at T=1.3, top_p=0.95, min_p=0.02 — 5 hypotheses
>     Step 3 Dali Interrupt → partial completions at T=1.0, stop at 50-100 tokens, 3-5× per hypothesis = 15-25 fragments
>     Step 4 Homuncular Observer → LLM call 2 at T=0.4, rate on (novelty > 0.5 AND relevance > 0.3 AND coherence > 0.4), typically 3-7 survive
>     Cost: ~2,000-4,000 tokens per session (~$0.01).
> (13) **Hypnagogia citations** — Lacaux et al. 2021 (Science Advances: 83% hidden rule discovery in N1 vs 30% awake), MIT Dormio (Haar Horowitz 2020/2023, 43% creativity boost), Derrida 1993 (hauntology).
> (14) **Sleepwalker** — reduced-capability sleep mode (3-step variant of CoALA).
> (15) **Oneirography** — dream journals (kept), self-appraisal (kept), auctions (kept), extended forms (kept). Skip death-masks entirely.
> (16) **Venice dreaming** integration.
> (17) **Dream scheduling** (09-innovations.md §XIX.G) — Dreams don't block the agent. Run concurrently using cheap models. NREM → Haiku-class. REM → Sonnet-class. Integration → no LLM. On task arrival during dreaming, pause via SIGPAUSE, serialize, resume later. Impact on throughput: ~0% when tasks available.
> (18) **Current status / gaps** — roko-dreams scaffold; roko-golem/dreams.rs placeholder to delete; roko-golem/hypnagogia.rs to move; wiring from 12a G1–G8.
>
> Include ALL dream/sleep citations. Rename: golem → agent.

---

- [ ] **11-safety.md** — Capabilities, audit, taint, sandboxing, cognitive kernel primitives, regulatory compliance

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"11-safety.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/01-synapse-architecture.md` §Provenance & Attestation
> - `refactoring-prd/09-innovations.md` §IX Forensic AI / Causal Replay, §XII Cognitive Kernel Primitives
> - `refactoring-prd/07-implementation-priorities.md` §Tier 1G
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/03-safety-hooks.md`
> - `roko/tmp/implementation-plans/11-inconsistencies.md` — **documents the #1 integration gap**
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/11-safety.md` covering:
> (1) **Defense-in-depth architecture**.
> (2) **Capability tokens** — typed, unforgeable authorization. Least-privilege per role.
> (3) **Content-addressed audit chain** — BLAKE3 lineage, tamper-evident.
> (4) **Taint-aware ingestion** — Trusted / Unverified / Suspicious levels. Taint propagates through the DAG.
> (5) **Permits and allowlists**.
> (6) **Loop detection and guard**.
> (7) **Sandboxing** (validation-only approach).
> (8) **Prompt security and injection prevention** — the ventriloquist defense (SHA-256 system prompt hash committed on-chain, verified by TEE before each job).
> (9) **Threat model** — 21 production failure catalog from mori-refactor-plan/00-issues-catalog.md.
> (10) **Adaptive risk management**.
> (11) **MEV protection**.
> (12) **Temporal logic verification**.
> (13) **Witness DAG**.
> (14) **Formal verification pipeline**.
> (15) **Engram Syscalls** — single enforcement point. Every meaningful agent action passes through Policy.decide() → permit/deny/modify/log. Security, auditing, rate limiting, cost tracking in one place.
> (16) **Cognitive Namespaces** — isolated knowledge spaces with explicit ACL and channels. Permissioned subnets use namespaces. An agent's private knowledge is isolated; sharing is through logged explicit channels.
> (17) **Cognitive Signals** — typed interrupts for behavior modification (Pause/Resume/Reprioritize/InjectContext/Escalate/Cooldown/Explore/Shutdown).
> (18) **Cognitive Scheduling** — priority × expected value × 1/cognitive_cost.
> (19) **Forensic AI / Causal Replay** — cryptographically verifiable replay. Full regulatory compliance mapping: EU AI Act Art. 14 (human oversight) + FRIA (fundamental rights), SEC/CFTC (trading reconstruction), HIPAA (clinical decisions), SOX (financial controls).
> (20) **Pre-certified agent templates** — SEC-Compliant Trading Agent (MiFID II), HIPAA-Compliant Clinical Agent, GDPR-Compliant Data Agent. Moat: once blessed by regulator, switching cost becomes astronomical.
> (21) **Valhalla privacy layer** — TEE attestation, PSI, ZK range proofs (cross-reference 08-chain.md §P).
> (22) **CRITICAL #1 integration gap**: Safety policies ARE implemented in `roko-agent/src/safety/mod.rs` (SafetyLayer, 256 lines) AND in `roko-orchestrator/src/safety/`, AND SafetyLayer is wired to ToolDispatcher via `.with_safety(layer)`. **BUT** `orchestrate.rs` never creates a ToolDispatcher — it just calls `ExecAgent::run()`. Dispatcher is never invoked from the CLI pipeline. This is the #1 integration gap (per `11-inconsistencies.md`).
> (23) **Current status / gaps** — enumerate what exists (SafetyLayer, ToolDispatcher.with_safety, bash/git/network/path/scrub/rate_limit guards) vs. what's missing (actually invoking the dispatcher from CLI pipeline).
>
> Include citations: CaMeL (Debenedetti et al.), OWASP Top 10, Constitutional AI, Cohen undecidability theorems, C2PA content credentials, DIDs, cryptographic attestation research.
>
> Rename: golem → agent; mori → Roko Orchestrator.

---

### Interfaces & Visualization

- [ ] **12-interfaces.md** — CLI, TUI (ROSEDUST + Spectre), Web Portal, MCP, sonification reframe

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"12-interfaces.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/06-interfaces.md` — **full interface spec** (CLI, HTTP API, 29-screen TUI, ROSEDUST palette, Spectre, Web Portal, port allocation)
> - `refactoring-prd/09-innovations.md` §XIV Generative Interfaces (A2UI)
> - `refactoring-prd/08-translation-guide.md` §INCOMPATIBLE: Death Phases as UX, §NEEDS REDESIGN: Sonification
> - `refactoring-prd/10-developer-guide.md` §1, §11 (CLI UX with progressive help, error-as-teacher, interactive config)
> - `refactoring-prd/07-implementation-priorities.md` §Tier 4
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/09-tui-dashboard.md`
> - `roko/tmp/implementation-plans/11-sections/phase-0-1.md` — roko-serve extraction
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/12-interfaces.md` covering:
> (1) **CLI modes** — REPL, oneshot, pipe, daemon. List of built commands (init, run, status, orchestrate, prd lifecycle, research, neuro, episode, daemon, serve, mesh, provider, replay, inject, dashboard, repl).
> (2) **Zero-to-agent in 60 seconds** — `roko init && roko run "prompt"` works with smart defaults. 5 starter templates (coding, research, ops, chain, blank).
> (3) **`roko new` scaffolders** — domain, gate, scorer, router, policy, substrate, probe, event-source, template. Every scaffold compiles immediately with working tests.
> (4) **Progressive help** — `roko status`, `roko explain gates/routing/cognitive`, error-as-teacher messages with what/why/how-to-fix/context sections.
> (5) **HTTP API** — `roko-serve`. Endpoint list for run/orchestrate/status/engrams/episodes, agents, neuro, providers, routing/explain, mesh, websockets (events/agent/cfactor/spectre).
> (6) **Port allocation** — 3000 (web portal dev), 8080 (roko-serve HTTP), 8443 (WebSocket TLS), 8545 (mirage-rs Anvil RPC).
> (7) **TUI design** — 29 screens across 6 window regions. ROSEDUST adapted for 256-color and truecolor terminals. Full screen inventory from 06-interfaces.md:
>     - Window 1 Navigation (6): Agent list / Plan list / Mesh / Knowledge browser / Episode timeline / Settings
>     - Window 2 Agent Detail (6): Output stream / Gate results / Daimon state / Prediction dashboard / Tool trace / Cost breakdown
>     - Window 3 Plan Detail (5): DAG / Task detail / Merge queue / Timeline / Worktree status
>     - Window 4 Knowledge (4): Neuro explorer / Tier progression / Cross-domain map / Knowledge graph
>     - Window 5 Collective (4): C-Factor dashboard / Agent comparison / Pheromone landscape / Stigmergy map
>     - Window 6 System (4): Provider health / Resource monitor / Event log / Spectre gallery
> (8) **ROSEDUST design language** — full color palette (void-black, twilight, dusk, rose-dim/rose/rose-bright/rose-glow, jade, amber, crimson, violet, sapphire, ghost/mist/frost/white). Typography (monospace throughout — JetBrains Mono, Berkeley Mono). Glass morphism (twilight 80% + blur 12px + rose-dim 20% border). Motion (luxury easing cubic-bezier(0.16, 1, 0.3, 1), ambient breathing, data transitions). Dark-only.
> (9) **Spectre creature visualization** — procedurally generated from agent ID hash (first 64 bits = body shape/symmetry/limb count; next 32 bits = color; domain = texture: coding geometric / chain flowing / research fractal). Reflects Daimon PAD state:
>     - Engaged (steady breathing, open eyes, warm rose glow)
>     - Struggling (rapid pulsing, constricted, amber/crimson)
>     - Coasting (relaxed, expanded, soft sapphire)
>     - Exploring (expanded, flowing tendrils, violet)
>     - Focused (compact, bright, sharp edges, jade)
>     - Resting (minimal form, slow breathing, dim rose — Dreams active)
>     **NEVER dies. Never has Terminal state.** Adapts.
> (10) **Spectre as information display** — NOT decoration. Encodes: behavioral state (body/animation), knowledge tier distribution (complexity of form), current activity (eyes/tendrils), health (saturation/glow), mesh connections (visible filaments), pheromone emission (particles).
> (11) **Web Portal** — P2, not started. Tech: React 19 + Next 15.5, Tailwind 4, Radix, recharts, Three.js/react-three-fiber for Spectre WebGL, TanStack Query, WebSocket real-time, Privy, viem. 9 pages (Home/Agents/Agent Detail/Knowledge/Mesh/Plans/C-Factor/Providers/Settings).
> (12) **Spectre renderings by interface** — TUI ASCII/Unicode + ANSI; Web Portal WebGL 3D + full ROSEDUST glow; CLI inline small spectre next to status; API JSON state.
> (13) **Spectre collective display** — Spectres arranged by connection topology. Mesh connections as glowing filaments. Pheromone fields as ambient color clouds. C-Factor visualized as movement harmony between Spectres.
> (14) **Agent onboarding flow** — choose domain → select template or compose traits → configure routing → set knowledge prefs → name → Spectre generated → live.
> (15) **Generative Interfaces (A2UI)** — Google A2UI protocol. Agents describe UI needs as JSONL → frameworks render. Inherit ROSEDUST. Spectre as persistent visual anchor.
> (16) **MCP server for code intelligence** — not yet implemented.
> (17) **Sonification — reframed**. KEPT but remapped to behavioral states (Engaged/Struggling/Coasting/Exploring/Focused/Resting), NOT mortality phases (Thriving→Terminal). **NO terminal requiem, NO death animations, NO degraded ambient music**. Eno mandate preserved ("simultaneously ignorable and interesting"). 5 musical layers preserved. 8 presets remapped. Use legacy `bardo-backup/prd/24-sonification/05-musical-language.md` and `06-preset-catalog.md` for music theory foundations.
> (18) **Portal concept** — first-person dashboard.
> (19) **Accessibility** — WCAG 2.1 AA, keyboard navigable, screen reader (Spectre states described textually), reduced motion mode.
> (20) **Current status / gaps** — CLI built (38 tests). TUI text-only (scaffold). HTTP API incomplete. Web portal not started. MCP server not started. Spectre visualization not started.
>
> **KEEP**: ROSEDUST, glass morphism, Spectre creatures, 29-screen TUI, Portal concept. **REMOVE**: all death animations, terminal requiem, vitality phases, mortality-mapped sonification presets.
> Rename: bardo-terminal → Roko TUI; Bardo Sanctum → Roko Portal; golem → agent; mori → Roko Orchestrator.

---

### Coordination & Economy

- [ ] **13-coordination.md** — Stigmergy, pheromones, mesh sync, morphogenetic fields (generalized beyond blockchain)

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"13-coordination.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/04-knowledge-and-mesh.md` §3, §4
> - `refactoring-prd/02-five-layers.md` §Stigmergy
> - `refactoring-prd/09-innovations.md` §VI Network Flywheel, §XIII Cross-Domain Resonance
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/13-coordination.md` covering:
> (1) **Stigmergy theory** — Grassé 1959 (termite paper), Theraulaz 1999, Dorigo 1997 (ant colony optimization). Indirect coordination through environmental modification.
> (2) **Beyond the termite metaphor — stigmergy generalized**:
>     - Coding: Git repository. Commits, code patterns, test results.
>     - Blockchain: Korai chain. Knowledge entries, pheromones.
>     - Research: Shared Substrate. Insights, citations, analyses.
>     - Operations: Infrastructure state. Config, runbooks.
>     - Cross-domain: HDC vector space. Structural patterns.
> (3) **Git as stigmergy** for coding agents.
> (4) **Digital pheromones** (Parunak et al. 2002) — typed Engrams with specific decay profiles.
> (5) **PheromoneKind enum** — Threat (fast, hours), Opportunity (medium, days), Wisdom (slow, weeks), Alpha (very fast, minutes), Pattern (medium, code), Anomaly (medium), Consensus (slow), Custom(String).
> (6) **PheromoneScope** — Local(SubstrateId) / Mesh(CollectiveId) / Global (Korai chain).
> (7) **Pheromone struct** — kind, intensity, decay_rate, source, scope.
> (8) **Fleet sync / Collective mesh sync** — WebSocket + Iroh + ERC-8004 (cross-reference 08-chain.md).
> (9) **Morphogenetic specialization** — agents differentiate roles through pheromone gradients. Agent ecology (Odling-Smee et al. 2003).
> (10) **Stigmergy scaling properties**:
>     - O(1) per agent (read/write shared state, not peer-to-peer)
>     - Self-organizing (useful knowledge rises, bad knowledge decays)
>     - Cross-domain (chain-agent pheromone readable by coding-agent if HDC vector encodes analogy)
>     - Asynchronous (no synchronized clocks)
>     - Fault-tolerant (individual failure doesn't break coordination)
> (11) **Exponential flywheel** — More agents → more knowledge → better collective → each agent performs better → more agents → superlinear (Reed's Law: value scales as 2^N for groups).
> (12) **Knowledge exchange marketplace** — cross-reference 14-identity-economy.md.
> (13) **P2P transport** — cross-reference 08-chain.md §B.
> (14) **Collective intelligence emergence** — C-Factor, Woolley et al. 2010. Cross-reference 00-architecture.md.
> (15) **Current status / gaps** — pheromone types designed but not implemented (Tier 5E P2). Code uses basic Engrams with Decay::THREAT/OPPORTUNITY/WISDOM constants; pheromone-specific routing and scope enforcement are target features.
>
> Include all coordination/stigmergy citations (Grassé 1959, Theraulaz 1999, Parunak 2002, Dorigo 1997, Reed's Law, Metcalfe, Beer VSM, Woolley 2010).
> Rename: clade → collective/mesh; styx → Agent Mesh; golem → agent; bardo → roko.

---

- [ ] **14-identity-economy.md** — ERC-8004, reputation, KORAI/DAEJI tokenomics, marketplace, x402, knowledge futures

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"14-identity-economy.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/04-knowledge-and-mesh.md` §2, §3
> - `refactoring-prd/09-innovations.md` §VIII x402 Micropayments, §XVI Knowledge Futures Market, §IX Forensic AI (regulatory moat)
> - `refactoring-prd/07-implementation-priorities.md` §What Makes This a Series A Story
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/12b-chain-layer.md` §A (Korai Passport, tiers, ventriloquist defense), §C (Spore/Sparrow/auctions), §K (Reputation 7-domain EMA), §L (DAEJI/x402/escrow), §N (ISFR), §O (Clearing)
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/14-identity-economy.md` covering:
> (1) **ERC-8004 agent identity** — ERC-721 soulbound NFT. Identity registry, Reputation registry, Validation registry.
> (2) **Korai Passport** — full fields (see 08-chain.md §A for cross-reference). 4 tiers: Protocol / Sovereign (25K) / Worker (5K) / Edge. Ventriloquist defense.
> (3) **Reputation system** — 7-domain EMA decay, halving, disputes, cross-collective aggregation.
> (4) **Knowledge marketplace** and commerce bazaar.
> (5) **Machine Payment Protocol (MPP)** — HTTP 402, ERC-3009 signatures, charge/session intents.
> (6) **x402 micropayments** — Coinbase protocol (Linux Foundation backing, AWS/Visa/Mastercard/Stripe). Per-API-call billing < $0.001. Sub-second USDC settlement on Base. Self-funding agent loop: KORAI earnings → USDC conversion → x402 payment for compute → output generates value → user pays agent via x402 → reinvest → accelerate.
> (7) **Agent economy** — revenue, billing, proposals. Agent-as-a-business.
> (8) **KORAI/DAEJI tokenomics** — 1% annual demurrage on KORAI (knowledge must be actively maintained). Vickrey reputation-adjusted auction: `s_i = p_i × (1 + (1 - R_i))`. Payment = `s_second / (1 + (1 - R_winner))`.
> (9) **3 hiring models** — Random VRF, Blind auction (FPSB/Vickrey/Dutch), Direct hire (1.5× premium, anti-centralization > 20% → 2× fee).
> (10) **ISFR** — Intersubjective Fact Registry. Collective price discovery, rate aggregation, disputed claim resolution, 3-arbitrator voting.
> (11) **Clearing & settlement** — QP solver, bisection, certificates, fallback, batch clearing, DVP, finality.
> (12) **Knowledge Futures Market** (P3, deferred) — on-chain escrow for committed knowledge production. Research agent posts Knowledge Future → operations agents purchase → escrow funds research → delivery triggers release → non-delivery slashes stake. **Predictive market for knowledge production**.
> (13) **Regulatory moat** — Forensic AI Causal Replay enables compliance (cross-reference 11-safety.md). Enterprise value $100-500K/month per regulated enterprise; compliance failures cost $10M-$1B.
> (14) **a16z-compatible framing** — KYA (Know Your Agent), agent-native infrastructure, agent economy, verifiable provenance, measurable collective intelligence (C-Factor), autocatalytic self-improvement, network effect via on-chain knowledge.
> (15) **Series A pitch points**: agent-native infra (not wrapper), verifiable provenance (Forensic AI), measurable CI (C-Factor), autocatalytic improvement, agent-speed infra (recursive fan-out), cross-domain knowledge transfer (HDC), on-chain agent economy (Korai + ERC-8004 + KORAI), open composable SDK.
> (16) **Current status / gaps** — Tier 6 deferred. Chain layer intentionally last; focus on Tiers 1-5 first. Solo agents don't need chain.
>
> Include: EIP analysis citations, x402 protocol spec, Forensic AI regulatory mapping.
> Rename: GNOS → KORAI/DAEJI; golem → agent; clade → collective/mesh; bardo → roko; mori → Roko Orchestrator.

---

### Technical Foundations

- [ ] **15-code-intelligence.md** — Indexing, symbol graphs, PageRank, HDC fingerprints

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"15-code-intelligence.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/05-agent-types.md` §2 Coding Agent, §Niche Construction (affordance assessment)
> - `refactoring-prd/00-overview.md` §Crate Map (roko-index)
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/15-code-intelligence.md` covering:
> (1) **Incremental code indexing** — tree-sitter based. Languages: rust, typescript, go (via roko-lang-{rust,ts,go}).
> (2) **Symbol extraction** and directed dependency graph.
> (3) **PageRank for symbol importance**.
> (4) **HDC fingerprints** — structural similarity search. `roko-index::hdc`. 10,240-bit BSC over file and symbol structure.
> (5) **Context assembly from code search** — how indexed code becomes LLM context. Integration with roko-compose.
> (6) **MCP context server design**.
> (7) **Performance considerations** — index.db scaling, snapshot optimization.
> (8) **Affordance assessment** — before acting, measure doc coverage, test coverage, coupling, information scent. High-affordance code gets simple strategies; low-affordance gets cautious strategies with exploration budgets (MVT stopping — cross-reference 03-composition.md).
> (9) **Niche construction** — every commit modifies affordances for future agents. Positive (docs, tests, clean APIs) vs. negative (tangled deps, missing tests).
> (10) **Current status / gaps** — roko-index built, language crates built.
>
> Frame as `roko-index` crate's design. Rename: mori-index → roko-index; mori-context → roko-compose/roko-index; mori-mcp → roko MCP.

---

- [ ] **16-heartbeat.md** — CoALA tick pipeline, adaptive clocks, 3-speed cognition, attention auctions, active inference

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"16-heartbeat.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/01-synapse-architecture.md` §3 Universal Loop, §Three Cognitive Speeds, §Dual-Process Cognition, §4 Active Inference
> - `refactoring-prd/03-cognitive-subsystems.md` — all (subsystems drive the heartbeat)
> - `refactoring-prd/02-five-layers.md` §Adaptive Clock (L0 Runtime)
> - `refactoring-prd/09-innovations.md` §I 16 T0 Probes, §II VCG Auction, §XIX.A Active Inference State Space
> - `refactoring-prd/05-agent-types.md` §3 CoALA Heartbeat mapping
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/12a-cognitive-layer.md` §I Operating Frequencies (I1–I5)
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/16-heartbeat.md` covering:
> (1) **CoALA 9-step pipeline**: OBSERVE → RETRIEVE → ANALYZE → GATE → SIMULATE → VALIDATE → EXECUTE → VERIFY → REFLECT. Cite Sumers et al. 2023 arXiv:2309.02427.
> (2) **Mapping to universal Synapse loop** — PERCEIVE/EVALUATE/ATTEND/INTEGRATE/ACT/VERIFY/PERSIST/ADAPT/META-COGNIZE. Table from 05-agent-types.md §3 showing both 9-step variants.
> (3) **Three cognitive speeds**:
>     - Gamma (~5-15s) — reactive. Main orchestration loop.
>     - Theta (~75s) — strategic reflection. "Step back and think." Fires periodically during long plan runs.
>     - Delta (hours) — consolidation. Dreams replay, knowledge distillation, meta-cognition.
> (4) **Adaptive clock** — gamma/theta/delta frequencies in `roko-runtime`. Periods adapt based on context (faster gamma when more issues, regime multipliers on theta).
> (5) **Gating** — when to suppress/escalate ticks. Prediction error threshold drives T0/T1/T2 cascade.
> (6) **Context governor** — token allocation governance.
> (7) **Attention auctions (VCG)** — bidding across subsystems for context budget. Cross-reference 03-composition.md.
> (8) **Sleepwalker 3-step variant** for sleep mode.
> (9) **CorticalState** and cognitive state management.
> (10) **Dual-process cognition** (System 1 / System 2) — T0 (direct tool call, no LLM) / T1 (fast model, shallow) / T2 (full model, deep). Automatic tier selection from prediction error.
> (11) **Active inference for compute allocation** — EFE decomposition (pragmatic + epistemic - ambiguity). Zero hyperparameters for explore/exploit. Agent's own uncertainty determines compute investment.
> (12) **Active inference state space** (09-innovations.md §XIX.A) — factorized discrete POMDP (not world model, epistemic situation). 6 task phases × 5 context quality × 3 uncertainty = 90 states (tractable). Standard A/B/C/D matrices from pymdp (likelihood, transitions, preferences, priors).
> (13) **16 T0 probes** — zero-LLM probes run at every gamma tick. Pure functions. 80% tier suppression. List from 09-innovations.md §I (blockchain probes, coding probes, universal probes). Cite FrugalGPT arXiv:2305.05176.
> (14) **Meta-cognition hook** (12a I5) — "Am I stuck? Am I thrashing? Should I escalate?"
> (15) **Frequency scheduler** — decides which loop runs based on time-since-last-theta, idle detection.
> (16) **Current status / gaps** — bardo-primitives tier (InferenceTier, TierRouter) exists; CascadeRouter exists; roko-compose ContextTier exists; 12a I1–I5 wiring missing (gamma/theta/delta loops, frequency scheduler, meta-cognition hook).
>
> Include all cognitive architecture citations. Rename: golem → agent.

---

- [ ] **17-lifecycle.md** — Agent creation, deletion, knowledge transfer (REPLACES mortality system)

> **Prompt**: **CRITICAL — NO DEATH.**
>
> ALWAYS READ FIRST (5 reference docs). **Pay special attention to** `refactoring-prd/08-translation-guide.md` §ALL incompatibility sections.
>
> Read all sources listed under **"17-lifecycle.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/04-knowledge-and-mesh.md` §5 Knowledge Backup & Restore
> - `refactoring-prd/08-translation-guide.md` — all reframe rules
> - `refactoring-prd/07-implementation-priorities.md` §Dropped Items
> - `refactoring-prd/03-cognitive-subsystems.md` §1 Tier Progression (Ebbinghaus × tier)
> - `refactoring-prd/01-synapse-architecture.md` §Decay enum (memory management, NOT mortality)
>
> **SKIP entirely** (do NOT incorporate these source files):
> - `bardo-backup/prd/02-mortality/00-thesis.md` (death thesis)
> - `bardo-backup/prd/02-mortality/01-architecture.md` (death clocks)
> - `bardo-backup/prd/02-mortality/03-stochastic-mortality.md` (random death)
> - `bardo-backup/prd/02-mortality/04-economic-mortality.md` (extract budget math only)
> - `bardo-backup/prd/02-mortality/06-thanatopsis.md`
> - `bardo-backup/prd/02-mortality/08-mortality-affect.md` (extract somatic marker citations only)
> - `bardo-backup/prd/02-mortality/09-fractal-mortality.md`
> - `bardo-backup/prd/02-mortality/11-immortal-control.md`
> - `bardo-backup/prd/02-mortality/16-necrocracy.md`
> - `bardo-backup/prd/02-mortality/18-antifragile-mortality.md`
> - `bardo-backup/prd/01-golem/04-mortality.md`
> - `bardo-backup/prd/01-golem/05-death.md`
>
> **KEEP academic citations** from all mortality source files: Ray, Lenski, Ebbinghaus, Hayflick, etc. — they ground knowledge transfer and decay mechanisms.
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/17-lifecycle.md` covering:
> (1) **Agent lifecycle** — creation, provisioning, deletion, knowledge transfer. User-directed, NOT biological.
> (2) **NO death**. Agents persist until user deletes them.
> (3) **Agent creation** — `roko init`, provisioning, configuration and operator model.
> (4) **Funding and compute provisioning**.
> (5) **Knowledge backup/restore** (4-step from 04-knowledge-and-mesh §5):
>     - BACKUP: Export NeuroStore (JSONL + HDC vectors + tier metadata + provenance)
>     - DELETE: User explicitly deletes agent → agent and local store removed
>     - CREATE: User creates new agent → fresh NeuroStore
>     - RESTORE: Selective import. Entries start at Transient tier (must re-prove). Provenance tracks origin.
> (6) **Knowledge transfer via mesh** — live agents share knowledge through collective/mesh (cross-reference 13-coordination.md), not through biological inheritance.
> (7) **Knowledge staleness as epistemic decay** — Ebbinghaus forgetting curve STILL applies, but to *knowledge freshness*, NOT agent lifespan. Frequently-used knowledge decays slower.
> (8) **Knowledge demurrage** — token-level analog of knowledge decay (cross-reference 14-identity-economy.md). KORAI demurrage mirrors Engram half-life.
> (9) **Replication** — spawning new agents from existing ones (user-initiated).
> (10) **Agent deletion/teardown** — user-initiated only. Clean shutdown, resource cleanup, backup prompt.
> (11) **Reframed "economic pressure"** — budget limits, NOT death clocks.
> (12) **Reframed "epistemic pressure"** — prediction accuracy declining, knowledge plateau, NOT death.
> (13) **REMOVED concepts** — list them: mortality clocks, stochastic death, Weibull distribution death, Thanatopsis, necrocracy, bloodstain, katabasis, fractal mortality, immortal control, antifragile mortality (as death-themed).
> (14) **Academic grounding for decay** — keep ALL mortality research citations (Ray 1991 Tierra, Lenski LTEE, Ebbinghaus 1885, Hayflick 1961 cellular senescence, Ray 1998). Reframe as knowledge lifecycle, not agent lifespan. Evolutionary CS is still relevant; biological mortality is not.
> (15) **Provenance (replaces lineage-as-genealogy)** — Engram lineage DAG across time, NOT agent families across generations.
> (16) **Current status / gaps** — CLI backup/restore commands exist (roko neuro backup/restore); wiring gaps for selective restore and knowledge integration on import.
>
> Rename: mortality → lifecycle; succession → backup/restore; golem → agent; clade → collective/mesh.

---

- [ ] **18-tools.md** — Tool system, DeFi tools, MCP servers, 16 agent templates, plugin SDK

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"18-tools.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/05-agent-types.md` §2-6 (tools per agent type), §8 Extensibility
> - `refactoring-prd/10-developer-guide.md` §6 Plugin System (EventSource, MCP, FeedbackCollector), §7 Integration Patterns
> - `refactoring-prd/06-interfaces.md` §1 `roko new` scaffolders
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/11-agent-dogfooding.md` §Phase 2 (MCP), §Phase 3-4 (templates, scheduler)
> - `roko/tmp/implementation-plans/11-sections/phase-2.md` — **roko-mcp-github (17 tools)**, **roko-mcp-slack (8 tools)**, **roko-mcp-scripts (script wrapper, any language)**
> - `roko/tmp/implementation-plans/11-sections/phase-3-4.md` — **16 agent template definitions** with full system prompts
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/18-tools.md` covering:
> (1) **Tool architecture** — ToolDef, ToolContext, ToolResult, ToolExecutor registry.
> (2) **19 built-in tools** (from roko-std).
> (3) **DeFi tools** — 423+ tools across Uniswap, Aave, Morpho, Pendle, Lido, EigenLayer, GMX. Framed as chain domain plugin, not core framework.
> (4) **Tool categories**: data, trading, LP, vault, lending, staking, restaking, derivatives, yield, safety, intelligence, memory, identity, wallet, streaming.
> (5) **Tool profiles and configuration**.
> (6) **Wallet management** (3 custody modes: Delegation/Embedded/Local — cross-reference 08-chain.md).
> (7) **Testing strategy**.
> (8) **Service integrations** — MetaMask Delegation, Venice, Bankr, AgentCash, Uniswap.
> (9) **MCP integration** — JSON-RPC client, tool converter, dynamic registry.
> (10) **MCP servers** (from plan 11 Phase 2):
>     - `roko-mcp-github` — 17 tools (PR review, issue triage, repo management)
>     - `roko-mcp-slack` — 8 tools (message processing, notifications)
>     - `roko-mcp-scripts` — config-driven wrapper for any script (wraps 30+ Python automations; zero-code tool extension)
>     - `roko-mcp-stdio` — scaffold stdio server
> (11) **roko-plugin SDK** — EventSource, FeedbackCollector, Integration traits. The developer interface for extending Roko.
> (12) **EventSource examples** — CronEventSource, FileWatchEventSource, GitHubEventSource, SlackEventSource, WebhookEventSource.
> (13) **16 agent templates** (from plan 11 Phase 3):
>     - collab: doc-lifecycle, digest, meeting, sync, conflict-detector, freshness
>     - knowledge-base: pm-board, enrich, triage, pm-health, action-tracker
>     - roko: pr-review, slack-notify, auto-plan, code-implementer, gate-fixer
> (14) **Plugin loading mechanisms** (from 10-developer-guide.md §6): Cargo workspace members (compile-time), config-declared plugins (runtime), MCP tool discovery (runtime).
> (15) **`roko new` scaffolders** for domain, gate, scorer, router, policy, substrate, probe, event-source, template.
> (16) **Current status / gaps** — 19 built-in tools shipped; MCP servers scaffold only; roko-plugin not yet created (plan 11 Phase 0.2); agent templates not yet created (Phase 3).
>
> Rename: golem-tools → roko tools; golem → agent; mori → Roko Orchestrator.

---

- [ ] **19-deployment.md** — Packaging, daemon (launchd/systemd), cloud (Fly.io), Docker, WASM, edge

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"19-deployment.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/10-developer-guide.md` §5 Deployment Targets (Native/WASM/Docker/Daemon/Cloud/Edge)
> - `refactoring-prd/05-agent-types.md` §8 Deployment Flexibility
> - `refactoring-prd/06-interfaces.md` §7 Port Allocation
> - `refactoring-prd/07-implementation-priorities.md` §Tier 3H
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/11-sections/phase-5-6.md` — daemon wiring, launchd, systemd, cloud deploy (Fly.io), remote orchestrator, multi-repo config, secret management
> - `roko/tmp/implementation-plans/modelrouting/15-operational-surface.md` — CLI commands, testing, validation, dashboard, routing log, config migration
> - `roko/tmp/implementation-plans/modelrouting/16-production-hardening.md` — timeouts, retries, concurrency, shutdown, hedging, serve API
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/19-deployment.md` covering:
> (1) **Packaging and distribution strategy**.
> (2) **Native** (x86/ARM) — full features, recommended for production.
> (3) **WebAssembly** — core + std (MemorySubstrate, scorers/routers, HDC). WASM-compatible agent backend via HTTP fetch. Use case: browser playground, edge, serverless. What works: Engram/Score/Decay/Kind, all Scorer/Router/Composer/Policy, MemorySubstrate, HDC. What doesn't: FileSubstrate, ExecAgent, ProcessSupervisor, git worktrees.
> (4) **Docker** — Dockerfile, isolated environment.
> (5) **Daemon mode** — `roko daemon --start/--stop/--status`. launchd plist generation (macOS `~/Library/LaunchAgents/com.nunchi.roko.plist`), systemd unit (Linux `/etc/systemd/user/roko.service`).
> (6) **Cloud deployment** — `roko daemon --export-fly > fly.toml`. Any container platform. Env var overrides (`ROKO_*`).
> (7) **Edge/embedded** — minimal feature set, ~500KB binary. IoT, resource-constrained.
> (8) **Subscription configuration** — webhooks, cron, file watchers, GitHub, Slack per `roko.toml`.
> (9) **Multi-repo config** — per-repo config, cross-repo coordination.
> (10) **Secret management**.
> (11) **Remote orchestrator**.
> (12) **Production hardening** (from modelrouting 16):
>     - Adaptive timeouts (p95×2)
>     - Full-jitter backoff
>     - Per-provider semaphores
>     - Graceful shutdown (3-phase drain: reject-new, drain-inflight, force-close)
>     - Content-addressed dedup cache
>     - Hedged requests
> (13) **Operational surface** (from modelrouting 15) — CLI commands for testing, validation, dashboard, routing log, config migration.
> (14) **Current status / gaps** — partial Docker; daemon mode not wired; cloud deploy not wired; WASM feature flags exist but edge binary not validated.
>
> Rename: mori → Roko Orchestrator; bardo → roko; golem → agent.

---

- [ ] **20-technical-analysis.md** — Generalized oracles, predictive foraging, active inference, coding TA equivalents

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read all sources listed under **"20-technical-analysis.md"** in SOURCE-INDEX.md. Core refactoring-prd sections:
> - `refactoring-prd/03-cognitive-subsystems.md` §4 Oracles & Predictive Systems (generalized Oracle trait, domain-specific oracles, predictive foraging integration)
> - `refactoring-prd/09-innovations.md` §VII Predictive Foraging, §XIX.A Active Inference State Space
> - `refactoring-prd/01-synapse-architecture.md` §4 Active Inference
>
> Read implementation plans:
> - `roko/tmp/implementation-plans/modelrouting/12-advanced-patterns.md` — predictive foraging calibration, residual aggregation
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/20-technical-analysis.md` covering:
> (1) **Generalized Oracle trait** — `predict(query, ctx) -> Prediction` and `evaluate(prediction, outcome) -> PredictionAccuracy`. Domain-agnostic.
> (2) **Chain oracles** (TA primitives):
>     - Price prediction (moving averages, Bollinger bands, RSI)
>     - Volatility estimation
>     - Gas price forecasting
>     - Liquidity depth analysis
>     - MEV opportunity detection
> (3) **Coding oracles (TA equivalents)**:
>     - Build time prediction (equivalent of price prediction)
>     - Test failure probability (equivalent of risk assessment)
>     - Complexity drift detection (equivalent of trend analysis)
>     - Dependency risk scoring (equivalent of portfolio risk)
>     - Performance regression forecasting (equivalent of volatility)
> (4) **Research oracles**:
>     - Source reliability estimation
>     - Information completeness assessment
>     - Contradiction detection
> (5) **Witness-as-technical-analyst** concept — chain witness generalized to any domain observer.
> (6) **Hyperdimensional technical analysis** — HDC for pattern matching in market data + code data + research data.
> (7) **Spectral liquidity manifolds**.
> (8) **Adaptive signal metabolism**.
> (9) **Causal microstructure discovery** — Pearl SCM.
> (10) **Predictive geometry**.
> (11) **Resonant pattern ecosystem**.
> (12) **DeFi-native TA**.
> (13) **Adversarial signal robustness**.
> (14) **Somatic technical analysis** — intersection with Daimon. Cross-reference 09-daimon.md.
> (15) **Emergent multiscale intelligence**.
> (16) **Predictive foraging** — PredictionStore, falsifiable predictions, external verification, CalibrationTracker (model × task_category bias correction). ~50ns per correction.
> (17) **Active inference integration** — EFE for action selection (pragmatic + epistemic). Zero hyperparameters for explore/exploit.
> (18) **Active inference state space** — factorized POMDP (6 task phases × 5 context quality × 3 uncertainty = 90 tractable states).
> (19) **Current status / gaps** — Oracle trait not yet in roko-learn; predictive foraging scaffolded.
>
> **Generalize** beyond blockchain. Frame TA as universal oracle primitives with domain-specific instances.
> Include citations (Kalman, Mallat wavelets, Damasio, Friston Free Energy, Pearl causality, MIDAS anomaly detection).
> Rename: golem → agent.

---

### Reference

- [ ] **references.md** — Master citation list (200+ papers grouped by domain)

> **Prompt**:
>
> ALWAYS READ FIRST (5 reference docs).
>
> Read ALL sources listed under **"references.md"** in SOURCE-INDEX.md. Additionally:
> - **Scan every refactoring-prd file** for embedded citations
> - **Scan every implementation-plans file** in `roko/tmp/implementation-plans/` for cited papers (especially `modelrouting/11-research-context.md` and `12a-cognitive-layer.md` §Source Documents table)
> - **Scan every generated doc** (00-20) for citations
>
> Produce `/Users/will/dev/nunchi/roko/roko/docs/references.md` as a master reference list with EVERY academic citation found anywhere. Group by domain:
>
> 1. **Lifecycle & Finite Agency** (Ray Tierra 1991, Lenski LTEE, Ebbinghaus 1885, Hayflick 1961) — reframed for knowledge lifecycle, not agent mortality
> 2. **Memory Consolidation** (McClelland 1995 CLS, Wilson & McNaughton 1994, Mattar & Daw 2018, Ha & Schmidhuber 2018 World Models, Hafner 2025 DreamerV3, Lin et al. 2025 sleep-time compute)
> 3. **Affective Computing** (Mehrabian 1996 PAD, Damasio 1994 Descartes' Error, Bechara, Bower 1981, Plutchik, Russell-Mehrabian, Scherer 2001 appraisal, Walker & van der Helm 2009 emotional depotentiation)
> 4. **Dreams & Offline Learning** (Lacaux 2021, Haar Horowitz 2020/2023 Dormio, Park 2023 Generative Agents, WSCL 2024, Derrida 1993 hauntology, Boden creativity, Pearl SCM)
> 5. **Coordination & Multi-Agent** (Grassé 1959, Theraulaz 1999, Parunak 2002, Dorigo 1997 ACO, Reed's Law, Metcalfe, Woolley 2010 C-Factor, Beer VSM, Conant & Ashby Good Regulator)
> 6. **Biological Analogues** (Odling-Smee 2003 niche construction, Charnov 1976 MVT, Pirolli & Card 1999 information foraging, Kauffman autocatalytic sets, Yerkes-Dodson)
> 7. **Self-Learning Systems** (Reflexion Shinn et al., ExpeL Zhao et al., DSPy Khattab et al., Voyager Wang et al. 2023, Meta-Harness Lee et al. 2026 arXiv:2603.28052, EvoSkills 2026, ADAS Hu et al. ICLR 2025, AgentPRM)
> 8. **Context Engineering** (Liu et al. 2023 Lost in the Middle arXiv:2307.03172, Karpathy 2025 context engineering, ACE, CSO Samsung, ACON, Lewis 2020 RAG arXiv:2005.11401, RAGAS, ARES)
> 9. **Security & Provenance** (CaMeL Debenedetti et al., OWASP Top 10, Constitutional AI, Cohen undecidability, C2PA content credentials, DIDs, EU AI Act, HIPAA, SOX, GDPR)
> 10. **HDC / VSA** (Kanerva 2009 arXiv/Cognitive Computation 1(2), Plate 1994, Frady 2021 resonator networks, Kleyko 2022 ACM Computing Surveys 55(6), Neubert 2022 Proc IEEE, Johnson-Lindenstrauss 1984, Rahimi & Recht random features)
> 11. **Market Microstructure** (Kalman, Mallat wavelets, MIDAS anomaly detection, Ousterhout 2013 power-of-two-choices)
> 12. **Streaming Algorithms** (Hyperloglog, Count-Min Sketch, Bloom filters)
> 13. **Signal Processing** (Kalman, Mallat wavelets, spectral methods)
> 14. **Philosophy** (Heidegger, Jonas, Camus, Derrida 1993)
> 15. **Generational Learning** (Lenski LTEE, Ray Tierra — reframed)
> 16. **Agent Harnesses & Tool Use** (Meta-Harness Lee et al. 2026, FrugalGPT Chen et al. 2023 arXiv:2305.05176, SWE-bench, RouteLLM, MixLLM, GVU, GEPA, SAGE, ABC)
> 17. **Cybernetics & VSM** (Ashby's Law, Beer 1972 Brain of the Firm, Conant & Ashby Good Regulator Theorem, Wiener)
> 18. **Active Inference** (Friston FEP, Parr et al. 2024 arXiv:2402.14460, Koudahl et al. 2024 arXiv:2412.10425, VERSES Genius, pymdp)
> 19. **Process Reward Models** (Lightman et al. "Let's Verify Step by Step", AgentPRM, Song et al. ICLR 2025 generation-verification gap)
> 20. **Collective Intelligence** (Woolley et al. 2010 Science 330(6004) c-factor, Metcalfe's Law, Reed's Law)
> 21. **Regulatory Compliance / C2PA / AI Act** (EU AI Act Art. 14 human oversight + FRIA, SEC/CFTC trading reconstruction, HIPAA clinical audit, SOX financial controls)
> 22. **Cognitive Architecture** (CoALA Sumers 2023 arXiv:2309.02427, ACT-R, SOAR, CLARION dual-level, Kahneman System 1/2)
> 23. **Mechanism Design** (Vickrey 1961, Clarke 1971, Groves 1973, FPSB, VCG truthfulness)
> 24. **Protocol Standards** (ERC-8004, ERC-721, ERC-3009, ERC-4337, x402 Coinbase/Linux Foundation)
>
> Use standard academic format. For each citation, note which roko subsystem it grounds. Remove duplicates. Add every citation found in refactoring-prd files.

---

## Additional Context — Implementation Plan Mappings

Cross-reference map: which implementation-plans files feed which target docs. This duplicates
info in SOURCE-INDEX.md but is collected here for planning sessions.

| Target Doc | Implementation Plans to Read |
|---|---|
| **01-orchestration** | `11-agent-dogfooding.md` §Phase 3-4, `11-sections/phase-3-4.md` |
| **02-agents** | `modelrouting/01–07.md` (provider architecture + adapters + translator + integrations), `modelrouting/13-architectural-gaps.md` §A, `modelrouting/14-integration-refinements.md`, `modelrouting/19-implementation-guide.md`, `modelrouting/20-21.md` (Perplexity, Gemini), `11-agent-dogfooding.md` + `11-sections/phase-0-1.md` |
| **03-composition** | `12a-cognitive-layer.md` §E (E1–E6) |
| **04-verification** | `modelrouting/12-advanced-patterns.md`, `modelrouting/13-architectural-gaps.md` §H (GVU) |
| **05-learning** | `05-learning-wiring.md`, `modelrouting/08-11.md`, `modelrouting/12-advanced-patterns.md`, `modelrouting/17-meta-learning-and-corrections.md` (**8 missing feedback loops**), `12a-cognitive-layer.md` §D |
| **06-neuro** | `12a-cognitive-layer.md` §D (D1–D18: distillation, knowledge types, HDC), §R1 |
| **07-conductor** | `modelrouting/08-learning-loops.md` (circuit breaker), `modelrouting/16-production-hardening.md` (timeouts, shutdown) |
| **08-chain** | `12b-chain-layer.md` (76 items, 11 sections — **FULL FILE**), `12-nunchi-integration.md` (historical) |
| **09-daimon** | `12a-cognitive-layer.md` §F (F1–F9), §R2 |
| **10-dreams** | `12a-cognitive-layer.md` §G (G1–G8), §R3 |
| **11-safety** | `03-safety-hooks.md`, `11-inconsistencies.md` (**#1 integration gap**), `12b-chain-layer.md` §P (Valhalla privacy) |
| **12-interfaces** | `09-tui-dashboard.md`, `11-sections/phase-0-1.md` (roko-serve) |
| **13-coordination** | `12b-chain-layer.md` §B (Gossip), §N (ISFR) |
| **14-identity-economy** | `12b-chain-layer.md` §A, §C, §K, §L, §N, §O |
| **15-code-intelligence** | — (referenced by cognitive layer; roko-index already built) |
| **16-heartbeat** | `12a-cognitive-layer.md` §I (I1–I5 operating frequencies + meta-cognition hook) |
| **17-lifecycle** | — (user-initiated CLI workflow + config) |
| **18-tools** | `11-agent-dogfooding.md` §Phase 2 (MCP) + §Phase 3 (16 templates), `11-sections/phase-2.md`, `11-sections/phase-3-4.md` |
| **19-deployment** | `11-sections/phase-5-6.md` (daemon, launchd, cloud), `modelrouting/15-operational-surface.md`, `modelrouting/16-production-hardening.md` |
| **20-technical-analysis** | `modelrouting/12-advanced-patterns.md` (predictive foraging calibration) |
| **references** | `modelrouting/11-research-context.md` (23 sections), `12a-cognitive-layer.md` §Source Documents (14-row table) |

---

## Completion Tracking

| # | Doc | Status | Notes |
|---|---|---|---|
| 00 | 00-architecture.md | [ ] | |
| 01 | 01-orchestration.md | [ ] | |
| 02 | 02-agents.md | [ ] | |
| 03 | 03-composition.md | [ ] | |
| 04 | 04-verification.md | [ ] | |
| 05 | 05-learning.md | [ ] | |
| 06 | 06-neuro.md | [ ] | Rename grimoire→neuro |
| 07 | 07-conductor.md | [ ] | |
| 08 | 08-chain.md | [ ] | **LARGEST DOC** |
| 09 | 09-daimon.md | [ ] | **NO mortality** |
| 10 | 10-dreams.md | [ ] | **NO death triggers** |
| 11 | 11-safety.md | [ ] | Flag #1 integration gap |
| 12 | 12-interfaces.md | [ ] | ROSEDUST + Spectre; sonification reframe |
| 13 | 13-coordination.md | [ ] | Generalized stigmergy |
| 14 | 14-identity-economy.md | [ ] | KORAI/DAEJI, x402, Knowledge Futures |
| 15 | 15-code-intelligence.md | [ ] | |
| 16 | 16-heartbeat.md | [ ] | CoALA + universal loop + 3 speeds |
| 17 | 17-lifecycle.md | [ ] | **REPLACES mortality system** |
| 18 | 18-tools.md | [ ] | 16 agent templates + MCP |
| 19 | 19-deployment.md | [ ] | Daemon, WASM, edge, cloud |
| 20 | 20-technical-analysis.md | [ ] | Generalized oracles |
| 21 | references.md | [ ] | 200+ citations, 24 domains |

## Optional future expansion (not in the 22-doc set)

The parallel `refactoring-prd/MIGRATION-CHECKLIST.md` includes 4 additional target docs that
this 22-doc checklist does NOT generate as standalone files. Content for these folds into
other docs:

| Doc | Where Content Lives |
|---|---|
| `12b-sonification.md` | Folded into `12-interfaces.md` (Sonification reframed section) |
| `21-references.md` | Our `references.md` (already in the 22-doc set) |
| `22-glossary.md` | Covered by `README.md` naming map + individual docs' terminology. Generate as separate doc if needed. |
| `23-config-reference.md` | Covered by roko.toml examples in `10-developer-guide.md` and various docs' config sections. Generate if needed. |

If you want these as standalone docs later, see `refactoring-prd/MIGRATION-CHECKLIST.md` for
their prompts (targeting `bardo-backup/prd-updated/` instead of `roko/docs/`).
