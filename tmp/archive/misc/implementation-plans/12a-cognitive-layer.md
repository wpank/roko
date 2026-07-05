# 12a — Cognitive Layer: Memory, Affect, Learning

> **Split from**: 12-nunchi-integration.md (cognitive sections D, E, F, G, I, J, R1-R3)
> **Master plan reference**: Tier 5 (sections 5A-5F)
> **Priority**: P3 — Agent intelligence and memory
> **Depends on**: Tier 1 (Mori Parity) complete
> **Can parallel with**: Tiers 2-4 (Agent Platform)
> **Status**: Not started
>
> These components benefit ALL roko agents — solo CLI, event-driven, or chain-based.
> The chain-specific items are in 12b-chain-layer.md.

## Source Documents & Academic Foundations

Every concept in this spec traces to concrete research. **Read the relevant papers before implementing.**

| Concept | Primary Source | Academic Paper | What It Explains |
|---------|---------------|----------------|-----------------|
| **HDC / VSA** | `agent-chain/04-hdc.md` | [Kanerva 2009] Hyperdimensional Computing (Cognitive Computation 1(2)); [Neubert 2022] Vector Symbolic Architectures (Proc. IEEE); [Kleyko 2022] Survey on HDC (ACM Computing Surveys 55(6)) | 10,240-bit BSC vectors, XOR binding, majority bundling, Hamming similarity. Why random high-dim vectors are nearly orthogonal. |
| **Knowledge distillation** | `agent-chain/05-knowledge-layer.md` | [Park 2023] Generative Agents (UIST 2023, arXiv:2304.03442) | Memory + reflection + planning architecture. Periodic synthesis of higher-order observations from raw episodes. |
| **Cognitive architecture** | `agent-chain/01-overview.md` | [Sumers 2023] CoALA framework (arXiv:2309.02427) | 9-step cognitive pipeline: perceive→retrieve→reason→act→learn. Maps to Gamma/Theta/Delta frequencies. |
| **PAD affect model** | `prd/03-daimon/01-appraisal.md` | [Mehrabian 1996] Pleasure-Arousal-Dominance (Current Psychology 14(4)) | Foundational PAD emotional state model. 3 dimensions × 2 polarities = 8 octant states. |
| **Context assembly** | `agent-chain/15-dynamic-context-assembly.md` | [Liu 2023] Lost in the Middle (TACL, arXiv:2307.03172) | LLMs attend most to start+end of context (U-curve). Place highest-value entries at beginning/end. |
| **RAG** | `prd/12-inference/04-context-engineering.md` | [Lewis 2020] RAG for Knowledge-Intensive NLP (NeurIPS 2020, arXiv:2005.11401) | Retrieval-augmented generation. HDC index is a decentralized RAG implementation. |
| **Stigmergy** | `agent-chain/03-stigmergy.md` | [Grassé 1959] (Insectes Sociaux 6(1)); [Theraulaz 1999] (Artificial Life 5(2)); [Dorigo 1997] Ant Colony (IEEE Trans. Evol. Comp. 1(1)) | Indirect coordination through environment. Knowledge entries are pheromone deposits that decay (demurrage). Confirmed entries reinforced, unconfirmed evaporate. |
| **Cybernetics / VSM** | `agent-chain/03-stigmergy.md` §5 | [Beer 1972] Brain of the Firm (Allen Lane) | Viable System Model: 5 nested systems. Maps: System 1=agents, System 2=pheromone field, System 3=tokenomics, System 4=HDC index, System 5=governance. |
| **Collective intelligence** | `agent-chain/proving-collective-intelligence.md` | Multiple (see `agent-chain/08-references.md`) | How to prove collective knowledge sharing actually causes improvement, not just correlation. |
| **Self-improvement** | `agent-chain/self-improvement-frameworks.md` | Meta-Harness (Stanford, March 2026): 6× performance gap from harness changes alone | Bayesian optimization over prompt templates, context weights, tool routing. |
| **Harness engineering** | `agent-chain/harness-engineering.md` + `context-quality-science.md` | [Meta-Harness 2026]; RAGAS/ARES frameworks | The scaffold around the model matters as much as the model. Context quality measurement. |
| **Exponential mechanisms** | `agent-chain/09-exponential-flywheels.md` | `agent-chain/exponential-mechanisms-research.md` | 10 mechanisms for compounding dynamics in knowledge networks. |
| **Predictive foraging** | `agent-chain/10-predictive-foraging.md` | Falsifiable prediction for self-improving knowledge quality | Agents make predictions, chain verifies, incorrect predictions decay knowledge. |
| **Autonomous eval** | `agent-chain/17-autonomous-eval-generation.md` | EVMbench; DSPy Bayesian optimizers; Karpathy autoresearch loop | EVM as deterministic oracle for grading agent performance. Closing the self-improvement loop. |

**Full bibliography**: `bardo-backup/tmp/agent-chain/08-references.md` — 50+ papers with full citations, links, and relevance notes.

