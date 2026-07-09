# Self-Improvement Systems: From Measurement to Autonomous Optimization

> **Audience**: ML engineers, optimization researchers, agent architects
> **Scope**: Four concrete optimization techniques, compound improvement timelines, and the SICA meta-improvement pattern

---

## The Four Techniques (Ordered by Complexity)

### 1. Autoresearch (Brute Force, Offline)

**What**: Change one variable at a time. Measure. Keep or revert.

**Cadence**: Weekly batch of 50-100 template variants against frozen historical tasks.
**Cost**: ~$5-15 per batch at Haiku tier.
**Discipline**: One Variable at a Time — change exactly one prompt section, one config parameter, or one model choice per experiment.

**Example progression**:
- Template A: 0.72 pass rate (baseline)
- Template B: 0.81 (concise instructions work better)
- Template C: 0.86 (added verification checklist)
- Template D: 0.84 (too verbose — reverted to C)

**Research**: Karpathy's `autoresearch` (2026) — 17 hours, 700 experiments, independently rediscovered RMSNorm and tied embeddings. Shopify Liquid: 93 automated commits → 53% faster rendering, 61% fewer allocations.

### 2. Bayesian Optimization (Intelligent Search, Continuous Parameters)

**What**: Gaussian Process surrogate model + Expected Improvement (EI) acquisition function.

**Requires**: 200+ data points and continuous parameter space.
**Implementation**: Optuna library with Tree-structured Parzen Estimators (TPE).

**Comparison**:
- Random search: 50 trials → best score 0.81
- Bayesian optimization: 25 trials → best score 0.89 (better result in half the trials)

**Applied to retrieval parameters** (discovered empirically per task category):

| Category | Context Budget | Semantic Weight | HDC Threshold | Repo Map Symbols | Compression |
|---|---|---|---|---|---|
| VaultDeployment | 8K | 0.7 | 0.75 | 30 | 0.4 |
| HookDevelopment | 12K | 0.5 | 0.80 | 50 | 0.3 |
| TokenSwap | 4K | 0.8 | 0.70 | 15 | 0.6 |
| BridgeOperation | 10K | 0.6 | 0.85 | 40 | 0.35 |
| SecurityAudit | 15K | 0.3 | 0.90 | 80 | 0.2 |

Humans can guess "bridges need more warnings." They cannot guess `warning_boost=2.5` (not 2.0, not 3.0) per domain.

### 3. Multi-Armed Bandits (Real-Time, Production)

**What**: UCB1 formula balances exploitation (use what works) with exploration (try alternatives).

```
score(arm) = mean_reward(arm) + C × √(ln(total_pulls) / pulls_of_arm)
```
Where C = √2 (exploration constant).

**Applications**: Backend selection, prompt template, retry strategy, context size, compression level.

**Three-month regime change example**:
- Month 1: Claude=0.72, Cursor=0.65, Codex=0.68 (few observations, high uncertainty)
- Month 2: Claude=0.81 (80% of tasks), Cursor=0.74 (15%), Codex=0.65 (5%)
- Month 3 (Day 61: Codex gets model update): Codex mean rises to 0.86, allocation shifts automatically — within days, Codex gets 40% of tasks

No human reconfigured anything. The bandit detected the regime change from reward signal alone.

### 4. Reinforcement Learning (Sequential Decisions, Long Horizon)

**What**: Learn the optimal retry policy from 500+ failure episodes.

**State representation** after failed task:
```
task_category, failure_type, failure_count, current_model_tier, error_pattern,
similar_task_success_rate, budget_remaining_pct, reflection_available, time_elapsed_seconds
```

**Four learned policies** (from episode data):

| State | Action | Recovery Rate |
|---|---|---|
| Compile errors + reflection available | Retry with reflection + same model | 78% |
| Test failures after 2+ retries | Escalate to Opus | 84% |
| No reflection + low budget | Generate reflection first | 71% |
| 3+ repeated failures | Rewrite task decomposition | 62% |

---

## Compound Improvement Timeline

| Timeline | Score | What Happened |
|---|---|---|
| Day 1 | ~0.65 | Baseline: default templates, random backend |
| Week 1 | ~0.75 | Bayesian optimization discovers per-category retrieval params |
| Week 2 | ~0.82 | Autoresearch finds winning prompt template |
| Month 1 | ~0.88 | Bandit converges on per-category backend selection |
| Month 3 | ~0.91 | RL retry policy matures with 500+ failure episodes |

