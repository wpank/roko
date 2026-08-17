# PRD-04: Context engineering

| Field | Value |
|-------|-------|
| Author | Will |
| Date | 2026-04-21 |
| Status | Draft |
| Scope | Learnable context assembly: CognitiveWorkspace, VCG auction, section effect tracking, cache architecture, InsightStore integration |

---

## 1. Why context is the highest-leverage intervention

The dominant assumption in AI infrastructure is that performance comes from the model. Upgrade the model, improve the agent. The data says otherwise.

On SWE-bench, GPT-4 scores 2.7% with a naive scaffold and 28.3% with an optimized one. Same model. Same weights. Same training data. The scaffold -- the context engineering, verification pipeline, retrieval strategy, and memory architecture wrapped around the model -- produces a 10x swing. Model upgrades, by comparison, move the needle one or two percentage points.

This is not a single anomalous result. Meta-Harness (Lee et al., March 2026; arXiv:2603.28052) proved it quantitatively across five model families. Optimizing what to store, what to retrieve, and what to show the model achieved +7.7 accuracy points on text classification and +4.7 points on IMO-level math, while using 4x fewer tokens. That improvement matched or exceeded swapping models entirely. Anthropic's own guidance (2025) distills the finding to one sentence: "The right 1,000 tokens outperform 100,000 tokens of wrong context."

Cursor beats raw Claude Opus using the identical underlying model. The difference is the harness.

If the scaffold is the product, then the most defensible piece of an agent runtime is the system that decides which context to assemble, how to budget it, and how to learn from the outcome. That system is what this PRD specifies.

### Why static context fails

A naive approach -- fixed templates with predetermined section allocations per role -- works for simple cases and breaks everywhere else. Four properties of real tasks defeat static allocation:

1. **Task complexity varies.** A single-file rename needs 4K tokens of context. A cross-crate refactor touching 30 files needs 40K. A static budget either over-provisions the simple case (wasting money and diluting attention) or under-provisions the complex case (causing failures).

2. **Section relevance varies by pattern.** Workspace maps help on architectural tasks and hurt on trivial fixes. Research context helps when the task involves unfamiliar domains and adds noise when the domain is well-trodden. The same section can have positive lift in one configuration and negative lift in another.

3. **Domain varies.** An agent working on chain settlement needs positions, strategy fragments, and market state. An agent working on CLI tooling needs file context, symbol signatures, and anti-patterns. Static templates cannot anticipate the category distribution ahead of time.

4. **Agent affect state varies.** An agent recovering from a gate failure benefits from increased warning and anti-knowledge context. An agent on a streak of successes benefits from expanded exploratory context. The optimal allocation depends on runtime state that templates cannot encode.

The solution is a context assembly system that measures which sections contribute to task success and evolves its allocations accordingly. This document specifies that system.

---

## 2. The CognitiveWorkspace

Every LLM call in Roko receives a `CognitiveWorkspace`: a typed, budgeted, audited context package assembled by the composition layer. The workspace is not a string. It is a structured object that records what went in, why it went in, how much budget it consumed, and which subsystem contributed it.

```rust
/// The assembled context package for a single LLM invocation.
///
/// Built by the composition layer (roko-compose) before each agent dispatch.
/// Carries the complete audit trail of what context was selected and why.
pub struct CognitiveWorkspace {
    /// Cognitive tier (Surgical / Focused / Full) that determined the
    /// baseline budget. Derived from the task's complexity and the target
    /// model's context window.
    pub tier: CognitiveTier,

    /// Ordered sections that compose the final prompt. The order reflects
    /// U-shaped placement: highest-priority sections at the beginning and
    /// end, lowest-priority in the middle.
    pub sections: Vec<ContextSection>,

    /// Total token budget allocated for this invocation. Set by the
    /// BudgetPredictor based on historical data for this role/complexity/
    /// domain combination.
    pub total_budget_tokens: u32,

    /// Tokens consumed by the sections that won the VCG auction.
    pub used_tokens: u32,

    /// Ordered log of assembly decisions. Each entry records why a section
    /// was included, excluded, or truncated. Persisted to the episode log
    /// for post-hoc analysis.
    pub assembly_log: Vec<AssemblyReason>,

    /// Content hash of the deterministic prefix (Role + Workspace layers).
    /// When two workspaces share this hash, the LLM provider can reuse its
    /// KV cache prefix, saving inference cost.
    pub cache_key: Option<ContentHash>,
}

/// One section within the assembled workspace.
pub struct ContextSection {
    /// The category this section belongs to (Role, Task, Knowledge, etc.).
    pub category: ContextCategory,

    /// Priority from 1 (drop first under pressure) to 5 (never drop).
    /// Learned per category per role via the section effect registry.
    pub priority: u8,

    /// Fraction of the total budget allocated to this section. Learned
    /// from the Beta posterior in the LearningBidder.
    pub allocation: f64,

    /// The rendered text content.
    pub content: String,

    /// Token count for this section (estimated at ~4 bytes/token).
    pub tokens: u32,

    /// Which extension (bidder) contributed this section.
    pub source: SectionSource,

    /// Provenance metadata: where the content came from, confidence
    /// score, recency, and emotional valence.
    pub metadata: SectionMetadata,
}
```

The workspace tiers map to the `ContextTier` enum already defined in `roko-compose/src/context_provider.rs`:

| Tier | Models | Token target | Included by default | Excluded by default |
|------|--------|-------------|---------------------|---------------------|
| Surgical | Haiku, Ollama, Gemma | ~4K | Inline files, symbol signatures, anti-patterns, verification spec | Plan context, research, enrichment |
| Focused | Sonnet | ~12K | Surgical + task brief, dependency graph, prior task outputs | Full plan brief, cross-plan context |
| Full | Opus | ~24K | Focused + plan brief, cross-plan context, research memo, invariants | Nothing excluded by default |

These baselines are starting points. The VCG auction and learnable policy modify them per invocation.

---

## 3. Context categories

The `ContextCategory` enum defines the 18 categories of content that can enter a workspace. Each category has a default priority, a default budget allocation, and a designated bidder that competes for its inclusion.

```rust
/// What kind of content a section carries.
///
/// Categories are the unit of measurement for section effect tracking.
/// When recording outcomes, the system tracks (category, role) pairs
/// so lift data is scoped to where the category was used.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextCategory {
    /// Agent role identity, behavioral instructions, temperament.
    Role,
    /// Workspace map, project structure, cross-references.
    Workspace,
    /// Plan brief, plan-level context, dependencies between tasks.
    Plan,
    /// Current task description, requirements, acceptance criteria.
    Task,
    /// Durable knowledge entries from the neuro store.
    Knowledge,
    /// Proven execution patterns matched by HDC similarity.
    Playbook,
    /// External research documents, citations, academic papers.
    Research,
    /// File contents, symbol signatures, dependency graphs.
    CodeContext,
    /// Daimon affect state: arousal, valence, dominance, somatic markers.
    Affect,
    /// Risk signals: gate failure history, anti-knowledge, warnings.
    Risk,
    /// Awareness signals: resource awareness, safety constraints.
    Mortality,
    /// DeFi positions: current holdings, exposure, PnL.
    Positions,
    /// Strategy fragments: trading logic, execution parameters.
    Strategy,
    /// Market state: rates, prices, volatility, liquidity depth.
    MarketState,
    /// Attribution and provenance of context sections.
    Sources,
    /// Active hypotheses under investigation.
    Hypotheses,
    /// On-chain state from Korai: validator set, epoch, InsightStore.
    ChainState,
    /// User-defined category for domain-specific extensions.
    Custom(String),
}
```

### Category defaults

