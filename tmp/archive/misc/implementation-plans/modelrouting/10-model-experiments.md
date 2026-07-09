# 10 — Model A/B Experiments

> **Priority**: 🟢 P2 — Active exploration, extends existing ExperimentStore
> **Status**: Not started
> **Depends on**: 08 (learning loops)
> **Blocks**: None

## Problem Statement

The existing `ExperimentStore` A/B tests prompt variants for the same model. There's no mechanism to A/B test models themselves (e.g., "is GLM-5.1 better than Claude Sonnet for Implementer tasks?"). The CascadeRouter learns passively from whatever model happens to be selected. Model experiments actively allocate traffic to test models head-to-head.

## What Exists

| Component | Path | Status |
|---|---|---|
| ExperimentStore | `crates/roko-learn/src/prompt_experiment.rs` | 🔌 UCB1 over prompt variants |
| PromptExperiment | `crates/roko-learn/src/prompt_experiment.rs` | 🔌 Variants, stats, conclusion |
| VariantStats | `crates/roko-learn/src/prompt_experiment.rs` | 🔌 trials, successes, UCB score |
| CascadeRouter | `crates/roko-learn/src/cascade_router.rs` | 🔌 Passive learning |

---

## Checklist

### 2I.01 — Define ModelExperiment struct

