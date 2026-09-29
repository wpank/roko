# 08-learning/11 -- Eight Cybernetic Feedback Loops

> The eight inter-subsystem connections that close the cybernetic circuit.
> Each loop has a source (where the signal originates), a target (where it
> flows), and a mechanism (how the signal is transformed into corrective
> action). Together they are designed as negative feedback for stability
> (Ashby 1956) within a Viable System architecture (Beer 1972). On Graph runs
> (checked at `7c556bc0a`) two of the eight close, provider health and the plan
> budget; [08-LEARNING](../../08-LEARNING.md) section 11 gives each loop's status.

**Parent:** [08-LEARNING](../../08-LEARNING.md) section 11

**Theoretical basis:** Ashby's Law of Requisite Variety (Ashby 1956),
Beer's Viable System Model (Beer 1972), Good Regulator Theorem
(Conant & Ashby 1970), double-loop learning (Argyris & Schon 1978)

**Source:** `crates/roko-learn/src/cascade_router.rs` (loops 1, 2, 6, 7, 8),
`crates/roko-learn/src/efficiency.rs` (loop 3),
`crates/roko-cli/src/runner/` (loop 4),
`crates/roko-learn/src/playbook_rules.rs` (loop 5),
`crates/roko-learn/src/prompt_experiment.rs` (loop 8)

---

## 1. Purpose

The Roko learning system was built in layers: episodes first, then patterns,
then bandits, then routing. Each layer works, but they needed connections to
form a self-regulating system. The eight feedback loops are the inter-layer
connections that close the cybernetic circuit -- signals that flow from one
subsystem's output to another subsystem's input, creating self-regulating
behavior that distinguishes a learning system from a collection of
independent optimizers.

Each loop implements **negative feedback**: detecting a deviation from
desired behavior and applying a corrective signal that opposes the
deviation, driving the system back toward equilibrium.

---

## 2. Overview

```
+---------------------------------------------------------------------+
|                    EIGHT FEEDBACK LOOPS                                |
|                                                                       |
|  1. Health -> Routing     Provider circuit breaker -> candidate set   |
|  2. Conductor -> Routing  System load signals -> routing bias         |
|  3. Section -> Scaffold   Section effectiveness -> prompt weights     |
|  4. Failure -> Replanning Gate failures -> plan revision              |
|  5. Skills -> Prompts     Skill library -> prompt injection           |
|  6. Cost -> Routing       Budget pressure -> model selection          |
|  7. Latency -> Reward     Response latency -> bandit reward signal    |
|  8. Experiments -> Static Experiment winners -> static routing table  |
|                                                                       |
+---------------------------------------------------------------------+
```

---

## 3. Loop 1: Health -> Routing

**Source:** `ProviderHealthRegistry` (circuit breaker state per provider)
**Target:** `CascadeRouter::select()` (candidate model filtering)
**Mechanism:** Before scoring candidates, filter out models whose provider
circuit breaker is Open.
**Status:** Wired.

### 3.1 Data Flow

```
+-----------------------+     +-------------------------+     +------------------+
|  Provider Response    |---->|  ProviderHealthRegistry  |---->|  CascadeRouter   |
|  (success/failure)    |     |  record_success/failure  |     |  select()        |
+-----------------------+     +-------------------------+     +------------------+
      Source type:                Transform:                   Sink type:
      ProviderResponse            ErrorClassifier ->           is_available() ->
      { status_code,              CircuitState update          bool filter on
        latency_ms,               (Closed/Open/HalfOpen)       candidate set
        error: Option }
```

### 3.2 Transform Function

```rust
// Source type
pub struct ProviderResponse {
    pub provider_id: String,
    pub model: String,
    pub status_code: u16,
    pub latency_ms: u64,
    pub error: Option<ProviderError>,
    pub timestamp: DateTime<Utc>,
}

// Transform: classify error and update circuit state
fn health_to_routing_transform(response: &ProviderResponse) -> CircuitAction {
    match response.error {
        None => CircuitAction::RecordSuccess,
        Some(ref err) => {
            let class = ErrorClassifier::classify(err);
            let cooldown = CooldownPolicy::for_class(&class);
            CircuitAction::RecordFailure { class, cooldown }
        }
    }
}

// Sink: CascadeRouter reads circuit state during select()
fn is_available(provider: &str, registry: &ProviderHealthRegistry) -> bool {
    matches!(registry.state(provider), CircuitState::Closed | CircuitState::HalfOpen)
}
```

