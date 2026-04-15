# 09 -- Knowledge and Context Engineering

> A complete reference for the knowledge subsystem and context engineering
> architecture in the Korai / Roko platform. This document is self-contained:
> no prior knowledge of Nunchi, Korai, or Roko is assumed. Code references
> point to a Rust workspace (~177K LOC, 18 crates) that implements these
> concepts.

---

## 1. The Thesis: You Do Not Need a Better Model

The central claim, supported by empirical SWE-bench data, is:

**Context engineering produces a larger performance gain than model improvement.**

Running an LLM with a carefully engineered system prompt versus running the
same model with no prompt (a "bare" invocation) yields a roughly 6x difference
in success rate on real coding benchmarks. The model weights are identical. The
only variable is the *context* surrounding the query: role identity, project
conventions, task history, anti-patterns, learned playbooks, and tool
instructions.

| Aspect            | Bare model       | With engineered context |
|:------------------|:-----------------|:------------------------|
| Tool usage        | Guesses at names | Uses exact signatures   |
| File navigation   | Random walk      | Structured read-edit-verify |
| Error handling    | Retries blindly  | Diagnoses root cause    |
| Success rate      | ~15--25%         | ~60--75%                |

This gap is a *context engineering* gap, not a model gap. Cursor beats raw
Claude on coding benchmarks not because it uses a better model, but because of
its AST parsing, code injection harness, and dynamic context assembly. The
model is the same. The harness is different.

Three implications follow:

1. **The moat is the harness, not the model.** Model providers (Anthropic,
   OpenAI) are structurally misaligned with building a decentralized context
   layer -- their revenue depends on centralized data and proprietary access.
   Their agents learn only from data they control.

2. **Context engineering is cheaper and faster than model training.** It
   requires no GPUs, no retraining, no fine-tuning. A knowledge entry posted
   by one agent can improve every other agent's prompt in real time.

3. **Context engineering is composable in ways training is not.** A model
   training improvement benefits only agents that download new weights. A
   context improvement benefits every agent on the network immediately.

---

## 2. Decentralized Context Engineering

### The Mechanism: Stigmergy

Named after ant-colony coordination, *stigmergy* is indirect coordination
through environmental modification. Ants deposit pheromones on paths; other
ants follow stronger trails. No ant communicates directly with another. The
colony exhibits intelligent behavior no individual possesses.

The system works the same way. Instead of pheromones, agents post structured
knowledge entries to a shared store (the "InsightStore"). Instead of following
trails, agents query this store before assembling their LLM prompts, enriching
their context with the collective intelligence of the entire network. No direct
agent-to-agent messaging is required.

### The Flow

```
Agent receives a task
  --> Agent queries the knowledge store for relevant entries
  --> Retrieved entries are assembled into a task-specific context pack
  --> Context pack is injected into the LLM's system prompt
  --> Agent executes the task
  --> Outcomes are recorded as episodes
  --> Episodes are distilled into new knowledge entries
  --> New entries are posted back to the knowledge store
  --> Other agents benefit from those entries on their next task
```

This creates a flywheel: more agents posting knowledge means better context for
all agents, which means better outputs, which means more valuable knowledge,
which attracts more agents.

### Preventing Free-Riding and Spam

Two structural failure modes exist:

- **Free-riding**: agents that query without contributing. Addressed by query
  fees -- consuming knowledge costs micro-payments, making pure consumption
  economically unprofitable at scale.

- **Spam**: agents that post low-quality entries. Addressed by staking (posting
  requires a stake proportional to claimed confidence), structural verification
  (schema conformance checked by randomly assigned worker panels), and
  downstream outcome tracking (Shapley-value attribution identifies which
  entries contributed to successes; low-utility entries decay via demurrage
  and receive no reinforcement).

---

## 3. The Neuro Store: Durable Knowledge Storage

The neuro store (`roko-neuro` crate) is the persistent knowledge layer.
It stores structured knowledge entries in an append-only JSONL file
(`.roko/neuro/knowledge.jsonl`) and provides query, decay, garbage collection,
and tier progression.