| Category | Default priority | Default allocation | Primary domains | Contributing bidder |
|----------|-----------------|-------------------|-----------------|---------------------|
| Role | 5 (never drop) | 8% | All | TaskContext |
| Workspace | 4 | 10% | All | CodeIntelligence |
| Plan | 3 | 8% | All | TaskContext |
| Task | 5 (never drop) | 15% | All | TaskContext |
| Knowledge | 3 | 10% | All | Neuro |
| Playbook | 3 | 5% | Code, Research | PlaybookRules |
| Research | 2 | 5% | Research, Chain | Research |
| CodeContext | 4 | 15% | Code | CodeIntelligence |
| Affect | 2 | 3% | All | Daimon |
| Risk | 4 | 5% | All | Oracles |
| Mortality | 3 | 2% | All | Daimon |
| Positions | 4 | 5% | Trading | Oracles |
| Strategy | 3 | 4% | Trading | Research |
| MarketState | 4 | 5% | Trading | Oracles |
| Sources | 1 | 2% | Research | Research |
| Hypotheses | 2 | 3% | Research | Research |
| ChainState | 3 | 4% | Chain | Neuro |
| Custom(_) | 2 | 2% | Varies | Varies |

These defaults are overridden by learned allocations after the system accumulates sufficient observations. The `SectionEffectivenessRegistry` in `roko-learn/src/section_effect.rs` tracks inclusion/exclusion outcomes scoped to `(category, role)` pairs and recommends priority changes once 20+ included trials and 5+ excluded trials have been recorded.

---

## 4. The VCG auction

Eight subsystems compete for the context window's token budget. The mechanism that adjudicates between them is a Vickrey-Clarke-Groves (VCG) auction -- a budget-feasible variant that allocates tokens to maximize total expected value while charging each bidder for the externality it imposes on others.

### Why VCG

The VCG mechanism has a key property that alternatives lack: **incentive compatibility**. No bidder benefits from lying about the value of including its section. This matters because the bidders are subsystems with different objectives -- the Daimon wants affect context, the code intelligence subsystem wants file contents, the neuro store wants knowledge entries. A first-price auction would incentivize strategic underbidding. VCG eliminates that incentive.

The mechanism works in three steps:

1. **Bid collection.** Each bidder reports the expected value of including its candidate sections. The value is the bidder's estimate of how much the section will contribute to task success, expressed as a float in `[0.0, 1.0]`. Bidders derive this from their own internal models (Beta posteriors for the `LearningBidder`, relevance scores for the neuro store, affect urgency for the Daimon).

2. **Allocation.** The allocator sorts bids by value density (`adjusted_bid / tokens`) and greedily includes sections until the token budget is exhausted. This greedy approach is not globally optimal in theory (that would require solving a knapsack problem, which is NP-hard), but it is within 5--8% of the unconstrained optimum in practice and runs in O(n log n) time.

3. **Payment computation.** For each winning section, the mechanism computes the externality: the total welfare the other bidders would have received if this section had not been present. The payment equals the highest excluded bid that would have fit in the freed slot. This is the VCG second-price clearing rule adapted for multi-item auctions.

### Implementation

The VCG auction is already implemented in `roko-compose/src/auction.rs`. The core function:

```rust
/// Allocate context window tokens using a greedy VCG-style mechanism.
///
/// Bids are sorted by `adjusted_bid / tokens` (value density). Sections
/// are included greedily until the budget is exhausted. VCG payments are
/// computed as the externality each winner imposes on others.
pub fn vcg_allocate(
    bids: Vec<VcgBid>,
    total_budget: usize,
    modulation: &AffectModulation,
) -> VcgAllocation {
    // ... (see roko-compose/src/auction.rs lines 293-414)
}
```

Each bid carries the full provenance of its value:

```rust
/// One subsystem's bid in the VCG auction.
pub struct VcgBid {
    /// Subsystem that placed the bid.
    pub bidder: SubsystemId,
    /// Section name.
    pub section_name: String,
    /// Token cost for this section.
    pub tokens: usize,
    /// Raw bid value before affect modulation.
    pub raw_bid: f64,
    /// Affect-adjusted bid value.
    pub adjusted_bid: f64,
    /// Emotional valence of the content (if applicable).
    pub valence: f64,
}
```

The allocation result includes diagnostics that feed back into the learning system:

```rust
/// Result of a VCG-style allocation.
pub struct VcgAllocation {
    /// Sections that won the auction.
    pub winners: Vec<VcgBid>,
    /// Sections excluded due to budget constraints.
    pub excluded: Vec<VcgBid>,
    /// VCG payments for each winner (second-price clearing).
    pub payments: Vec<(String, f64)>,
    /// Total tokens allocated.
    pub total_tokens_used: usize,
    /// Budget utilization fraction.
    pub budget_utilization: f64,
    /// Diagnostics: welfare, payments, Pareto check, displaced sections.
    pub diagnostics: AuctionDiagnostics,
}
```

### The eight bidders

The `AttentionBidder` enum in `roko-compose/src/prompt.rs` defines the eight subsystems that compete:

```rust
pub enum AttentionBidder {
    Neuro,            // Durable knowledge from the neuro store
    Daimon,           // Affect and somatic guidance
    IterationMemory,  // Recent turns, retries, prior task outputs
    CodeIntelligence, // Symbols, files, structural workspace context
    PlaybookRules,    // Skills, playbooks, distilled reusable rules
    Research,         // Research memos and external domain context
    TaskContext,      // Task brief, plan brief, verification, PRD
    Oracles,          // Predictions, warnings, gate hints
}
```

Each bidder has a distinct retrieval strategy and value model:

**1. NeuroContextBidder.** Queries the local neuro knowledge store and, when available, the Korai InsightStore. Candidate entries are ranked by HDC Hamming similarity to the current task encoding. The bid value combines relevance score (cosine similarity mapped through HDC encoding) and track record (past success rate when this entry was included). Uses the `LearningBidder` with Beta posteriors to learn per-entry value over time.

**2. TaskContextBidder.** Provides the task description, acceptance criteria, plan brief, PRD extracts, and verification specs. These sections are marked `SectionPriority::Critical` and receive a baseline bid of 1.0 (maximum). The auction allocates them first because they have the highest value density -- a task without its own description cannot succeed.

**3. PlaybookBidder.** Matches the current task against proven execution patterns in `roko-learn/src/playbook.rs` using HDC fingerprint similarity. Playbooks are distilled from successful episodes: if a sequence of tool calls consistently led to gate passes for similar tasks, the sequence is stored as a playbook. The bid value is the similarity score multiplied by the playbook's historical success rate.

**4. CodeIntelligenceBidder.** Resolves workspace symbols via the `SymbolResolver` in `roko-compose/src/symbol_resolver.rs`. Produces file contents, dependency graph excerpts, and type signatures. The bid value is derived from the information density heuristic in `roko-compose/src/attention.rs` -- sections with higher term overlap with the task description receive higher bids.

**5. ResearchBidder.** Retrieves external research documents, academic citations, and domain context from `roko-learn/src/research_pipeline.rs`. Bids are weighted by document relevance and recency. The bidder applies a decay function so older research loses value unless independently confirmed by recent task outcomes.

**6. DaimonBidder.** Reads the agent's PAD (Pleasure-Arousal-Dominance) state from the Daimon subsystem. Contributes affect guidance, somatic warnings (patterns that historically preceded failures), and anti-knowledge (things the agent learned NOT to do). The bid value is modulated by arousal: high-arousal states inflate warning bids; high-valence states inflate exploratory bids. See section 10.

**7. OracleBidder.** Provides gate hints (expected failure modes for the current task), threshold adjustments from the adaptive gate system, and ISFR predictions for trading agents. Bids are proportional to the oracle's calibrated confidence. Poorly calibrated oracles (tracked by `roko-learn/src/calibration_policy.rs`) have their bids discounted.

**8. IterationMemoryBidder.** Contributes context from previous attempts at the same or similar tasks. When a task is retried after a gate failure, this bidder includes the error output, the gate verdict, and the previous prompt's diff against the current one. The bid value decays exponentially with the number of turns since the relevant attempt.