### 3.3 Specification

| Property | Value |
|----------|-------|
| Latency requirement | Real-time (< 1ms). Circuit state check is a HashMap lookup |
| Update frequency | Every episode (per-response) |
| Failure mode if loop breaks | Router sends requests to degraded provider -> timeouts -> wasted budget -> cascading failures as the provider's queue backs up |
| Recovery | Manual provider blacklist in `roko.toml` |
| Impact | Prevents routing to degraded providers, reducing retry waste and improving first-attempt pass rates |

---

## 4. Loop 2: Conductor -> Routing

**Source:** Conductor subsystem (system load, resource utilization, queue
depth)
**Target:** `CascadeRouter::select()` (routing bias)
**Mechanism:** When system load is high, bias toward cheaper/faster models.
When load is low, allow more expensive/thorough models.
**Status:** Wired. `RoutingContext` carries conductor pressure derived from
active agent count, ready-queue depth, and queue wait.

### 4.1 Data Flow

```
+-----------------------+     +-------------------------+     +------------------+
|  SystemLoadSnapshot   |---->|  Load Threshold Check    |---->|  CascadeRouter   |
|  (cpu, mem, agents,   |     |  active_agents >=        |     |  routing bias    |
|   queue_depth)        |     |  max_agents * 0.8?       |     |  adjustment      |
+-----------------------+     +-------------------------+     +------------------+
```

### 4.2 Transform Function

```rust
// Source type (exists in roko-conductor)
pub struct SystemLoadSnapshot {
    pub cpu_load: f32,
    pub memory_pct: f32,
    pub active_agents: u32,
    pub queue_depth: u32,
    pub timestamp: DateTime<Utc>,
}

// Transform: load to routing bias
fn conductor_to_routing_transform(
    load: &SystemLoadSnapshot,
    config: &ConductorConfig,
) -> RoutingBiasAdjustment {
    let agent_utilization = load.active_agents as f64 / config.max_agents as f64;
    let memory_pressure = load.memory_pct as f64 / 100.0;

    if agent_utilization > 0.8 || memory_pressure > 0.85 {
        RoutingBiasAdjustment::PreferCheaper { cost_weight_multiplier: 1.5 }
    } else if agent_utilization < 0.3 && memory_pressure < 0.50 {
        RoutingBiasAdjustment::AllowExpensive { quality_weight_multiplier: 1.2 }
    } else {
        RoutingBiasAdjustment::Neutral
    }
}

pub enum RoutingBiasAdjustment {
    PreferCheaper { cost_weight_multiplier: f64 },
    AllowExpensive { quality_weight_multiplier: f64 },
    Neutral,
}
```

### 4.3 Specification

| Property | Value |
|----------|-------|
| Latency requirement | Batch (every 5 episodes). System load changes slowly relative to task execution |
| Update frequency | Every 5 episodes |
| Failure mode if loop breaks | System routes to expensive models during high load -> resource exhaustion -> agent spawn failures -> plan stalls |
| Recovery | Manual cost ceiling in `roko.toml` |
| Impact | Prevents resource exhaustion during high-load periods by dynamically adjusting quality-cost tradeoffs |

---

## 5. Loop 3: Section -> Scaffold

**Source:** `PromptSectionMeta` from efficiency events (per-section token
attribution + gate outcomes)
**Target:** Prompt composer section weights (priority values during context
assembly)
**Mechanism:** Track which prompt sections correlate with gate passes.
Increase weight of sections that correlate with success; decrease weight of
sections that consume tokens without contributing to outcomes.
**Status:** Wired for the live orchestration path.

### 5.1 Data Flow