**Key research files** (read in order for full theoretical context):
1. `agent-chain/14-academic-foundations.md` — 15 research traditions that converge on one architecture
2. `agent-chain/04-hdc.md` — HDC math from first principles (most mathematically dense)
3. `agent-chain/03-stigmergy.md` — stigmergic coordination theory
4. `agent-chain/05-knowledge-layer.md` — 6 knowledge types, context assembly
5. `agent-chain/15-dynamic-context-assembly.md` — from stigmergy to perfect prompts
6. `agent-chain/09-exponential-flywheels.md` — compounding collective intelligence

---

## Architecture Principles

1. **roko-neuro** replaces grimoire. It's a standalone crate — memory/knowledge for ANY roko
   agent, not just blockchain agents. A solo agent running `roko plan run` benefits equally.

2. **Modular cognition**: Each cognitive subsystem (memory, affect, dreams) is a separate crate
   with a clean trait boundary. They compose through the signal bus — no tight coupling.

3. **HDC-first retrieval**: Knowledge entries, episodes, and signals all get 10,240-bit BSC
   vectors via `bardo-primitives::HdcVector`. Retrieval uses Hamming similarity — no embedding
   API calls, no GPU, no external service.

4. **Existing code first**: `bardo-primitives` (HDC vectors), `roko-index` (symbol graph + HDC
   fingerprints), and `roko-learn` (episodes, playbooks, patterns, baselines, cascade router)
   are already built. Most of this plan is wiring and extending, not building from scratch.

5. **Three-speed cognition**: Operating frequencies (fast/medium/slow) map to the existing
   `InferenceTier` (T0/T1/T2) in `bardo-primitives::tier`. The cognitive layer adds
   per-frequency scheduling and budget allocation on top.

---

## D. Knowledge & Memory (roko-neuro)

### D.1 Distillation Pipeline (PRD: 04-memory/01-grimoire.md, 04-memory/01b-grimoire-memetic.md)

**What exists**:
- `roko-learn::episode_logger` — append-only JSONL episode log (`crates/roko-learn/src/episode_logger.rs`)
- `roko-learn::playbook` — reusable action sequences (`crates/roko-learn/src/playbook.rs`)
- `roko-learn::playbook_rules` — rule extraction (`crates/roko-learn/src/playbook_rules.rs`)
- `roko-learn::pattern_discovery` — trigram mining (`crates/roko-learn/src/pattern_discovery.rs`)
- `roko-learn::skill_library` — named capabilities (`crates/roko-learn/src/skill_library.rs`)
- `roko-learn::baseline` — historical performance baselines (`crates/roko-learn/src/baseline.rs`)
- `roko-learn::regression` — regression detection (`crates/roko-learn/src/regression.rs`)
- `roko-learn::runtime_feedback` — unified learning orchestration (`crates/roko-learn/src/runtime_feedback.rs`)

**What's missing** (the distillation pipeline that converts raw episodes into reusable knowledge):

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| D1 | **4-tier distillation**: Raw Episodes -> Insights -> Heuristics -> PLAYBOOK | [neuro] | 🆕 | Core spec from PRD. Each tier compresses + validates |
| D2 | Episode -> Insight extraction (pattern detection across episodes) | [neuro] | 🆕 | "When X happened, Y consistently followed" |
| D3 | Insight -> Heuristic promotion (3+ confirmations -> actionable rule) | [neuro] | 🆕 | Confidence threshold for promotion |
| D4 | Heuristic -> PLAYBOOK compilation (top heuristics -> `PLAYBOOK.md` action rules) | [neuro] | 🆕 | Human-readable + machine-parseable playbook |
| D5 | Temporal decay (half-life per knowledge type, configurable defaults) | [neuro] | 🆕 | Insights: 30d, Heuristics: 90d, Facts: 365d |
| D6 | Confirmation boost (independent validation extends weight by 1.5x) | [neuro] | 🆕 | Prevents premature decay of validated knowledge |

**Reference**:
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/04-memory/01-grimoire.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/04-memory/01b-grimoire-memetic.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/episode-logger.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/playbook.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/skill-library.md`

**Verification**:
- [ ] `roko plan run` on a 5-task plan produces >=1 insight in `.roko/neuro/knowledge.jsonl`
- [ ] Insights with <5 episodes stay at "insight" tier; those with >=5 promote to "heuristic"
- [ ] Knowledge entries older than 2x half-life have confidence < 0.5

### D.2 Knowledge Types & Storage (PRD: 04-memory/01-grimoire.md)

**What exists**:
- `roko-fs::FileSubstrate` — JSONL storage with GC (`crates/roko-fs/`)
- Episode logger JSONL format as pattern for append-only stores

**What's missing**:

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| D7 | `KnowledgeKind` enum: Fact, Strategy, Insight, Heuristic, AntiKnowledge | [neuro] | 🔧 | mirage has `KnowledgeKind`; neuro needs local equivalents |
| D8 | Local JSONL knowledge store (`.roko/neuro/knowledge.jsonl`) | [neuro] | 🆕 | Append-only, GC'd by decay |
| D9 | Knowledge entry struct (content, type, confidence, source_episodes, hdc_vector, created, half_life) | [neuro] | 🆕 | Core data model |
| D10 | AntiKnowledge (challenge mechanism: contradicts existing knowledge, 2x stake requirement on chain) | [neuro] | 🆕 | Locally: "this insight was wrong" with evidence |
| D11 | Knowledge query API (semantic search + temporal relevance + affect filters) | [neuro] | 🆕 | Used by context assembly |

**Reference**:
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/04-memory/01-grimoire.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/04-memory/00-overview.md`