---

## 5. ContextPolicy: learnable allocation

The VCG auction determines allocation for a single invocation. The `ContextPolicy` determines how bidder priors evolve across invocations. Three feedback loops operate at different timescales.

### Loop 1: per-episode attribution (fast -- every task)

After each task completes and the gate pipeline returns a verdict, the system records which sections were present in the workspace and whether the task passed. This data feeds two parallel trackers:

**SectionEffectivenessRegistry** (`roko-learn/src/section_effect.rs`): Tracks inclusion/exclusion statistics per `(section_name, role)` pair. Each entry maintains four counters: `included_trials`, `included_passes`, `excluded_trials`, `excluded_passes`. The lift is the difference in pass rate:

```
lift = (included_passes / included_trials) - (excluded_passes / excluded_trials)
```

When lift exceeds +0.05 with at least 20 included trials and 5 excluded trials, the registry recommends `PriorityChange::Increase`. Below -0.02, it recommends `PriorityChange::Decrease`. The multiplicative budget weight is `(1.0 + lift).clamp(0.5, 1.5)`.

**LearningBidder** (`roko-compose/src/auction.rs`): Updates a Beta posterior per section name. On success with the section included, the posterior shifts toward higher bids:

```rust
pub fn update(&mut self, section_name: &str, was_included: bool, gate_passed: bool) {
    if !was_included { return; }
    let entry = self.section_betas
        .entry(section_name.to_string())
        .or_insert((1.0, 1.0));
    if gate_passed {
        entry.0 += 1.0;   // alpha (success count)
    } else {
        entry.1 += 1.0;   // beta (failure count)
    }
}
```

The bid for a section is the Thompson-like sample from the posterior, multiplied by the bidder's relevance estimate and prior weight:

```rust
pub fn bid(&self, section_name: &str, relevance: f64) -> f64 {
    let (alpha, beta) = self.section_betas
        .get(section_name)
        .copied()
        .unwrap_or((1.0, 1.0));
    let sampled_track_record = thompson_like_sample(section_name, alpha, beta);
    sampled_track_record * relevance.max(0.0) * self.prior_bid.max(0.0)
}
```

The Thompson-like sample is a deterministic approximation (to avoid the `rand` dependency) that uses the posterior mean plus a hash-derived exploration offset scaled by the posterior standard deviation. This produces exploration behavior without true randomness.

### Loop 2: policy evolution (medium -- every 50 ticks)

Every 50 completed tasks, the system runs a policy evolution step. This step:

1. Reads the cumulative lift weights from the `SectionEffectivenessRegistry`.
2. Reads the current Beta posteriors from all `LearningBidder` instances.
3. Computes an updated allocation vector where each category's budget share is proportional to its estimated utility (the mean of its Beta posterior, weighted by the lift from the effectiveness registry).
4. Bounds the result: no category drops below 1% or exceeds 50% of the total budget.
5. Writes the updated policy to `.roko/learn/context-policy.json`.

The `BudgetPredictor` in `roko-compose/src/budget_predictor.rs` participates in this step by adjusting the total token budget per feature key. Successful tasks that used fewer tokens than budgeted cause the EMA to converge downward (saving money). Failed tasks inflate the EMA by 30% (preventing repeated under-provisioning).

### Loop 3: cross-agent aggregation (slow -- via Korai InsightStore)

When a Korai chain connection is available, the system publishes context policy effectiveness as InsightEntries. Each entry contains:

- The category name and role.
- The measured lift over the last 50 tasks.
- The current Beta posterior parameters.
- The agent's reputation-weighted confidence in the measurement.

Other agents query these entries during context assembly. When a remote agent's policy data shows strong lift for a category that the local agent has insufficient data on, the local agent incorporates the remote posterior as a prior (weighted by the submitter's reputation score).

This creates a network-wide optimization effect: the thousandth agent to encounter a new domain starts with the aggregated context policies of the 999 agents that came before it. The InsightStore's reputation weighting and automatic decay (demurrage) prevent stale or adversarial data from persisting.

### Loop 3 in detail: cross-agent context learning via Korai

Loop 3 is the slowest feedback loop and the most powerful. It operates across agents, across sessions, and across time. Where Loop 1 optimizes a single agent's context within a task, and Loop 2 optimizes a single agent's policy across tasks, Loop 3 optimizes the entire network's context strategy across all agents.

**Publication.** After every Loop 2 policy evolution step (every 50 tasks), the agent publishes a context policy effectiveness report to the Korai InsightStore as a `Heuristic` entry:

```
{
    "type": "context_policy_effectiveness",
    "domain": "coding",
    "role": "implementer",
    "categories": {
        "CodeContext": { "allocation": 0.18, "lift": 0.23, "n": 147 },
        "Playbook":   { "allocation": 0.07, "lift": 0.15, "n": 89 },
        "Risk":       { "allocation": 0.04, "lift": 0.31, "n": 52 },
        "Research":   { "allocation": 0.02, "lift": -0.08, "n": 34 }
    },
    "composite_pass_rate": 0.87,
    "total_tasks": 322,
    "agent_reputation": 0.74
}
```

**Query.** When a new agent starts in a domain it has limited data for, the NeuroContextBidder queries the InsightStore for `context_policy_effectiveness` entries matching the current domain and role. If a remote agent with reputation > 0.5 has published effectiveness data showing strong lift for a category the local agent has insufficient observations on (fewer than 20 included trials), the local agent incorporates the remote posterior as an informative prior.

The incorporation formula:

```
local_alpha = local_alpha + (remote_alpha * reputation_weight * confidence_factor)
local_beta  = local_beta  + (remote_beta * reputation_weight * confidence_factor)
```

Where `confidence_factor = min(1.0, remote_n / 100)` -- remote posteriors with fewer than 100 observations are discounted proportionally.

**Convergence effect.** The result is that 1,000 agents in a domain converge on optimal context policies faster than any individual agent could. An individual agent needs ~200 tasks to stabilize its context policy (enough observations per category for the Beta posteriors to tighten). With Loop 3, a new agent inherits the network's aggregated experience and stabilizes in ~20 tasks -- an order of magnitude faster.

**C-factor for context.** The collective factor for context quality is defined as:

```
C_context = mean(collective_pass_rate / best_individual_pass_rate)
```

across task types. When C_context > 1.0, the network's shared context policies outperform the best individual agent's isolated policies. Target: C_context >= 1.10 (10% collective uplift on context quality alone).

**Anti-gaming.** Loop 3 is vulnerable to adversarial agents publishing false effectiveness data to degrade competitors. Three defenses:

1. **Reputation gating.** Only entries from agents with reputation > 0.5 are considered. Building reputation requires successful task outcomes verified by the gate pipeline.
2. **Demurrage.** False entries that are not independently confirmed decay within 30 days.
3. **Outlier rejection.** If a remote agent's reported lift deviates by more than 3 standard deviations from the network mean for that category, the entry is excluded from prior incorporation.

---

## 5.5 WorldGraph context injection

The WorldGraph is the agent's dynamic, queryable model of the world -- entities, relationships, states, and temporal dynamics. Where the neuro store holds distilled knowledge (facts, heuristics, anti-patterns), the WorldGraph holds structured state (what exists, how things connect, what changed recently).

### WorldGraphBidder

The `WorldGraphBidder` participates in the VCG auction alongside the other bidders. It contributes entity and relationship context relevant to the current task.

**Entity resolution.** When the context system receives a task, the WorldGraphBidder identifies entities mentioned in the task description:

1. Extract named entities from the task text (contract addresses, function names, crate names, protocol names, metric identifiers).
2. Look up each entity in the WorldGraph. For each match, retrieve the entity's current state, its direct relationships (up to 2 hops), and its recent state changes (last 24 hours).
3. Score each entity by relevance to the task using HDC similarity between the entity's fingerprint and the task's fingerprint.
4. Submit the top-scoring entities as candidate sections in the `ChainState` or `Workspace` category (depending on domain).

