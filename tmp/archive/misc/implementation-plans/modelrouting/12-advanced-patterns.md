# 12 — Advanced Patterns: Self-Improvement, Memory, Contracts, Search

> **Priority**: 🟢 P2 — Highest-leverage improvements from mori reference + academic SOTA
> **Status**: Not started
> **Depends on**: 08 (learning loops), 03 (provider adapters)
> **Blocks**: None

## Sources

This document synthesizes patterns from three reference codebases and cutting-edge research:

- **Mori agent system** (`/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-agents/`) — 28-agent production system
- **Mori refactor** (`/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/`) — 5-layer architecture theory
- **Agent-chain** (`/Users/will/dev/nunchi/roko/bardo-backup/tmp/agent-chain/`) — Predictive Foraging, stigmergy, knowledge distillation
- **Academic**: GVU Framework, GEPA, SAGE, AgentPRM, ACON, SEC, Agent Behavioral Contracts

---

## A. Thompson Sampling for Model Routing

### Why

The mori-refactor docs and academic literature consistently recommend Thompson Sampling over UCB1 for bandit-based model routing. Thompson Sampling has **stronger empirical performance** and adapts better to non-stationary environments (model updates, API changes, provider degradation).

Roko's current `CascadeRouter` uses UCB1 for experiments and LinUCB for routing. Both should offer Thompson Sampling as an alternative.

### How it works

Per-arm state is a Beta distribution `Beta(alpha, beta)` where:
- `alpha` = successes + 1 (prior)
- `beta` = failures + 1 (prior)

Selection: sample from each arm's Beta distribution, pick highest sample.
Update: success → alpha += 1; failure → beta += 1.

For cost-aware routing, use a **weighted reward**: `reward = gate_passed * (1 - normalized_cost)`, then maintain separate `sum_reward` and `sum_reward_sq` for Gaussian Thompson Sampling.

### Non-Stationary Adaptation (f-dsw TS)

Apply discount factor to historical observations, so recent data weighs more:
```
alpha_effective = 1 + gamma * sum(successes in window)
beta_effective = 1 + gamma * sum(failures in window)
```
Where `gamma ∈ (0, 1)` is the discount factor and the window slides forward.

This combats concept drift — when a model gets updated or a provider degrades, the router adapts within a few observations rather than being anchored to historical performance.