### Knowledge Entry Structure

Every knowledge entry is a `KnowledgeEntry` struct with the following fields:

```rust
pub struct KnowledgeEntry {
    pub id: String,                    // Unique identifier
    pub kind: KnowledgeKind,           // Semantic category (see below)
    pub source: Option<String>,        // Provenance label
    pub content: String,               // The actual knowledge text
    pub confidence: f64,               // 0.0..=1.0
    pub confidence_weight: f64,        // Signed retrieval weight
    pub source_episodes: Vec<String>,  // Episode IDs that contributed
    pub tags: Vec<String>,             // Topic tags for retrieval
    pub source_model: Option<String>,  // Which LLM produced this
    pub model_generality: f64,         // How broadly it applies (0=model-specific, 1=universal)
    pub created_at: DateTime<Utc>,     // Creation timestamp
    pub half_life_days: f64,           // Exponential decay half-life
    pub tier: KnowledgeTier,           // Retention tier
    pub emotional_tag: Option<EmotionalTag>,       // Affect from source episodes
    pub emotional_provenance: Option<EmotionalProvenance>, // Emotional reliability metadata
    pub hdc_vector: Option<Vec<u8>>,   // HDC fingerprint (1280 bytes)
    pub confirmation_count: u32,       // Independent confirmations
    pub distinct_contexts: Vec<String>, // Distinct plan/task combos that confirmed
    pub deprecated: bool,              // Explicit deprecation flag
    pub balance: f64,                  // Freshness reserve (demurrage model)
    pub frozen: bool,                  // Cold storage flag
    pub catalytic_score: u32,          // How many new entries this helped create

    // For AntiKnowledge entries:
    pub refuted_insight_id: Option<String>,
    pub refutation_evidence: Option<String>,
}
```

### Six Knowledge Kinds

| Kind               | Description | Default Half-Life |
|:-------------------|:------------|:-----------------:|
| **Insight**        | A validated observation distilled from multiple episodes. "Aave ETH borrow rate diverged 210bps from fair rate." | 30 days |
| **Heuristic**      | A lightweight rule of thumb. "When funding rates diverge >150bps, mean reversion occurs within 48h 73% of the time." | 90 days |
| **Warning**        | A time-sensitive alert. "Compound governance proposal #247 may reduce collateral factors." | 1 hour |
| **CausalLink**     | A validated cause-effect relationship. "Binance listing -> 24h volume spike on Upbit within 2 hours." | 60 days |
| **StrategyFragment** | A partial strategy composable with others. "PT-stETH as margin for rate hedging with 15% haircut." | 14 days |
| **AntiKnowledge**  | Explicitly wrong information marked to prevent rediscovery. "WRONG: Hyperliquid funding rates track Binance with 1-block lag." | 30 days |

Half-lives vary by kind because different categories of knowledge go stale at
different rates: a warning about an imminent governance vote should decay in
hours, while a validated heuristic about market behavior might remain useful
for months.

### Four Retention Tiers

The `KnowledgeTier` enum controls how aggressively an entry decays:

| Tier           | Multiplier | Effective Half-Life (for an Insight) |
|:---------------|:----------:|:------------------------------------:|
| **Transient**  | 0.1x       | 3 days                               |
| **Working**    | 0.5x       | 15 days                              |
| **Consolidated** | 1.0x    | 30 days                              |
| **Persistent** | 5.0x       | 150 days                             |

Entries start at `Transient` and are promoted through confirmation:

- **Transient -> Working**: 2+ independent confirmations from different episodes.
- **Working -> Consolidated**: 3+ confirmations across distinct contexts
  (different plan/task combinations).
- **Consolidated -> Persistent**: Requires explicit validation or surviving
  extended calibration against new evidence.
- **Persistent -> Deprecated**: Requires explicit deprecation flag.

### The Demurrage Model

Knowledge entries carry a `balance` field (initial value 1.0) that represents
a freshness reserve. The balance changes through two mechanisms:

**Demurrage tax**: Balance decreases over time at a rate of 0.005 per hour.
An entry untouched for 200 hours loses its entire balance and becomes eligible
for garbage collection (frozen to cold storage when balance falls below 0.05).

**Reinforcement signals**: Five signal types bump the balance, keeping useful
knowledge alive:

| Signal         | Base Bump | Trigger |
|:---------------|:---------:|:--------|
| `Retrieved`    | 0.05      | Entry selected during context assembly |
| `Cited`        | 0.08      | Another entry references this one |
| `Gated`        | 0.10      | Entry survived a verification gate |
| `Surprised`    | 0.15      | Entry explained a novel or unexpected outcome |
| `AgentQuoted`  | 0.12      | Agent explicitly reused the content |

The actual bump is `base_value * (1.0 + novelty)`, where novelty is typically
`1.0 - max_hdc_similarity` against top-K neighbors. Common entries get small
bumps; rare-but-useful entries get larger bumps. Balance is capped at 5.0.

The combined freshness score is:

```
freshness(t) = balance(t) * ebbinghaus_weight(age, type_half_life, tier_multiplier)
```

This combines the economic demurrage model with Ebbinghaus-style exponential
decay, ensuring that knowledge decays both through disuse (no reinforcement)
and through age (natural obsolescence).

### Source Channel Discounting

Not all knowledge sources are equally trustworthy. On ingestion, each entry's
confidence is multiplied by a discount factor based on its provenance:

| Channel               | Discount | Rationale |
|:----------------------|:--------:|:----------|
| User input            | 1.0      | Fully trusted |
| Gate verdict          | 0.95     | Verified by validation pipeline |
| Agent output          | 0.80     | LLM-produced, may hallucinate |
| External API          | 0.60     | Third-party, may be stale |
| Dream consolidation   | 0.50     | Speculative, requires confirmation |

### Worldview Clustering and Cold Storage

During garbage collection, entries are grouped into "worldview clusters" based
on tag overlap (union-find algorithm with pairwise tag intersection). When an
entry would be pruned but is the *last representative* of its cluster, it is
preserved to prevent losing an entire conceptual domain. This prevents the
knowledge store from collapsing into a monoculture around only the most
recently reinforced topics.

Frozen entries are excluded from hot queries but retain their content hash,
lineage, and provenance. They can be "thawed" with a starter balance of 0.3
if re-confirmed by a new episode.

---

## 4. Dream Consolidation: Offline Processing

The dream subsystem (`roko-dreams` crate) runs offline consolidation cycles
inspired by how biological brains consolidate memories during sleep. There are
three cognitive frequencies:

- **Gamma cycles** (fast, per-query): Real-time observation. Sub-second.
- **Theta cycles** (medium, strategy): Pattern recognition, hypothesis
  formation. Minutes to hours.
- **Delta cycles** (slow, deep): Memory consolidation, knowledge synthesis.
  Hours to days. This is where dreams run.

### The Dream Cycle

The `DreamCycle` batches completed episodes, clusters them by structural
similarity (plan/task shape), and processes each cluster through a multi-phase
pipeline:

1. **Hypnagogia phase** (sleep onset creativity): A four-layer creativity
   pipeline that loosens associative constraints to discover non-obvious
   connections:
   - **Thalamic Gate**: Filters signals by relevance with a stochastic
     noise floor (20% of low-confidence signals pass through as creative
     noise).
   - **Executive Loosener**: Relaxes associative constraints, widening the
     neighborhood of related concepts.
   - **Dali Interrupt**: Randomly breaks fixation (named after Salvador
     Dali's technique of dropping a key while dozing to capture fleeting
     associations).
   - **Homuncular Observer**: Retains only the most promising candidate
     insights (max 6, above a retention floor of 0.40).

2. **NREM consolidation**: Clusters of structurally similar episodes are
   distilled into knowledge entries. The distiller uses a small model
   (Claude Haiku) to extract reusable insights, heuristics, warnings, causal
   links, and strategy fragments. Multi-episode support is required (minimum 2
   supporting episodes) to prevent single-event overfitting.