```
+-----------------------+     +-------------------------+     +------------------+
|  PromptSectionMeta +  |---->|  SectionEffectiveness    |---->|  SectionWeights  |
|  gate_passed (from    |     |  Tracker (conditional    |     |  (HashMap<String |
|  efficiency events)   |     |  pass rate analysis)     |     |   , f32>)        |
+-----------------------+     +-------------------------+     +------------------+
```

### 5.2 Transform Function

```rust
// Source type (exists in efficiency events)
pub struct SectionEffectivenessInput {
    pub section_name: String,
    pub was_included: bool,
    pub tokens_consumed: u64,
    pub gate_passed: bool,
    pub role: String,
    pub complexity_band: String,
}

// Transform: conditional pass rate analysis
pub struct SectionStats {
    pub included_count: u32,
    pub included_pass_count: u32,
    pub excluded_count: u32,
    pub excluded_pass_count: u32,
}

impl SectionStats {
    fn effectiveness_delta(&self) -> f64 {
        let included_rate = self.included_pass_count as f64
            / self.included_count.max(1) as f64;
        let excluded_rate = self.excluded_pass_count as f64
            / self.excluded_count.max(1) as f64;
        included_rate - excluded_rate
        // Positive = section helps, Negative = section hurts
    }
}

fn section_to_scaffold_transform(
    tracker: &SectionEffectivenessTracker,
    min_samples: u32,           // default: 50
    significance_delta: f64,    // default: 0.05
) -> HashMap<String, f32> {
    let mut weights = HashMap::new();
    for (name, stats) in &tracker.stats {
        if stats.included_count + stats.excluded_count < min_samples {
            weights.insert(name.clone(), 1.0); // Not enough data
            continue;
        }
        let delta = stats.effectiveness_delta();
        if delta > significance_delta {
            // Boost helpful sections
            weights.insert(name.clone(), 1.0 + delta as f32);
        } else if delta < -significance_delta {
            // Reduce harmful sections (floor at 0.1)
            weights.insert(name.clone(), (1.0 + delta as f32).max(0.1));
        } else {
            weights.insert(name.clone(), 1.0); // Neutral
        }
    }
    weights
}
```

### 5.3 Specification

| Property | Value |
|----------|-------|
| Latency requirement | Batch (every 20 episodes). Section effectiveness needs meaningful sample |
| Update frequency | Every 20 episodes |
| Failure mode if loop breaks | Wasteful prompt sections consume tokens without contributing -> inflated costs, confused agents |
| Recovery | Manual section weights in `roko.toml` prompt configuration |
| Impact | Highest-leverage self-improvement loop. Adaptive context assembly can reduce prompt size by 30-50% while improving pass rates |

### 5.4 Why This Is the Highest-Leverage Loop

The predecessor system (mori) identified adaptive context dropping as the
single highest-leverage self-improvement technique. Most prompt sections in
an agent's system prompt are irrelevant to the current task, but they
consume tokens and may confuse the agent. Learning which sections to drop
(or heavily truncate) for each task type can:

- Reduce prompt size by 30-50% (saving input token costs)
- Improve pass rates by 5-15% (less noise in the prompt)
- Reduce latency by 20-40% (fewer tokens to process)

---

## 6. Loop 4: Failure -> Replanning

**Source:** Gate failure patterns (repeated failures on the same task,
regression alerts)
**Target:** Plan generator (re-decompose the failing task)
**Mechanism:** When a task fails N consecutive times, trigger replanning:
break the task into smaller subtasks, change the approach, or escalate to
human review.
**Status:** Wired for the orchestrator path.

### 6.1 Data Flow

```
+-----------------------+     +-------------------------+     +------------------+
|  Consecutive gate     |---->|  Failure Analyzer        |---->|  Plan Generator  |
|  failures for task    |     |  (pattern detection,     |     |  (decompose into |
|  (Vec<GateVerdict>)   |     |   root cause grouping)   |     |   subtasks)      |
+-----------------------+     +-------------------------+     +------------------+
```

### 6.2 Transform Function