**Relationship context.** The WorldGraph stores typed relationships between entities:

| Relationship type | Example | Context value |
|------------------|---------|---------------|
| `depends_on` | `roko-cli depends_on roko-compose` | Reveals build and import dependencies for refactoring tasks |
| `calls` | `orchestrate.rs calls dispatch_agent_with()` | Shows call graph for debugging and impact analysis |
| `monitors` | `blockchain-1 monitors ETH/USDC LP` | Connects agents to the positions they track |
| `derives_from` | `ISFR derives_from Aave_rate, Compound_rate, Ethena_rate` | Shows data lineage for metrics |
| `conflicts_with` | `flash_loan_strategy conflicts_with low_gas_strategy` | Surfaces known incompatibilities |

The bidder selects relationships where either endpoint is relevant to the current task and formats them as structured context:

```
[WorldGraph: 12 entities, 27 relationships relevant to this task]

Entity: roko-compose (crate)
  State: 8,421 LOC, 14 public modules, last modified 2h ago
  Relationships:
    - depends_on: roko-core, roko-learn, roko-neuro
    - depended_on_by: roko-cli, roko-serve
    - contains: auction.rs, context_provider.rs, system_prompt_builder.rs
```

**Cross-chain state summarization.** For blockchain-domain tasks, the WorldGraph holds state across multiple chains. The bidder summarizes this at a temporal resolution appropriate to the task:

- For real-time trading tasks: latest block data, current gas prices, pending transaction counts.
- For research tasks: 24-hour aggregates, trend indicators, anomaly flags.
- For governance tasks: active proposals, voting deadlines, quorum status.

The temporal resolution is determined by the task's cognitive frequency (gamma/theta/delta from PRD-02 section 3). A gamma-frequency task gets real-time state. A delta-frequency task gets daily summaries.

**HDC similarity for entity discovery.** Not all relevant entities are explicitly named in the task. The WorldGraph bidder performs a "neighborhood walk" from explicitly mentioned entities:

1. Start with entities directly mentioned in the task.
2. For each, compute HDC similarity between the entity's fingerprint and the task fingerprint.
3. Walk outward along relationships to entities 1-2 hops away.
4. Include any neighbor whose HDC similarity to the task exceeds 0.7.
5. Cap the total at 20 entities to prevent context bloat.

This discovers implicitly relevant entities. A task that mentions "update the VCG auction" will surface `roko-compose/src/auction.rs` (directly mentioned), but the neighborhood walk also discovers `LearningBidder`, `AffectModulation`, and `BudgetPredictor` (related entities that the task description did not name but that the implementation will need to touch).

---

## 6. Section effect tracking

The feedback loops above depend on measurement. Two subsystems in `roko-learn` provide it.

### SectionEffectivenessRegistry

Located in `roko-learn/src/section_effect.rs`. This is the primary measurement system for individual section impact.

```rust
/// Thread-agnostic registry of section effectiveness keyed by (section, role).
pub struct SectionEffectivenessRegistry {
    effects: HashMap<(String, String), SectionEffect>,
}

/// Inclusion/exclusion outcome statistics for one prompt section.
pub struct SectionEffect {
    pub section_name: String,
    pub included_trials: u64,
    pub included_passes: u64,
    pub excluded_trials: u64,
    pub excluded_passes: u64,
}
```

The registry persists to `.roko/learn/section-effects.json` via atomic rename (write to `.json.tmp`, then rename). It exposes:

- `lift()` -- pass-rate difference between included and excluded states.
- `lift_weight()` -- multiplicative budget weight in `[0.5, 1.5]`, centered at 1.0.
- `recommend_priority_change()` -- `Increase`, `Decrease`, `NoChange`, or `InsufficientData`.
- `positive_lift_sections(role)` -- all sections with positive lift for a role, sorted by descending lift.

### SectionInfluence

Located in `roko-compose/src/budget_predictor.rs`. This is the leave-one-out influence scorer that approximates the causal effect of each section on task success.

```rust
/// Leave-one-out section influence scorer.
///
/// For each prompt section, tracks whether its presence correlates with
/// task success. Sections with positive lift get higher token budgets;
/// sections with negative lift get dropped or deprioritized.
pub struct SectionInfluence {
    sections: HashMap<String, SectionRecord>,
    pub min_observations: u32,  // default: 10
}
```

True leave-one-out would require re-running each task without each section, which is prohibitively expensive. Instead, the system observes natural variation: some tasks include a section (because it was available), others do not (because context was missing or budget pressure dropped it). Over many observations, this approximates the causal effect.

The `weights()` method returns per-section multipliers in `[0.5, 1.5]`. These feed directly into the `PromptComposer` to adjust section token caps and prioritization.

### C-factor metric

Located in `roko-learn/src/cfactor.rs`. The C-factor (Collective Factor) measures whether the multi-agent collective performs better than its best individual. It is defined as:

```
C = (1/K) * SUM(collective_score_k / best_individual_score_k)
```

Where K is the number of task types and the scores are pass rates. C > 1.0 means the collective outperforms any individual agent. C < 1.0 means coordination overhead is destroying value.

The C-factor includes per-agent leave-one-out Shapley attribution (from `roko-learn/src/shapley.rs`): for each agent, compute the collective C-factor with and without that agent's episodes. The difference is the agent's marginal contribution.

### Shapley value attribution

Located in `roko-learn/src/shapley.rs`. Implements exact Shapley values for n <= 12 agents (O(2^n * n)) and Monte Carlo approximation for larger groups (O(samples * n^2)).

Shapley values satisfy four axioms: efficiency (values sum to total outcome), symmetry (equal contributors get equal shares), null player (zero-contribution agents get zero), and additivity (values from independent games compose). This makes Shapley the theoretically optimal attribution method for fair credit distribution in multi-agent plans.

For context engineering, Shapley attribution answers: "When multiple sections were present simultaneously, how much of the task's success should be attributed to each?" The `SectionEffectivenessRegistry` answers a simpler question (was the section present?), but Shapley handles the harder case where section interactions matter.

---

## 7. Cache architecture

Context assembly is expensive. Building the full context pack -- workspace map, PRD extract, plan content, playbook matches, research prepass -- takes 500ms to 2s. Caching prevents redundant assembly when two agent spawns share inputs.

### Local cache: four tiers

The cache architecture uses four stability tiers, aligned with the `CacheLayer` enum in `roko-compose/src/prompt.rs`:

```rust
pub enum CacheLayer {
    /// System prompt, role instructions, tool definitions.
    Role = 0,
    /// Workspace map, cross-plan context, durable project context.
    Workspace = 1,
    /// Plan/task brief content that is stable within a plan.
    Plan = 2,
    /// Turn-local content such as review feedback or error output.
    Volatile = 3,
}
```

These map to cache tiers:

| Tier | CacheLayer | Content | Hit rate | Invalidation |
|------|-----------|---------|----------|--------------|
| L0: Volatile | Volatile | Per-turn unique content (review feedback, error output, iteration memory) | Never cached | Every turn |
| L1: Role | Role | System prompt, role identity, tool definitions, conventions | ~90% after first request | On agent role change or tool set change |
| L2: Workspace | Workspace | Structure map, cross-references, workspace state | ~70% within a plan run | On workspace file change (tracked by `notify::RecommendedWatcher` in `tui/fs_watch.rs`) |
| L3: Plan | Plan | PRD extract, plan brief, task content | ~80% within a plan run | On plan revision |

The key insight: Roko assembles the prompt in cache-layer order (Role first, Volatile last) and uses `BTreeMap` for deterministic serialization within each layer. This guarantees that two prompts sharing the same Role + Workspace layers produce identical byte sequences for those layers, enabling provider-side KV cache prefix reuse.