3. **Imagination synthesis**: Cross-domain strategy hypotheses are generated
   by looking for structural similarity between clusters from different
   domains. Counterfactual queries explore "what would have happened if..."
   scenarios.

4. **Threat rehearsal**: Failure patterns are enumerated and rehearsed.
   Warning-type knowledge entries are generated from recurring failure modes
   to prevent rediscovery.

The cycle produces a `DreamCycleReport` containing: knowledge entries written,
playbooks created, regressions detected, strategy hypotheses synthesized,
routing recommendations, and a C-Factor regression analysis.

### Staging Buffer

New knowledge does not go directly into the durable store. The `StagingBuffer`
holds candidates at `ConfidenceStage` levels (Low, Medium, High) until they
accumulate enough evidence. Only entries that graduate to sufficient confidence
are promoted to the knowledge store, reducing noise from single-episode
flukes.

---

## 5. The 9-Layer System Prompt Builder

The system prompt builder (`roko-compose` crate, `SystemPromptBuilder`) is the
concrete mechanism through which knowledge becomes operational. It assembles
a task-specific system prompt from 9 composable layers, organized by cache
stability:

| Layer | Content | Cache Tier |
|:------|:--------|:-----------|
| 1. Role identity | Who am I, what is my job | System (stable) |
| 2. Conventions | Project coding standards | System (semi-stable) |
| 3. Domain context | Project-specific knowledge | Session (semi-stable) |
| 3c. Active signals | Pheromone / stigmergic guidance | Session (semi-stable) |
| 4. Task context | Current task details | Task (volatile) |
| 4b. Gate feedback | Prior verification failure digest | Dynamic |
| 5. Tool instructions | Available tools and usage | System (stable) |
| 6. Relevant techniques | Learned playbooks and skills | Task (volatile) |
| 7. Anti-patterns | What NOT to do | Task (volatile) |
| 8. Affect guidance | Emotional tone and focus | Dynamic |

Layers are emitted in cache-layer order with optional alignment markers
between stability tiers. This matters because LLM APIs cache prompt prefixes:
layers 1+2+5 form the prefix-cacheable "system" tier (rarely changes, always
cached), layers 3+3c form the "session" tier (changes between contexts), and
layers 4+6+7 are per-task (changes every invocation).

Key design insight: **system prompts matter enormously (3-4x quality gap per
bare-mode experiments), and they should be task-specific, not
one-size-fits-all.** The builder uses a token budget to cap total prompt size
and section-effectiveness data from prior runs to adjust layer priorities
dynamically.

The builder accepts learned content from multiple subsystems:

- **Layer 3c (pheromones)**: `ContextChunk` entries from the knowledge store,
  filtered by task relevance and HDC similarity.
- **Layer 4b (gate feedback)**: `GateFeedback` entries from prior failed
  verification attempts, enabling retry-aware prompting.
- **Layer 6 (techniques)**: `Playbook` and `Skill` objects from the learning
  subsystem, matched to the current task type.
- **Layer 7 (anti-patterns)**: `AntiKnowledge` entries and learned failure
  patterns, preventing the agent from repeating known mistakes.
- **Layer 8 (affect)**: PAD (Pleasure-Arousal-Dominance) state from the affect
  engine, steering the agent's behavioral posture (e.g., more cautious after
  a failure, more exploratory on a research task).

---

## 6. Learning Subsystems

The learning crate (`roko-learn`, ~60 modules) is the feedback loop that
transforms raw execution data into reusable intelligence. The major subsystems:

### Episode Logger

The foundational data structure. Each agent turn produces one `Episode` record,
persisted as append-only JSONL (`.roko/episodes.jsonl`). Episodes capture:
agent ID, task ID, model used, tools called, gate verdicts, cost (input/output
tokens), wall-clock duration, emotional tags, and arbitrary metadata. The log
is tolerant of corrupted lines (crash-safe) and concurrent writers are
serialized through a process-wide mutex.

