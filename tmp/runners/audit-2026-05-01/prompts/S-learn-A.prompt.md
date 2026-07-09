# S-learn-A: Cascade router learning_stage() exposes stage state

## Task
Add `CascadeRouter::learning_stage() -> LearningStage` that exposes current stage / observation count / threshold / top-N model performance. Add a CLI command `roko learn show` that prints it.

## Runner Context
Runner audit-2026-05-01, group S. No dependencies. Wave 1.

## Source plan
`tmp/subsystem-audits/implementation-plans/25-learning-feedback-completion.md` § Phase A.

## Why
"Learning is invisible" (audit doc 42). Surface it through a CLI command and (later) a TUI tab.

## Read first

```bash
rg 'fn current_stage|LearningStage|observations|stage_threshold' crates/roko-learn/src/cascade_router.rs
```

## Exact changes

### 1. Add `LearningStage` type

```rust
// crates/roko-learn/src/cascade_router.rs

#[derive(Debug, Clone, Serialize)]
pub struct LearningStage {
    pub stage: LearningStageKind,
    pub observations: u64,
    pub stage_threshold: u64,
    pub top_models_by_pass_rate: Vec<ModelPerformance>,
    pub top_role_mappings: Vec<RoleMapping>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum LearningStageKind { ConfidenceOnly, Contextual }

#[derive(Debug, Clone, Serialize)]
pub struct ModelPerformance {
    pub slug: String,
    pub pass_rate: f64,
    pub observations: u64,
    pub avg_cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoleMapping {
    pub role: String,
    pub model_slug: String,
    pub success_rate: f64,
}

impl CascadeRouter {
    pub fn learning_stage(&self) -> LearningStage {
        LearningStage {
            stage: self.current_stage(),
            observations: self.total_observations(),
            stage_threshold: self.stage_threshold(),
            top_models_by_pass_rate: self.top_models(5),
            top_role_mappings: self.top_role_mappings(5),
        }
    }
    // implement helpers if missing
}
```

### 2. CLI command

`crates/roko-cli/src/commands/learn.rs` (or extend existing):

```rust
pub async fn show_learning(workdir: &Path) -> anyhow::Result<()> {
    let router = load_cascade_router(workdir)?;
    let stage = router.learning_stage();
    println!("Learning Stage: {:?}", stage.stage);
    println!("Observations:   {}", stage.observations);
    println!("Threshold:      {}", stage.stage_threshold);
    println!();
    println!("Top models by pass rate:");
    for m in &stage.top_models_by_pass_rate {
        println!("  {:<32} {:>4.0}% pass  {:>5} obs  ${:6.3} avg",
            m.slug, m.pass_rate * 100.0, m.observations, m.avg_cost_usd.unwrap_or(0.0));
    }
    println!();
    println!("Top role mappings:");
    for r in &stage.top_role_mappings {
        println!("  {:<16} → {:<32}  {:>4.0}% success", r.role, r.model_slug, r.success_rate * 100.0);
    }
    Ok(())
}
```

Wire as a subcommand of `roko learn`.

## Write Scope
- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-cli/src/commands/learn.rs`

## Verify

```bash
rg 'fn learning_stage|LearningStage' crates/roko-learn/src/cascade_router.rs
rg 'show_learning|fn run_show' crates/roko-cli/src/commands/learn.rs
```

## Do NOT

- Do NOT change the router's internal observation logic.
- Do NOT add an HTTP endpoint here (separate concern).
- Do NOT use random `top_k` defaults; use 5 consistently.
- Do NOT bundle with other S-learn batches.