The `ContextPackCache` in `roko-learn/src/context_pack_cache.rs` implements the in-memory tier with LRU eviction:

```rust
pub struct ContextPackCache {
    capacity: usize,
    path: PathBuf,
    inner: Mutex<Inner>,
}
```

It stores `ContextPack` entries keyed by a fingerprint string derived deterministically from the composition inputs (scope files, tags, mtimes, playbook IDs). Each pack carries:

- The assembled prompt text.
- Content hashes of the signals folded into the prompt (for audit/provenance).
- Token count.
- Access statistics (`created_at_ms`, `access_count`, `last_access_ms`).

The cache persists to disk via `tokio::fs` with atomic rename. On restart, the snapshot warms the in-memory tier so the first agent spawns hit cache instead of paying the full assembly cost.

### Dollar impact

For an 80-agent plan run (a typical medium-complexity PRD execution), cache reuse saves approximately $1.75 per run via provider-side KV cache prefix reuse. At 10 plan runs per day, that is $17.50/day or $525/month in inference costs eliminated through deterministic prefix construction alone.

### Inference gateway: two additional tiers

When Roko operates with an inference gateway (a proxy between agents and LLM providers), two additional cache tiers become available:

| Tier | Mechanism | Hit rate | Savings |
|------|-----------|----------|---------|
| L4: Deterministic | SHA-256 hash of the full prompt. Exact duplicate detection. | ~10% of L3 misses | 100% (cached response returned) |
| L5: Semantic | Embedding similarity > 0.92. Near-duplicate detection for prompts that differ in whitespace, ordering, or minor phrasing. | ~30% of L4 misses | 100% (cached response returned) |

L4 is cheap: hash comparison is O(1). L5 requires embedding computation (~5ms per prompt via a local embedding model), but the savings from avoiding a full LLM inference call vastly exceed the embedding cost.

---

## 8. U-shaped placement

LLMs do not attend uniformly to their context window. Liu et al. (2023) demonstrated the "lost in the middle" effect: models attend strongly to the beginning and end of their context but poorly to the middle. Performance on multi-document QA drops 15-25% for facts placed in the middle positions versus the first or last positions.

Roko implements the `PositionAttentionModel` in `roko-compose/src/attention.rs`:

```rust
pub struct PositionAttentionModel {
    /// Primacy contribution at the beginning of the prompt.
    pub primacy_weight: f64,   // default: 0.35
    /// Decay rate for primacy contribution.
    pub primacy_decay: f64,    // default: 3.0
    /// Recency contribution near the end of the prompt.
    pub recency_weight: f64,   // default: 0.30
    /// Decay rate for recency contribution.
    pub recency_decay: f64,    // default: 3.0
    /// Baseline attention across the full prompt.
    pub baseline: f64,         // default: 0.35
}
```

The attention multiplier at normalized position `p` in `[0.0, 1.0]`:

```
attention(p) = primacy_weight * exp(-primacy_decay * p)
             + recency_weight * exp(-recency_decay * (1 - p))
             + baseline
```

This produces a U-shaped curve with peaks at the start and end. The composition layer uses this to place sections:

```
[Priority 5] [Priority 4] [Priority 2] [Priority 1] [Priority 3] [Priority 4] [Priority 5]
 ^-- start                    ^-- middle (trough)                               ^-- end
```

The `dynamic_placement` function in the same module reassigns non-critical sections based on information density relative to the task query. Sections with high term overlap with the task description get placed near the edges; sections with low overlap go to the middle where attention loss matters less.

Critical sections (`SectionPriority::Critical`) always keep their assigned placement. The task description always goes at the end (recency position). Role instructions always go at the start (primacy position). Everything else is dynamically assigned.

### Per-model calibration

The `ModelAttentionCurves` struct stores fitted attention curves per model ID. Different models exhibit different attention patterns -- some (like Claude) have shallower U-curves than others (like GPT-4). As Roko accumulates data on which placements correlate with success per model, the curves are updated and persisted to `.roko/learn/attention-curves.json`.

---

## 9. Complexity-based context dropping

Not every task needs 24K tokens of context. The composition layer estimates task complexity and drops unnecessary sections for simpler tasks. This prevents the "context dilution" failure mode where the model's attention is spread across irrelevant sections instead of concentrated on the task.

The `Complexity` enum in `roko-compose/src/budget.rs`:

```rust
pub enum Complexity {
    /// Single-file, trivial change. Drop PRD, research, skills.
    Trivial,
    /// Standard multi-file task. Full budget at role defaults.
    Standard,
    /// Cross-crate or architectural work. Inflated budgets.
    Complex,
}
```

The mapping:

| Complexity | Token target | Included | Dropped | Budget adjustments |
|-----------|-------------|----------|---------|-------------------|
| Trivial | ~4K | Task + inline files | PRD extract, research, skills, cross-plan context | Workspace map halved, brief halved |
| Standard | ~25K | All sections at role defaults | Nothing dropped | Base budget used as-is |
| Complex | ~40K | All + surrounding file context | Nothing dropped | Workspace map +50%, cross-plan context +100%, file context +50% |

Complexity is estimated from four signals:

1. **Task file count.** Tasks touching 1 file are likely Trivial. Tasks touching 5+ files are likely Complex.
2. **Tier label.** The `OperatingFrequency` tier from `roko-core` maps directly: Surgical -> Trivial, Focused -> Standard, Full -> Complex.
3. **Domain.** Architecture and integration tasks default to Complex. Fix and rename tasks default to Trivial.
4. **HDC similarity to historical tasks.** If the current task's HDC fingerprint is highly similar (Hamming distance < 500) to a previously completed task, the system uses the prior task's actual token usage as a complexity indicator.

The `adjusted_budget_for` function in `roko-compose/src/budget.rs` computes the final per-section budget:

```rust
pub fn adjusted_budget_for(role: AgentRole, complexity: Complexity) -> AdjustedBudget {
    let mut budget = budget_for(role);
    match complexity {
        Complexity::Trivial => {
            budget.prd2 = 0;      // drop
            budget.context = 0;    // drop
            budget.skills = 0;     // drop
            budget.workspace_map /= 2;
            budget.brief /= 2;
        }
        Complexity::Standard => { /* use base budget */ }
        Complexity::Complex => {
            budget.workspace_map = budget.workspace_map.saturating_mul(3) / 2;
            budget.context = budget.context.saturating_mul(2);
            budget.file_context = budget.file_context.saturating_mul(3) / 2;
        }
    }
    // ... attach cache break hints
}
```

The result includes cache break hints at three boundaries:

- After `conventions` (end of System/Role layer).
- After `workspace_map` (end of Session/Workspace layer).
- After `file_context` (end of Task layer).

These boundaries tell the prompt renderer where to insert `<!-- cache:session -->` markers that downstream API calls can use to set `cache_control` for provider-side KV cache reuse.

---

## 10. Affect-modulated allocation

Agent affect state -- modeled by the Daimon subsystem via the PAD (Pleasure-Arousal-Dominance) dimensional model (Gebhard, 2005) -- modulates context allocation at auction time.

The `AffectModulation` struct in `roko-compose/src/auction.rs`:

```rust
pub struct AffectModulation {
    /// Arousal-derived urgency multiplier (range [0.5, 2.0]).
    pub urgency_multiplier: f64,
    /// Pleasure-derived valence bias (range [-1.0, 1.0]).
    pub affect_weight: f64,
}

impl AffectModulation {
    pub fn from_pad(pleasure: f64, arousal: f64) -> Self {
        Self {
            urgency_multiplier: (1.0 + arousal * 0.5).clamp(0.5, 2.0),
            affect_weight: pleasure.clamp(-1.0, 1.0),
        }
    }

    pub fn adjust_bid(&self, base_bid: f64, entry_valence: f64) -> f64 {
        let valence = entry_valence.clamp(-1.0, 1.0);
        base_bid * self.urgency_multiplier * (1.0 + self.affect_weight * valence)
    }
}
```