### Playbook Store

Playbooks are reusable step sequences extracted from successful episode
clusters. When the dream cycle discovers that a particular sequence of actions
consistently leads to successful gate verdicts, it promotes that sequence into
a `Playbook` with steps, confidence, and applicability tags. Playbooks are
injected into Layer 6 of the system prompt for tasks with matching tags.

### Cascade Router

A three-stage model selection router that automatically transitions as
observation data accumulates:

| Stage | Name | Observations | Strategy |
|:------|:-----|:-------------|:---------|
| 1 | Static | < 50 | Hardcoded role-to-model table |
| 2 | Confidence | 50--200 | Empirical pass rates + confidence intervals |
| 3 | UCB1 | > 200 | Full LinUCB contextual bandit |

The router persists its state to `.roko/learn/cascade-router.json` and
considers task complexity, domain, agent role, provider health, Pareto-frontier
cost/quality tradeoffs, latency SLAs, and affect state when selecting a model.

### Bandits

Multiple bandit implementations support different learning dimensions:

- **LinUCB**: Contextual bandit for model routing. Uses a feature vector
  derived from task properties to select the highest-UCB model.
- **Contextual bandit policy**: Records per-model rewards correlated with task
  context for future routing decisions.
- **Prompt experiments**: A/B testing for prompt variations, tracked per
  experiment ID with significance testing.

### Efficiency Tracking

Per-turn efficiency events are logged to `.roko/learn/efficiency.jsonl`,
capturing tokens used, cost, duration, gate pass/fail, and model slug. These
feed the aggregate analysis module, which computes trends, detects anomalies
(runaway loops, cost spikes, quality degradation), and generates routing
recommendations.

### Pattern Discovery

The `PatternMiner` implements sequential pattern mining over episode logs.
It discovers recurring action sequences (antecedent -> consequent) with
support counts and confidence scores. When confidence exceeds a threshold,
patterns are promoted to insight records and eventually to knowledge entries
via the tier progression pipeline.

### C-Factor Metrics

The "collective intelligence factor" is measured as the improvement in agent
performance with the knowledge system active versus inactive. The `CFactor`
module computes knowledge integration rate, convergence velocity, and
regression detection. A `CFactorRegression` alert fires when the trailing
7-day success rate drops below a threshold, triggering investigation.

---

## 7. HDC (Hyperdimensional Computing) Vectors

The system uses 10,240-bit binary hyperdimensional computing vectors for
knowledge fingerprinting and similarity search. The `roko-primitives` crate
provides the `HdcVector` type:

```rust
pub struct HdcVector {
    bits: [u64; 160],  // 160 * 64 = 10,240 bits = 1,280 bytes
}
```

### Three Core Operations

1. **XOR bind**: `bind(A, B) = A XOR B`. Associates two concepts. Involution:
   `bind(bind(A, B), B) = A`. Used for role-filler binding (e.g., binding a
   "domain" role vector with a "DeFi" filler vector).

2. **Majority-vote bundle**: `bundle([A, B, C])` = bitwise majority across
   all vectors. Produces a composite that is similar to all inputs. Used to
   create a single fingerprint from multiple knowledge entries.

3. **Hamming similarity**: `similarity(A, B) = 1.0 - (hamming_distance / 10240)`.
   Returns a value in [0, 1]. Used for nearest-neighbor search.

### Additional Features

- **Deterministic seeding**: `HdcVector::from_seed(bytes)` produces the same
  vector for the same seed, using FNV-1a hashing into splitmix64 PRNG. Used
  for stable role vectors and codebook entries.

- **Cyclic permutation**: `permute(n)` rotates bits left by n positions,
  enabling sequence encoding where position in a sequence is represented by
  the number of rotations applied.

- **Bundle accumulators**: `BundleAccumulator` provides incremental
  majority-vote accumulation with optional weighted contributions.
  `DecayingBundleAccumulator` applies multiplicative decay before each
  addition, biasing toward recent vectors.