**Reference**: [arXiv:2512.00930](https://arxiv.org/html/2512.00930) — Multi-objective Thompson Sampling

---

### 2J.01 — Add Thompson Sampling arms to CascadeRouter

**File**: `crates/roko-learn/src/model_router.rs`
**What**: Add `ThompsonArm` alongside `ArmState`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThompsonArm {
    pub slug: String,
    pub alpha: f64,           // successes + 1
    pub beta: f64,            // failures + 1
    pub sum_reward: f64,      // for continuous rewards
    pub sum_reward_sq: f64,
    pub observations: u64,
    pub discount: f64,        // gamma for non-stationarity
}

impl ThompsonArm {
    pub fn sample(&self) -> f64 {
        // Sample from Beta(alpha, beta) using rand crate
        // For continuous: sample from Normal(mu, sigma) where
        //   mu = sum_reward / observations
        //   sigma = sqrt(sum_reward_sq / observations - mu^2) / sqrt(observations)
    }

    pub fn update(&mut self, reward: f64, success: bool) {
        // Apply discount to existing observations
        self.alpha = 1.0 + self.discount * (self.alpha - 1.0);
        self.beta = 1.0 + self.discount * (self.beta - 1.0);
        // Then add new observation
        if success { self.alpha += 1.0; } else { self.beta += 1.0; }
        self.sum_reward += reward;
        self.sum_reward_sq += reward * reward;
        self.observations += 1;
    }
}
```

**Context**: Thompson Sampling is empirically superior to UCB1 and adapts better to non-stationary environments. The `discount` parameter (default 0.99) ensures recent observations weigh more.

**Acceptance**: `ThompsonArm::sample()` returns values in [0, 1] for Beta. Updates shift the distribution correctly.
**Verification**: `cargo test -p roko-learn -- thompson_arm`

---

### 2J.02 — Add routing_algorithm config option

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Allow users to choose between LinUCB and Thompson Sampling:

```toml
[routing]
algorithm = "thompson"  # or "linucb" (default for backwards compat)
discount_factor = 0.99  # for thompson non-stationarity
```

**Acceptance**: `routing.algorithm = "thompson"` uses Thompson Sampling; `"linucb"` uses existing LinUCB.
**Verification**: `cargo test -p roko-core -- routing_algorithm_config`

---

## B. Predictive Foraging (from agent-chain)

### Why

The agent-chain architecture introduces **Predictive Foraging (PF)**: agents register predictions about outcomes _before_ task execution, then the system computes residuals after. This provides two signals the current system lacks:

1. **Calibration**: Is the router's confidence well-calibrated? If it predicts 80% success for GLM-5.1 on implementation tasks, does it actually pass 80%?
2. **Collective calibration**: Aggregate residuals across all tasks reveal systematic biases (e.g., "all models overperform on test tasks, underperform on refactoring").

### How it works

```
Before task:
  prediction = router.predict(task_context, model_slug)
  → { estimated_success_probability: 0.82, estimated_cost_usd: 0.25 }

After task:
  actual = { success: true, cost_usd: 0.31 }
  residual = { success: 0.82 - 1.0 = -0.18, cost: 0.25 - 0.31 = -0.06 }
  → Model was overconfident on success, underestimated cost

Correction:
  bias_correction = aggregate_residuals(model, task_category)
  adjusted_prediction = raw_prediction - bias_correction
```

**Reference**: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/agent-chain/` — Predictive Foraging engine (contract 0x0B)

---

### 2J.03 — Add PredictionRecord struct

**File**: `crates/roko-learn/src/prediction.rs` (new)
**What**: Track predictions and residuals:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRecord {
    pub task_id: String,
    pub model_slug: String,
    pub task_category: String,
    pub complexity: String,

    // Predicted
    pub predicted_success_prob: f64,
    pub predicted_cost_usd: f64,
    pub predicted_duration_ms: u64,

    // Actual
    pub actual_success: Option<bool>,
    pub actual_cost_usd: Option<f64>,
    pub actual_duration_ms: Option<u64>,

    // Residuals (computed on resolution)
    pub residual_success: Option<f64>,
    pub residual_cost: Option<f64>,
    pub residual_duration: Option<f64>,

    pub timestamp: String,
}
```

**Acceptance**: Predictions can be registered before task, resolved after.
**Verification**: `cargo test -p roko-learn -- prediction_record`

---

### 2J.04 — Implement calibration tracking

**File**: `crates/roko-learn/src/prediction.rs`
**What**: Aggregate residuals per (model, task_category) for bias correction:

```rust
pub struct CalibrationTracker {
    residuals: HashMap<(String, String), Vec<f64>>,  // (model, category) → residuals
}

impl CalibrationTracker {
    pub fn mean_bias(&self, model: &str, category: &str) -> f64 {
        // Average residual = systematic bias
    }

    pub fn coverage_rate(&self, model: &str, category: &str, confidence: f64) -> f64 {
        // What fraction of predictions fall within confidence interval?
    }

    pub fn adjust_prediction(&self, model: &str, category: &str, raw_pred: f64) -> f64 {
        raw_pred - self.mean_bias(model, category)
    }
}
```

**Context**: From agent-chain's Predictive Foraging. New agents learn instantly from aggregated calibration data — a fresh model deployment inherits bias corrections from all prior observations.

**Acceptance**: After 50 observations, `mean_bias` converges to stable value. `adjust_prediction` corrects systematic over/underconfidence.
**Verification**: `cargo test -p roko-learn -- calibration_tracker`

---

## C. Gate-to-Scaffold Feedback Loop

### Why

From mori-refactor: "When gate fails, don't just inject error digest. Learn which context sections would have prevented failure. Adjust section priorities for next attempt."

Currently, when a gate fails, the error output is fed back to the agent. But the system doesn't learn _which prompt sections_ correlate with success/failure. Over time, the system should discover that (for example) including the workspace_map section increases pass rate by 12% for implementation tasks, while the prd_extract section has no effect on test tasks.

### How it works

```
For each efficiency event:
  sections_included = event.prompt_sections (names + was_truncated + was_dropped)
  outcome = event.gate_passed (bool)

Statistical model:
  For each section S:
    included_pass_rate = pass_rate when S was included
    excluded_pass_rate = pass_rate when S was dropped/truncated
    lift = included_pass_rate - excluded_pass_rate
    significance = chi_squared_test(included_outcomes, excluded_outcomes)

Section priority adjustment:
  If lift > 0.05 and p < 0.05: increase priority
  If lift < -0.02 and p < 0.05: decrease priority (section hurts)
  Otherwise: no change
```

**Reference**: `/Users/will/dev/nunchi/roko/bardo-backup/tmp/mori-refactor/` — "Gate-to-scaffold feedback loops"

---

### 2J.05 — Add SectionEffectiveness tracker

**File**: `crates/roko-learn/src/section_effect.rs` (new)
**What**: Track per-section pass rate lift:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionEffect {
    pub section_name: String,
    pub included_trials: u64,
    pub included_passes: u64,
    pub excluded_trials: u64,
    pub excluded_passes: u64,
}

impl SectionEffect {
    pub fn lift(&self) -> f64 {
        let included_rate = self.included_passes as f64 / self.included_trials.max(1) as f64;
        let excluded_rate = self.excluded_passes as f64 / self.excluded_trials.max(1) as f64;
        included_rate - excluded_rate
    }

    pub fn recommend_priority_change(&self) -> PriorityChange {
        if self.included_trials < 20 || self.excluded_trials < 5 {
            return PriorityChange::InsufficientData;
        }
        let lift = self.lift();
        if lift > 0.05 { PriorityChange::Increase }
        else if lift < -0.02 { PriorityChange::Decrease }
        else { PriorityChange::NoChange }
    }
}

pub struct SectionEffectivenessRegistry {
    effects: HashMap<(String, String), SectionEffect>,  // (section, role) → effect
}
```

**Persistence**: `.roko/learn/section-effects.json`

**Acceptance**: After 50 events, sections with positive lift are identified.
**Verification**: `cargo test -p roko-learn -- section_effectiveness`

---

### 2J.06 — Wire section effectiveness into prompt assembly

**File**: `crates/roko-compose/src/system_prompt_builder.rs`
**What**: When assembling prompts, adjust section priorities based on learned effectiveness:

```rust
fn adjusted_priority(base_priority: u8, section: &str, role: &str, registry: &SectionEffectivenessRegistry) -> u8 {
    match registry.recommend_priority_change(section, role) {
        PriorityChange::Increase => base_priority.saturating_add(1),
        PriorityChange::Decrease => base_priority.saturating_sub(1),
        _ => base_priority,
    }
}
```

**Context**: This closes the loop between harness (what succeeded) and scaffold (what was in the prompt). The system learns which context matters per role.

**Acceptance**: Sections with positive lift get higher priority; sections that hurt get lower priority.
**Verification**: `cargo test -p roko-compose -- section_priority_adjustment`

---

## D. Skill Library (Procedural Memory)

> **CORRECTION (doc 17 final audit)**: `skill_library.rs` already EXISTS and is WIRED via
> `runtime_feedback.rs` L~557. It has skills with confidence scores and usage telemetry.
> `playbook.rs` and `pattern_discovery.rs` also exist. `roko-neuro/distiller.rs` is actively
> building the Episode → Insights → Heuristics → Playbook cascade.
>
> Tasks 2J.07–08 below should EXTEND the existing skill library with model-specific tagging
> (see doc 17 task 2O.12) and connect it to prompt assembly (see doc 17 task 2O.03),
> NOT rebuild skill extraction from scratch.

### Why

From SAGE (arXiv:2512.17102) and Voyager (Wang et al., 2023): agents that accumulate reusable tool-use patterns across tasks use **26% fewer steps and 59% fewer tokens**. ~~Roko's episode logger captures raw execution data but doesn't extract reusable skills.~~ Roko's skill library exists but skills aren't injected into prompts and aren't tagged by source model.

Mori's learning hierarchy defines 3 tiers:
- **Tier 1**: Episodes (raw execution records)
- **Tier 2**: Patterns (extracted from 5+ similar episodes)
- **Tier 3**: Playbook (validated 5+ times, injected into context)

### How it works

```
After successful task:
  1. Extract (precondition, procedure, postcondition) from episode
  2. Cluster with similar episodes (embedding similarity > 0.85)
  3. If cluster has 5+ members: promote to Pattern
  4. If Pattern validated 5+ times in production: promote to Playbook rule

Playbook rules injected into context via prompt section "skills"
  with priority based on match score to current task
```

**Reference**: SAGE (arXiv:2512.17102), Voyager (Wang et al., 2023), mori-agents "Learning Pack Tiers"

---

### 2J.07 — Define Skill struct

**File**: `crates/roko-learn/src/skill_library.rs` (check if exists, may need extension)
**What**: Structured skill representation:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub precondition: String,        // When to apply this skill
    pub procedure: String,           // What to do (tool call sequence summary)
    pub postcondition: String,       // Expected outcome
    pub confidence: f64,             // [0, 1] based on validation count
    pub source_episodes: Vec<String>, // Episode IDs this was extracted from
    pub validations: u64,            // Times successfully applied
    pub failures: u64,               // Times applied but failed
    pub task_categories: Vec<String>, // Which task types this applies to
    pub created_at: String,
    pub last_validated_at: Option<String>,
}
```

**Context**: Roko already has a `skill_library.rs` in roko-learn. Check its current state and extend if needed.

**Acceptance**: Skills can be created, validated, and queried by task category.
**Verification**: `cargo test -p roko-learn -- skill_library`

---

### 2J.08 — Implement skill extraction from episodes

**File**: `crates/roko-learn/src/skill_library.rs`
**What**: Extract candidate skills from successful episodes:

```rust
pub async fn extract_skill_candidates(
    episodes: &[Episode],
    judge_agent: &dyn Agent,
) -> Vec<SkillCandidate> {
    // For each successful episode with gate_passed=true:
    //   1. Summarize the tool call sequence
    //   2. Identify the precondition (task type, complexity, files involved)
    //   3. Identify the postcondition (gate results, output)
    //   4. Generate a natural-language skill description
    // Use a cheap model (haiku-class) for summarization
}
```

**Context**: This is the "Episode → Pattern" promotion in mori's learning hierarchy. Only successful episodes are candidates. The judge model extracts structured skills from raw execution traces.

**Acceptance**: Extraction produces structured `SkillCandidate` from a set of successful episodes.
**Verification**: `cargo test -p roko-learn -- skill_extraction` (mock test)

---

## E. Error Digest Enrichment

### Why

From mori-agents: a single Haiku call (~$0.005) before the AutoFixer can classify and diagnose a compile/test error, saving a failed AutoFixer iteration ($0.50+). The error output from gates is often noisy (hundreds of lines of rustc output). A cheap model can produce a 2-sentence diagnosis.

### 2J.09 — Implement error digest enrichment

**File**: `crates/roko-learn/src/error_enrichment.rs` (new)
**What**: Pre-process gate error output before feeding to retry agent:

```rust
pub async fn enrich_error_digest(
    raw_error: &str,
    agent: &dyn Agent,
    task_context: &str,
) -> String {
    let prompt = format!(
        "Diagnose this compilation/test error in 2 sentences. \
         Be specific about which file, line, and type mismatch.\n\
         Error output:\n{}\n\
         Task context:\n{}\n\
         Diagnosis:",
        &raw_error[..raw_error.len().min(4000)],
        &task_context[..task_context.len().min(1000)],
    );
    // Send to haiku-class model
    // Return 2-sentence diagnosis
}
```

**Context**: Uses the cheapest available model (mechanical tier). ROI: $0.005 enrichment cost vs $0.50+ saved AutoFixer iteration. Net savings ~99%.

**Acceptance**: Enrichment produces concise diagnosis from noisy error output.
**Verification**: `cargo test -p roko-learn -- error_enrichment` (mock test)

---

## F. Context Compaction (ACON Pattern)

### Why

From ACON (arXiv:2510.00615): long-running agent sessions accumulate context that approaches the context window limit. Compaction achieves **26-54% token reduction** while preserving task-relevant information. Without compaction, either the context overflows or early information is lost.

### 2J.10 — Implement anchored iterative summarization

**File**: `crates/roko-compose/src/compaction.rs` (new)
**What**: Compress conversation history while preserving critical anchors:

```rust
pub struct CompactionPolicy {
    pub trigger_threshold: f64,        // Compact when context > threshold% of window
    pub anchor_roles: Vec<String>,     // Never compact these (e.g., "system")
    pub preserve_last_n_turns: usize,  // Keep last N turns verbatim
    pub summary_budget_tokens: usize,  // Max tokens for summary
}

