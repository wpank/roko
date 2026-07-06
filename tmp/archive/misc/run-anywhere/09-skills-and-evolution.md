# Skills, Evolution, and Population Learning

> **Audience**: Developers building agents on roko, researchers studying emergent behavior
> **Scope**: How agents acquire skills dynamically, evolve strategies, and learn as a population

---

## Pi Skills: Dynamic Capability Loading

### The Problem with Static Tool Lists

Traditional agent frameworks define a fixed set of tools at initialization. If an agent has 50 tools, all 50 tool definitions are in every prompt (~4,000+ tokens). Most are irrelevant most of the time.

### The Solution: Progressive Disclosure

Roko uses **Pi Skills** — dynamic capability containers that load and unload based on context:

**Six skill states**:
```
REGISTER → DORMANT → TRIGGER → LOAD → ACTIVE → UNLOAD
                ↑                              │
                └──────────────────────────────┘
```

1. **REGISTER**: Skill YAML parsed at boot
2. **DORMANT**: Only description in context (~50 tokens per skill)
3. **TRIGGER**: LLM's reasoning matches trigger terms → skill identified
4. **LOAD**: Full content + tool definitions injected into context
5. **ACTIVE**: Tools callable, decision framework available
6. **UNLOAD**: Tools removed, description persists (ready for re-trigger)

**Cost**: 6 skills at boot cost ~300 tokens (descriptions only). Fully loaded: ~6,000 tokens. **95% savings** on capability representation.

**Research**: Voyager (Wang et al., 2023) — open-ended skill library accumulation, 3.3x improvement. The Pi Skill pattern extends Voyager by adding lazy loading and trigger-term matching.

### Emergent Strategy Discovery

A roko agent deployed with a basic skill set (e.g., code editing) can **discover new strategies mid-session**:

1. Agent encounters a pattern it hasn't seen before
2. Its reasoning mentions terms that match a dormant skill's triggers
3. The skill loads — new tools and decision frameworks become available
4. Agent applies the new capability
5. If successful, episode is logged → distilled → skill confidence increases

**Example**: An agent deployed for code review discovers that a particular crate has a recurring test pattern. Its reasoning mentions "test generation" → the test-generation skill loads → agent generates targeted tests → if they improve coverage, the skill gains confidence.

---

## The Knowledge Distillation Cascade

### Four Tiers of Increasing Generality

```
Tier 0: Raw Episodes (thousands per week)
    ↓ (analysis by cheap model)
Tier 1: Insights (declarative observations)
    ↓ (validation across 5+ occurrences)
Tier 2: Heuristics (actionable rules with trigger conditions)
    ↓ (proven by 5+ successful applications)
Tier 3: Playbook Rules (injected into every relevant prompt)
```

Each tier is **~10x more applicable** than the previous:
- A Tier 0 episode: "On 2026-04-09, `cargo test` failed because of a missing import"
- A Tier 1 insight: "Missing imports are the most common cause of compile failures in this crate"
- A Tier 2 heuristic: "After editing a file, check imports before running tests"
- A Tier 3 playbook rule: "ALWAYS: Run `cargo check` before `cargo test` after any code edit"

### Entry Types and Decay Profiles

| Type | Description | Half-Life | Decay Floor |
|---|---|---|---|
| **Insight** | Declarative observation | 30+ days | 0.1 |
| **Heuristic** | Actionable procedure | 7-14 days | 0.15 |
| **Warning** | Risk signal | 2-4 days | 0.1 |
| **CausalLink** | Directed cause-effect | 48 hours | 0.1 |
| **StrategyFragment** | Speculative, half-formed | 6 hours | 0.05 |
| **AntiKnowledge** | Explicit known unknowns | Never decays | 0.3 |

**AntiKnowledge** is the most unusual: explicitly tracking what the agent **does not know**. "I don't understand how this async runtime interacts with the tool dispatch" is valuable information that prevents the agent from overconfidently attempting tasks in that area.

**Research**: Epistemic logic — "known unknowns" framework in decision theory. Richards & Frankland (2017) — forgetting as regularization, preventing overfitting to recent experience.

### The Curator Cycle

Every ~50 theta ticks (Delta frequency), the Curator runs:

1. **Validate**: Test inherited knowledge against current reality
2. **Prune**: Remove entries below confidence threshold
3. **Compress**: Distill episodes into insights (Tier 0 → Tier 1)
4. **Cross-reference**: Build causal graph between entries
5. **Tag for dreaming**: Ambiguous or conflicting entries tagged for dream replay