The formula: `adjusted_bid = base_bid * urgency * (1 + affect_weight * valence)`.

### Behavioral effects

**High arousal (stressed agent -- recovering from gate failure):**
- `urgency_multiplier` approaches 1.5-2.0.
- Warning sections (Risk category, valence = -0.8) get their bids multiplied by up to 2x.
- Anti-knowledge entries (things the agent learned NOT to do) get +15-20% effective budget.
- Exploratory context (Research category, Hypotheses) gets deprioritized because the agent should stabilize before exploring.

**High valence (confident agent -- on a success streak):**
- `affect_weight` is positive (0.3-0.8).
- Positive-valence entries (novel research, exploratory hypotheses) get bid boosts.
- Warning sections (negative valence) get bid penalties of 10-15%.
- This encourages exploration when the agent has earned confidence through prior successes.

**Low arousal, neutral valence (baseline):**
- `urgency_multiplier` = 1.0, `affect_weight` = 0.0.
- No modulation. Bids are determined purely by learned value and relevance.

The affect modulation connects the context system to the Daimon's ALMA temporal affect model (Gebhard, 2005). ALMA models affect decay over time, so a gate failure raises arousal sharply and then decays it back to baseline over subsequent successful turns. The context allocation tracks this trajectory: immediately after failure, the workspace is loaded with warnings and error context; after recovery, it relaxes back to normal allocation.

---

## 11. HDC-based context retrieval

Hyperdimensional computing (HDC) provides the fast similarity search that backs the NeuroContextBidder and PlaybookBidder. The HDC encoder in `roko-primitives` produces 10,240-bit binary vectors that represent semantic content. Hamming distance between vectors approximates semantic similarity.

### Retrieval pipeline

1. **Encode the current task.** The task title, description, file paths, and symbol names are concatenated and encoded as an HDC vector. Encoding takes approximately 5 microseconds on a single core.

2. **Query the local neuro store.** The neuro store maintains an index of knowledge entries with precomputed HDC fingerprints. A Hamming distance scan over the index finds the top-k most similar entries. For a store with 10,000 entries, this takes approximately 10 milliseconds.

3. **Query the Korai InsightStore (optional).** When a chain connection is available, the agent queries the InsightStore via the HTC (Hamming Threshold Comparison) precompile. The precompile computes Hamming distance in the EVM execution environment at approximately 170 microseconds per query at 10,000 entries for on-chain execution, or approximately 100 milliseconds via RPC for off-chain queries. This is fast enough to include in the context assembly hot path without blocking agent dispatch.

4. **Rank candidate sections.** The top-k results from both stores become candidate sections. Each candidate gets a relevance score based on Hamming similarity and a track record score from the `LearningBidder`'s Beta posterior.

5. **Submit to VCG auction.** The NeuroContextBidder packages each candidate as a `VcgBid` with value = relevance * track_record * prior_bid. The auction decides which candidates win allocation.

### Social foraging boost

The `MultiPatchForager` in `roko-compose/src/foraging.rs` implements optimal foraging theory for context retrieval. Each context source (local knowledge, chain InsightStore, inline files, recent signals) is modeled as a "patch" with a diminishing-returns gain curve:

```rust
pub struct SourceForagingProfile {
    pub source: ContextSource,
    pub g_max: f64,     // asymptotic relevance available
    pub lambda: f64,    // saturation rate
    pub travel_cost: f64, // setup/switching cost
}
```

The forager determines the optimal visitation order (highest initial gain first), the optimal number of iterations per source (via marginal value theorem: leave a patch when the marginal gain falls below the environment average plus switching cost), and whether a source is worth visiting at all.

An `active_inference_bias` parameter (range `[0.0, 1.0]`) modulates the exploration/exploitation tradeoff. Higher values lower the visit threshold (explore more patches) and shorten patch stays (explore more broadly). This implements a simplified expected-free-energy influence on foraging behavior (Friston, 2010).

When multiple agents work the same plan, the `social_foraging_boost` function applies capped boosts to entries that other agents found useful for similar task categories:

```rust
pub fn social_foraging_boost(
    candidate_entries: &mut [ContextChunk],
    recent_signals: &[RetrievalSignal],
    task_category: &str,
    decay_half_life: Duration,
) {
    // For each entry, find signals from other agents that:
    //   - Used this entry for the same task category
    //   - Had their gate pass
    // Apply a decaying relevance boost capped at +0.3
}
```

The cap prevents herding: social signals inform but do not dominate the retrieval ranking.

---

## 12. InsightStore context integration (Korai chain)

The InsightStore is Korai's on-chain knowledge substrate. Agents query it during context assembly to access collective intelligence from the network. This is the mechanism by which knowledge compounds across agents: the thousandth agent to solve a problem in a given domain inherits the distilled context policies, heuristics, and warnings that the first 999 agents discovered.

### Entry types

The InsightStore supports six entry types, each with different context engineering implications:

| Entry type | Description | Context category | Typical priority |
|-----------|-------------|-----------------|-----------------|
| Insight | A validated observation about a domain | Knowledge | 3 |
| Heuristic | A proven rule of thumb for a task pattern | Playbook | 3 |
| Warning | A known failure mode or anti-pattern | Risk | 4 |
| CausalLink | A validated A -> B causal relationship | Knowledge | 3 |
| StrategyFragment | A partial strategy for a domain | Strategy | 3 |
| AntiKnowledge | Something the agent learned NOT to do | Risk | 4 |

### Reputation-weighted scoring

Entries from higher-reputation submitters score higher in VCG bids. Reputation is computed from on-chain history: agents that submit entries which other agents later validate through successful task outcomes earn higher reputation. Agents that submit entries which correlate with failures lose reputation.

The scoring formula:

```
bid_value = relevance * track_record * reputation_weight * demurrage_factor
```

Where:
- `relevance` is the HDC similarity to the current task.
- `track_record` is the entry's historical success rate from the local Beta posterior.
- `reputation_weight` is the submitter's reputation score (0.0 to 1.0).
- `demurrage_factor` decays old entries: `0.5 ^ (age_days / half_life_days)`. Default half-life: 30 days. Older entries score lower unless independently confirmed by recent observations.

### Causal link composition

One of the InsightStore's most powerful features for context engineering: causal link composition. If agent A discovers that `API timeout -> retry with backoff` works, and agent B discovers that `retry with backoff -> exponential ceiling at 30s` works, then a third agent querying the InsightStore gets the composed insight `API timeout -> exponential ceiling at 30s` -- a transitive causal chain that no single agent discovered.

The NeuroContextBidder performs transitive closure over CausalLink entries during retrieval. If the current task matches the source of a causal chain, the entire chain becomes a candidate section. The bid value scales with chain length (longer chains represent more valuable distilled knowledge) but decays per hop to prevent unbounded growth.

### Competing fairly

Crucially, InsightStore entries compete in the same VCG auction as local knowledge. The chain is not given privileged access to the context window. If a local playbook is more relevant than a chain-sourced heuristic, the local entry wins. This prevents the pathological case where network-sourced context dilutes high-quality local context.

### NeuroContextBidder dual-source query

The `NeuroContextBidder` does not query the InsightStore in isolation. It queries both the local neuro store (in-process, sub-millisecond) and the Korai InsightStore (on-chain, ~100ms via RPC) in parallel, then merges the results into a single ranked candidate list.

**Local neuro store query.** The durable knowledge store in `roko-neuro` holds the agent's own accumulated knowledge: insights distilled from dream consolidation, playbooks extracted from successful episodes, anti-knowledge from failures. Query is by HDC similarity against the current task's fingerprint. Results are trusted implicitly (the agent generated them from its own experience).

**Korai InsightStore query via HTC precompile.** The on-chain query follows a specific pipeline:

1. **Encode.** Convert the current task description to an HDC vector using the same `ItemMemory` codebook the agent uses for local fingerprinting. This produces a 10,240-bit query vector.
2. **Submit.** Call the HTC precompile's `topk_similar(query_vector, k=20, min_similarity=0.75)` function. The precompile performs a brute-force Hamming distance search over all InsightStore entries in the current epoch.
3. **Receive.** The precompile returns up to 20 entries sorted by similarity, each carrying its entry type, content hash, submitter address, reputation score, and creation timestamp.
4. **Resolve.** For each returned entry, fetch the full content from the InsightStore's content-addressed storage (IPFS CID or inline for entries under 1KB).
5. **Score.** Apply reputation weighting and demurrage to produce final bid values.

**Six entry types and their context implications:**

| Entry type | What the agent receives | How it enters context |
|-----------|------------------------|----------------------|
| Insight | A validated domain observation (e.g., "Aave V3 rate oracle has 15-minute lag during high-volatility periods") | Injected as Knowledge category section |
| Heuristic | A proven rule of thumb (e.g., "For cross-crate refactors, always update Cargo.toml before source files") | Injected as Playbook category section |
| Warning | A known failure mode (e.g., "Compound governance proposals that change interest rate models trigger cascading liquidations within 4 hours") | Injected as Risk category section with elevated priority |
| CausalLink | A validated A -> B relationship (e.g., "Ethena yield spike -> basis trade unwinding -> USDT depeg pressure") | Injected as Knowledge, composable with other CausalLinks |
| StrategyFragment | A partial execution strategy (e.g., "When rebalancing ETH/USDC LP, split into 3 tranches at 5-minute intervals to minimize slippage") | Injected as Strategy category section |
| AntiKnowledge | Something proven NOT to work (e.g., "Do not use multicall for Aave flashloan repayments -- gas estimation fails silently") | Injected as Risk category section with highest priority |

**CausalLink composition in detail.** The transitive closure over CausalLinks is one of the InsightStore's most distinctive properties for context engineering. Consider three independent agent discoveries:

- Agent A discovers: `API timeout -> retry with exponential backoff`
- Agent B discovers: `retry with exponential backoff -> cap at 30-second ceiling`
- Agent C discovers: `30-second ceiling exceeded -> circuit break and alert`

The NeuroContextBidder composes these into a single causal chain: `API timeout -> retry with exponential backoff -> cap at 30-second ceiling -> circuit break and alert`. Agent D, encountering an API timeout for the first time, receives the complete recovery strategy -- a strategy that no single agent discovered end-to-end.

The composition algorithm:

1. Start with CausalLinks whose source matches the current task (HDC similarity > 0.8).
2. For each matching link, check if its target is the source of another CausalLink.
3. Follow the chain up to 5 hops (configurable via `max_causal_depth` in roko.toml).
4. Score the composed chain: `base_score * (0.9 ^ hop_count)` -- each hop applies a 10% discount to account for increasing uncertainty.
5. Deduplicate chains that reach the same conclusion through different paths (keep the highest-scoring path).

**Reputation weighting.** Every InsightStore entry carries the submitter's reputation score at time of submission. Higher-reputation submitters' entries receive a multiplicative boost in bid scoring:

```
reputation_multiplier = 0.5 + (reputation_score * 0.5)
```

This means a reputation-0.0 agent's entries bid at 50% strength, while a reputation-1.0 agent's entries bid at full strength. The floor of 0.5 prevents complete dismissal of low-reputation entries -- new agents with no reputation can still contribute useful knowledge.

**Pheromone demurrage.** Inspired by ant colony optimization, InsightStore entries lose potency over time unless independently confirmed:

```
demurrage_factor = 0.5 ^ (age_days / half_life_days)
```

Default half-life: 30 days. An entry that is 60 days old with no confirmations bids at 25% of its original strength. But each independent confirmation (another agent validates the entry through successful task outcome) resets the clock. An entry confirmed yesterday bids at full strength regardless of when it was originally created. This mechanism ensures the InsightStore naturally purges stale knowledge while preserving knowledge that remains relevant.

---

## 13. Context mesh (cross-agent sharing)

Within a single plan run, multiple agents work concurrently. The `ContextMesh` in `roko-compose/src/context_mesh.rs` enables cross-agent context sharing within the plan scope.

```rust
pub struct ContextMesh {
    inner: Arc<Mutex<MeshState>>,
}
```

Agents publish discoveries, errors, and patterns to the mesh:

```rust
mesh.publish("agent-3", "error", "build failed: missing import in crate X", 0.9, now_ms);
mesh.publish("agent-5", "pattern", "use builder pattern for config structs", 0.7, now_ms);
```

Other agents query the mesh and receive entries sorted by relevance, excluding their own publications (to prevent echo):

```rust
let entries = mesh.query("agent-7", "error", 5); // top 5 errors from other agents
let sections = ContextMesh::to_prompt_sections(&entries);
```

Cross-agent deduplication prevents the same knowledge from appearing in multiple agents' prompts simultaneously. The deduplication uses Jaccard-like content overlap (tokenize both entries, compute intersection/union):

```rust
pub fn deduplicate(entries: Vec<SharedContextEntry>) -> Vec<SharedContextEntry> {
    // For entries with the same topic and >60% content overlap,
    // keep only the higher-relevance entry.
}
```

Stale entries are evicted based on age:

```rust
mesh.evict_stale(now_ms, max_age_ms);
```

The mesh is thread-safe (`Arc<Mutex<_>>`) for concurrent access from the `MultiAgentPool`. It sits below the VCG auction in the data flow: mesh entries become candidate sections that the NeuroContextBidder bids for, competing alongside local knowledge and chain-sourced entries.

---

## 14. Measurement

Context engineering improvements must be measured, not assumed. The following metrics define success:

### Primary metric: task pass rate lift

Run A/B tests comparing learnable allocation against static baseline allocation on identical tasks. The test protocol:

1. Select a task set of 100+ tasks across at least 3 domains (code, research, chain).
2. Run each task twice: once with the learnable context system, once with static per-role templates at fixed allocations.
3. Measure gate pass rate for each group.
4. Target: the learnable system achieves >= 10% higher pass rate than static allocation.

### Secondary metrics

**Token efficiency.** Track average tokens used per successful task. The learnable system should converge toward lower token usage over time as it learns to drop unhelpful sections and right-size budgets. Target: 15% fewer tokens per successful task after 200 observations.

**C-factor.** The collective factor `C = (1/K) * SUM(collective_score / best_individual_score)` across task types. A C-factor above 1.0 means the multi-agent collective with shared context outperforms any individual agent. Target: C >= 1.15 (15% collective uplift).

**InsightStore query utility.** For tasks where chain-sourced context was included, compare the pass rate against tasks where it was excluded (natural experiment via budget pressure). If chain-sourced context has zero or negative lift after 100 observations, re-examine the retrieval ranking.

**Section effect convergence.** After N observations, the `SectionEffectivenessRegistry` should stabilize: the lift values for high-impact sections should have small confidence intervals (< 0.05 standard error). If they oscillate, the system is either in a non-stationary environment or the measurement methodology has confounders.

**Cache hit rate.** Target: L1 (Role) >= 85%, L2 (Workspace) >= 60%, L3 (Plan) >= 70% within a plan run. Below these thresholds, investigate whether the fingerprint function is too sensitive to input changes.

**Auction diagnostics.** Track budget utilization (should be > 85% -- under-utilization means the budget is too large or too few bidders are competing) and Pareto optimality (should be true > 90% of the time -- if not, the greedy allocation is making poor tradeoffs).

---

## 15. Integration map

The context engineering system touches multiple crates. Here is how the pieces connect in the runtime data flow:

```
orchestrate.rs (roko-cli)
    |
    +--> BudgetPredictor.predict(features)     [roko-compose/budget_predictor.rs]
    |        returns: total token budget
    |
    +--> adjusted_budget_for(role, complexity)  [roko-compose/budget.rs]
    |        returns: per-section caps + cache breaks
    |
    +--> ContextProvider.assemble(task_input)   [roko-compose/context_provider.rs]
    |    |   queries: neuro store, InsightStore, symbol resolver
    |    |   returns: Vec<ContextSection>
    |    |
    |    +--> MultiPatchForager.optimal_order() [roko-compose/foraging.rs]
    |    |       returns: source visitation order
    |    |
    |    +--> ContextMesh.query(agent, topic)   [roko-compose/context_mesh.rs]
    |    |       returns: cross-agent context entries
    |    |
    |    +--> ContextAssembler.assemble()       [roko-neuro -> re-exported]
    |            returns: Vec<ContextChunk> with PAD state
    |
    +--> LearningBidder.bid(section, relevance) [roko-compose/auction.rs]
    |        returns: per-section bid values
    |
    +--> AffectModulation.from_pad(p, a)        [roko-compose/auction.rs]
    |        returns: urgency + valence bias
    |
    +--> vcg_allocate(bids, budget, modulation) [roko-compose/auction.rs]
    |        returns: VcgAllocation (winners, excluded, payments)
    |
    +--> dynamic_placement(sections, query)     [roko-compose/attention.rs]
    |        mutates: section placement (U-shaped)
    |
    +--> SystemPromptBuilder.build()            [roko-compose/system_prompt_builder.rs]
    |        returns: assembled system prompt with 9 layers
    |
    +--> ContextPackCache.put(fingerprint, pack)[roko-learn/context_pack_cache.rs]
    |        caches: assembled workspace for reuse
    |
    +--> [agent dispatch via roko-agent]
    |
    +--> [gate pipeline returns verdict]
    |
    +--> SectionEffectivenessRegistry.record()  [roko-learn/section_effect.rs]
    |        records: which sections were present + outcome
    |
    +--> SectionInfluence.record()              [roko-compose/budget_predictor.rs]
    |        records: leave-one-out influence data
    |
    +--> LearningBidder.update()                [roko-compose/auction.rs]
    |        updates: Beta posteriors for next invocation
    |
    +--> BudgetPredictor.record()               [roko-compose/budget_predictor.rs]
             updates: EMA token usage for next prediction
```

---

## 16. Persistence layout

All learned context data persists under `.roko/learn/`:

| File | Module | What | Format |
|------|--------|------|--------|
| `section-effects.json` | `roko-learn/section_effect.rs` | Section inclusion/exclusion outcomes per (section, role) | JSON, atomic rename |
| `section-influence.json` | `roko-compose/budget_predictor.rs` | Leave-one-out influence scores | JSON |
| `budget-predictor.json` | `roko-compose/budget_predictor.rs` | EMA token usage per feature key | JSON |
| `context-policy.json` | `roko-compose` (planned) | Evolved allocation policy from Loop 2 | JSON |
| `attention-curves.json` | `roko-compose/attention.rs` | Per-model fitted U-curve parameters | JSON |
| `cascade-router.json` | `roko-learn/cascade_router.rs` | Model routing data (includes context tier feedback) | JSON |
| `gate-thresholds.json` | `roko-learn` | Adaptive gate thresholds (EMA per rung) | JSON |
| `experiments.json` | `roko-learn/prompt_experiment.rs` | A/B prompt experiment state | JSON |
| `efficiency.jsonl` | `roko-learn/efficiency.rs` | Per-turn efficiency events | JSONL, append-only |

Additionally, the context pack cache persists to `.roko/memory/context-packs.json` for warm restart.

---

## 17. Academic references

The context engineering system draws on research from mechanism design, cognitive science, information retrieval, and machine learning:

1. **VCG mechanism.** Vickrey, W. (1961). "Counterspeculation, auctions, and competitive sealed tenders." Clarke, E.H. (1971). "Multipart pricing of public goods." Groves, T. (1973). "Incentives in teams." The VCG mechanism is the unique mechanism satisfying efficiency, incentive compatibility, and individual rationality for combinatorial allocation problems.

2. **Lost in the middle.** Liu, N.F. et al. (2023). "Lost in the Middle: How Language Models Use Long Contexts." Demonstrated that LLMs attend more to the beginning and end of their context windows, with a 15-25% performance drop for information placed in the middle. This motivates Roko's U-shaped placement strategy.

3. **Effective context engineering.** Anthropic (2025). "Building Effective Agents." Introduced the "context engineering" framing and the finding that context quality dominates model quality for agent performance. The 1,000-token vs. 100,000-token insight directly motivates budget-constrained allocation.

4. **Agentic context engineering.** Fourney, A. et al. (2025). "Agentic Context Engineering: A Framework for LLM-Based Agents." arXiv:2510.04618. Formalizes context engineering as the central design problem for LLM-based agents. Introduces the hierarchy of context management (static, dynamic, adaptive) that maps to Roko's three feedback loops.

5. **Meta-Harness.** Lee, J. et al. (2026). "Meta-Harness: A Unified Scaffold for Cross-Model Evaluation." arXiv:2603.28052. Proved that a single scaffold change can improve accuracy by 7.7 points across five model families while using 4x fewer tokens. The strongest published evidence that scaffold engineering dominates model selection.

6. **Active inference for context selection.** Friston, K. (2010). "The free-energy principle: a unified brain theory?" The expected-free-energy framework motivates the `active_inference_bias` parameter in the foraging module: under uncertainty, the agent explores more context sources (patches) rather than exploiting the current best source deeply.

7. **Shapley value attribution.** Shapley, L.S. (1953). "A Value for n-Person Games." The unique attribution method satisfying efficiency, symmetry, null player, and additivity axioms. Used in Roko for fair multi-agent credit distribution and for multi-section attribution when interaction effects are present.

8. **Marginal value theorem.** Charnov, E.L. (1976). "Optimal Foraging, the Marginal Value Theorem." The theoretical basis for the `MultiPatchForager`'s stopping rule: leave a source when the marginal gain falls below the environment average plus travel cost.

9. **ALMA temporal affect model.** Gebhard, P. (2005). "ALMA -- A Layered Model of Affect." Provides the PAD dimensional model and temporal decay dynamics that underpin the Daimon's affect-modulated context allocation.

10. **Thompson sampling.** Thompson, W.R. (1933). "On the Likelihood that One Unknown Probability Exceeds Another in View of the Evidence of Two Samples." The `LearningBidder` uses a deterministic approximation of Thompson sampling to balance exploration (trying sections with uncertain value) and exploitation (preferring sections with known high value).

---

## 18. Open questions

1. **Loop 2 frequency.** The spec sets policy evolution at every 50 tasks. This is a guess. Too frequent and the policy oscillates; too infrequent and it adapts slowly to distribution shifts. Need empirical data from production runs.

2. **Affect modulation strength.** The current formula applies up to 2x urgency multiplier at maximum arousal. This might be too aggressive -- a stressed agent that floods its context with warnings may crowd out the actual task content. Need to run controlled experiments varying the multiplier range.

3. **InsightStore latency budget.** On-chain queries via RPC add approximately 100ms to context assembly. For agents operating in latency-sensitive trading domains, this may be unacceptable. Options: pre-fetch and cache InsightStore results per task category, or skip chain queries for sub-second dispatch targets.

4. **Confounders in leave-one-out.** The `SectionInfluence` tracker relies on natural variation in section inclusion. But section inclusion is not random -- it is determined by budget pressure, which correlates with task complexity. A section that is only excluded from complex tasks (due to budget pressure) may show artificially negative lift because complex tasks have lower pass rates. Need to add complexity normalization.

5. **Cross-agent context privacy.** The `ContextMesh` shares all published entries with all agents in the plan. In multi-tenant environments, some entries may contain sensitive information (API keys, proprietary strategies). Need access control beyond the current topic-based filtering.