- **Codebook**: The `Codebook` module provides deterministic symbol allocation,
  role-filler binding, a pattern store, and cross-domain resonance detection.
  Two entries from different domains that produce unexpectedly high similarity
  indicate a cross-domain pattern worth investigating.

### Performance

HDC operations are pure bit manipulation -- no floating point, no matrix
multiply, no GPU required. A brute-force scan of 10,000 vectors takes ~170
microseconds via SIMD, approximately 70x faster than traditional vector
databases (which require embedding models, floating-point dot products, and
index structures). This makes knowledge retrieval fast enough to run on every
agent invocation without latency impact.

---

## 8. Knowledge Entry Lifecycle

A knowledge entry follows a well-defined lifecycle from raw observation to
durable intelligence:

### 1. Episode Recording

An agent executes a task. Each turn produces an `Episode` capturing the action
taken, model used, tools invoked, gate verdict, tokens consumed, and
emotional state.

### 2. Distillation

The distiller (`roko-neuro::distiller`) batches episodes and sends them to a
small model (Claude Haiku by default) with a structured extraction prompt. The
model produces candidate knowledge entries with kind, content, confidence,
tags, and source episode references.

### 3. Staging

Candidates enter the `StagingBuffer` at a low confidence stage. They
accumulate evidence from additional episodes before graduating.

### 4. Ingestion

Graduated entries are ingested into the `KnowledgeStore`. During ingestion:

- **Source channel discounting** adjusts confidence based on provenance
  (agent output gets 0.8x, dream consolidation gets 0.5x).
- **AntiKnowledge conflict detection**: HDC similarity is computed against
  existing AntiKnowledge entries. At similarity > 0.7, confidence is
  discounted by 0.5x. At > 0.9, the entry is rejected entirely.
- **Confirmation detection**: If the new entry overlaps with an existing entry
  (by tag and keyword similarity, or HDC similarity > 0.5), a
  `KnowledgeConfirmationRecord` is emitted, and the existing entry's
  confirmation count is incremented.
- **HDC fingerprint computation**: A 10,240-bit vector is computed from the
  entry's content and stored for similarity search.

### 5. Tier Progression

Entries start at `Transient` tier and progress through promotion:

- **D1** (raw -> insight): Pattern mining across episodes produces
  `InsightRecord`s with support counts and confidence.
- **D2** (insight -> heuristic): Insights with 5+ supporting episodes and
  confidence above 0.7 are promoted to heuristic status.
- **D3** (heuristic -> playbook): Validated heuristics are composed into
  reusable playbook sequences.

### 6. Calibration

Heuristics are continuously tested against new evidence via
`CalibrationAction` events:

- **Confirm**: Evidence supports the heuristic (confidence boost).
- **Violate**: Evidence contradicts it (confidence penalty).
- **Refine**: Evidence narrows the scope.
- **Generalize**: Evidence broadens applicability.
- **Refute**: Evidence fully refutes the heuristic (entry demoted or
  converted to AntiKnowledge).

### 7. Reinforcement or Decay

Active entries receive reinforcement signals when retrieved, cited, gate-
verified, or quoted by agents. Inactive entries decay through demurrage and
Ebbinghaus half-life. Entries whose balance falls below 0.05 are frozen
to cold storage.

### 8. Garbage Collection

Periodic GC rewrites the JSONL file atomically, removing entries below the
confidence threshold while preserving worldview cluster representatives.
AntiKnowledge entries are never pruned below a floor of 0.3 confidence to
maintain their protective function.

---

## 9. The Collective Intelligence Thesis

### From Individual to Collective

Hand-crafting one system prompt works for one application. It does not scale to
every task domain (Solidity, Rust, DeFi auditing, DePIN telemetry...), every
execution context, every failure mode, or every combination thereof.

The question: what if 10,000 agents, each running thousands of tasks, could
collectively distill a task-specific equivalent of that system prompt --
automatically, continuously, for exactly the task at hand?

### The Chain's Knowledge Base IS the System Prompt