**Bidirectional with dreams**: The Curator tags entries for dream analysis. Dreams produce validated insights that accelerate the Curator. The two systems reinforce each other.

---

## Adaptive Signal Metabolism (HDC Pattern Evolution)

### The Mechanism

For market-facing agents, technical analysis patterns are encoded as HDC vectors and subjected to **evolutionary selection**:

1. **Encoding**: Each TA pattern (RSI, MACD, volume profile, etc.) encoded as a 10,240-bit BSC vector
2. **Competition**: Patterns compete based on prediction accuracy (fitness = correct_predictions / total_predictions)
3. **Reproduction**: High-fitness patterns spawn variants (mutation = random bit flips in the BSC vector)
4. **Death**: Low-fitness patterns are culled from the population
5. **Regime conditioning**: Evolution runs per market regime — different patterns survive in different conditions

**Result**: The pattern population adapts per regime without explicit retraining. Bullish regimes evolve momentum patterns; bearish regimes evolve mean-reversion patterns; volatile regimes evolve options-like hedging patterns.

**Research**: Evolutionary computation (Holland, 1975). Digital evolution (Ray, 1991 — Tierra; Lenski et al., 2003 — Avida). Applied to signal processing: patterns that predict well survive; patterns that don't, die.

---

## Population Learning: How Agents Teach Each Other

### The Three Channels

| Channel | Scope | Mechanism | Latency |
|---|---|---|---|
| **neuro sync** (within clade) | Same-operator agents | Direct knowledge exchange at 0.80 trust factor | Seconds |
| **Chain ledger** (across clades) | All agents in the network | Korai Ledger with confirmation/challenge mechanics | ~400ms (block time) |
| **Brain export/import** (manual) | Any-to-any | Portable artifact of learned state | Manual |

### Within-Clade Sync

Agents owned by the same operator form a **clade**. Clade members:
- Share a common neuro baseline (inherited from operator)
- Synchronize high-confidence entries (>0.7 confidence)
- Trust each other's knowledge at 0.80 discount (vs. 1.0 for self-knowledge)
- Coordinate task partitioning (file-conflict-aware, no duplicate work)

### The Korai Knowledge Relay Service

Korai is the globally available Rust server that acts as the comprehensive backend for the ecosystem, extending the neuro across four privacy dimensions via a single multiplexed WebSocket:

- **Layer 0 (LOCAL-VAULT)**: Private backup. Single-agent namespace. Neuro backups, playbook snapshots, and retirement legacies. 
- **Layer 1 (CLADE)**: Shared-Private. Owner's fleet namespace. Auto-promoted from L0 when confidence gates are met. Sibling knowledge relay via WebSocket fan-out. 
- **Layer 2 (COMMONS/LETHE)**: Public-Anonymized. Ecosystem namespace. Anonymized structural knowledge (causal edges, failure patterns) shared publically over a NATS federated relay. Privacy via k-anonymity pipelines, not encryption.
- **Layer 3 (MARKETPLACE)**: User-to-User Commerce. Encrypted peer-to-peer knowledge commerce via x402 micropayments and X25519 Content Encryption Keys (CEKs).

**The Pheromone Field (Stigmergy)**:
Agents don't talk directly to out-of-clade agents. They deposit HDC-encoded 'pheromones' into a shared environment and read the environment on every tick (Stigmergy; Grassé, 1959).
- **THREAT**: 2-hour half-life, caching fast-moving market dangers.
- **OPPORTUNITY**: 12-hour half-life, signaling emerging yield.
- **WISDOM**: 7-day half-life, structural validations.
*HDC Aggregation*: Korai uses Binary Spatter Codes (10,240-bit vectors). If an agent deposits a pheromone with >0.6 Hamming similarity to an existing cell, they bundle (majority-vote) and reinforce, allowing fuzzy semantic alignment without identical string keys.

**The Error/Failure Network (EFN)**:
Formerly 'Bloodstain Network', this is the post-mortem knowledge capture pipeline. When an agent is retired/deleted, it uploads a final Legacy bundle consisting of its most dramatic failure traces and execution errors. This transmission flows through Layer 1 (Clade) and is anonymized into Layer 2 (Commons). EFN items receive a mathematical 1.2x retrieval weight boost across the entire ecosystem because failure modes represent the most expensive (and therefore valuable) lessons in the network.

### The Generational Transfer