**File**: `crates/roko-learn/src/model_experiment.rs` (new)
**What**: A/B testing for model selection:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelExperiment {
    pub experiment_id: String,
    pub description: String,
    pub role: Option<String>,                    // scope to specific role
    pub task_category: Option<String>,           // scope to task type
    pub variants: Vec<ModelVariant>,
    pub stats: HashMap<String, ModelVariantStats>,
    pub status: ExperimentStatus,
    pub winner_id: Option<String>,
    pub min_trials_per_variant: u64,             // default: 20
    pub min_effect_size: f64,                    // default: 0.05
    pub created_at: String,                      // ISO-8601
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVariant {
    pub id: String,
    pub model_key: String,                       // key into [models.*] config
    pub slug: String,                            // API model ID
    pub provider: String,                        // provider key
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelVariantStats {
    pub trials: u64,
    pub successes: u64,
    pub total_cost_usd: f64,
    pub total_tokens: u64,
    pub total_duration_ms: u64,

    // Derived
    pub pass_rate: f64,
    pub avg_cost_usd: f64,
    pub cost_per_success: f64,
    pub avg_duration_ms: f64,
}
```

**Acceptance**: `ModelExperiment` compiles with Serialize/Deserialize.
**Verification**: `cargo test -p roko-learn -- model_experiment_types`

---

### 2I.02 — Implement UCB1 variant assignment for models

**File**: `crates/roko-learn/src/model_experiment.rs`
**What**: Same UCB1 algorithm as prompt experiments:

```rust
impl ModelExperiment {
    pub fn assign_variant(&self) -> Option<&ModelVariant> {
        if self.status == ExperimentStatus::Concluded {
            return self.variants.iter().find(|v| Some(&v.id) == self.winner_id.as_ref());
        }
        // UCB1: select variant with highest UCB score
        let total: u64 = self.stats.values().map(|s| s.trials).sum();
        let mut best = None;
        let mut best_score = f64::NEG_INFINITY;
        for variant in &self.variants {
            let score = self.stats.get(&variant.id)
                .map(|s| s.ucb_score(total))
                .unwrap_or(f64::MAX);  // unsampled = infinity
            if score > best_score {
                best_score = score;
                best = Some(variant);
            }
        }
        best
    }

    pub fn record_outcome(&mut self, variant_id: &str, success: bool, cost_usd: f64, tokens: u64, duration_ms: u64) {
        let stats = self.stats.entry(variant_id.to_string()).or_insert_with(Default::default);
        stats.trials += 1;
        if success { stats.successes += 1; }
        stats.total_cost_usd += cost_usd;
        stats.total_tokens += tokens;
        stats.total_duration_ms += duration_ms;
        stats.recalculate();

        self.check_conclusion();
    }
}
```

**Acceptance**: Unsampled variant always selected first. After min_trials, winner is identified.
**Verification**: `cargo test -p roko-learn -- model_experiment_ucb`

---

### 2I.03 — Create ModelExperimentStore

**File**: `crates/roko-learn/src/model_experiment.rs`
**What**: Registry of active model experiments:

```rust
pub struct ModelExperimentStore {
    experiments: HashMap<String, ModelExperiment>,
}

impl ModelExperimentStore {
    pub fn load_or_new(path: &Path) -> Self;
    pub fn save(&self, path: &Path) -> Result<()>;
    pub fn register(&mut self, experiment: ModelExperiment);
    pub fn active_for_role(&self, role: &str) -> Option<&ModelExperiment>;
    pub fn active_for_category(&self, category: &str) -> Option<&ModelExperiment>;
    pub fn assign_model(&self, role: &str, category: &str) -> Option<ModelVariant>;
    pub fn record_outcome(&mut self, experiment_id: &str, variant_id: &str, success: bool, cost: f64, tokens: u64, duration: u64);
    pub fn running_count(&self) -> usize;
    pub fn concluded_experiments(&self) -> Vec<&ModelExperiment>;
}
```

**Persistence**: `.roko/learn/model-experiments.json`

**Acceptance**: Store persists and loads experiments.
**Verification**: `cargo test -p roko-learn -- model_experiment_store`

---

### 2I.04 — Wire ModelExperimentStore into CascadeRouter

**File**: `crates/roko-learn/src/cascade_router.rs`
**What**: When a model experiment is active for the current role/task, override the router's selection:

```rust
pub fn route_with_experiments(
    &self,
    ctx: &RoutingContext,
    experiments: &ModelExperimentStore,
) -> CascadeModel {
    // Check if there's an active experiment for this role+category
    if let Some(variant) = experiments.assign_model(&ctx.role.label(), &ctx.task_category.label()) {
        return CascadeModel {
            primary: ModelSpec::from_slug(&variant.slug),
            fallback: None,
            latency_sla_ms: 30_000,
            stage: CascadeStage::Static,  // experiments override routing
        };
    }
    // Fall back to normal routing
    self.route(ctx)
}
```

**Acceptance**: Active experiment overrides normal model selection.
**Verification**: `cargo test -p roko-learn -- experiment_override`

---

### 2I.05 — Add CLI command to create model experiments

**File**: `crates/roko-cli/src/commands/experiment.rs` (new or extend existing)
**What**: `roko experiment model create` command:

```bash
cargo run -p roko-cli -- experiment model create \
    --id "glm-vs-kimi-impl" \
    --role implementer \
    --variant "glm-5-1:glm-5.1:zai" \
    --variant "kimi-k2-5:kimi-k2.5:moonshot" \
    --min-trials 20
```

**Acceptance**: Creates experiment in `.roko/learn/model-experiments.json`.
**Verification**: `cargo run -p roko-cli -- experiment model list`

---

### 2I.06 — Add CLI command to view experiment results

**File**: `crates/roko-cli/src/commands/experiment.rs`
**What**: `roko experiment model show <id>` command:

```
Experiment: glm-vs-kimi-impl (Running)
Role: implementer | Category: any

┌──────────┬────────┬─────────┬──────────┬───────────┬──────────┐
│ Variant  │ Trials │ Pass %  │ Avg Cost │ $/Success │ UCB Score│
├──────────┼────────┼─────────┼──────────┼───────────┼──────────┤
│ glm-5-1  │   15   │  80.0%  │  $0.19   │   $0.24   │   0.92   │
│ kimi-k2-5│   12   │  75.0%  │  $0.08   │   $0.11   │   0.89   │
└──────────┴────────┴─────────┴──────────┴───────────┴──────────┘

Status: Need 8 more trials for glm-5-1, 8 more for kimi-k2-5
```

**Acceptance**: Shows variant stats table with UCB scores.
**Verification**: `cargo run -p roko-cli -- experiment model show glm-vs-kimi-impl`

---

### 2I.07 — Record experiment outcomes from orchestrate.rs

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: After each agent turn, if the model was selected by an experiment, record the outcome:

```rust
if let Some(experiment_id) = selected_experiment_id {
    experiment_store.record_outcome(
        &experiment_id,
        &variant_id,
        gate_passed,
        result.usage.cost_usd,
        result.usage.input_tokens + result.usage.output_tokens,
        result.usage.wall_ms,
    );
    experiment_store.save(&experiment_path)?;
}
```

**Acceptance**: Experiment stats update after each turn.
**Verification**: Run 2 tasks → check experiment stats increase.

---

### 2I.08 — Add experiment conclusion notification

**File**: `crates/roko-learn/src/model_experiment.rs`
**What**: When an experiment concludes (winner identified), log and optionally update the CascadeRouter's static table:

```rust
fn on_conclusion(&self, experiment: &ModelExperiment) {
    if let Some(ref winner_id) = experiment.winner_id {
        tracing::info!(
            experiment = %experiment.experiment_id,
            winner = %winner_id,
            "model experiment concluded"
        );
        // Optionally: update cascade_router static table for this role
    }
}
```

**Context**: When an experiment concludes, the winner should become the default for that role in the static routing table. This feeds the CascadeRouter's Stage 1 (static) so it starts with the proven best model for cold starts.

**Acceptance**: Concluded experiment logs winner and updates static table.
**Verification**: `cargo test -p roko-learn -- experiment_conclusion`
