# 17 — Meta-Learning Integration, Feedback Loop Architecture, and Final Corrections

> **Priority**: 🟡 P1 — Connects all learning subsystems into a coherent self-improving system
> **Status**: Not started
> **Depends on**: 08 (learning loops), 14 (tool loop wiring)

## Critical Discoveries (Final Code Audit)

### Everything that already exists and is wired

The final code audit reveals that roko's learning infrastructure is **far more complete than the earlier plans assumed**. The `LearningRuntime` at `crates/roko-learn/src/runtime_feedback.rs` is the central hub that already coordinates:

| Component | File | Wired Via |
|---|---|---|
| Episode logger | `episode_logger.rs` | `runtime_feedback.rs` L~400 |
| Cost logs | `costs_log.rs` | `runtime_feedback.rs` L~450 |
| Provider health | `provider_health.rs` | `runtime_feedback.rs` L~521 |
| Playbook outcomes | `playbook.rs` | `runtime_feedback.rs` L~530 |
| Playbook rules | `playbook_rules.rs` | `runtime_feedback.rs` L~537 |
| Skill library usage | `skill_library.rs` | `runtime_feedback.rs` L~557 |
| Pattern mining | `pattern_discovery.rs` | `runtime_feedback.rs` L~575 |
| Cascade router | `cascade_router.rs` | `runtime_feedback.rs` L~603 |
| Experiment store | `prompt_experiment.rs` | `runtime_feedback.rs` L~591 |
| Task metrics | `task_metric.rs` | `runtime_feedback.rs` L~548 |
| Regression detection | `regression.rs` | `runtime_feedback.rs` |

Additionally:
- **`bandits.rs`** — Already has UCB1 + TrackAndStop (not just UCB1)
- **`context_pack_cache.rs`** — LRU cache of composed prompts by task fingerprint
- **10 conductor watchers** — compile_fail_repeat, context_window_pressure, cost_overrun, ghost_turn, iteration_loop, review_loop, spec_drift, stuck_pattern, test_failure_budget, time_overrun
- **`roko-neuro`** — Knowledge distillation (Episode → Insights → Heuristics → Playbook) is actively being built

### What this means for the plans

Many tasks in docs 08, 12, and 13 propose building things that already exist:

| Proposed Task | What Actually Exists | Correct Action |
|---|---|---|
| 2G.01–03: Build ProviderHealth | `provider_health.rs` with 3-state breaker | **Extend** with error classification |
| 2G.12–15: Build AnomalyDetector | 10 conductor watchers | **Wire** existing watchers into provider routing |
| 2J.07–08: Build skill library | `skill_library.rs` + `runtime_feedback.rs` wiring | **Extend** with model-specific skills |
| 2J.03–04: Build calibration tracker | Not yet built (genuinely new) | Build as proposed |
| 2K.20–23: Build EventBus | `runtime_feedback.rs` is the event hub | **Extend** with async pub/sub |
| 2J.05–06: Section effectiveness | Not yet built (genuinely new) | Build as proposed |

### Genuinely new work

These are truly new and not covered by existing infrastructure:

1. **`LlmBackend` implementations** for HTTP providers (doc 14 — confirmed needed)
2. **`ProviderKind`/`ProviderConfig`/`ModelProfile`** config types (doc 02 — confirmed needed)
3. **`ProviderAdapter`** trait (doc 03 — confirmed needed)
4. **Thompson Sampling** as alternative to UCB1 (doc 12 — `bandits.rs` has UCB1/TrackAndStop but no Thompson)
5. **Token counting** client-side (doc 14 — not found anywhere)
6. **Rate limiting** with governor (doc 14 — not found)
7. **Cache layer alignment** in prompt assembly (doc 13 — `context_pack_cache.rs` caches composed prompts but doesn't optimize prefix ordering)
8. **CLI commands** for providers/models (doc 15 — zero surface area confirmed)
9. **Serve API routes** for providers (doc 16 — zero routes confirmed)
10. **Predictive Foraging** calibration (doc 12 — genuinely new)
11. **Section effectiveness tracking** (doc 12 — genuinely new)
12. **Multi-objective reward weights** (doc 12 — reward function exists but weights are hardcoded)

---

## A. The Learning System Interaction Graph

### Current Data Flow

```
Agent Turn Completes
    │
    └── LearningRuntime::record_completed_run()  ← THE HUB
            │
            ├── EpisodeLogger.append()           ── .roko/episodes.jsonl
            ├── CostLog.append()                 ── .roko/learn/costs.jsonl
            ├── ProviderHealth.record()           ── in-memory
            ├── Playbook.record_outcome()         ── .roko/learn/playbook.json
            ├── PlaybookRules.record_outcome()    ── .roko/learn/playbook-rules.json
            ├── SkillLibrary.record_usage()       ── .roko/learn/skills.json
            ├── PatternDiscovery.mine()           ── .roko/learn/patterns.json
            ├── CascadeRouter.observe()           ── .roko/learn/cascade-router.json
            ├── ExperimentStore.record()          ── .roko/learn/experiments.json
            ├── TaskMetric.record()               ── .roko/learn/task-metrics.jsonl
            └── RegressionDetector.check()        ── alerts

Background:
    FeedbackLoop (every 15min)
        ├── GitHub: PR merge status, reactions → sentiment
        └── Slack: reactions, replies → sentiment
        └── Both → CascadeRouter.record_observation()

In Progress (roko-neuro):
    KnowledgeDistiller
        ├── Episodes → Insights (pattern extraction)
        ├── Insights → Heuristics (principle formation)
        └── Heuristics → Playbook (rule promotion)
```

### Missing Feedback Loops

Despite the comprehensive infrastructure, several feedback connections are not wired:

```
1. Provider Health ──✗──→ CascadeRouter
   (health doesn't influence model selection — they're independent)

2. Conductor Watchers ──✗──→ CascadeRouter
   (stuck detection doesn't negatively impact the model that caused it)

3. Section Effectiveness ──✗──→ SystemPromptBuilder
   (no tracking of which prompt sections correlate with success)

4. Gate Failure Patterns ──✗──→ Plan Generator
   (failed gates don't inform re-planning or task decomposition)

5. Skill Library ──✗──→ Prompt Assembly
   (skills exist but aren't injected into prompts based on task match)

6. Cost Anomalies ──✗──→ Model Routing
   (cost spikes don't trigger model downgrade or provider switch)

7. Latency Stats ──✗──→ Routing Reward
   (routing reward uses static duration, not observed latency)

8. Model Experiments ──✗──→ CascadeRouter Static Table
   (experiment conclusions don't update the cold-start routing table)
```

---

### 2O.01 — Wire ProviderHealth into CascadeRouter selection

**File**: `crates/roko-learn/src/runtime_feedback.rs`
**What**: Before model selection, filter out unhealthy providers:

```rust
// In the routing path (orchestrate.rs or wherever route() is called):
let healthy_models = provider_health.filter_arms(&all_model_slugs);
let selected = cascade_router.route_among(&healthy_models, &ctx);
```

**Context**: `ProviderHealthTracker::filter_arms()` already exists but isn't called before routing. This connects health → routing so unhealthy providers are excluded.

**Acceptance**: Model on unhealthy provider is not selected.
**Verification**: `cargo test -p roko-learn -- health_filters_routing`

---

### 2O.02 — Wire Conductor stuck detection into negative routing signal

**File**: `crates/roko-learn/src/runtime_feedback.rs`
**What**: When a conductor watcher detects a stuck pattern, record a negative observation for the model that caused it:

```rust
fn on_conductor_intervention(model_slug: &str, intervention: &ConductorDecision) {
    if matches!(intervention, ConductorDecision::Abort | ConductorDecision::Restart) {
        cascade_router.record_observation(
            &routing_context,
            model_slug,
            0.0,    // zero reward
            false,  // failure
        );
    }
}
```

**Context**: Currently, conductor interventions (abort, restart) don't feed back into the router. The router thinks the model is fine because no observation was recorded. This creates a blind spot.

**Acceptance**: Aborted task records negative observation for the model.
**Verification**: `cargo test -p roko-learn -- conductor_negative_feedback`

---

### 2O.03 — Wire Skill Library into prompt assembly

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Before assembling the prompt, query the skill library for relevant skills and inject them:

```rust
let relevant_skills = skill_library.query_for_task(&task_category, &crate_name, top_k: 5);
if !relevant_skills.is_empty() {
    let skills_section = format_skills_for_prompt(&relevant_skills);
    prompt_builder.with_skills(skills_section);
}
```

**Context**: The skill library has skills with confidence scores and usage telemetry. They should be injected into prompts the same way playbook rules are (mori-agents has a "skills" budget of 4-8K tokens per prompt).

**Acceptance**: High-confidence skills appear in the agent's prompt for matching tasks.
**Verification**: `cargo test -p roko-cli -- skill_injection`

---

### 2O.04 — Wire cost anomaly detection into model downgrade

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: When cost EWMA detects a spike (z-score > 3.0), force routing to cheaper model:

```rust
if let Some(Anomaly::CostSpike { z_score }) = anomaly_detector.check_cost(turn_cost) {
    tracing::warn!(z_score, "cost anomaly detected — forcing cheaper model");
    // Override next model selection to mechanical tier
    force_model_override = Some(config.agent.tier_models.mechanical.clone());
}
```

**Context**: Conductor watcher `cost_overrun.rs` already detects cost threshold exceeded. This connects it to the routing decision so the system self-corrects.

**Acceptance**: Cost spike triggers model downgrade on next turn.
**Verification**: `cargo test -p roko-cli -- cost_anomaly_downgrade`

---

### 2O.05 — Wire observed latency into routing reward

**File**: `crates/roko-learn/src/runtime_feedback.rs`
**What**: Replace static `normalized_duration` with actual latency from efficiency events:

```rust
fn compute_reward_with_latency(
    gate_passed: bool,
    cost_usd: f64,
    wall_time_ms: u64,
    latency_stats: &LatencyRegistry,
    model: &str,
    provider: &str,
) -> f64 {
    let pass_rate = if gate_passed { 1.0 } else { 0.0 };
    let max_cost = 5.0;  // reference ceiling
    let normalized_cost = (cost_usd / max_cost).min(1.0);
    let p50 = latency_stats.get(model, provider).map(|s| s.p50_ms()).unwrap_or(30_000.0);
    let sla = 120_000.0;  // 2 minute reference SLA
    let normalized_duration = (wall_time_ms as f64 / sla).min(1.0);

    pass_rate * 0.5 + (1.0 - normalized_cost) * 0.3 + (1.0 - normalized_duration) * 0.2
}
```

**Context**: The existing `compute_routing_reward` at `model_router.rs` L180 takes `normalized_duration` but it's computed without actual latency tracking. This connects the latency registry (doc 08 task 2G.05–06) to the reward function.

**Acceptance**: Faster models get higher rewards, slower models get lower rewards.
**Verification**: `cargo test -p roko-learn -- latency_aware_reward`

---

### 2O.06 — Wire experiment conclusions into CascadeRouter static table

**File**: `crates/roko-learn/src/runtime_feedback.rs`
**What**: When a model experiment concludes, update the static routing table:

```rust
fn on_experiment_concluded(experiment: &ModelExperiment) {
    if let (Some(winner), Some(role)) = (&experiment.winner_id, &experiment.role) {
        let winner_variant = experiment.variants.iter().find(|v| &v.id == winner);
        if let Some(variant) = winner_variant {
            cascade_router.update_static_table(
                &AgentRole::from_label(role),
                &variant.slug,
            );
            tracing::info!(
                experiment = %experiment.experiment_id,
                winner = %variant.slug,
                role = role,
                "experiment concluded — updated static routing table"
            );
        }
    }
}
```

**Context**: Currently, experiment conclusions are logged but don't update the router's cold-start table. This means a new session starts from the same static defaults even after an experiment proved a different model is better.

**Acceptance**: After experiment concludes, cold-start routing uses the winner.
**Verification**: `cargo test -p roko-learn -- experiment_updates_static_table`

---

## B. Feedback Loop Stability

### 2O.07 — Add hysteresis to model routing decisions

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: Once a model is selected for a task type, require a significant score delta before switching:

```rust
const HYSTERESIS_THRESHOLD: f64 = 0.10;  // 10% score improvement needed to switch

fn select_with_hysteresis(
    candidates: &[(String, f64)],
    previous_model: Option<&str>,
) -> String {
    let best = candidates.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    if let (Some(best), Some(prev)) = (best, previous_model) {
        if let Some(prev_score) = candidates.iter().find(|(s, _)| s == prev).map(|(_, sc)| sc) {
            // Only switch if new model is significantly better
            if best.1 - prev_score < HYSTERESIS_THRESHOLD {
                return prev.to_string();  // Stay with current model
            }
        }
    }
    best.map(|(s, _)| s.clone()).unwrap_or_default()
}
```

**Context**: From control theory — hysteresis prevents oscillation between models that are close in score. Without it, noise in observations causes the router to flip-flop between models on each task. This is especially important when cache affinity bonus (doc 08 task 2G.09) is in effect: switching models costs cache hits.

**Acceptance**: Model switch requires 10% score improvement over incumbent.
**Verification**: `cargo test -p roko-learn -- routing_hysteresis`

---

### 2O.08 — Add update frequency separation across learning subsystems

**File**: `crates/roko-learn/src/runtime_feedback.rs`
**What**: Different subsystems should update at different frequencies to prevent coupled oscillation:

```rust
pub struct UpdateFrequency {
    pub router_every_n_episodes: u32,        // 1 (every episode)
    pub gate_thresholds_every_n: u32,        // 5 (batch updates)
    pub experiments_every_n: u32,            // 1 (every episode)
    pub skill_mining_every_n: u32,           // 10 (batch processing)
    pub pattern_discovery_every_n: u32,      // 20 (expensive HDC)
    pub distiller_every_n: u32,             // 50 (major consolidation)
}

impl LearningRuntime {
    pub fn record_completed_run(&mut self, ...) {
        self.episode_count += 1;

        // Always: episode log, costs, provider health
        self.episode_logger.append(&episode)?;
        self.costs.record(cost_record);
        self.provider_health.record(provider_id, success);

        // High frequency: routing, experiments
        if self.episode_count % self.freq.router_every_n_episodes == 0 {
            self.cascade_router.record_observation(...);
        }

        // Medium frequency: gate thresholds, skills
        if self.episode_count % self.freq.gate_thresholds_every_n == 0 {
            self.adaptive_thresholds.update_batch(...);
        }

        // Low frequency: pattern mining, distillation
        if self.episode_count % self.freq.pattern_discovery_every_n == 0 {
            self.pattern_miner.mine_batch(...);
        }
    }
}
```

**Context**: From control theory — coupled systems with different natural frequencies need frequency separation to avoid resonance. The router should respond quickly (per-episode). Gate thresholds should smooth over many episodes (EMA already does this). Pattern discovery is expensive and should run in batches.

**Acceptance**: Subsystems update at configured frequencies, not all on every episode.
**Verification**: `cargo test -p roko-learn -- update_frequency_separation`

---

## C. Compound System Optimization

### 2O.09 — Add Optimas-style local reward function per subsystem

**File**: `crates/roko-learn/src/local_reward.rs` (new)
**What**: Each subsystem gets a reward function trained to predict whether its local decisions lead to globally good outcomes:

```rust
/// Local reward function that predicts global outcome from local decision.
/// Trained from (local_decision, global_outcome) pairs.
pub struct LocalRewardFunction {
    /// Per-subsystem EMA of: "when I made this choice, did the task succeed?"
    decision_outcomes: HashMap<String, (u64, u64)>,  // (successes, total)
}

impl LocalRewardFunction {
    /// Estimate how good a local decision is based on historical global outcomes.
    pub fn score(&self, decision_key: &str) -> f64 {
        self.decision_outcomes.get(decision_key)
            .map(|(s, t)| *s as f64 / (*t).max(1) as f64)
            .unwrap_or(0.5)  // prior: 50% for unknown decisions
    }

    /// Update after observing global outcome.
    pub fn observe(&mut self, decision_key: &str, global_success: bool) {
        let entry = self.decision_outcomes.entry(decision_key.to_string()).or_insert((0, 0));
        entry.1 += 1;
        if global_success { entry.0 += 1; }
    }
}
```

**Example**: For the routing subsystem, `decision_key` = `"glm-5.1:implementer:integrative"`. For the prompt subsystem, `decision_key` = `"workspace_map:included"`. Each subsystem learns which of its decisions correlate with global task success.

**Context**: From the Optimas framework (Stanford, 2025). The key insight: local reward functions that predict global outcomes enable independent optimization of each subsystem while maintaining system-level coherence. 11.92% average improvement across compound systems.

**Acceptance**: After 100 episodes, LRFs can predict global success from local decisions with > 60% accuracy.
**Verification**: `cargo test -p roko-learn -- local_reward_functions`

---

### 2O.10 — Add progressive stage transitions for learning maturity

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: The existing 3-stage cascade (Static → Confidence → UCB) should log stage transitions and expose them for monitoring:

```rust
impl CascadeRouter {
    pub fn check_stage_transition(&self) -> Option<StageTransition> {
        let obs = self.total_observations();
        let current = self.current_stage();
        let next = match obs {
            0..=49 => CascadeStage::Static,
            50..=199 => CascadeStage::Confidence,
            _ => CascadeStage::Ucb,
        };
        if next != current {
            Some(StageTransition {
                from: current,
                to: next,
                observations: obs,
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }
}

pub struct StageTransition {
    pub from: CascadeStage,
    pub to: CascadeStage,
    pub observations: u64,
    pub timestamp: DateTime<Utc>,
}
```

**Context**: Stage transitions are significant events. When the router moves from Static to Confidence (50 obs), it starts learning. When it moves to UCB (200 obs), it's fully adaptive. These should be logged and visible in the dashboard.

**Acceptance**: Stage transitions are logged with observation count and timestamp.
**Verification**: `cargo test -p roko-learn -- stage_transition_logging`

---

## D. Knowledge Distillation Integration (with roko-neuro)

### 2O.11 — Connect roko-neuro distiller to prompt assembly

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: The roko-neuro distiller (currently WIP) produces heuristics at different tiers. Connect Playbook-tier heuristics to prompt assembly:

```rust
// In orchestrate.rs, before prompt assembly:
let heuristics = neuro_store.query_tier(
    Tier::Playbook,                    // Highest-confidence tier
    &task.category,
    &task.crate_name,
    top_k: 3,
);
if !heuristics.is_empty() {
    let heuristic_section = format_heuristics(&heuristics);
    prompt_builder.with_heuristics(heuristic_section);
}
```

**Context**: roko-neuro's tier progression (Episode → Insights → Heuristics → Playbook) is being actively built. This task connects the output of that pipeline to the prompt assembly, closing the loop: execution → episodes → distillation → heuristics → better prompts → better execution.

**Acceptance**: Playbook-tier heuristics appear in agent prompts for matching task types.
**Verification**: `cargo test -p roko-cli -- neuro_heuristic_injection`

---

### 2O.12 — Add model-specific heuristic tagging

**File**: `crates/roko-neuro/src/distiller.rs` (or wherever heuristics are created)
**What**: Tag extracted heuristics with which model produced them:

```rust
pub struct Heuristic {
    // ... existing fields ...
    pub source_model: Option<String>,  // Which model generated the episode this came from
    pub model_generality: f64,         // 1.0 = works for all models, 0.0 = model-specific
}
```

When injecting heuristics into prompts, prefer model-general heuristics over model-specific ones (unless the same model is being used):

```rust
fn filter_heuristics(heuristics: &[Heuristic], current_model: &str) -> Vec<&Heuristic> {
    heuristics.iter()
        .filter(|h| {
            h.model_generality > 0.7  // General heuristic
            || h.source_model.as_deref() == Some(current_model)  // Same model
        })
        .collect()
}
```

**Context**: A heuristic like "always read the file before editing" is model-general. But "use XML tags for tool call formatting" is model-specific. Injecting model-specific heuristics for the wrong model degrades performance.

**Acceptance**: Model-specific heuristics are only injected when using the matching model.
**Verification**: `cargo test -p roko-neuro -- model_specific_heuristics`

---

## E. Compound System Dashboard

### 2O.13 — Add learning system interaction diagram to dashboard

**File**: `crates/roko-cli/src/tui/pages/efficiency.rs`
**What**: Add a dashboard view showing the data flow between learning subsystems:

```
╔══════════════════════════════════════════════════╗
║  Learning System Status                           ║
╠══════════════════════════════════════════════════╣
║                                                   ║
║  Stage: UCB (423 observations)                    ║
║  Last transition: Confidence → UCB at obs 201     ║
║                                                   ║
║  Subsystem         Updates  Last      Health      ║
║  CascadeRouter     423      2m ago    ● learning  ║
║  GateThresholds    84       10m ago   ● stable    ║
║  Experiments       2 running           ● active   ║
║  SkillLibrary      37 skills           ● growing  ║
║  PatternMiner      12 patterns         ● mining   ║
║  ProviderHealth    3 providers         ● healthy  ║
║  KnowledgeStore    [WIP]                          ║
║                                                   ║
║  Feedback Loops:  6/8 connected                   ║
║  Missing: GateFail→Replan, SectionEffect→Prompt  ║
║                                                   ║
╚══════════════════════════════════════════════════╝
```

**Acceptance**: Dashboard page shows all learning subsystems with update counts and health.
**Verification**: `cargo run -p roko-cli -- dashboard --page learning`

---

## Summary

| Section | Tasks | IDs | Key Contribution |
|---|---|---|---|
| **A. Missing feedback wires** | 6 | 2O.01–2O.06 | Connect 6 of 8 disconnected feedback loops |
| **B. Stability mechanisms** | 2 | 2O.07–2O.08 | Hysteresis + frequency separation prevent oscillation |
| **C. Compound optimization** | 2 | 2O.09–2O.10 | Local reward functions + stage transition logging |
| **D. Knowledge integration** | 2 | 2O.11–2O.12 | Connect roko-neuro distiller to prompt assembly |
| **E. Dashboard** | 1 | 2O.13 | Visualize learning system interactions |
| **Total** | **13** | **2O.01–2O.13** | |

## All Corrections Summary

This document represents the final audit. These are ALL the corrections across the entire plan suite:

| What Exists | Where Found | Which Docs Affected | Correction |
|---|---|---|---|
| `ToolLoop` | `roko-agent/src/tool_loop/mod.rs` | Doc 13 (2K.05–09) | Superseded by doc 14 (2L.01–05) |
| `ProviderHealthTracker` | `roko-learn/src/provider_health.rs` | Doc 08 (2G.01–03) | Extend, don't rebuild |
| `runtime_feedback.rs` hub | `roko-learn/src/runtime_feedback.rs` | Doc 13 (2K.20–23) EventBus | Extend existing hub |
| `bandits.rs` UCB1+TrackAndStop | `roko-learn/src/bandits.rs` | Doc 12 (Thompson) | Add Thompson alongside existing |
| `skill_library.rs` wired | `roko-learn/src/skill_library.rs` | Doc 12 (2J.07–08) | Extend, not build |
| `playbook.rs` wired | `roko-learn/src/playbook.rs` | Doc 12 (knowledge) | Already exists |
| `pattern_discovery.rs` | `roko-learn/src/pattern_discovery.rs` | Doc 12 (skills) | Already exists with HDC |
| `context_pack_cache.rs` | `roko-learn/src/context_pack_cache.rs` | Doc 13 (cache) | Separate from KV prefix cache |
| 10 conductor watchers | `roko-conductor/src/watchers/*.rs` | Doc 08 (2G.12–16) | Wire existing, don't rebuild |
| `roko-neuro` distiller | `roko-neuro/src/distiller.rs` | Doc 12 (2J.07–08) | In active development |
| `SystemPromptBuilder` cache | `roko-compose/src/system_prompt_builder.rs` | Doc 13 (2K.11–15) | Has cache markers already |
| `mcp_to_tool_def()` | `roko-agent/src/mcp/` | Doc 14 (2L.10) | Exists, needs HTTP wiring |
| `BuildSystem` 6 variants | `roko-gate/src/payload.rs` | Doc 14 (2L.16) | Add auto-detection |
| `ToolDef` is a struct | `roko-core/src/tool/def.rs` | Doc 04 (2C.04) | Don't change to enum |
| CLI Config wraps RokoConfig | `roko-cli/src/config.rs` | Doc 02 (2A.04) | Update both |
| ChatRequest/Response must be in roko-core | Dependency audit | Doc 13 (2K.01) | roko-compose can't import from roko-agent |
| 8 agent creation sites (not 2) | grep for `ClaudeCliAgent::new` | Doc 03 (2B.08-09) | run.rs, agent_exec.rs, orchestrate.rs ×4, dispatch.rs |
| Local ChatRequest in codex/ollama agents | `codex_agent.rs` L69, `ollama_agent.rs` | Doc 13, 19 | Replace with canonical types from roko-core |
| Cascade router hardcodes Claude slugs | `runtime_feedback.rs` L277 | Doc 14, 19 | One-line fix: load from config.effective_models() |
| ExecAgent is NOT a ProviderKind | `exec.rs` — stdin/stdout only | Doc 19 | Legacy fallback, no tools, below routing layer |
| SystemPromptBuilder is model-agnostic | `system_prompt_builder.rs` — no model field | Doc 18 (2P.11), 19 | Add optional model_hint for Phase 3+ |
| `roko run` doesn't use CascadeRouter | `run.rs` L280-360 | Doc 19 | One-shot uses config default, not bandit routing |
| 3 agent dispatch paths (run/plan/serve) | run.rs, orchestrate.rs, dispatch.rs | Doc 03 (2B.08-10), 19 | All 3 need create_agent_for_model() |
| Agent trait returns AgentResult, not Result | `agent.rs` L94 | Doc 03, 19 | Errors wrapped in success=false, not propagated via ? |
| LlmError has only 2 variants | `tool_loop/mod.rs` L58 | Doc 03, 19 | Extend with RateLimit/Auth/etc, don't create separate ProviderError |
| roko-plugin has EventSource+FeedbackCollector | `roko-plugin/src/lib.rs` | Doc 18 (2P.08-10), 19 | Extension mechanism exists; add ProviderPlugin trait Phase 5+ |
