# 09 — Cost Normalization & Budget Guardrails

> **Priority**: 🟡 P1 — Enables fair cross-provider cost comparison and budget enforcement
> **Status**: Not started
> **Depends on**: 02 (provider registry for ModelProfile.cost_* fields)
> **Blocks**: None

## Problem Statement

The CascadeRouter's `compute_routing_reward()` uses `normalized_cost` but doesn't define how to normalize across providers with different pricing structures and tokenizers. $1.40/M tokens on Z.AI and $15.00/M tokens on Anthropic use different tokenizers — the same text might be 1000 tokens on one and 1200 on another. Without normalization, cost comparisons are meaningless.

Additionally, there's no budget enforcement — a runaway agent loop can accumulate unbounded costs.

## What Exists

| Component | Path | Status |
|---|---|---|
| CostRecord struct | `crates/roko-learn/src/costs_db.rs` L24 | 🔌 Has model, provider, tokens, cost |
| CostSummary struct | `crates/roko-learn/src/costs_db.rs` L59 | 🔌 Aggregate stats |
| CostsDb | `crates/roko-learn/src/costs_db.rs` L122 | 🔌 In-memory DB |
| Usage struct | `crates/roko-agent/src/usage.rs` | 🔌 Has cache fields |
| Budget config | `roko.toml` L61–64 | 🔌 max_plan_usd, max_task_usd, warn_at_percent |
| RoleCostProfile | `crates/roko-learn/src/efficiency.rs` L336 | 🔌 Per-role cost analysis |

---

## Checklist

### 2H.01 — Define CostTable struct

**File**: `crates/roko-learn/src/cost_table.rs` (new)
**What**: Per-model pricing loaded from config:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTable {
    pub models: HashMap<String, ModelPricing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_per_m: f64,           // $ per million tokens
    pub output_per_m: f64,
    pub cache_read_per_m: f64,
    pub cache_write_per_m: f64,
    pub tokenizer_ratio: f64,       // ratio vs OpenAI o200k_base (default 1.0)
}

impl CostTable {
    /// Calculate cost from raw token counts.
    pub fn calculate(&self, model_slug: &str, usage: &Usage) -> f64 {
        let pricing = match self.models.get(model_slug) {
            Some(p) => p,
            None => return 0.0,  // unknown model
        };
        (usage.input_tokens as f64 * pricing.input_per_m / 1_000_000.0)
            + (usage.output_tokens as f64 * pricing.output_per_m / 1_000_000.0)
            + (usage.cache_read_tokens as f64 * pricing.cache_read_per_m / 1_000_000.0)
            + (usage.cache_write_tokens as f64 * pricing.cache_write_per_m / 1_000_000.0)
    }

    /// Normalize cost to OpenAI-equivalent tokens for cross-provider comparison.
    /// Uses Artificial Analysis methodology: 3:1 input/output ratio for blended cost.
    pub fn blended_cost_per_m(&self, model_slug: &str) -> f64 {
        let pricing = match self.models.get(model_slug) {
            Some(p) => p,
            None => return 0.0,
        };
        let ratio = pricing.tokenizer_ratio;
        // Blended = (3 * input + 1 * output) / 4, normalized by tokenizer ratio
        ((3.0 * pricing.input_per_m + pricing.output_per_m) / 4.0) * ratio
    }