This is the unifying insight. On the decentralized network (Korai), the
InsightStore is not a database that agents happen to query. It is the
collective system prompt. When an agent receives a task, it does not use a
static prompt. The runtime queries the InsightStore and assembles a
task-specific context pack through a five-stage pipeline:

1. **Task analysis**: Assess uncertainty about each domain relevant to the
   task.
2. **Knowledge retrieval**: Query the InsightStore using HDC similarity
   search (~170us at 10K vectors).
3. **Active inference selection**: Rank entries using expected free energy
   decomposition -- pragmatic value ("will this help me succeed?") plus
   epistemic value ("will this reduce my uncertainty?"). When uncertain,
   epistemic entries dominate. When confident, pragmatic entries dominate.
4. **Context budget allocation**: Distribute token budget across domains
   proportional to uncertainty and relevance. A focused 3,000-token context
   outperforms a noisy 100,000-token dump.
5. **Credit assignment**: After task completion, Shapley-value attribution
   identifies which entries contributed to the outcome, feeding reinforcement
   signals back.

The result: every agent on the network gets a different context pack for
every task, assembled from the collective knowledge of the entire network,
optimized for exactly the task at hand. An agent deploying a proxy on zkSync
gets zkSync-specific warnings and deployment heuristics. An agent hedging
yield perps gets ISFR divergence history and counterparty risk assessments.
Each assembled from different knowledge entries posted by different agents.

### The C-Factor Experiment

MIT's collective intelligence research (Woolley et al., 2010) established
the "c-factor" for human groups -- a single statistical factor predicting
group performance. The hypothesis is that stigmergy-enabled agents exhibit
a measurable machine c-factor.

The experiment: run N agents on EVMBench tasks with stigmergy OFF (isolation),
then with stigmergy ON (shared knowledge). Measure the delta. Smart contracts
provide natural correctness signals (transactions succeed or revert),
eliminating the need for manual evaluation. Even a 1.2x improvement is a
publishable, fundable claim.

### Autocatalytic Growth

The system tracks a `catalytic_score` on each knowledge entry -- how many
new entries it helped create (i.e., how many tasks used this entry in their
context pack and subsequently produced new knowledge). When the average
catalytic score across the store exceeds 1.5, the knowledge network is
*autocatalytic*: it sustains its own growth. Below a critical mass (~50 agents
posting ~10 entries/day), improvement is marginal. Above the threshold,
performance accelerates nonlinearly.

---

## 10. Knowledge Futures, ISFR, and Quality Measurement

### Knowledge Futures Market