**Verification**:
- [ ] `KnowledgeEntry` serializes/deserializes round-trip to JSONL
- [ ] Query API returns entries sorted by (confidence * temporal_relevance)
- [ ] AntiKnowledge entries reduce confidence of contradicted entries

### D.3 HDC Integration

> **HDC** = Hyperdimensional Computing (aka Vector Symbolic Architectures / VSA).
> Uses 10,240-bit Binary Spatter Code (BSC) vectors. Operations: XOR for binding,
> majority-vote for bundling, cyclic-shift for permutation. Similarity: Hamming distance.
> No embedding API, no GPU, no external service — pure bitwise ops, sub-microsecond per comparison.
>
> **Why 10,240 bits?** Johnson-Lindenstrauss lemma [1984]: for N=100K entries and ε=0.1,
> need D≥4,604 dimensions. 10,240 provides generous headroom.
>
> **Research**: `agent-chain/04-hdc.md` (math from first principles),
> [Kanerva 2009], [Neubert 2022] (VSA survey), [Kleyko 2022] (comprehensive HDC survey),
> [Frady 2021] (resonator networks for advanced retrieval).

**What exists**:
- `bardo-primitives::HdcVector` — 10,240-bit BSC vector with XOR bind, majority bundle, Hamming similarity (`crates/bardo-primitives/src/hdc.rs`)
- `roko-index::hdc` — HDC fingerprinting for source files and symbols (`crates/roko-index/src/hdc.rs`)
- `roko-learn::hdc_clustering` — K-medoids clustering over HDC vectors (`crates/roko-learn/src/hdc_clustering.rs`)

**What's missing** (wiring the existing HDC primitives into the knowledge/memory layer):

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| D12 | HDC encoding for knowledge entries (text -> 10,240-bit BSC vector) | [neuro] | ✅ | `bardo-primitives::HdcVector` exists; needs wrapper for text content |
| D13 | HDC index (Hamming-nearest-neighbor search over knowledge vectors) | [neuro] | ✅ | `roko-index::hdc::similarity` exists; needs wiring to knowledge store |
| D14 | HDC encoding for signals (signal payload -> BSC vector for retrieval) | [neuro] | 🆕 | Extends `roko-core::Signal` with optional HDC field |
| D15 | HDC encoding for episodes (episode summary -> BSC vector for clustering) | [neuro] | 🆕 | Feeds `roko-learn::hdc_clustering::KMedoids` |
| D16 | HDC-based knowledge retrieval in context assembly | [neuro + compose] | 🆕 | Replace keyword matching with Hamming similarity |
| D17 | HDC cluster labels (human-readable labels for K-medoids output) | [neuro] | 🆕 | Auto-generate from cluster centroid's nearest knowledge entries |
| D18 | HDC similarity threshold tuning (configurable in `roko.toml`) | [neuro] | 🆕 | Default 0.6; too low = noise, too high = misses |

**Reference**:
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/bardo-primitives/src/hdc.rs`
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/hdc.rs`
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/hdc_clustering.rs`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/04-memory/01c-grimoire-hdc.md`

**Verification**:
- [ ] Text content round-trips through HDC encode -> similarity check with >0.9 self-similarity
- [ ] Knowledge query via HDC retrieves top-5 relevant entries in <10ms for 10K entries
- [ ] Episode clustering produces meaningful groups (manual inspection on real episode data)

---

## E. Context Assembly Pipeline

### E.1 Neuro-Aware Context (PRD: 12-inference/04-context-engineering.md)