pub async fn compact_history(
    messages: &[ChatMessage],
    policy: &CompactionPolicy,
    summarizer: &dyn Agent,
) -> Vec<ChatMessage> {
    // 1. Identify anchor messages (system prompt, tool results with errors)
    // 2. Identify compactable region (old turns beyond preserve_last_n)
    // 3. Summarize compactable region into summary_budget_tokens
    // 4. Replace compactable region with summary message
    // 5. Return anchors + summary + recent turns
}
```

**Context**: Triggers at 70% context utilization. Uses anchored iterative pattern (highest accuracy per academic comparison). The summary preserves tool call outcomes and gate results as structured data, not prose.

**Acceptance**: 50-turn conversation compacted to 15 messages while preserving all gate results.
**Verification**: `cargo test -p roko-compose -- context_compaction`

---

## G. Agent Behavioral Contracts

### Why

From Agent Behavioral Contracts (arXiv:2602.22302): formal specifications with runtime enforcement, <10ms overhead, composable across multi-task execution. Roko's safety layer uses role-based permissions but lacks formal contracts, drift detection, or recovery mechanisms.

The key insight: **behavioral drift is measurable** in multi-turn LLM interactions. Agents gradually deviate from expected behavior patterns. Contracts detect this via JSD (Jensen-Shannon Divergence) from a reference action distribution.

### 2J.11 — Define Contract struct

**File**: `crates/roko-agent/src/safety/contract.rs` (new)
**What**: Formal behavioral contract:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContract {
    pub role: String,
    pub invariants: Vec<Invariant>,
    pub governance: Vec<GovernanceRule>,
    pub recovery: Vec<RecoveryAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariant {
    pub name: String,
    pub kind: InvariantKind,           // Hard (abort) or Soft (recover)
    pub predicate: String,             // e.g., "cost_usd < 2.0"
    pub check_frequency: CheckFreq,    // PerAction, PerTurn, PerTask
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GovernanceRule {
    MaxToolCallsPerTurn(u32),
    ForbiddenTools(Vec<String>),
    MaxCostPerTurn(f64),
    MaxConsecutiveFailures(u32),
    RequireToolBeforeEdit(String),     // e.g., must Read before Edit
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    pub trigger: String,               // condition
    pub action: RecoveryKind,          // Retry, Downgrade, Abort, Alert
}
```