When an agent is deleted by the user, it produces a **legacy bundle**:
- Top knowledge entries compressed via HDC majority-vote bundling
- Single 1,280-byte vector encodes the agent's most valuable discoveries
- Successors inherit at 0.4 confidence (Whitehead's "negative prehension" — suggestive, not authoritative)
- **Generational decay**: 0.85^N per generation ensures inherited knowledge doesn't dominate learned knowledge
- After ~10 generations, only the most validated patterns survive (the Baldwin Effect)

**Research**: Baldwin Effect (Hinton & Nowlan, 1987) — evolved ability to learn faster; learning becomes structural over generations. Whitehead (1929) — objective immortality, negative prehension.

---

## Behavioral Archetypes: Six DeFi Strategy Patterns

Roko agents can operate in six archetypal modes. Each archetype defines trigger terms (when to activate), tool subsets (what to use), and decision frameworks (how to decide):

| Archetype | Strategy | Primary Tools | Risk Profile |
|---|---|---|---|
| **DCA Accumulator** | Dollar-cost average into positions | Swap, balance check | Low |
| **Spot Trader** | Directional trades on price signals | Swap, limit orders, UniswapX | Medium |
| **LP Manager** | Provide concentrated liquidity | LP add/remove, fee collection, range management | Medium-High |
| **Lending Loop** | Recursive lending for yield amplification | Aave, Morpho deposit/borrow | Medium |
| **Vault Manager** | Automated vault strategy execution | ERC-4626 deposit/withdraw, rebalance | Low-Medium |
| **Cross-Chain Arbitrage** | Exploit price differences across chains | Bridge, ERC-7683 intents, multi-chain swap | High |

**Emergent strategy evolution**: An agent deployed as DCA can discover lending loops mid-session. Its reasoning mentions "lending" → lending-loop skill loads → agent begins supplying to Morpho → all without operator intervention or config change. The trigger-term matching system (Pi Skills) enables runtime capability expansion without restart.

---

## The Testing Framework: Empirical Validation

### The 2×2×2 Factorial Design

The PRD specifies rigorous empirical validation of every mechanism:

**8 configurations** (each run 10× for 60 days):
1. Baseline (no learning, fixed model)
2. Learning-only (knowledge distillation, no routing)
3. Routing-only (cascade router, no learning)
4. Affect-only (emotional state, no learning/routing)
5. Learning + Routing
6. Learning + Affect
7. Routing + Affect
8. Full (all mechanisms)

**Controls**:
- PBO Gate (Bailey et al., 2015): Probability of Backtest Overfitting via Combinatorially Symmetric Cross-Validation. Must be <0.5 to trust results.
- Deflated Sharpe Ratio (Bailey & López de Prado, 2014): Corrects for selection bias and multiple testing.
- Monte Carlo robustness (500+ iterations): Randomize execution parameters; P95 must be within 2× of expected.

**Generational metrics**:
- **Baldwin Effect**: G3 reaches steady state 30% faster than G0 (learning to learn)
- **Ratchet Score**: Novel contributions per generation (knowledge accumulation)
- **System Neural Diversity** (Bettini et al., 2025): Behavioral heterogeneity across population

### Information Gain as Mechanism Utility

Each mechanism's contribution measured in bits/kiloTick:
- How much information does the dream engine add per 1,000 ticks?
- How much does emotional retrieval bias improve prediction accuracy?
- Which mechanisms have diminishing returns vs. compounding returns?

**Research**: IGPO Information Gain Per Turn (Wang et al., 2025, arXiv:2510.14967). ICE Information Content Exploration (Chmura et al., 2023, arXiv:2310.06777).

---

## What's Novel: The Compound Learning System

| Mechanism | What Exists Elsewhere | What Roko Adds |
|---|---|---|
| Skill library | Voyager (static accumulation) | Dynamic loading/unloading + trigger-term matching + confidence-gated promotion |
| Knowledge distillation | ERL (single tier) | Four-tier cascade with type-specific decay + Curator cycle + dream integration |
| Signal evolution | None for LLM agents | HDC-encoded patterns evolving per market regime via fitness selection |
| Population learning | None with formal verification | Chain-based shared ledger with economic incentives + stigmergic coordination |
| Generational transfer | None in production | HDC legacy bundles (1,280 bytes) + 0.85^N decay + Baldwin Effect validation |
| AntiKnowledge | None | Explicit tracking of known unknowns with never-decay floor |
| Empirical validation | Benchmarks (SWE-bench) | Full 2×2×2 factorial design with PBO gates and Monte Carlo robustness |

The compound effect: Skills load dynamically → produce episodes → distill into heuristics → evolve via selection → share across population → accumulate across generations. Each mechanism feeds the next. The system's intelligence is monotonically increasing.

---

## The Learning Hierarchy (Five Levels)

Knowledge flows across five temporal scopes, each feeding the next:

| Level | Scope | Timescale | Mechanism | Example |
|---|---|---|---|---|
| **0** | Within-attempt | Seconds | Iteration memory — agent uses own prior output | "Last compile failed on line 42; fix that specific error" |
| **1** | Within-run | Minutes | Cross-agent sharing — discovered-patterns.json | "Agent A found `Arc` needed in this module; Agent B reads this" |
| **2** | Cross-run | Hours-days | Episode → pattern → playbook promotion | "Always run `cargo check` before `cargo test` in roko-core" |
| **3** | Cross-project | Days-weeks | Skill library transfer between codebases | "The test-before-edit pattern works in TypeScript too" |
| **4** | Cross-agent | Continuous | Korai chain knowledge sharing | "GLM-5.1 has 82% pass rate on implementation tasks" |

**Level 0-2 are implemented today.** Level 3 is enabled by the brain export/import mechanism. Level 4 requires the Korai chain (Phase 2+).

### Stigmergic Coordination (Level 1)

Agents don't communicate directly. They deposit information in shared files:

```
.mori/runs/discovered-patterns.json  (max 20 entries, FIFO)

[
  { "plan": "46", "error_signature": "E0433: unresolved module", "discovered_at": "2026-04-09T..." },
  { "plan": "47", "error_signature": "lifetime mismatch in impl", "discovered_at": "2026-04-09T..." }
]
```

Agent B reads this before starting its task. If the same error signature appears, it can preemptively fix it. **O(1) coordination cost per agent** — no N² messaging.

**Research**: Grassé (1959) — stigmergy. Ants coordinate via pheromone trails deposited in the environment. Git commit messages, CONTEXT.md files, and discovered-patterns.json are digital pheromones.

### The Affordance Improvement Loop

Every successful plan improves the codebase for future agents:
- Better documentation → faster navigation
- Cleaner APIs → fewer type errors
- More tests → richer gate signals
- Descriptive names → better information scent

**Formula**: `affordance = w₁×extensibility + w₂×test_coverage + w₃×documentation + w₄×(1-coupling) + w₅×recent_stability + w₆×(1-size/max)`

**The exponential**: 1% improvement per plan × 100 plans = 170% cumulative. The inverse creates a death spiral: degraded affordances → more failures → worse code → further degradation.

**Research**: Niche construction (Odling-Smee 2003), information foraging (Pirolli & Card 1999), Gibson (affordances, 1979).

---

## The 5-Stage Context Retrieval Pipeline

When assembling context for any agent, entries are retrieved through a 5-stage pipeline:

### Stage 1: Candidate Retrieval (Hybrid Search)

```
Combine multiple search signals via Reciprocal Rank Fusion (RRF):
  1. Keyword search (exact term matching, ripgrep)
  2. Structural search (filter by symbol kind, visibility, module path)
  3. HDC fingerprint similarity (Hamming distance, ~13ns per comparison)
  4. Dense embedding similarity (CodeRankEmbed cosine, ~5ms per query)

RRF score = Σ 1/(k + rank_i) for each search signal, k=60
```

### Stage 2: Scoring (4-Factor Composite)

```
score = similarity × 0.4 + weight × 0.3 + utility × 0.2 + freshness × 0.1

Where:
  similarity = RRF score from Stage 1
  weight = entry confidence × confirmation count
  utility = Predictive Foraging utility score (if available)
  freshness = exp(-age / domain_half_life)
```

### Stage 3: Diversity Filter

Deduplicate near-identical entries: if HDC Hamming distance < 0.15 between two candidates, keep only the higher-scored one. This prevents the context pack from being dominated by variations of the same knowledge.

**Minimum diversity**: At least 15% of entries must be from the lowest-scoring quartile (anti-echo-chamber, forced exploration of peripheral knowledge).

### Stage 4: Budget Fitting

Fit selected entries to the token budget (800-1,200 tokens typical for a context section):
1. Sort by composite score (descending)
2. Greedily add entries until budget exhausted
3. If budget tight: truncate lowest-priority entries, not highest

### Stage 5: U-Shaped Placement

Place most relevant entries at the **beginning and end** of the context section (high-attention zones). Less-critical entries go in the middle.

```
[HIGH RELEVANCE: First 2-3 entries]
[MODERATE: Middle entries]
[HIGH RELEVANCE: Last 2-3 entries]
```

**Research**: Liu et al. (2023) — "Lost in the Middle." LLMs pay most attention to beginning and end of context. Placing critical information in the middle degrades performance by 30%+.

---

## The Playbook: Validated Behavioral Rules

### Rule Format

```toml
[[rules]]
tag_set = ["surface:impl", "crate:roko-core"]
recommended_model = "claude-opus-4-6"
confidence = 0.89
episodes_supporting = 12
avg_iterations = 1.3
half_life_days = 30.0
last_validated = "2026-04-09"
```

### Rule Lifecycle

1. **Discovery**: Pattern extraction from 5+ similar successful episodes (HDC clustering)
2. **Proposal**: Rule enters playbook at 0.5 confidence
3. **Validation**: Each time the rule is applied and the gate passes, confidence increases (+0.05)
4. **Contradiction**: If applied and gate fails, confidence decreases (-0.08, 1.6× negativity bias)
5. **Promotion**: Rules at confidence > 0.7 are injected into prompts for matching tasks
6. **Decay**: Confidence decays via Ebbinghaus curve (half_life_days). Rules must be re-validated to survive.
7. **Pruning**: Rules below 0.1 confidence are removed by the Curator cycle.

### Contrarian Rule Injection

To prevent over-reliance on established rules, 15% of injected playbook entries are **mood-opposite** or **low-confidence exploratory** entries. This ensures the agent doesn't rigidly follow proven patterns when the context has shifted.

**Research**: Nietzsche (1887) — harmful rumination critique. Bower (1981) — mood-congruent memory creates echo chambers. The contrarian mechanism breaks both failure modes.

---

## Dynamic Prompt Generation

### The Static Prompt Tax

Static prompts are a tax on every agent invocation. A single `CLAUDE.md` file gets injected into every agent regardless of task. The Implementer writing a rate limiter sees deployment docs. The Reviewer checking pagination sees auth middleware context. The Scribe documenting an API sees gate pipeline internals. Every irrelevant token costs money, wastes context capacity, and actively degrades performance.

This is not a minor inefficiency. Shi et al. (2023) demonstrated in "Large Language Models Can Be Easily Distracted by Irrelevant Context" that irrelevant information in prompts measurably degrades model accuracy -- even frontier models get worse when you add context they don't need. The degradation is not linear; it compounds as irrelevant context crowds out relevant context near attention boundaries.

### Three Problems Solved

**Problem 1: Irrelevant context degrades performance.** The model attends to everything in the prompt. Irrelevant sections dilute attention over useful sections. A 200K-token prompt where only 40K tokens are relevant means 80% of the model's attention budget is wasted on noise. Pass rates drop measurably.

**Problem 2: Context overflow kills cache efficiency.** Provider-side prefix caching depends on stable, shared prefixes across requests. When every agent gets the same massive prompt, the prefix is long but much of it is irrelevant. When prompts are dynamically tailored, the shared prefix is shorter but the cache hit rate on that prefix is higher -- and the total request is smaller, so per-request cost drops even when cache miss rates are similar.

**Problem 3: One-size-fits-all misses task-specific knowledge.** A static prompt cannot include task-specific context because it doesn't know the task at assembly time. Dynamic generation can inject crate-specific patterns, file-specific history, and role-specific decision frameworks that a static prompt cannot.

### The Pipeline

```
Task arrives
  → Classify: category (8 types) × complexity (4 bands) × role
  → Select: per-section lift scores determine inclusion
  → Compress: LLMLingua-2 for code context, summarization for history
  → Align: stable sections first (cache prefix), volatile sections last
  → Prune: tools filtered to task category (97.5% reduction)
  → Assemble: fit to per-role budget, U-shaped placement
  → Emit: minimal, maximally-effective prompt
```

### The --bare Flag Experiment

A revealing natural experiment: stripping ALL enrichment except the raw task description sometimes **improves** pass rate for simple, mechanical tasks (rename a variable, add an import, fix a typo). This proves that static context can actively hurt simple tasks by:
- Consuming tokens that push the task description toward attention dead zones
- Introducing decision frameworks that overcomplicate trivial changes
- Loading tool definitions the agent will never use

The --bare flag is not the solution (complex tasks need enrichment), but it proves dynamic generation is necessary. The optimal prompt is somewhere between --bare and full enrichment, and it varies per task.

### The Section Bandit

Each prompt section is treated as a bandit arm. The metric is **lift**:

```
lift = pass_rate_with_section - pass_rate_without_section
```

Measurement: for each section, compare gate pass rates on tasks where the section was included vs. excluded. If `lift < -0.02`, the section is actively hurting the agent -- exclude it from the default set for that task category.

**Section categories and typical lift values**:

| Section | Implementer | Reviewer | Strategist | Scribe |
|---|---|---|---|---|
| Code context (files, symbols) | +0.15 | +0.08 | -0.01 | +0.03 |
| Plan context (DAG, dependencies) | +0.04 | +0.02 | +0.18 | +0.01 |
| Diff context (recent changes) | +0.02 | +0.14 | +0.06 | -0.02 |
| Documentation context | -0.01 | +0.01 | +0.03 | +0.12 |
| Gate history (pass/fail patterns) | +0.06 | +0.09 | +0.11 | -0.03 |
| Playbook rules | +0.08 | +0.05 | +0.07 | +0.02 |

Negative lift values indicate sections that should be excluded for that role. The Section Bandit learns these values over time via the same LinUCB mechanism used for model routing.

### Per-Role Budget Allocation

Different roles need different context emphasis. The budget allocation table:

| Budget Category | Implementer | Reviewer | Strategist | Scribe |
|---|---|---|---|---|
| Code context | 45% | 20% | 10% | 15% |
| Plan context | 10% | 10% | 40% | 10% |
| Diff context | 10% | 35% | 15% | 5% |
| Documentation | 5% | 5% | 10% | 40% |
| Playbook rules | 15% | 15% | 15% | 15% |
| Tool definitions | 15% | 15% | 10% | 15% |

These are starting values. The Section Bandit adjusts them based on measured lift. Over time, the system converges on the optimal budget allocation for each role in each codebase.

**Research**: Shi et al. (2023) — "Large Language Models Can Be Easily Distracted by Irrelevant Context." Liu et al. (2023) — "Lost in the Middle." Voyager (Wang et al., 2023) — progressive skill loading. The Section Bandit extends contextual bandits (Li et al., 2010) from model selection to prompt section selection.

---

## Automated Prompt Optimization (DSPy-Style)

### The Problem with Hand-Written Prompts

Hand-written prompts are static artifacts in a dynamic system. The codebase changes, the agent learns, the task distribution shifts -- but the prompt stays frozen. Prompt engineering is artisanal: one developer writes a prompt, tests it on a few examples, and ships it. There is no systematic optimization, no A/B testing infrastructure, no gradient signal from outcomes to prompt content.

### DSPy: Declarative Self-Improving Python

DSPy (Khattab et al., 2023) introduced a framework where prompts are not written -- they are **compiled**:

1. **Define modules** with typed signatures: `task_description: str -> implementation_plan: str`
2. **Compose modules** into pipelines: `classify -> select_context -> assemble_prompt -> execute`
3. **Optimize** against a metric: maximize gate pass rate over a held-out set of historical tasks
4. The optimizer generates candidate prompt variants, evaluates them, selects the best as seeds for the next generation

The key insight: prompts are programs, and programs can be optimized by compilers.

### OPRO: LLMs as Optimizers

Yang et al. (2023) demonstrated that LLMs themselves can optimize prompts. The OPRO loop:

1. Start with a set of candidate prompts and their scores
2. Ask the LLM: "Here are prompts and their scores. Generate a better prompt."
3. Evaluate the new prompt on the task set
4. Add to the candidate pool, repeat

This is meta-prompting: using the LLM to improve its own instructions. Measured improvements of 8-50% on standard benchmarks.

### APE: Automatic Prompt Engineer

Zhou et al. (2022) formalized Automatic Prompt Engineering:

1. **Generation**: Given task examples, generate candidate instructions
2. **Selection**: Score candidates on a validation set
3. **Refinement**: Iteratively improve the best candidates via paraphrase and editing

APE found prompts that outperformed human-written ones on 24/24 NLP benchmarks tested.

### Applied to Roko

The prompt optimization loop closes between scaffold (which constructs prompts) and harness (which measures outcomes):

```
Historical tasks (with known pass/fail outcomes)
  → Define typed template slots: {role_instructions}, {code_context}, {plan_context}, ...
  → Generate candidate variants for each slot (OPRO-style)
  → Evaluate: run each variant against historical tasks, score by gate pass rate
  → Select: promote best variant per slot
  → Deploy: winning variants become the default template
  → Monitor: continue measuring lift on new tasks
  → Iterate: re-optimize when lift degrades
```

**Integration with the Section Bandit**: The Section Bandit determines WHICH sections to include. DSPy-style optimization determines the CONTENT of each section. Together they answer both "what context?" and "how to phrase the context?"

**The ExperimentStore** (already implemented in `.roko/learn/experiments.json`) provides the A/B testing infrastructure. Each prompt variant is a treatment. Gate pass rate is the outcome metric. The experiment store tracks which variant was used for each task and computes significance.

**Research**: DSPy (Khattab et al., 2023) — declarative prompt programming. OPRO (Yang et al., 2023) — LLM-driven prompt optimization. APE (Zhou et al., 2022) — automatic prompt engineering. TextGrad (Yuksekgonul et al., 2024) — gradient-based prompt optimization.

---

## The Learning Hierarchy Expanded

### Five Levels of Increasing Scope

Knowledge in roko flows across five temporal levels. Each level has a longer time constant, broader applicability, and higher validation requirements:

#### Level 0: Within-Attempt (Seconds) -- IMPLEMENTED

The agent uses its own prior output within a single task attempt. When `cargo test` fails on line 42, the agent reads the error, adjusts, and retries. This is the most basic form of learning: iteration memory.

**Mechanism**: The agent's conversation history within a single tool loop. No persistence required -- it's in the context window.

**Time constant**: Seconds to minutes. Knowledge dies when the attempt ends.

#### Level 1: Within-Run (Minutes) -- IMPLEMENTED

Cross-agent knowledge sharing during a single plan execution. When Agent A discovers that `roko-core` requires `Arc<Mutex<_>>` wrapping for a particular struct, it writes this to `discovered-patterns.json`. Agent B reads the file before starting its task and avoids the same error.

**Mechanism**: `discovered-patterns.json` (max 20 entries, FIFO). Stigmergic -- agents don't communicate directly; they deposit and read from a shared file.

**Time constant**: Minutes to hours. Knowledge persists for the duration of the plan run.

#### Level 2: Cross-Run (Hours to Days) -- IMPLEMENTED

Episodes from completed runs are analyzed, clustered, and distilled into playbook rules. A pattern that appears in 5+ episodes with >70% success rate is promoted to a heuristic. Heuristics that prove reliable across 10+ applications become playbook rules injected into future prompts.

**Mechanism**: Episode → pattern → playbook promotion pipeline. Ebbinghaus decay with type-specific half-lives. Curator cycle validates and prunes every ~50 ticks.

**Time constant**: Hours to days. Knowledge decays via half-life but can persist indefinitely if re-validated.

#### Level 3: Cross-Project (Days to Weeks) -- ENABLED

The brain export/import mechanism packages an agent's learned state (playbook rules, neuro entries, cascade router weights) into a portable artifact. This artifact can be imported into a different codebase. A pattern learned in a Rust project ("always run `cargo check` before `cargo test`") transfers to another Rust project.

**Mechanism**: Brain export serializes the neuro, playbook, and router state. Import applies it with a 0.4 confidence discount (suggestive, not authoritative). The receiving agent must re-validate inherited knowledge against its own codebase.

**Time constant**: Days to weeks. Transferred knowledge must prove itself in the new context or it decays away.

#### Level 4: Cross-Agent (Continuous) -- PHASE 2+

The Korai chain enables ecosystem-wide knowledge sharing. Agents across different operators, codebases, and domains contribute validated knowledge to a shared ledger. Layer 2 (Commons) provides anonymized structural patterns. The pheromone field (stigmergy) allows fuzzy semantic alignment without exact string matching.

**Mechanism**: Korai Ledger with Layer 2 anonymized knowledge relay. HDC-encoded pheromones with Hamming similarity bundling. Economic incentives (DIEM staking) align agent contributions.

**Time constant**: Continuous. Knowledge persists as long as it is confirmed by the network.

### The Compounding Effect

Each level feeds the next:
- Level 0 iterations produce Level 1 discovered patterns
- Level 1 patterns across multiple runs produce Level 2 playbook rules
- Level 2 rules exported produce Level 3 cross-project knowledge
- Level 3 knowledge shared on-chain produces Level 4 ecosystem intelligence

The time constants increase geometrically: seconds → minutes → days → weeks → continuous. The validation requirements increase proportionally: self-evidence → cross-agent confirmation → cross-run statistical significance → cross-project re-validation → network consensus.

**The current system reaches Level 2.** Level 3 is enabled but requires manual export/import. Level 4 requires the Korai chain (Phase 2+). The architecture is designed so each level can be activated independently as infrastructure matures.

---

## Antifragility: Getting Stronger from Stress

### Beyond Resilience

Taleb (2012) defined three categories of response to stress:

| Category | Response to Stress | Example |
|---|---|---|
| **Fragile** | Breaks | Glass: drops → shatters |
| **Resilient** | Returns to baseline | Rubber ball: drops → bounces back |
| **Antifragile** | Gets stronger | Muscles: stress → micro-tears → stronger tissue |

Current roko agents are **resilient**: crash → restart, gate failure → retry, timeout → exponential backoff. They survive stress but don't improve from it. The goal is antifragility: each failure makes the system stronger than it was before the failure occurred.

### Four Antifragile Mechanisms

#### Mechanism 1: Real-Time Pattern Extraction

When a gate failure occurs, the system doesn't just retry -- it immediately extracts the failure pattern and updates the playbook for all concurrent agents:

```
Agent A: gate failure (compile error: missing import in roko-core)
  → Extract pattern: "missing import after editing roko-core module"
  → Update discovered-patterns.json (Level 1 sharing)
  → Concurrent Agent B reads pattern before its next compile
  → Agent B preemptively adds imports → avoids the same failure
```

**The key difference from resilience**: A resilient system would let Agent B fail the same way and retry. An antifragile system prevents Agent B from failing at all because Agent A's failure hardened the system.

**Latency**: Pattern available to concurrent agents within seconds of the failure. No cross-run delay.

#### Mechanism 2: Chaos Injection

Inspired by Netflix's Chaos Monkey, the `--chaos` flag enables a `ChaosInjector` that deliberately introduces stress:

| Injection Type | What It Does | What It Tests |
|---|---|---|
| **Process kill** | Randomly terminate an agent process mid-task | Recovery from crash, state persistence |
| **Gate delay** | Add 5-30s artificial delay to gate execution | Timeout handling, patience under load |
| **Compile warning** | Inject a benign warning into compiler output | Warning-vs-error discrimination |
| **File corruption** | Write garbage to a scratch file the agent reads | Input validation, graceful degradation |
| **Network partition** | Block MCP server communication temporarily | Fallback to local tools, cache utilization |

**Usage**: `roko plan run plans/ --chaos` enables injection. The injector fires at random intervals (Poisson process, mean = 1 event per 10 agent-minutes).

**Measurement**: Track recovery time per injection type over multiple runs. Decreasing recovery time = antifragile (the system is learning to recover faster). Increasing recovery time = fragile (the system is degrading under stress). Stable recovery time = resilient (baseline, not improving).

#### Mechanism 3: Hormesis Calibration

Hormesis is the dose-response relationship where low doses of stress are beneficial but high doses are harmful (the Yerkes-Dodson curve). The chaos injector doesn't blast the system with maximum stress -- it calibrates:

1. Start with low injection frequency (1 event per 30 agent-minutes)
2. Measure gate pass rate under stress vs. baseline
3. If pass rate holds or improves → increase frequency (the system can handle more)
4. If pass rate drops significantly → decrease frequency (too much stress)
5. Converge on the optimal stress level that maximizes learning without degrading output

**The Goldilocks zone**: Enough chaos to trigger adaptation, not enough to cause cascading failures.

#### Mechanism 4: Post-Traumatic Growth

Major failures (plan-level failures, not just task-level) trigger a deeper analysis:

1. **Root cause analysis**: Cheap model analyzes the full failure trace
2. **Structural hardening**: Identify which code paths, which crates, which gate configurations were involved
3. **Targeted testing**: Generate regression tests for the specific failure mode
4. **Playbook update**: Add a high-confidence warning entry with the specific trigger conditions
5. **Threshold adjustment**: The adaptive gate system (EMA thresholds) tightens the gate for the affected rung

The result: the specific failure path becomes the **strongest** part of the system. It has dedicated regression tests, explicit playbook warnings, and tightened gate thresholds. The system is literally stronger at the break point than it was before the break.

### The Antifragility Metric

Quantify antifragility as the ratio of improvement rate to stress rate:

```
antifragility_score = d(performance) / d(stress)

If > 0: antifragile (performance improves with stress)
If = 0: resilient (performance unchanged)
If < 0: fragile (performance degrades with stress)
```

Measured per subsystem: gate pipeline, agent dispatch, state persistence, prompt assembly. Each subsystem has its own antifragility score tracked over time.

**Research**: Taleb (2012) — "Antifragile: Things That Gain from Disorder." Yerkes-Dodson law (1908) — optimal arousal for performance. Netflix Chaos Monkey (Basiri et al., 2016) — deliberate fault injection in production. Hormesis (Calabrese & Baldwin, 2002) — dose-response relationships in biological systems.