```rust
// Source type
pub struct FailureSequence {
    pub task_id: String,
    pub plan_id: String,
    pub failures: Vec<GateVerdict>,
    pub total_cost_burned: f64,
    pub models_tried: Vec<String>,
}

// Transform: failure analysis
pub struct FailureAnalysis {
    pub is_repeated_error: bool,
    pub dominant_signature: Option<String>,
    pub model_escalation_helped: bool,
    pub recommendation: FailureRecommendation,
}

pub enum FailureRecommendation {
    /// Decompose into smaller subtasks.
    Decompose { suggested_split: Vec<String> },
    /// Change approach entirely.
    ChangeApproach { reason: String },
    /// Escalate to human review.
    HumanReview { context: String },
    /// Skip task (may be impossible given current capabilities).
    Skip { reason: String },
}

fn failure_to_replan_transform(seq: &FailureSequence) -> FailureAnalysis {
    let signatures: Vec<_> = seq.failures.iter()
        .filter_map(|v| v.signature.as_ref())
        .collect();

    let is_repeated = signatures.windows(2).all(|w| w[0] == w[1]);
    let model_set: HashSet<_> = seq.models_tried.iter().collect();
    let tried_multiple_models = model_set.len() >= 2;

    let recommendation = if is_repeated && tried_multiple_models {
        // Same error with multiple models = fundamental approach problem
        FailureRecommendation::Decompose {
            suggested_split: suggest_decomposition(&seq.task_id),
        }
    } else if seq.failures.len() > 5 && seq.total_cost_burned > 10.0 {
        // Many failures, high cost = escalate
        FailureRecommendation::HumanReview {
            context: format!(
                "Task {} failed {} times, burned ${:.2}",
                seq.task_id, seq.failures.len(), seq.total_cost_burned
            ),
        }
    } else {
        FailureRecommendation::ChangeApproach {
            reason: "Varied errors suggest the approach needs revision".into(),
        }
    };

    FailureAnalysis {
        is_repeated_error: is_repeated,
        dominant_signature: signatures.first().map(|s| s.to_string()),
        model_escalation_helped: !is_repeated && tried_multiple_models,
        recommendation,
    }
}
```

### 6.3 Specification

| Property | Value |
|----------|-------|
| Latency requirement | Per-task (triggered after max_iterations failures). Must complete before executor moves on |
| Update frequency | On failure threshold breach |
| Failure mode if loop breaks | System retries same failing task indefinitely -> budget burn on intractable tasks -> plan stalls |
| Recovery | Manual task skip or plan abort |
| Impact | Prevents budget burn on intractable tasks. Replanning turns a hard task into multiple easier tasks that may succeed individually |

---

## 7. Loop 5: Skills -> Prompts

**Source:** `SkillLibrary` (accumulated skills with confidence scores)
**Target:** Prompt composer (skill injection into agent prompts)
**Mechanism:** When a new task matches a skill's trigger pattern (file paths,
task category, tags), inject the skill's prompt template into the agent's
system prompt.
**Status:** Wired. Matching skills are rendered into a dedicated
`skill-library` prompt section before composition.

### 7.1 Data Flow

```
+-----------------------+     +-------------------------+     +------------------+
|  SkillLibrary         |---->|  Skill Matcher           |---->|  SystemPrompt    |
|  (accumulated skills  |     |  (tag + file + HDC       |     |  Builder layer   |
|   with confidence)    |     |   similarity search)     |     |  ("skills" sect) |
+-----------------------+     +-------------------------+     +------------------+
```

### 7.2 Transform Function

```rust
// Source: SkillLibrary::search_by_task()
pub struct SkillMatch {
    pub skill_name: String,
    pub confidence: f64,
    pub match_type: SkillMatchType,
    pub prompt_template: String,
    pub max_tokens: usize,
}

pub enum SkillMatchType {
    FileMatch { overlap_files: Vec<String> },
    TagMatch { matching_tags: Vec<String> },
    HdcSimilarity { similarity: f64 },
}

// Transform: filter and rank
fn skills_to_prompt_transform(
    matches: Vec<SkillMatch>,
    max_skills: usize,          // default: 3
    min_confidence: f64,        // default: 0.50
    max_total_tokens: usize,    // default: 500
) -> Vec<SkillInjection> {
    let mut qualified: Vec<_> = matches.into_iter()
        .filter(|m| m.confidence >= min_confidence)
        .collect();
    qualified.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

    let mut injections = Vec::new();
    let mut token_budget = max_total_tokens;
    for skill in qualified.into_iter().take(max_skills) {
        if skill.max_tokens <= token_budget {
            token_budget -= skill.max_tokens;
            injections.push(SkillInjection {
                skill_name: skill.skill_name,
                template: skill.prompt_template,
                confidence: skill.confidence,
            });
        }
    }
    injections
}
```