**Context**: <10ms per action check. Hard invariant violations abort immediately. Soft violations trigger recovery.

**Acceptance**: Contracts can be defined per role with invariants and governance rules.
**Verification**: `cargo test -p roko-agent -- agent_contract`

---

### 2J.12 — Implement drift detection

**File**: `crates/roko-learn/src/drift.rs` (new)
**What**: Detect behavioral drift via JSD from reference action distribution:

```rust
pub struct DriftDetector {
    reference_distributions: HashMap<String, ActionDistribution>,  // role → reference
    window_size: usize,
}

pub struct ActionDistribution {
    pub tool_frequencies: HashMap<String, f64>,  // tool_name → frequency
    pub avg_tokens_per_turn: f64,
    pub avg_tool_calls_per_turn: f64,
}

impl DriftDetector {
    pub fn compute_jsd(&self, role: &str, recent: &ActionDistribution) -> f64 {
        // Jensen-Shannon Divergence between reference and recent distributions
        // JSD ∈ [0, 1]; > 0.15 indicates meaningful drift
    }

    pub fn check_drift(&self, role: &str, recent_events: &[AgentEfficiencyEvent]) -> Option<DriftAlert> {
        let recent_dist = ActionDistribution::from_events(recent_events);
        let jsd = self.compute_jsd(role, &recent_dist);
        if jsd > 0.15 {
            Some(DriftAlert { role: role.to_string(), jsd, description: "..." })
        } else {
            None
        }
    }
}
```