> **Key insight** [Liu 2023, "Lost in the Middle"]: LLMs attend most strongly to the
> **beginning** and **end** of their context window, with degraded attention to middle
> positions (U-shaped attention curve). Context assembly must place highest-value
> entries at start+end, medium-value in the middle.
>
> **Active inference scoring** [E2]: `score = track_record(entry) × belief_change(entry) / uncertainty`.
> Pragmatic value (has this knowledge helped before?) × epistemic value (how much would this
> change the agent's beliefs?) balanced by uncertainty.
>
> **Research**: `agent-chain/15-dynamic-context-assembly.md` (from stigmergy to perfect prompts),
> `agent-chain/context-quality-science.md` (measuring whether retrieved knowledge actually helps),
> `agent-chain/harness-engineering.md` (why scaffold > model).

**What exists**:
- `roko-compose::SystemPromptBuilder` — 6-layer prompt assembly (`crates/roko-compose/src/system_prompt_builder.rs`)
- `roko-compose::context_provider` — Tier-aware context (Surgical/Focused/Full) (`crates/roko-compose/src/context_provider.rs`)
- `roko-compose::enrichment` — 13-module enrichment pipeline (`crates/roko-compose/src/enrichment/`)
- `roko-compose::scorer::SectionScorer` — priority x recency x relevance ranking (`crates/roko-compose/src/scorer.rs`)
- `roko-compose::role_prompts::RoleSystemPromptSpec` — wired into orchestrate.rs (`crates/roko-compose/src/role_prompts.rs`)

**What's missing** (extending context assembly to use neuro knowledge + affect):

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| E1 | 5-stage pipeline: Query -> Score -> Deduplicate -> Budget -> Format | [roko] | 🔧 | SystemPromptBuilder does 6-layer assembly; needs scoring/dedup stages |
| E2 | **Active inference scoring** (pragmatic value x epistemic value, balanced by uncertainty) | [roko] | 🆕 | PRD formula: `score = track_record(entry) x belief_change(entry) / uncertainty` |
| E3 | **Attention-curve positioning** (Liu et al. U-shape: high-value at start+end of prompt) | [roko] | 🆕 | Reorder retrieved entries by attention curve |
| E4 | Affect-modulated retrieval (PAD state biases what knowledge is surfaced) | [roko] | 🆕 | Depends on Daimon (F1); high arousal -> recent + action-oriented |
| E5 | Dynamic token budget (fit within model context window, prioritize by score) | [roko] | 🔧 | SystemPromptBuilder has layers; needs dynamic budget allocation |
| E6 | Neuro injection (pull from local knowledge store during context assembly) | [roko] | 🆕 | Bridge between roko-neuro and roko-compose |

**Reference**:
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/12-inference/04-context-engineering.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/enrichment-pipeline.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/enrichment-steps.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/context-pack-cache.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/compose/system-prompt-builder.md`
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_provider.rs`
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs`

**Verification**:
- [ ] A task with matching playbook gets playbook steps injected into its system prompt
- [ ] Token budget is respected: neuro sections are truncated before code sections for Surgical tier
- [ ] Anti-knowledge entries appear in prompts for tasks that match their failure patterns

---

## F. Daimon (Affect / Motivation)

### F.1 Core Affect Model (PRD: 03-daimon/00-overview.md, 03-daimon/01-appraisal.md)

> **PAD model** [Mehrabian 1996]: Pleasure-Arousal-Dominance. Three orthogonal dimensions,
> each in [-1, 1]. Defines 8 octant states: Excited (+P+A+D), Anxious (-P+A-D),
> Confident (+P-A+D), Bored (-P-A+D), etc. Agent affect modulates behavior: anxious
> agents prefer proven playbooks, confident agents explore novel approaches, high-arousal
> agents escalate model tier.
>
> **Not just sentiment**: PAD captures motivational state, not just "happy/sad". Dominance
> dimension is critical — it captures agency (am I in control?) which determines whether
> the agent should act autonomously or seek help.
>
> **Research**: [Mehrabian 1996] (foundational PAD), `prd/03-daimon/01-appraisal.md` (appraisal rules),
> `prd/03-daimon/03-behavior.md` (behavior modulation table), `prd/03-daimon/07-runtime-daimon.md` (runtime integration).

**What exists**:
- `roko-golem::daimon::DaimonEngine` — scaffold only (`crates/roko-golem/src/daimon.rs`)
- `roko-learn::efficiency::AgentEfficiencyEvent` — per-turn quality/cost snapshot (`crates/roko-learn/src/efficiency.rs`)
- `roko-learn::regression` — regression detection (`crates/roko-learn/src/regression.rs`)

**What's missing** (a standalone affect system that benefits all agents, not just golem):

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| F1 | `PadVector` (Pleasure, Arousal, Dominance: each f32 in [-1, 1]) | [roko] | 🔧 | `DaimonEngine` stub in roko-golem; extract to standalone |
| F2 | 8 affect states from PAD octants (+P+A+D=Excited, -P+A-D=Anxious, etc.) | [roko] | 🆕 | Maps vector -> named state |
| F3 | Appraisal triggers (task success/failure, gate results, reputation changes, time pressure) | [roko] | 🆕 | Events -> PAD vector updates via appraisal rules |
| F4 | Decay toward baseline (affect decays to [0,0,0] with configurable half-life) | [roko] | 🆕 | Prevents permanent affect drift |
| F5 | Affect -> behavior modulation table (state -> risk tolerance, communication style, exploration rate) | [roko] | 🆕 | E.g., Anxious -> conservative, lower exploration |
| F6 | Affect signatures on episodes (every agent turn tagged with current PAD) | [roko] | 🆕 | Enriches episode logging (extends plan 11 S7.1) |
| F7 | Affect -> SystemPromptBuilder (emotional state modifies prompt tone/focus) | [roko] | 🆕 | "You are under time pressure" vs "You have time to explore" |
| F8 | Affect -> CascadeRouter (arousal level influences model selection: high arousal -> faster model) | [roko] | 🆕 | Extends existing CascadeRouter |
| F9 | Affect persistence (`.roko/daimon/affect.json`, survives restart) | [roko] | 🆕 | Agent "wakes up" with residual affect |

**Reference**:
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/03-daimon/00-overview.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/03-daimon/01-appraisal.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/03-daimon/03-behavior.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/03-daimon/07-runtime-daimon.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/03-daimon/09-evaluation.md`
- Scaffold: `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/src/daimon.rs`

**Verification**:
- [ ] After 3 consecutive gate failures, PAD pleasure drops below -0.3 and arousal exceeds 0.3
- [ ] Affect decays to within 0.1 of neutral after 2x half-life with no new events
- [ ] Affect -> CascadeRouter path tested: high arousal routes to faster (smaller) model
- [ ] Agent turns in `.roko/episodes.jsonl` include PAD vector in their extra fields

---

## G. Dreams (Offline Learning)

### G.1 Dream Replay & Consolidation (PRD: 05-dreams/00-overview.md, 05-dreams/02-replay.md, 05-dreams/04-consolidation.md)

> **Dreams = offline learning during idle time.** Inspired by biological sleep consolidation
> (hippocampal replay). When no active tasks, the agent reviews accumulated episodes to
> extract higher-level patterns, strengthen/weaken heuristics, and generate novel strategies.
>
> **Key operations**: (1) Re-evaluate past episodes with current knowledge — "would I do this
> differently now?" (2) Cluster episodes via HDC K-medoids to find meta-patterns. (3) Cross-
> pollinate knowledge across unrelated domains. (4) Counterfactual simulation via HDC vector
> permutation — "what if X had been different?"
>
> **Existing code to wire**: `roko-learn::pattern_discovery::PatternMiner` (trigram mining),
> `roko-learn::hdc_clustering::KMedoids` (clustering), `roko-learn::baseline::compute_baselines`,
> `roko-learn::regression::detect_regressions`.
>
> **Research**: [Park 2023] Generative Agents reflection cycles, `prd/05-dreams/02-replay.md`,
> `prd/05-dreams/04-consolidation.md`, `agent-chain/10-predictive-foraging.md` (falsifiable prediction).

**What exists**:
- `roko-golem::dreams::DreamsEngine` — scaffold only (`crates/roko-golem/src/dreams.rs`)
- `roko-learn::pattern_discovery::PatternMiner` — trigram mining (`crates/roko-learn/src/pattern_discovery.rs`)
- `roko-learn::hdc_clustering::KMedoids` — episode clustering (`crates/roko-learn/src/hdc_clustering.rs`)
- `roko-learn::baseline::compute_baselines` — performance baselines (`crates/roko-learn/src/baseline.rs`)
- `roko-learn::regression::detect_regressions` — regression detection (`crates/roko-learn/src/regression.rs`)
- Episode JSONL log with full turn data (`roko replay` CLI subcommand walks signal DAG)
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/dream-consolidation.md`

**What's missing** (the offline "sleep" cycle that processes accumulated episodes):

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| G1 | Episode replay scheduler (trigger during idle time -- no active tasks) | [roko] | 🆕 | When agent has nothing to do, it dreams |
| G2 | Re-evaluate past episodes with current knowledge (would I do this differently now?) | [roko] | 🆕 | Compare past decision vs current heuristics |
| G3 | Mistake identification (failed episodes -> what went wrong -> insight) | [roko] | 🆕 | Feed into Neuro distillation pipeline (D2) |
| G4 | Heuristic strengthening/weakening from replay (confirm or revise) | [roko] | 🆕 | Update confidence scores in knowledge store |
| G5 | Dreams output -> Neuro (replay generates new Insights/Heuristics) | [roko] | 🆕 | Direct pipe: dreams -> D1-D4 pipeline |
| G6 | Counterfactual simulation (HDC vector shifting: "what if X had been different?") | [roko] | 🆕 | Use HDC permutation to explore semantic neighborhoods |
| G7 | Cross-episode consolidation (discover meta-patterns across unrelated episodes) | [roko] | 🆕 | HDC bundling of episode vectors -> cluster detection |
| G8 | Novel strategy generation (combine heuristics from different domains) | [roko] | 🆕 | Cross-pollination of knowledge |

**Reference**:
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/05-dreams/00-overview.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/05-dreams/01-architecture.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/05-dreams/01b-dream-evolution.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/05-dreams/02-replay.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/05-dreams/03-imagination.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/05-dreams/04-consolidation.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/05-dreams/06-integration.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/dream-consolidation.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/pattern-discovery.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/baseline-computation.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/plateau-detection.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/skill-library.md`
- Scaffold: `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/src/dreams.rs`

**Verification**:
- [ ] Dream replay identifies at least one mistake from a set of 10+ episodes with failures
- [ ] Heuristic confidence scores update after replay (strengthen confirmed, weaken refuted)
- [ ] Cross-episode consolidation finds meta-patterns via HDC bundling over 20+ episodes
- [ ] Dream output feeds D1-D4: new insights appear in `.roko/neuro/knowledge.jsonl` after a dream cycle
- [ ] Counterfactual simulation produces alternative strategies logged to `.roko/dreams/counterfactuals.jsonl`

---

## I. Operating Frequencies (3-Speed Cognition)

> **Named after EEG frequency bands** (from [Sumers 2023] CoALA framework):
> - **Gamma** (~10s): reactive — perceive, retrieve, act. Current orchestration loop.
> - **Theta** (~2-5min): strategic — re-plan, update goals, evaluate progress. "Step back and think."
> - **Delta** (~30min+): consolidation — dream replay, knowledge distillation, meta-cognition.
>
> Maps to existing code: `bardo-primitives::tier::InferenceTier` (T0/T1/T2),
> `roko-compose::context_provider::ContextTier` (Surgical/Focused/Full).
>
> **Research**: `agent-chain/01-overview.md` (CoALA mapping),
> `prd/12-inference/01-deployment-modes.md` (deployment modes),
> `prd/12-inference/01a-routing.md` (model routing).

**What exists**:
- `bardo-primitives::tier::InferenceTier` — T0/T1/T2 tiers (`crates/bardo-primitives/src/tier.rs`)
- `bardo-primitives::tier::TierRouter` — tier + vitality -> model name (`crates/bardo-primitives/src/tier.rs`)
- `roko-learn::cascade_router::CascadeRouter` — 3-stage model routing (`crates/roko-learn/src/cascade_router.rs`)
- `roko-compose::context_provider::ContextTier` — Surgical/Focused/Full context (`crates/roko-compose/src/context_provider.rs`)

**What's missing** (three cognitive frequencies and their scheduling):

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| I1 | **Gamma loop** (~10s): reactive -- perceive, retrieve, act | [roko] | 🆕 | Current orchestration loop is mostly gamma |
| I2 | **Theta loop** (~2-5min): strategic -- re-plan, update goals, evaluate progress | [roko] | 🆕 | Periodic "step back and think about the plan" |
| I3 | **Delta loop** (~30min+): consolidation -- dreams replay, knowledge distillation, meta-cognition | [roko] | 🆕 | Trigger dreams (G1), playbook compilation (D4) |
| I4 | Frequency scheduler (decides which loop to run based on context) | [roko] | 🆕 | Time-since-last-theta, idle-detection, etc. |
| I5 | Meta-cognition hook (agent reflects on its own cognitive state) | [roko] | 🆕 | "Am I stuck? Am I thrashing? Should I escalate?" |

**Reference**:
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/bardo-primitives/src/tier.rs`
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/context_provider.rs`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/12-inference/01-deployment-modes.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/12-inference/01a-routing.md`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/12-inference/15-inference-profiles.md`

**Verification**:
- [ ] Gamma loop handles reactive tasks (~10s cycle time) and is the default orchestration mode
- [ ] Theta loop fires periodically (~2-5min) during long plan runs to re-evaluate strategy
- [ ] Delta loop triggers dreams (G1) and knowledge distillation (D4) after plan completion or on idle
- [ ] Meta-cognition hook detects "stuck" state (>3 retries on same task) and suggests escalation

---

## J. C-Factor Metrics

> **C-Factor** = collective intelligence factor, inspired by psychometrics' g-factor.
> A single composite metric capturing how effectively the system translates plans into
> working code. Derived from [Woolley 2010] "Evidence for a Collective Intelligence Factor"
> (Science 330(6004)): groups have a measurable general intelligence factor (c) analogous
> to individual g-factor. Key finding: c correlates more with social sensitivity and
> turn-taking equality than with max individual ability.
>
> **For roko**: c-factor = `gate_pass_rate × 0.3 + cost_efficiency × 0.2 + speed × 0.15 + first_try_rate × 0.25 + knowledge_growth × 0.1`. High c-factor → prefer cheaper models (system is performing well). Low c-factor → prefer stronger models.
>
> **Research**: `agent-chain/proving-collective-intelligence.md` (from correlation to causal proof),
> `agent-chain/17-autonomous-eval-generation.md` (autonomous evaluation), `agent-chain/self-improvement-frameworks.md` (Bayesian optimization over parameters).

### J.1 Cognitive Performance Tracking

**What exists**:
- `roko-learn::efficiency::AgentEfficiencyEvent` — per-turn cost/quality (`crates/roko-learn/src/efficiency.rs`)
- `roko-learn::baseline` — aggregate performance by role/complexity (`crates/roko-learn/src/baseline.rs`)
- `roko-learn::task_metric` — per-task metrics (`crates/roko-learn/src/task_metric.rs`)
- `roko-learn::costs_db` + `roko-learn::costs_log` — cost tracking (`crates/roko-learn/src/costs_db.rs`, `crates/roko-learn/src/costs_log.rs`)

**What's missing** (collective intelligence factor measuring agent cognitive performance):

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| J1 | **Information flow rate** (messages sent/received per unit time, latency) | [both] | 🆕 | Measure signal throughput; locally measure signal processing speed |
| J2 | **Turn-taking equality** (Gini coefficient of agent contributions) | [both] | 🆕 | Even participation = higher c-factor. Locally: are all agents productive? |
| J3 | **Social sensitivity proxy** (response quality to other agents' outputs) | [roko] | 🆕 | How well does agent incorporate context from others? |
| J4 | **Knowledge integration rate** (how fast shared insights get confirmed) | [both] | 🆕 | Track confirmation chains in Neuro |
| J5 | **Task diversity coverage** (are agents specializing effectively?) | [both] | 🆕 | Capability utilization vs overlap |
| J6 | **Convergence velocity** (time from divergent opinions to shared conclusion) | [both] | 🆕 | Measure via knowledge agreement |
| J7 | Per-agent c-factor contribution score | [roko] | 🆕 | How much does this agent improve collective intelligence? |
| J8 | Per-fleet c-factor (across agents in a `roko plan run` session) | [roko] | 🆕 | Measurable today: multi-agent plan execution |
| J9 | C-factor -> agent selection (prefer agents that improve collective c-factor) | [both] | 🆕 | Route tasks to agents that fill gaps |
| J10 | C-factor metrics endpoint (`GET /api/metrics/c_factor`) | [roko] | 🆕 | Per plan 11 cybernetic metrics dashboard |
| J11 | C-factor time-series tracking (`.roko/learn/c-factor.jsonl`) | [roko] | 🆕 | Historical trend for self-improvement velocity |
| J12 | C-factor visualization in TUI (plan 09) | [roko] | 🆕 | Show collective intelligence trends |

**Reference**:
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/efficiency.rs`
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/baseline.rs`
- Existing code: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/task_metric.rs`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/agent-efficiency-event.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/baseline-computation.md`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/regression-detection.md`

**Verification**:
- [ ] C-factor composite score is computed after every plan run and logged to `.roko/learn/c-factor.jsonl`
- [ ] Per-agent c-factor contribution scores distinguish productive vs unproductive agents
- [ ] Per-fleet c-factor computed for multi-agent plan runs
- [ ] C-factor time-series tracking shows historical trend over 20+ plan runs
- [ ] TUI dashboard page renders c-factor trends with component breakdown

---

## R. Crate Architecture (Cognitive Crates Only)

### R1. roko-neuro (new crate)

**What exists**:
- `bardo-primitives::HdcVector` — HDC primitives (`crates/bardo-primitives/src/hdc.rs`)
- `roko-index` — parser + graph + HDC fingerprints (`crates/roko-index/src/`)
- `roko-learn` — episodes, playbooks, patterns (`crates/roko-learn/src/`)
- `roko-golem::grimoire::GrimoireEngine` — scaffold placeholder (`crates/roko-golem/src/grimoire.rs`)

**What's needed**:

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| R1 | Create `roko-neuro` crate (extract from roko-golem grimoire scaffold + roko-learn relevant parts) | [neuro] | 🆕 | Knowledge store, distillation, HDC retrieval |
| R1a | Move knowledge types (D7-D9) into roko-neuro | [neuro] | 🆕 | `KnowledgeKind`, `KnowledgeEntry`, JSONL store |
| R1b | Move distillation pipeline (D1-D6) into roko-neuro | [neuro] | 🆕 | Episode -> insight -> heuristic promotion |
| R1c | Move HDC knowledge integration (D14-D18) into roko-neuro | [neuro] | 🆕 | Wraps `bardo-primitives::HdcVector` for knowledge retrieval |
| R1d | Public API: `NeuroStore` (init, query, ingest, decay, gc) | [neuro] | 🆕 | Single entry point for all knowledge operations |
| R1e | Wire into orchestrate.rs (inject knowledge into agent context per task) | [cli] | 🆕 | Integration point |

**Reference**:
- Scaffold to replace: `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/src/grimoire.rs`
- HDC primitives: `/Users/will/dev/nunchi/roko/roko/crates/bardo-primitives/src/hdc.rs`
- Index crate: `/Users/will/dev/nunchi/roko/roko/crates/roko-index/src/`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/04-memory/01-grimoire.md`

**Verification**:
- [ ] `cargo build -p roko-neuro` succeeds with no golem dependency
- [ ] `roko plan run` uses `NeuroStore` to inject knowledge into agent prompts
- [ ] Unit tests pass for ingest -> query -> decay lifecycle

### R2. roko-daimon (new crate)

**What exists**:
- `roko-golem::daimon::DaimonEngine` — scaffold placeholder (`crates/roko-golem/src/daimon.rs`)
- `roko-learn::efficiency` — per-turn quality signals that feed appraisals (`crates/roko-learn/src/efficiency.rs`)

**What's needed**:

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| R2 | Extract Daimon to `roko-daimon` crate with trait interface | [roko] | 🆕 | `trait AffectEngine { fn appraise(&mut self, event) -> PadVector }` |
| R2a | Move affect model (F1-F9) into roko-daimon | [daimon] | 🆕 | `PadVector`, appraisal, decay, modulation, persistence |
| R2c | Public API: `DaimonState` (update, query, modulate, persist) | [daimon] | 🆕 | Single entry point for affect operations |
| R2d | Wire into orchestrate.rs (update affect after each gate result) | [cli] | 🆕 | Integration point |

**Reference**:
- Scaffold to replace: `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/src/daimon.rs`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/03-daimon/`

**Verification**:
- [ ] `cargo build -p roko-daimon` succeeds with no golem dependency
- [ ] `roko plan run` updates `DaimonState` after each task gate
- [ ] Affect state persists across `roko plan run` invocations

### R3. roko-dreams (new crate)

**What exists**:
- `roko-golem::dreams::DreamsEngine` — scaffold placeholder (`crates/roko-golem/src/dreams.rs`)
- `roko-learn::pattern_discovery` — trigram mining (`crates/roko-learn/src/pattern_discovery.rs`)
- `roko-learn::hdc_clustering` — K-medoids (`crates/roko-learn/src/hdc_clustering.rs`)
- `roko-learn::baseline` — baseline computation (`crates/roko-learn/src/baseline.rs`)
- `roko-learn::regression` — regression detection (`crates/roko-learn/src/regression.rs`)
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/dream-consolidation.md`

**What's needed**:

| # | Item | Where | Status | Notes |
|---|------|-------|--------|-------|
| R3 | Extract Dreams to `roko-dreams` crate with trait interface | [roko] | 🆕 | `trait DreamEngine { fn replay(&mut self, episodes) -> Vec<Insight> }` |
| R3a | Move dream replay + consolidation (G1-G8) into roko-dreams | [dreams] | 🆕 | Scheduler, replay, consolidation, counterfactual, strategy generation |
| R3c | Public API: `DreamRunner` (run, report, schedule) | [dreams] | 🆕 | Single entry point for dream operations |
| R3d | Wire `roko dream` CLI subcommand | [cli] | 🆕 | `roko dream run`, `roko dream report`, `roko dream schedule` |

**Reference**:
- Scaffold to replace: `/Users/will/dev/nunchi/roko/roko/crates/roko-golem/src/dreams.rs`
- PRD: `/Users/will/dev/nunchi/roko/bardo-backup/prd/05-dreams/`
- Component spec: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/roko-progress/COMPONENTS/learn/dream-consolidation.md`

**Verification**:
- [ ] `cargo build -p roko-dreams` succeeds
- [ ] `roko dream run` produces a dream report after processing episodes
- [ ] Dream report includes: clusters found, playbooks extracted, regressions detected, knowledge consolidated

---

## Dependency Graph (Implementation Order)

```
Layer 0 (Foundation — no external deps, enables everything):
  R1       Create roko-neuro crate
  D7-D9    Knowledge types + storage (core data model)
  D12-D13  HDC encoding + index (ALREADY BUILT, wire it)
  F1       PadVector struct

Layer 1 (Core cognitive — needs Layer 0):
  D1-D6    Distillation pipeline (episodes -> insights -> heuristics -> PLAYBOOK)
  D14-D18  HDC wiring (signals, episodes, knowledge retrieval)
  F2-F5    Daimon affect model + behavior modulation
  E1-E6    Context assembly pipeline (extends SystemPromptBuilder)
  R2       Create roko-daimon crate

Layer 2 (Integration — needs Layer 1):
  D10-D11  AntiKnowledge + query API
  F6-F9    Affect persistence + CascadeRouter bridge
  I1-I5    Operating frequencies (gamma/theta/delta loops)
  J1-J8    C-factor core metrics

Layer 3 (Offline learning — needs Layer 2):
  G1-G8    Dream replay + consolidation
  R3       Create roko-dreams crate

Layer 4 (Polish — needs Layer 3):
  J9-J12   C-factor endpoint, time-series, TUI
  R1e      Final wiring into orchestrate.rs
  R2d      Final wiring into orchestrate.rs
  R3d      Wire roko dream CLI subcommand
```

---

## Item Count

| Section | Total | New | Extend | Exists |
|---------|-------|-----|--------|--------|
| D. Knowledge & Memory | 18 | 15 | 1 | 2 |
| E. Context Assembly | 6 | 4 | 2 | 0 |
| F. Daimon (Affect) | 9 | 8 | 1 | 0 |
| G. Dreams | 8 | 8 | 0 | 0 |
| I. Operating Frequencies | 5 | 5 | 0 | 0 |
| J. C-Factor Metrics | 12 | 12 | 0 | 0 |
| R. Crate Architecture (R1-R3) | 14 | 14 | 0 | 0 |
| **TOTAL** | **72** | **66** | **4** | **2** |

Status legend:
- **New** = not yet built, needs implementation from scratch
- **Extend** = code exists but needs wiring, wrapping, or extension
- **Exists** = already built and available, just needs to be referenced/used