### 7.3 Specification

| Property | Value |
|----------|-------|
| Latency requirement | Per-task (< 5ms). Must complete during prompt assembly before dispatch |
| Update frequency | Every task dispatch |
| Failure mode if loop breaks | Agents rediscover solutions the skill library already contains -> wasted iterations -> higher cost |
| Recovery | Skills still accumulate but are not injected; no data loss |
| Impact | Reduces iterations by providing agents with proven approaches. The 100th modification to a crate is dramatically cheaper than the 1st |

---

## 8. Loop 6: Cost -> Routing

**Source:** Budget guardrails (per-task, per-session, per-day cost tracking)
**Target:** `CascadeRouter::select()` (model tier bias)
**Mechanism:** When spending approaches budget limits, force the router to
select cheaper models.
**Status:** Wired. `BudgetGuardrail` checks current spend before dispatch.

### 8.1 Data Flow

```
+-----------------------+     +-------------------------+     +------------------+
|  CostsLog (running    |---->|  BudgetGuardrail         |---->|  CascadeRouter   |
|  cost accumulator)    |     |  check() -> action       |     |  cost_weight or  |
|                       |     |                          |     |  candidate filter |
+-----------------------+     +-------------------------+     +------------------+
```

### 8.2 Transform Function

```rust
// Source: accumulated costs
pub struct BudgetState {
    pub task_cost_usd: f64,
    pub session_cost_usd: f64,
    pub day_cost_usd: f64,
    pub task_limit: f64,
    pub session_limit: f64,
    pub day_limit: f64,
}

// Transform: multi-level budget check
fn cost_to_routing_transform(state: &BudgetState) -> BudgetRoutingAction {
    let task_pct = state.task_cost_usd / state.task_limit;
    let session_pct = state.session_cost_usd / state.session_limit;
    let day_pct = state.day_cost_usd / state.day_limit;
    let max_pct = task_pct.max(session_pct).max(day_pct);

    if max_pct >= 1.0 {
        BudgetRoutingAction::HardStop
    } else if max_pct >= 0.95 {
        BudgetRoutingAction::Block
    } else if max_pct >= 0.80 {
        BudgetRoutingAction::Downgrade {
            max_cost_per_m: 0.50,
        }
    } else {
        BudgetRoutingAction::Continue
    }
}

pub enum BudgetRoutingAction {
    Continue,
    Downgrade { max_cost_per_m: f64 },
    Block,
    HardStop,
}
```

### 8.3 Specification

| Property | Value |
|----------|-------|
| Latency requirement | Per-task (< 1ms). Budget check is arithmetic comparison |
| Update frequency | Every episode (per-dispatch) |
| Failure mode if loop breaks | System exceeds budget -> unexpected charges -> operator loses trust |
| Recovery | Hard stop at 100% budget via separate watchdog |
| Impact | Prevents cost overruns. System degrades gracefully (cheaper models) rather than halting |

---

## 9. Loop 7: Latency -> Reward

**Source:** `LatencyRegistry` (per-model, per-provider latency statistics)
**Target:** Bandit reward signal (used to update LinUCB/UCB1 arms)
**Mechanism:** Include latency as a component of the reward signal, so the
bandit learns to avoid slow models when latency SLAs are tight.
**Status:** Wired. Runtime feedback computes routing reward with observed
latency plus model/provider latency registries.

### 9.1 Data Flow