**Context**: From ABC paper. Combines compliance component (lagging indicator) with JSD (leading indicator for early intervention). Reference distribution built from first 50 successful task completions per role.

**Acceptance**: Drift > 0.15 JSD triggers alert. Normal variation stays below threshold.
**Verification**: `cargo test -p roko-learn -- drift_detection`

---

## H. Multi-Objective Pareto Bandits

### Why

The current `compute_routing_reward` collapses cost, quality, and latency into a single scalar via fixed weights (0.5/0.3/0.2). This means a 10% quality improvement and a 50% cost reduction are traded off by predetermined weights, not by the user's actual preference.

Multi-objective bandits maintain a **Pareto frontier** over the objective space and allow the user to specify preferences at query time.

### 2J.13 — Add multi-objective reward tracking

**File**: `crates/roko-learn/src/model_router.rs`
**What**: Track per-arm reward vectors instead of scalars:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiObjectiveStats {
    pub quality_sum: f64,
    pub quality_sq_sum: f64,
    pub cost_sum: f64,
    pub cost_sq_sum: f64,
    pub latency_sum: f64,
    pub latency_sq_sum: f64,
    pub observations: u64,
}

impl MultiObjectiveStats {
    pub fn scalarize(&self, weights: &RewardWeights) -> f64 {
        let q = self.quality_sum / self.observations.max(1) as f64;
        let c = 1.0 - (self.cost_sum / self.observations.max(1) as f64);
        let l = 1.0 - (self.latency_sum / self.observations.max(1) as f64);
        q * weights.quality + c * weights.cost + l * weights.latency
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardWeights {
    pub quality: f64,  // default 0.5
    pub cost: f64,     // default 0.3
    pub latency: f64,  // default 0.2
}
```

**Context**: Weights can be configured per-role or per-tier. Mechanical tier might use `{quality: 0.3, cost: 0.6, latency: 0.1}` (cost-sensitive). Architectural tier might use `{quality: 0.8, cost: 0.1, latency: 0.1}` (quality-sensitive).

```toml
[routing.weights]
quality = 0.5
cost = 0.3
latency = 0.2

[routing.weights.mechanical]
quality = 0.3
cost = 0.6
latency = 0.1
```

**Acceptance**: Per-tier reward weights are configurable.
**Verification**: `cargo test -p roko-learn -- multi_objective_routing`

---

## I. Warm Pool for Agent Pre-Spawning

### Why

From mori-agents: pre-spawning agents while others work saves 5-15 seconds per agent. When an Implementer is running, the reviewer agent can be warming up (process started, handshake complete, sitting idle). When the Implementer finishes, the reviewer starts instantly.

### 2J.14 — Document warm pool integration point

**File**: This is tracked as a reference for future implementation. The warm pool pattern from mori-agents maps to Roko's `AgentPool` and `MultiAgentPool` in `crates/roko-agent/src/pool.rs`.

**What would change**:
1. `MultiAgentPool` gains a `warm_pool: HashMap<(AgentRole, String), Box<dyn Agent>>` field
2. `pre_spawn_warm(role, model_key)` creates agent but doesn't start a turn
3. `promote_warm(role)` moves from warm pool to active
4. `evict_warm(role)` kills unused warm agents

**Not a task yet** — depends on the multi-agent orchestration path being active. Documented here for architectural awareness.

---

## J. Complexity-Based Pipeline Selection

### Why

From mori-agents: different task complexities warrant different pipelines. Trivial tasks (rename a variable) don't need strategist + 3 reviewers. Complex tasks (new subsystem) need the full pipeline.

### 2J.15 — Add complexity-to-pipeline mapping config

**File**: `crates/roko-core/src/config/schema.rs`
**What**: Allow configuring which pipeline stages run per complexity band:

```toml
[pipeline.mechanical]
strategist = false
reviewers = false
max_iterations = 1

[pipeline.focused]
strategist = false
reviewers = false
max_iterations = 2

[pipeline.integrative]
strategist = true
reviewers = true
reviewer_mode = "quick"     # single QuickReviewer
max_iterations = 2

[pipeline.architectural]
strategist = true
reviewers = true
reviewer_mode = "full"      # Architect + Auditor + Scribe
max_iterations = 3
```

**Context**: This is configuration, not runtime logic. The orchestrator reads these settings and adjusts the pipeline accordingly. Reduces cost for simple tasks while maintaining quality for complex ones.

**Acceptance**: Config parses. Pipeline for `mechanical` skips strategist and reviewers.
**Verification**: `cargo test -p roko-core -- pipeline_config`

---

## K. GVU-Informed Verification Investment

### Why

The GVU Framework (arXiv:2512.02731) proves: **self-improvement succeeds when the verifier is strong, not when the generator is strong**. The Variance Inequality shows that oracle verifiers (sigma_V ≈ 0) enable improvement despite high generation noise.

Roko's gate pipeline (compile, test, clippy) is already an oracle verifier for code correctness. The GVU theory suggests investing in richer verification rather than better prompts:

1. **Process reward models**: Evaluate intermediate steps, not just final output
2. **Ensemble gates**: Multiple independent checks reduce verification noise
3. **Property-based testing**: Richer signal than pass/fail unit tests

### 2J.16 — Add process reward tracking per tool call

**File**: `crates/roko-learn/src/efficiency.rs`
**What**: Extend `ToolCallMeta` with a reward signal:

```rust
pub struct ToolCallMeta {
    pub tool_name: String,
    pub duration_ms: u64,
    pub result_tokens: u64,
    pub succeeded: bool,
    // New fields:
    pub advanced_task: bool,       // Did this call advance the task?
    pub was_redundant: bool,       // Was this call unnecessary?
    pub error_category: Option<String>,  // If failed, what kind of error?
}
```

**Context**: From AgentPRM (arXiv:2502.10325). Per-step rewards provide 10x richer signal than final pass/fail. The `advanced_task` flag can be computed post-hoc: if the tool call's output was referenced in the final solution, it advanced the task.

**Acceptance**: ToolCallMeta has per-call reward indicators.
**Verification**: `cargo test -p roko-learn -- tool_call_reward`

---

## Summary: New Task Count

| Section | Tasks | IDs |
|---|---|---|
| A. Thompson Sampling | 2 | 2J.01–2J.02 |
| B. Predictive Foraging | 2 | 2J.03–2J.04 |
| C. Gate-to-Scaffold Feedback | 2 | 2J.05–2J.06 |
| D. Skill Library | 2 | 2J.07–2J.08 |
| E. Error Digest Enrichment | 1 | 2J.09 |
| F. Context Compaction | 1 | 2J.10 |
| G. Agent Behavioral Contracts | 2 | 2J.11–2J.12 |
| H. Multi-Objective Bandits | 1 | 2J.13 |
| I. Warm Pool | 1 | 2J.14 (reference) |
| J. Pipeline Selection | 1 | 2J.15 |
| K. GVU Verification | 1 | 2J.16 |
| **Total** | **16** | **2J.01–2J.16** |