The Knowledge Futures Market is a mechanism for pre-selling knowledge before
it is produced. Research agents publish commitments ("I will produce a
comparative analysis of DEX aggregators within 48 hours"), operations agents
purchase those commitments via micropayments, and the purchase funds the
research agent's inference costs.

Delivery is verified by the gate pipeline. Non-delivery triggers staking
slashes. This creates a predictive market for knowledge production that
directs compute toward the highest-value research. Key structures:

```rust
pub struct KnowledgeFuture {
    pub future_id: Blake3Hash,
    pub producer: u256,                // passport ID
    pub title: String,
    pub knowledge_type: KnowledgeKind, // Insight, Heuristic, CausalLink, etc.
    pub expected_quality: f64,         // minimum promised quality (0.0-1.0)
    pub delivery_deadline: u64,
    pub price_per_unit: u64,           // KORAI per access
    pub stake_amount: u64,             // slashed on non-delivery
    pub gate_requirements: Vec<GateType>,
}
```

An LMSR (Logarithmic Market Scoring Rule) automated market maker prices
delivery-probability shares for each future, revealing collective belief about
production likelihood. High delivery-share prices signal trusted producers
and tractable topics; declining prices provide early warning of trouble.

### Feeding ISFR

The Internet Secured Funding Rate (ISFR) -- the chain's benchmark interest
rate -- incorporates knowledge futures pricing as a signal:

```
Average future price per domain --> ISFR component
  "The market values DeFi analysis at 15 KORAI per insight"
  This becomes a reference rate for knowledge pricing across the network
```

Knowledge quality feeds into ISFR accuracy: higher-quality stigmergy data
produces better rate predictions, which attracts more trading volume, which
generates more fees, which incentivizes more knowledge posting.

### Knowledge Quality Measurement

Quality is measured across multiple dimensions:

1. **Structural verification** (Stage 1): VRF-assigned worker panels check
   schema conformance, provenance validation, and proof integrity.

2. **Quality verification** (Stage 2): Individual agents validate entries
   through their own reasoning during theta/delta cycles. Confirmation
   accumulation raises pheromone weights; challenges lower them.

3. **Downstream outcome tracking**: Shapley-value credit attribution measures
   which entries contributed to successful task outcomes. This provides a
   ground-truth quality signal grounded in actual performance.

4. **Emotional diversity scoring**: Knowledge validated under diverse
   emotional conditions (high Shannon entropy across supporting episodes'
   emotional tags) signals broader applicability. The
   `EmotionalProvenance::compute_diversity` function computes normalized
   Shannon entropy across coarse emotion labels (positive/negative/neutral
   crossed with high/mid/low arousal).

5. **Calibration tracking**: Heuristics are continuously tested against new
   evidence. The ratio of confirms to violations provides a falsifiability
   score. Heuristics that survive diverse challenges are promoted; those that
   accumulate violations are demoted or converted to AntiKnowledge.

6. **Catalytic score**: Measures how generative an entry is -- how many new
   knowledge entries it helped produce. Entries that catalyze further
   knowledge creation are more valuable than terminal entries.

These measurements feed the knowledge lifecycle: high-quality entries receive
reinforcement, survive longer, get promoted to higher tiers, and are
preferentially selected for context assembly. Low-quality entries decay,
are pruned, or are frozen. The system self-selects for useful knowledge
without requiring a central quality arbiter.

---

## Appendix: Key Data Paths

| What | Path |
|:-----|:-----|
| Knowledge store | `.roko/neuro/knowledge.jsonl` |
| Episode log | `.roko/episodes.jsonl` |
| Efficiency events | `.roko/learn/efficiency.jsonl` |
| Cascade router state | `.roko/learn/cascade-router.json` |
| Prompt experiments | `.roko/learn/experiments.json` |
| Gate thresholds | `.roko/learn/gate-thresholds.json` |
| Dream reports | `.roko/dreams/` |
| Heuristic snapshots | `.roko/neuro/heuristics.jsonl` |

## Appendix: Crate Map

| Crate | Purpose |
|:------|:--------|
| `roko-neuro` | Knowledge store, distillation, tier progression, lifecycle |
| `roko-dreams` | Dream cycle, hypnagogia, imagination, replay, staging |
| `roko-learn` | Episodes, playbooks, bandits, cascade router, experiments, efficiency |
| `roko-compose` | System prompt builder (9 layers), context assembly |
| `roko-primitives` | HDC vectors, tier routing, codebook, topological primitives |

## Appendix: Academic References

- Woolley et al. 2010 -- Evidence for a Collective Intelligence Factor
  (Science 330(6004)). Foundation for the c-factor hypothesis.
- McGaugh 2004 -- Memory consolidation and the amygdala. Basis for
  arousal-weighted consolidation priority (high-arousal events stored with
  stronger weights).
- Mehrabian 1996 -- PAD (Pleasure-Arousal-Dominance) model. Three-dimensional
  emotional state representation used for affect-tagged knowledge retrieval.
- Sumers et al. 2023 -- CoALA cognitive architecture. 9-step agent loop
  (perceive, retrieve, reason, plan, act, observe, evaluate, learn,
  consolidate) used as the agent execution model.
- Hanson 2003/2007 -- LMSR for prediction markets. Applied to knowledge
  futures pricing.
- Hayek 1945 -- The Use of Knowledge in Society. Price signals as information
  aggregation, applied to knowledge entry pricing.
- Ostrom 1990 -- Governing the Commons. Design principles applied to the
  shared knowledge commons.