```
+-----------------------+     +-------------------------+     +------------------+
|  LatencyRegistry      |---->|  Latency Reward          |---->|  Bandit Update   |
|  (EWMA per model,     |     |  Computation (SLA        |     |  (composite      |
|   p50/p95/p99)        |     |   compliance scoring)    |     |   reward signal) |
+-----------------------+     +-------------------------+     +------------------+
```

### 9.2 Transform Function

```rust
// Source: LatencyRegistry state
pub struct LatencyStats {
    pub model: String,
    pub provider: String,
    pub ewma_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
    pub sample_count: u64,
}

// Transform: latency -> reward component
fn latency_to_reward_transform(
    stats: &LatencyStats,
    sla_ms: u64,  // from RoutingConfig, default: 5000
) -> f64 {
    let sla = sla_ms as f64;
    if stats.ewma_ms <= sla * 0.5 {
        1.0  // Well within SLA -> full reward
    } else if stats.ewma_ms <= sla {
        // Linear decay from 1.0 to 0.5 as latency approaches SLA
        1.0 - 0.5 * ((stats.ewma_ms - sla * 0.5) / (sla * 0.5))
    } else {
        // Beyond SLA -> penalty proportional to overshoot
        (0.5 * sla / stats.ewma_ms).max(0.0)
    }
}

// Sink: composite reward for bandit update
fn composite_reward(
    quality: f64,        // gate pass = 1.0, fail = 0.0
    cost_reward: f64,    // 1.0 - normalized_cost
    latency_reward: f64, // from transform above
    weights: &RewardWeights,
) -> f64 {
    weights.quality * quality
        + weights.cost * cost_reward
        + weights.latency * latency_reward
}

pub struct RewardWeights {
    pub quality: f64,   // default: 0.60
    pub cost: f64,      // default: 0.25
    pub latency: f64,   // default: 0.15
}
```

### 9.3 Specification

| Property | Value |
|----------|-------|
| Latency requirement | Per-episode. Computed as part of bandit update |
| Update frequency | Every episode |
| Failure mode if loop breaks | Bandit selects high-quality but slow models -> SLA violations -> user-facing delays |
| Recovery | Manual latency SLA enforcement in `roko.toml` routing config |
| Impact | Prevents bandit from selecting slow models when latency SLA is tight |

---

## 10. Loop 8: Experiments -> Static

**Source:** `ExperimentStore` (concluded experiments with identified winners)
**Target:** Static routing table (stage-1 defaults)
**Mechanism:** When a prompt experiment concludes with a clear winner, update
the static configuration to use the winning variant. When a model experiment
identifies the best model for a (role, complexity) pair, update the static
routing table.
**Status:** Wired. Concluded experiments persist their winner; the cascade
router's static table is updated.

### 10.1 Data Flow

```
+-----------------------+     +-------------------------+     +------------------+
|  ExperimentStore      |---->|  Significance Tester     |---->|  Static Config   |
|  (concluded expts     |     |  (chi-squared or         |     |  (roko.toml      |
|   with variant data)  |     |   z-test for winner)     |     |   updates)       |
+-----------------------+     +-------------------------+     +------------------+
```

### 10.2 Transform Function

```rust
// Source: ExperimentStore concluded experiments
pub struct ExperimentConclusion {
    pub experiment_id: String,
    pub section_name: String,
    pub winner_variant: String,
    pub winner_pass_rate: f64,
    pub baseline_pass_rate: f64,
    pub delta: f64,
    pub p_value: f64,
    pub sample_size: usize,
}

// Transform: statistical significance test
fn experiments_to_static_transform(
    conclusion: &ExperimentConclusion,
    min_delta: f64,     // default: 0.05 (5% improvement required)
    max_p_value: f64,   // default: 0.05
    min_samples: usize, // default: 50
) -> Option<ConfigUpdate> {
    if conclusion.delta < min_delta
        || conclusion.p_value > max_p_value
        || conclusion.sample_size < min_samples
    {
        return None; // Not significant enough to promote
    }

    Some(ConfigUpdate {
        key: format!("prompt.{}.variant", conclusion.section_name),
        old_value: "baseline".into(),
        new_value: conclusion.winner_variant.clone(),
        reason: format!(
            "Experiment {} concluded: variant '{}' improved pass rate \
             by {:.1}% (p={:.4}, n={})",
            conclusion.experiment_id,
            conclusion.winner_variant,
            conclusion.delta * 100.0,
            conclusion.p_value,
            conclusion.sample_size,
        ),
        requires_review: true,
    })
}
```