    /// Load from ModelProfile entries in RokoConfig.
    pub fn from_config(models: &HashMap<String, ModelProfile>) -> Self {
        let mut table = HashMap::new();
        for (key, profile) in models {
            if let (Some(input), Some(output)) = (profile.cost_input_per_m, profile.cost_output_per_m) {
                table.insert(profile.slug.clone(), ModelPricing {
                    input_per_m: input,
                    output_per_m: output,
                    cache_read_per_m: profile.cost_cache_read_per_m.unwrap_or(input * 0.5),
                    cache_write_per_m: profile.cost_cache_write_per_m.unwrap_or(input * 1.25),
                    tokenizer_ratio: profile.tokenizer_ratio.unwrap_or(1.0),
                });
            }
        }
        Self { models: table }
    }
}
```

**Acceptance**: `CostTable::calculate("glm-5.1", usage)` returns correct cost.
**Verification**: `cargo test -p roko-learn -- cost_table_calculate`

---

### 2H.02 — Add default pricing for known models

**File**: `crates/roko-learn/src/cost_table.rs`
**What**: Fallback pricing when not in config:

```rust
impl CostTable {
    pub fn with_defaults(mut self) -> Self {
        let defaults = [
            ("claude-opus-4-6",    15.00, 75.00, 3.75, 18.75, 1.0),
            ("claude-sonnet-4-6",   3.00, 15.00, 0.30,  3.75, 1.0),
            ("claude-haiku-4-5",    0.80,  4.00, 0.08,  1.00, 1.0),
            ("glm-5.1",            1.40,  4.40, 0.26,  1.75, 1.05),
            ("glm-5",              1.00,  3.20, 0.50,  1.25, 1.05),
            ("kimi-k2.5",          0.60,  3.00, 0.10,  0.75, 0.98),
            ("gpt-5.2",            2.00,  8.00, 0.50,  2.50, 1.0),
            ("gpt-5.4",            3.00, 12.00, 0.75,  3.75, 1.0),
        ];
        for (slug, input, output, cache_r, cache_w, ratio) in defaults {
            self.models.entry(slug.to_string()).or_insert(ModelPricing {
                input_per_m: input, output_per_m: output,
                cache_read_per_m: cache_r, cache_write_per_m: cache_w,
                tokenizer_ratio: ratio,
            });
        }
        self
    }
}
```

**Acceptance**: Default table has 8+ models.
**Verification**: `cargo test -p roko-learn -- cost_defaults`

---

### 2H.03 — Wire CostTable into compute_routing_reward

**File**: `crates/roko-learn/src/model_router.rs`
**What**: Use `blended_cost_per_m()` for normalization:

```rust
pub fn normalized_cost(model_slug: &str, cost_table: &CostTable) -> f64 {
    let blended = cost_table.blended_cost_per_m(model_slug);
    let max_blended = 75.0;  // Claude Opus as reference ceiling
    (blended / max_blended).min(1.0)
}
```

This replaces the current ad-hoc cost normalization with a principled calculation.

**Acceptance**: GLM-5.1 normalized cost ≈ 0.04 (very cheap). Claude Opus ≈ 0.67. Kimi ≈ 0.02.
**Verification**: `cargo test -p roko-learn -- normalized_cost`

---

### 2H.04 — Define BudgetGuardrail struct

**File**: `crates/roko-learn/src/budget.rs` (new)
**What**: Enforce cost limits at multiple levels:

```rust
#[derive(Debug, Clone)]
pub struct BudgetGuardrail {
    pub per_task_limit_usd: f64,
    pub per_session_limit_usd: f64,
    pub per_day_limit_usd: f64,
    pub warn_at_percent: f64,         // 0.0-1.0
    task_spent: f64,
    session_spent: f64,
    day_spent: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BudgetAction {
    Ok,
    Warn { percent_used: f64, level: &'static str },
    RouteToCheaper,                   // > 80% of budget
    BlockNewSessions,                 // > 95% of budget
    Block,                            // >= 100% of budget
}

impl BudgetGuardrail {
    pub fn record_cost(&mut self, cost_usd: f64, level: &str) -> BudgetAction {
        match level {
            "task" => { self.task_spent += cost_usd; self.check_budget(self.task_spent, self.per_task_limit_usd) },
            "session" => { self.session_spent += cost_usd; self.check_budget(self.session_spent, self.per_session_limit_usd) },
            "day" => { self.day_spent += cost_usd; self.check_budget(self.day_spent, self.per_day_limit_usd) },
            _ => BudgetAction::Ok,
        }
    }

    fn check_budget(&self, spent: f64, limit: f64) -> BudgetAction {
        if limit <= 0.0 { return BudgetAction::Ok; }
        let pct = spent / limit;
        if pct >= 1.0 { BudgetAction::Block }
        else if pct >= 0.95 { BudgetAction::BlockNewSessions }
        else if pct >= 0.80 { BudgetAction::RouteToCheaper }
        else if pct >= self.warn_at_percent { BudgetAction::Warn { percent_used: pct, level: "budget" } }
        else { BudgetAction::Ok }
    }

    pub fn reset_task(&mut self) { self.task_spent = 0.0; }
    pub fn reset_session(&mut self) { self.session_spent = 0.0; }
}
```

**Context**: Loaded from `roko.toml` `[budget]` section (already exists: `max_plan_usd = 10.0`, `max_task_usd = 1.0`, `warn_at_percent = 80`).

**Acceptance**: 81% budget → `RouteToCheaper`. 96% → `BlockNewSessions`. 100% → `Block`.
**Verification**: `cargo test -p roko-learn -- budget_guardrail`

---

### 2H.05 — Wire BudgetGuardrail into orchestrate.rs

**File**: `crates/roko-cli/src/orchestrate.rs`
**What**: Check budget before each agent turn:

```rust
let action = budget.record_cost(last_cost_usd, "task");
match action {
    BudgetAction::Block => return Err(OrchestrateError::BudgetExhausted),
    BudgetAction::RouteToCheaper => {
        // Override model to mechanical tier
        model_key = config.agent.tier_models.mechanical.clone();
    },
    BudgetAction::Warn { percent_used, .. } => {
        tracing::warn!(pct = percent_used, "budget warning");
    },
    _ => {},
}
```

**Acceptance**: Task exceeding $1.00 budget triggers model downgrade.
**Verification**: `cargo test -p roko-cli -- budget_enforcement`

---

### 2H.06 — Add cost_per_successful_task metric

**File**: `crates/roko-learn/src/efficiency.rs`
**What**: Add a computed metric to `RoleCostProfile`:

```rust
pub fn cost_per_successful_task(&self) -> f64 {
    if self.pass_rate <= 0.0 { return f64::INFINITY; }
    self.avg_cost_usd / self.pass_rate
}
```

This is the key metric for comparing models: how much does a successful outcome cost?

**Acceptance**: Role with $0.50 avg cost and 80% pass rate → $0.625 per success.
**Verification**: `cargo test -p roko-learn -- cost_per_success`

---

### 2H.07 — Add cost_per_success to dashboard output

**File**: `crates/roko-cli/src/dashboard.rs` (or wherever dashboard renders)
**What**: Add a table showing cost per successful task by model:

```
Model Cost Comparison (last 7 days)
┌──────────────────┬──────────┬──────────┬──────────┬───────────────┐
│ Model            │ Pass %   │ Avg Cost │ $/Success│ Observations  │
├──────────────────┼──────────┼──────────┼──────────┼───────────────┤
│ kimi-k2.5        │   78%    │  $0.08   │  $0.10   │     145       │
│ glm-5.1          │   82%    │  $0.19   │  $0.23   │     203       │
│ claude-sonnet-4-6│   88%    │  $0.42   │  $0.48   │     312       │
│ claude-opus-4-6  │   94%    │  $2.10   │  $2.23   │      47       │
└──────────────────┴──────────┴──────────┴──────────┴───────────────┘
```

**Acceptance**: Dashboard displays cost comparison table when multiple models have observations.
**Verification**: `cargo run -p roko-cli -- dashboard 2>&1 | grep "Cost Comparison"`

---

### 2H.08 — Add token normalization for cross-provider comparison

**File**: `crates/roko-learn/src/cost_table.rs`
**What**: Function to normalize token counts to OpenAI-equivalent:

```rust
pub fn normalize_tokens(&self, model_slug: &str, tokens: u64) -> u64 {
    let ratio = self.models.get(model_slug)
        .map(|p| p.tokenizer_ratio)
        .unwrap_or(1.0);
    (tokens as f64 * ratio) as u64
}
```

**Context**: Follows Artificial Analysis methodology. If GLM's tokenizer produces 5% more tokens for the same text (ratio = 1.05), then 1000 GLM tokens ≈ 1050 OpenAI tokens.

**Acceptance**: 1000 GLM tokens → 1050 normalized tokens.
**Verification**: `cargo test -p roko-learn -- token_normalization`

---

### 2H.09 — Wire CostTable into CostRecord creation

**File**: `crates/roko-learn/src/costs_db.rs`
**What**: When creating a `CostRecord`, use `CostTable::calculate()` if the agent didn't report cost:

```rust
pub fn create_cost_record(
    model: &str, provider: &str, usage: &Usage, cost_table: &CostTable,
    // ... other fields
) -> CostRecord {
    let cost_usd = if usage.cost_usd > 0.0 {
        usage.cost_usd  // Agent self-reported cost
    } else {
        cost_table.calculate(model, usage)  // Calculate from token counts
    };
    CostRecord { cost_usd, /* ... */ }
}
```

**Acceptance**: CostRecord always has a non-zero cost when token counts are available.
**Verification**: `cargo test -p roko-learn -- cost_record_calculation`

---

### 2H.10 — Write cost comparison integration test

**File**: `crates/roko-learn/tests/cost_comparison.rs` (new)
**What**: End-to-end test that:
1. Creates a CostTable with GLM, Kimi, Claude pricing
2. Simulates 100 agent turns across 3 models
3. Computes RoleCostProfile per model
4. Verifies cost_per_successful_task ordering: Kimi < GLM < Claude
5. Computes blended_cost_per_m ordering: same

**Acceptance**: Ordering matches expected pricing.
**Verification**: `cargo test -p roko-learn -- cost_comparison_e2e`