**Multiplicative compounding**: Four independent 10% improvements: 0.9⁴ = 0.66 → **34% fewer failures** total. Each mechanism improves independently AND improves the signal quality for others.

---

## The SICA Pattern (Self-Improving Coding Agent)

**Research**: Robeyns et al. (ICLR 2025 Workshop) — archive-based self-improvement.

**Architecture**:
1. Maintain archive of agent configurations, each with benchmark scores
2. Best-performing configuration becomes meta-agent proposing next improvement
3. Improvement evaluated on full task distribution
4. If better: enters archive. If worse: discarded.

**Result**: 17% → 53% on SWE-bench Verified through iterative self-improvement.

**Key insight**: Improvements must be evaluated against the FULL task distribution, not just easy tasks. Optimizing for easy tasks degrades hard-task performance (Goodhart's Law).

---

## Dream Consolidation Cycle (Offline, 8 Steps)

During downtime (between plan executions), the system runs offline analysis:

1. **Ingest**: Process daily episodes into unified event stream
2. **Cluster**: Group similar episodes via HDC fingerprints (k-medoids)
3. **Cross-plan analysis**: Identify patterns invisible to individual plans
4. **Model profiling**: Update model performance profiles per task type
5. **Error taxonomy**: Generate error pattern taxonomies with mitigations
6. **Context optimization**: Determine which prompt sections predict success
7. **Skill extraction**: Identify reusable patterns from successful plans
8. **Pruning**: Remove stale knowledge contradicted by recent evidence

**Output artifacts** (updated nightly):
- Error taxonomy (JSON) → Playbook rule generator
- Model performance profiles (SQLite) → Bandit arm priors
- Context section weights (TOML) → Scaffold assembler
- Skill library entries (Markdown + code) → Context engine
- Stale pattern report (JSON) → Operator review

---

## Continual Learning Framework

| Strategy | What It Does |
|---|---|
| **Experience replay** | Rehearse old episodes during consolidation (prevent forgetting) |
| **Elastic weight consolidation** | Protect high-confidence patterns (10+ observations) from noisy updates |
| **Progressive growth** | Add new pattern categories when HDC similarity < 0.5 to all existing |
| **MAML-style meta-learning** | Initialize scaffold weights at MAML-optimized defaults for fast adaptation |

### Health Metrics

| Metric | Healthy Range | Alarm |
|---|---|---|
| Pattern library growth | 1-3 per 10 plans | >10 (noise) or 0 (stagnation) |
| Pattern contradiction rate | <5% per month | >15% (unstable) |
| Cold-start adaptation | <5 tasks to 80% converged | >15 tasks (poor meta-learning) |
| Forgetting rate | <2% score drop per month | >5% (catastrophic forgetting) |

---

## The Autocatalytic Loop

```
Gate outcomes → Episode storage → Pattern extraction → Playbook rules
  → Scaffold weights → Better prompts → Better outcomes → (loop)
```

Each 10% improvement at any point multiplies with improvements elsewhere. The system accelerates itself — better learning produces better outcomes which produce richer learning data.

---

## The Data Directory: What Gets Persisted

All learning state lives under `.roko/` in the project root:

```
.roko/
├── state/
│   └── executor.json           # Crash-recovery checkpoint (plan phases, task progress)
├── episodes.jsonl              # Agent turn recordings (append-only)
├── signals.jsonl               # Signal DAG (content-addressed, with lineage)
├── learn/
│   ├── efficiency.jsonl        # Per-turn metrics (tokens, cost, tools, timing, outcome)
│   ├── cascade-router.json     # 3-stage model routing state (static → confidence → UCB)
│   ├── gate-thresholds.json    # Adaptive EMA per gate rung
│   ├── experiments.json        # A/B prompt experiment variants + results
│   ├── costs.jsonl             # Cost records per model per provider
│   ├── playbook.json           # Validated behavioral rules (injected into prompts)
│   ├── playbook-rules.json     # Rule confidence tracking
│   ├── skills.json             # Reusable tool-use patterns with confidence scores
│   ├── patterns.json           # Mined patterns from episodes (HDC clustering)
│   ├── provider-health.json    # Circuit breaker state per provider
│   ├── latency-stats.json      # Per-model, per-provider latency percentiles
│   ├── section-effects.json    # Which prompt sections correlate with success
│   ├── model-experiments.json  # Model A/B test results
│   └── routing.jsonl           # Routing decision log (why was this model picked?)
├── prd/                        # PRD storage
├── research/                   # Research artifacts
└── memory/                     # Agent memory (session persistence)
```

**Data lifecycle**: Episode logs older than 90 days are compacted. Playbook rules below 0.1 confidence are pruned. Patterns contradicted by recent evidence are removed. Everything else persists indefinitely.

**Portability**: The entire `.roko/learn/` directory can be exported as a "brain dump" and imported into another instance via CRDT merge.

---

## Configuration Reference (Key Settings)

### Model/Provider Selection

```toml
[agent]
default_model = "claude-sonnet-4-6"
fallback_model = "claude-haiku-4-5"
effort = "high"                         # low | medium | high | max
timeout_ms = 300000

[agent.tier_models]
mechanical = "claude-haiku-4-5"
focused = "claude-sonnet-4-6"
integrative = "claude-sonnet-4-6"
architectural = "claude-opus-4-6"

[agent.escalation]
max_retries = 3
escalate_model = true                   # mechanical → focused → integrative → architectural
```

### Per-Role Overrides

```toml
[role_models]
implementer = "claude-opus-4-6"
strategist = "claude-sonnet-4-6"
auto-fixer = "claude-sonnet-4-6"
critic = "claude-haiku-4-5"

[role_effort]
implementer = "High"
architect = "Max"
critic = "Low"

[role_context_k]
critic = 80
scribe = 100
quick-reviewer = 60
```

### Execution Presets (CLI Flags)

| Preset | Model | Iterations | Reviews | Optimization |
|---|---|---|---|---|
| `--preset quality` | Opus | 5 | Full pipeline | Balanced |
| `--preset balanced` | Sonnet | 3 | Standard | Balanced |
| `--preset cost` | Sonnet | 2 | Skip reviews | Token minimization |
| `--preset speed` | Sonnet | 2 | Express mode | Latency minimization |

---

## The Efficiency Monitoring Dashboard

### Per-Turn Metrics (AgentEfficiencyEvent)

Every agent turn records 20+ fields:

```rust
pub struct AgentEfficiencyEvent {
    // Identity
    pub agent_id: String,
    pub role: String,
    pub backend: String,
    pub model: String,
    pub plan_id: String,
    pub task_id: String,

    // Tokens
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,

    // Cost
    pub cost_usd: f64,
    pub cost_usd_without_cache: f64,  // Counterfactual: what it WOULD have cost

    // Prompt composition
    pub prompt_sections: Vec<PromptSectionMeta>,  // name, tokens, priority, truncated, dropped
    pub total_prompt_tokens: u64,
    pub system_prompt_tokens: u64,

    // Tool utilization
    pub tools_available: u32,
    pub tools_used: u32,
    pub tool_calls: Vec<ToolCallMeta>,  // name, duration_ms, result_tokens, succeeded

    // Timing
    pub wall_time_ms: u64,
    pub time_to_first_token_ms: u64,
    pub was_warm_start: bool,

    // Outcome
    pub iteration: u32,
    pub gate_passed: bool,
    pub outcome: String,
    pub gate_errors: Vec<String>,
    pub model_used: String,
    pub timestamp: String,
}
```

### Computed Metrics

```
cache_hit_rate = cache_read_tokens / input_tokens
tool_utilization = tools_used / tools_available
cache_savings_usd = cost_usd_without_cache - cost_usd
```

### Efficiency Grading

```
composite = signal_ratio × 0.4 + budget_headroom × 0.2 + cache_efficiency × 0.2 + outcome × 0.2

Grade A: composite >= 0.75 (excellent)
Grade B: composite >= 0.50 (good)
Grade C: composite >= 0.25 (fair)
Grade D: composite < 0.25 (poor)
```

### Per-Role Cost Profiles

Aggregated from efficiency events:

```rust
pub struct RoleCostProfile {
    pub role: String,
    pub observations: u64,
    pub avg_input_tokens: f64,
    pub avg_output_tokens: f64,
    pub avg_cache_hit_rate: f64,
    pub avg_cost_usd: f64,
    pub p95_cost_usd: f64,           // 95th percentile cost
    pub cost_per_pass: f64,           // Total cost / successful passes
    pub avg_tool_utilization: f64,
    pub avg_wall_time_ms: f64,
    pub warm_start_pct: f64,
    pub pass_rate: f64,
}
```

The `cost_per_pass` metric is the most actionable: **how much does a successful outcome cost?** A model with 90% pass rate at $0.50/task has cost_per_pass = $0.56. A model with 60% pass rate at $0.10/task has cost_per_pass = $0.17 — cheaper despite lower pass rate.

---

## Six Scaffolding Principles (Meta-Harness Research)

**Evidence**: Lee et al. (Stanford, 2026) — 6× performance gap between best and worst scaffold. Same model (Sonnet) + good scaffold beats unscaffolded Opus at 1/10th cost.

### 1. Design Tools for the Model, Not Humans
- Meta-Tool pattern: 2 meta-tools outperform 29 tool schemas (49% → 74%, 98.7% token reduction)
- Structured search with JSON results, not raw grep
- Syntax-aware edit validation — reject before applying
- Explicit "no output" message when Unix commands succeed silently

### 2. Give the Right Context, Not More Context
- Lost-in-the-middle: 30%+ degradation when relevant info is in middle
- Unnecessary context: -3% success, +20% cost
- PageRank repo map: 1K tokens > 100K full contents

### 3. Validate Before Executing
- Validation stack (ordered by speed): syntax (<10ms) → linter (<1s) → type check (1-10s) → build (10-60s) → tests (10-300s)
- Fail fast at first failure

### 4. Compress History When Context Gets Long
- Keep last 2-3 turns verbatim
- Summarize older turns to key facts
- Prevent lost-in-middle from long conversation accumulation

### 5. Graduate Autonomy
- Suggest → Auto-edit → Full-auto
- Risk classification: read (always) → write (auto-edit) → execute (full-auto) → destructive (always confirm)

### 6. Close the Feedback Loop
- Write → Execute → Observe → Iterate
- Reflexion (Shinn et al., NeurIPS 2023): 80% → 91% on HumanEval
- Structured error analysis, not raw compiler output

---

## Research Citations

| Paper | Year | Key Finding |
|---|---|---|
| Lee et al. (Meta-Harness) | 2026 | 6× gap from scaffold; 4× fewer tokens |
| Shinn et al. (Reflexion) | NeurIPS 2023 | 80% → 91% on HumanEval via self-reflection |
| Qu et al. (Meta-Tool) | 2025 | 2 tools > 29 tools (49% → 74%) |
| Liu et al. (Lost in Middle) | TACL 2024 | 30%+ degradation from middle placement |
| Robeyns et al. (SICA) | ICLR 2025 | 17% → 53% SWE-bench via self-improvement |
| Karpathy (autoresearch) | 2026 | 700 experiments, 17 hours, autonomous |
| Khattab et al. (DSPy) | 2023 | 25-65% improvement from prompt compilation |
| Lightman et al. (PRM800K) | 2023 | Process verification > outcome verification |
| Huang et al. | ICLR 2024 | LLMs cannot self-correct without external feedback |
| Pan et al. | ICML 2024 | Self-refinement causes reward hacking |
| Finn et al. (MAML) | 2017 | Meta-learning for few-shot adaptation |

---

## Gate-to-Scaffold Feedback Loop

### The Problem: Gates Detect Failures, But Don't Prevent Them

The gate pipeline catches errors after the agent produces output. This is necessary but insufficient — the same error categories recur because the scaffold (which assembles the agent's context) doesn't learn from gate outcomes. The Gate-to-Scaffold Feedback Loop closes this gap.

### Mechanism: Section Attribution for Gate Failures

When a gate fails, the system performs **section attribution** — determining which context sections, if included, would have prevented the failure:

1. **Gate failure occurs**: The gate produces a structured error digest (error type, location, expected vs actual)
2. **Context audit**: The system inspects which prompt sections were included in the agent's context and which were excluded (due to budget constraints, low priority, or relevance filtering)
3. **Attribution analysis**: Compare the error type against a mapping of section → error-prevention capability
4. **Lift score update**: If a section was excluded that would have prevented the error, boost that section's **lift score** for the relevant task category

### Section-to-Error Prevention Mapping

| Error Type | Preventive Section | Mechanism |
|---|---|---|
| Missing import / unresolved reference | Dependency graph | Agent sees which crates/modules provide which symbols |
| Type mismatch | Type signatures of related functions | Agent sees expected types before writing |
| Trait bound violation | Trait implementation summary | Agent knows which traits are implemented for which types |
| Test failure (assertion mismatch) | Existing test patterns | Agent follows established assertion conventions |
| Clippy lint violation | Project lint configuration | Agent knows which lints are enforced |
| Style inconsistency | Codebase conventions section | Agent follows established patterns |

### Lift Score Dynamics

Each prompt section maintains a **lift score** per task category:

```
lift_score[section][category] += learning_rate × (1.0 - lift_score[section][category])
```

When a section's exclusion correlates with a gate failure, its lift score increases. When a section is included and the gate passes, no change (inclusion is the default positive case). When a section is included and the gate STILL fails, the lift score decreases slightly (the section didn't help).

The scaffold assembler uses lift scores to prioritize sections when the context budget is tight:

```
section_priority = base_priority × (1.0 + lift_score[section][current_category])
```

Sections with high lift scores get included even when the budget is tight. Sections with low lift scores get dropped first.

### Timescale: Per-Run Feedback

This feedback loop operates on a **per-run** timescale — the fastest learning loop in the system:

- Gate fails at iteration 1 of a task
- Section attribution runs immediately
- Lift scores update before iteration 2
- The scaffold for iteration 2 includes the preventive section
- Gate passes at iteration 2

The agent corrects its own context assembly within a single build session. No offline analysis needed. No human intervention needed.

### Interaction with Other Learning Loops

The Gate-to-Scaffold loop is the innermost feedback loop. It feeds into broader loops:

```
Per-run:    Gate failure → section attribution → lift score update → better scaffold (this loop)
Per-plan:   Aggregated lift scores → section priority rebalancing → better defaults
Per-week:   Bayesian optimization → global section weight optimization → better priors
Per-month:  Dream consolidation → error taxonomy update → better section-to-error mapping
```

Each loop operates at its own timescale. The per-run loop provides immediate correction. The longer loops provide strategic optimization.

---

## Learned Intervention Thresholds

### The Problem: Hardcoded Conductor Heuristics

The current conductor uses hardcoded rules for intervention decisions:

- "Abort after 5 consecutive failures"
- "Nudge (inject reflection) after 3 minutes idle"
- "Restart agent after OOM"
- "Escalate model after 2 retries"

These rules work for average cases but fail at the margins. A complex architectural task legitimately needs 6 attempts. A simple rename task should abort after 2. Hardcoded thresholds cannot adapt to task context.

### Proposed: RL-Style Learned Intervention Policy

Replace hardcoded rules with a learned policy that observes pipeline state and selects the optimal intervention:

**State representation**:
```
[
    task_category,          // enum: implementation, refactor, test, integration, etc.
    failure_count,          // 0-10+
    failure_type,           // enum: compile, test, clippy, timeout, etc.
    consecutive_same_error, // 0-5+
    current_model_tier,     // 0-3 (haiku → sonnet → sonnet → opus)
    budget_remaining_pct,   // 0.0-1.0
    elapsed_time_pct,       // fraction of allocated time consumed
    reflection_available,   // bool: has the agent produced a reflection?
    similar_task_pass_rate, // 0.0-1.0 from historical data
    plan_phase,             // enum: early, middle, late
]
```

**Action space**:
```
Continue      — let the agent try again with same configuration
Nudge         — inject a reflection prompt with error analysis
Restart       — kill agent, respawn with fresh context
Escalate      — upgrade to a higher model tier
Decompose     — split the task into smaller subtasks
Abort         — mark the task as failed, move on
```

**Reward signal**:
```
reward = task_success × 0.5
       + (1 - normalized_cost) × 0.3
       + (1 - normalized_time) × 0.2
```

A successful outcome with low cost and fast completion gets the highest reward. A successful outcome after 8 retries and model escalation gets a moderate reward (success matters, but efficiency also matters). A failed outcome after exhausting the budget gets zero reward.

### Adaptive Patience by Project Lifecycle

The learned policy naturally develops different intervention strategies based on context:

**Early in project lifecycle** (few episodes, low similar_task_pass_rate):
- The agent is learning the codebase — failures are expected
- Optimal policy: be patient, allow more retries, inject reflections
- Premature abort wastes the learning investment

**Late in project lifecycle** (many episodes, high similar_task_pass_rate):
- Patterns are established — failures indicate real problems
- Optimal policy: be aggressive, abort quickly, escalate or decompose
- Prolonged retries on a well-understood codebase waste budget

**During integration phases** (cross-crate tasks, high complexity):
- Integration failures are harder to diagnose — reflection helps more
- Optimal policy: nudge with cross-crate context, escalate to higher model tiers
- Simple retry rarely fixes integration issues

### Persistence

The learned intervention policy is persisted alongside other learning state:

```
.roko/learn/
├── cascade-router.json       # Model routing weights
├── gate-thresholds.json       # Adaptive gate EMA
├── intervention-policy.json   # Learned conductor policy (NEW)
└── ...
```

The policy serializes as a simple Q-table (state → action → expected reward). With ~1000 state combinations and 6 actions, the table is <100KB. Updates are incremental (online Q-learning with learning rate 0.1, discount factor 0.95).

---

## Bayesian Retrieval Tuning

### The Problem: Too Many Knobs in Context Assembly

The scaffold assembler has many tunable parameters that determine what context the agent receives:

| Parameter | Range | Effect |
|---|---|---|
| Section weights (per section) | 0.0 - 1.0 | How much priority each context section gets |
| Similarity threshold | 0.5 - 0.95 | Minimum relevance score for retrieved snippets |
| Diversity minimum | 0.0 - 0.5 | Minimum dissimilarity between retrieved snippets |
| Budget allocation (system vs user vs retrieved) | 0-100% each | How the token budget is split |
| Compression aggressiveness | 0.0 - 1.0 | How aggressively to summarize older context |
| Repo map symbol count | 10 - 200 | How many symbols to include in the repo map |
| History window (turns to keep verbatim) | 1 - 5 | How many recent turns to keep uncompressed |

Manually tuning these parameters is impractical. The space is high-dimensional, the interactions are nonlinear, and the optimal values differ by task category.

### Solution: Bayesian Optimization with Expected Improvement

**Bayesian Optimization** (Snoek et al., 2012) is designed for exactly this problem: optimizing expensive, noisy, black-box functions with minimal evaluations.

**Setup**:
- **Objective**: Maximize gate pass rate (the downstream metric that matters)
- **Parameters**: The weight vector described above (~15-20 continuous parameters)
- **Evaluation**: Each evaluation = a full agent run on a representative task from the category. Cost: ~$0.50-$2.00 per evaluation.
- **Surrogate model**: Gaussian Process (GP) mapping parameter configurations → expected gate pass rate
- **Acquisition function**: Expected Improvement (EI) — choose the next configuration that maximizes expected improvement over current best

### The Optimization Loop

```
1. Initialize with 10 random configurations (seed the GP)
2. Run each configuration on a representative task
3. Record (configuration, gate_pass_rate) as observation
4. Fit GP to all observations
5. Use EI to select next configuration to evaluate
6. Evaluate → record → fit → select → repeat
7. Stop after 50 evaluations or convergence (EI < 0.01)
```

With 50 evaluations at ~$1.00 each, the total optimization cost is ~$50. The resulting configuration improvement typically yields 10-20% higher gate pass rate — paid back within 25-50 tasks.

### Per-Regime Tuning

Different task categories have different optimal retrieval parameters. The system maintains separate optimized configurations:

| Task Category | Key Differences from Default |
|---|---|
| **Greenfield** (new code) | Higher repo map weight (need to understand structure), lower retrieval threshold (cast wide net) |
| **Refactoring** | Higher similarity threshold (need precise existing code), higher compression (less history needed) |
| **Integration** | Higher diversity minimum (need cross-crate context), larger history window (track multi-step reasoning) |
| **Bug fix** | Higher test section weight, lower compression (need full error context), more history |
| **Documentation** | Higher compression (large context, need overview), lower repo map (less code structure needed) |

### Warm-Starting New Categories

When a new task category appears (no historical data), the system warm-starts from the closest existing category:

```
new_category_config = weighted_average(
    existing_configs,
    weights = HDC_similarity(new_category_embedding, existing_category_embeddings)
)
```

The HDC similarity between task category embeddings provides a principled prior. A "migration" task warm-starts from "refactoring" (high similarity). A "security audit" task warm-starts from "code review" (moderate similarity).

### Research

Bayesian Optimization: Mockus (1975) — the original formulation. Gaussian Process surrogate model with Expected Improvement acquisition function. Snoek, Larochelle, Adams (2012) — "Practical Bayesian Optimization of Machine Learning Hyperparameters." Extended to high-dimensional spaces with automatic relevance determination (ARD) kernel.

---

## Cross-Domain Cognitive Transfer

### The Hypothesis: Domain-Independent Principles

When a DeFi agent learns "high-volatility markets require conservative position sizing," is that insight specific to DeFi? Or is it an instance of a domain-independent principle: "high uncertainty requires more caution"?

If it's domain-independent, then a coding agent facing high-complexity code should also adopt more caution (more review, smaller changes, more testing). The insight transfers across domains even though the surface-level details are completely different.

### The Unified Cognitive Architecture as Transfer Medium

Both coding and DeFi agents share the same cognitive architecture:

```
PERCEIVE  → Read environment state (code diff / chain state)
REMEMBER  → Retrieve relevant knowledge from neuro
REASON    → Analyze situation, generate candidates
GATE      → Verify candidates against constraints
ACT       → Execute chosen action (code edit / transaction)
EVALUATE  → Measure outcome (gate pass / PnL)
REFLECT   → Extract lessons from outcome
META-COGNIZE → Update cognitive parameters
```

Each stage operates on domain-specific data but follows domain-independent patterns. The transfer mechanism operates at the pattern level, not the data level.

### Extracting Domain-Independent Principles

After each episode, the REFLECT stage extracts lessons. Some lessons are domain-specific:

- "Uniswap V4 hooks require implementing `beforeSwap` and `afterSwap` in the correct order" (DeFi-specific)
- "This crate requires `#![feature(async_closure)]` for async closures in trait methods" (Rust-specific)

Other lessons are domain-independent:

- "When prediction confidence is low, reduce position size / change scope" (uncertainty → caution)
- "When multiple independent signals agree, confidence is high" (corroboration → conviction)
- "When a strategy worked 10 times and fails once, the environment has changed, not the strategy" (regime detection)
- "When cost per success increases monotonically, stop and reassess the approach" (diminishing returns)

### HDC-Encoded Heuristic Storage

Domain-independent principles are stored in the neuro as HDC-encoded heuristics:

```
Heuristic {
    fingerprint: BinarySpatter<10240>,   // HDC encoding of the principle
    principle: String,                    // Human-readable description
    confidence: f64,                      // How many episodes support this
    source_domain: String,               // Where it was first discovered
    transfer_count: u32,                 // How many times applied cross-domain
    transfer_success_rate: f64,          // How often cross-domain application succeeds
}
```

### Cross-Domain Retrieval

When a coding agent faces a novel situation, it queries the neuro for relevant heuristics. The HDC similarity search retrieves heuristics based on **structural similarity**, not domain labels:

- Coding agent encounters: "Test pass rate dropped from 95% to 60% after adding a new dependency"
- HDC query encodes the structural pattern: "metric degradation after environmental change"
- Retrieval returns: "When prediction confidence drops suddenly, the environment has changed — reassess assumptions before continuing" (originally learned by a DeFi agent after a protocol upgrade)
- The coding agent applies the principle: check if the new dependency changed API semantics, don't just retry

### Transfer Validation

Not all cross-domain transfers are valid. The system tracks transfer success rate per heuristic:

- **High transfer success (>70%)**: True domain-independent principle. Increase confidence, promote to population-level heuristic.
- **Moderate transfer success (40-70%)**: Partially transferable. Useful as a prior but needs domain-specific adaptation.
- **Low transfer success (<40%)**: Domain-specific disguised as general. Reduce transfer weight, keep in source domain only.

Over time, the system discovers which principles genuinely transfer and which only appear to. The neuro accumulates a library of validated cross-domain heuristics — the closest thing to "wisdom" an agent system can develop.

### Research

Transfer Learning: Pan & Yang (2010) — survey of transfer learning. Domain adaptation: the source domain provides a useful prior for the target domain. Analogical Reasoning: Gentner (1983) — structure-mapping theory. Analogies are based on relational similarity (structural), not surface similarity (superficial). The HDC encoding captures relational structure, enabling structural analogy across domains.