### 10.3 Specification

| Property | Value |
|----------|-------|
| Latency requirement | Batch (checked every 50 episodes). Config changes need human review |
| Update frequency | On experiment conclusion |
| Failure mode if loop breaks | Experiment results are transient -> system re-runs same experiments indefinitely |
| Recovery | Manual config update based on experiment logs |
| Impact | Makes experiment improvements permanent in the static routing table |

---

## 11. Summary Table

| # | Loop | Source | Target | Status |
|---|------|--------|--------|--------|
| 1 | Health -> Routing | ProviderHealthRegistry | CascadeRouter candidate filter | **Wired** |
| 2 | Conductor -> Routing | Conductor load signals | CascadeRouter bias | **Wired** |
| 3 | Section -> Scaffold | PromptSectionMeta | Composer section weights | **Wired** |
| 4 | Failure -> Replanning | Gate failure patterns | Plan generator | **Wired** |
| 5 | Skills -> Prompts | SkillLibrary | SystemPromptBuilder | **Wired** |
| 6 | Cost -> Routing | Budget guardrails | CascadeRouter tier | **Wired** |
| 7 | Latency -> Reward | LatencyRegistry | Bandit reward signal | **Wired** |
| 8 | Experiments -> Static | ExperimentStore | Static config | **Wired** |

---

## 12. Cross-Loop Interaction Matrix

The eight loops do not operate independently -- they interact. These
interactions must be understood to prevent cascading oscillation.

| Source Loop | Affected Loop | Interaction |
|-------------|---------------|-------------|
| 1 (Health->Routing) | 6 (Cost->Routing) | Provider failure forces fallback to more expensive provider |
| 2 (Conductor->Routing) | 7 (Latency->Reward) | High system load increases latency, penalizing reward signals |
| 3 (Section->Scaffold) | 5 (Skills->Prompts) | Section weight changes may truncate skill injection section |
| 4 (Failure->Replan) | 6 (Cost->Routing) | Replanning creates new tasks, increasing session cost |
| 6 (Cost->Routing) | 1 (Health->Routing) | Cost-forced downgrade to cheap provider may hit rate limits |
| 7 (Latency->Reward) | 2 (Conductor->Routing) | Latency-optimal routing may increase system load |
| 8 (Experiments->Static) | 3 (Section->Scaffold) | Winner changes section content, resetting effectiveness data |

---

## 13. Interaction-Aware Scheduling

To prevent cascading oscillation from loop interactions, updates are
scheduled with awareness of their dependencies. Safety-critical loops
always run before learning loops, preventing a scenario where a learning-
driven change causes a safety-critical failure.

```
Priority 1 (every episode): Loop 1 (Health), Loop 6 (Cost)
    -> Safety-critical: prevent provider failures and budget overruns

Priority 2 (every 5 episodes): Loop 7 (Latency), Loop 2 (Conductor)
    -> Performance: optimize for speed and resource utilization

Priority 3 (every 20 episodes): Loop 3 (Section), Loop 5 (Skills)
    -> Learning: adjust prompt composition based on accumulated evidence

Priority 4 (every 50 episodes): Loop 4 (Failure->Replan), Loop 8 (Experiments)
    -> Strategic: structural changes with high confidence requirements
```

The priority ordering ensures safety-critical loops (health, cost) run
every episode, while strategic loops (replanning, experiment conclusion)
run only with substantial accumulated evidence.

---

## 14. Cybernetic Theory

### 14.1 Ashby's Law of Requisite Variety (Ashby 1956)

A control system must have at least as much variety (number of distinct
states) as the system it controls. The eight loops implement this by
providing a different corrective mechanism for each type of deviation:

- Provider health degrades -> route away (loop 1)
- System overloaded -> use cheaper models (loop 2)
- Section wasteful -> reduce its weight (loop 3)
- Task intractable -> decompose differently (loop 4)
- Skill available -> inject it (loop 5)
- Budget exhausted -> downgrade quality (loop 6)
- Latency excessive -> penalize slow models (loop 7)
- Experiment concluded -> lock in winner (loop 8)

Each type of deviation has a matched corrective mechanism. If any loop were
missing, the system would lack the variety needed to regulate that type of
disturbance, and the disturbance would propagate uncorrected.

Ashby formulated the law in *An Introduction to Cybernetics* (1956): "Only
variety can destroy variety." A thermostat with one setting cannot regulate
a room with two heat sources; a routing system with one model cannot
regulate eight types of performance deviation.

### 14.2 Beer's Viable System Model (Beer 1972)

Beer's VSM defines five systems required for organizational viability.
The eight feedback loops map to VSM functions:

| VSM System | Function | Roko Implementation |
|-----------|----------|---------------------|
| System 1 | Operations | Individual learning subsystems |
| System 2 | Coordination | Frequency separation, scheduling order |
| System 3 | Control | Regression detection, C-Factor monitoring |
| System 4 | Intelligence | Pattern discovery, skill extraction |
| System 5 | Policy | Hysteresis thresholds, EMA parameters |

The feedback loops primarily implement Systems 2 and 3: coordination
between subsystems and control over aggregate behavior. System 1 is the
individual subsystems themselves. Systems 4 and 5 are the slower learning
and policy layers described in the four-loop timescale architecture
(section 13 of the parent chapter).

Beer's contribution to Roko's architecture is the insight that a viable
system must be **recursively composed**: each subsystem must itself be a
viable system. The cascade router is viable independently (it has its own
health check, fallback, and persistence), as is the playbook store, the
episode logger, and every other subsystem. The eight loops connect these
independently viable subsystems into a collectively viable whole.

### 14.3 Double-Loop Learning (Argyris & Schon 1978)

Single-loop learning corrects errors by adjusting actions within the current
strategy. Double-loop learning corrects errors by questioning and revising
the strategy itself.

The eight loops implement both levels:

- **Single-loop:** Loops 1, 6, 7 adjust routing decisions within the
  existing cascade framework (correcting selection within strategy).
- **Double-loop:** Loops 3, 4, 5, 8 modify the strategy itself (changing
  prompt composition, decomposing tasks, locking in experiment winners).

Loop 4 (Failure -> Replanning) is the clearest example of double-loop
learning: rather than retrying the same task strategy, it questions
whether the task decomposition itself is correct and generates a new
strategy.

### 14.4 Good Regulator Theorem (Conant & Ashby 1970)

A system that is a good regulator of another system must be a model of that
system. The C-Factor composite metric is the system's model of its own
health -- capturing the key performance indicators in a single scalar. The
regression detector uses this model to identify when the system deviates
from expected behavior, triggering the corrective actions described in the
eight loops.

---

## 15. Compound Effect

The compound effect of all eight loops operating simultaneously is that the
system converges toward an optimal operating point without manual tuning.
Each loop independently applies negative feedback to correct a specific
type of deviation. The interactions between loops (section 12) create
second-order effects that can be either stabilizing or destabilizing. The
interaction-aware scheduling (section 13) and stability mechanisms
(hysteresis, frequency separation, EMA damping) ensure that the compound
effect is convergent rather than oscillatory.

See [stability-mechanisms.md](stability-mechanisms.md)
for how oscillation is prevented, and
[autocatalytic-compounding.md](autocatalytic-compounding.md)
for why the compound improvement rate can be super-linear once all loops
are connected.

---

## References

- Ashby, W.R. (1956). *An Introduction to Cybernetics*. Chapman & Hall.
- Beer, S. (1972). *Brain of the Firm*. Allen Lane.
- Conant, R.C. & Ashby, W.R. (1970). Every good regulator of a system must
  be a model of that system. *International Journal of Systems Science*
  1(2), 89-97.
- Argyris, C. & Schon, D.A. (1978). *Organizational Learning: A Theory
  of Action Perspective*. Addison-Wesley.
