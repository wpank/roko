# PAE_04: Wire LLM judge gate through CascadeRouter for model selection

## Task
Make the LLM judge gate use CascadeRouter for model selection instead of a hardcoded model fallback.

## Runner Context
Runner PAE (Gate Pipeline Completeness), batch 4 of 4. Depends on PAE_02.

## Problem
GP-4 anti-pattern: "Hardcoded model in automated judge." When the LLM judge gate activates (rung 6), it uses whatever model the `JudgeOracle` implementation provides. If PAE_02's `McsJudgeOracle` hardcodes "claude-haiku-4-5", the judge model never improves from routing observations.

## Exact Changes

### Step 1: Make McsJudgeOracle consult CascadeRouter

Extend the `McsJudgeOracle` from PAE_02 to use routing:

```rust
struct McsJudgeOracle {
    mcs: Arc<ModelCallService>,
    cascade_router: Option<Arc<Mutex<CascadeRouter>>>,
    default_model: String,  // fallback when no router
}

impl JudgeOracle for McsJudgeOracle {
    async fn judge(&self, prompt: &str, output: &str) -> Result<JudgeVerdict> {
        // Select model via router (judge context)
        let model = if let Some(router) = &self.cascade_router {
            let ctx = RoutingContext {
                task_category: TaskCategory::Verification,
                complexity: TaskComplexityBand::Low,
                role: AgentRole::Reviewer,
                ..Default::default()
            };
            let router = router.lock().unwrap();
            router.select_model(ctx.to_features(), &RewardWeights::default())
                .map(|s| s.model.clone())
                .unwrap_or(self.default_model.clone())
        } else {
            self.default_model.clone()
        };

        let response = self.mcs.call(ModelCallRequest {
            prompt: format!("...judge prompt..."),
            model: Some(model.clone()),
            ..Default::default()
        }).await?;

        let verdict = parse_judge_verdict(&response.content)?;

        // Record observation for the judge call
        if let Some(router) = &self.cascade_router {
            let mut router = router.lock().unwrap();
            if let Some(idx) = router.model_index(&model) {
                let quality = if verdict.passed { 1.0 } else { 0.5 };  // judge verdicts are informational
                let ctx = RoutingContext {
                    task_category: TaskCategory::Verification,
                    role: AgentRole::Reviewer,
                    ..Default::default()
                };
                router.observe_multi_objective(
                    ctx.to_features(), idx, quality,
                    response.cost_usd.unwrap_or(0.0) / 5.0,  // normalize
                    response.elapsed_ms as f64 / 30000.0,     // normalize
                    &RewardWeights::default(),
                );
            }
        }

        Ok(verdict)
    }
}
```

### Step 2: Pass CascadeRouter to GateService

When constructing GateService (from PAE_01), pass the router:

```rust
let gate_service = GateService::new(&workdir)
    .with_llm_oracle(McsJudgeOracle {
        mcs: model_call_service.clone(),
        cascade_router: cascade_router.clone(),
        default_model: "claude-haiku-4-5".to_string(),
    });
```

## Write Scope
- `crates/roko-gate/src/llm_judge_gate.rs` or gate_service.rs (router-aware oracle)

## Read-Only Context
- `crates/roko-learn/src/cascade_router.rs` (select_model, observe_multi_objective)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- LLM judge model selected via CascadeRouter when available
- Judge observations feed back into router (learn which models judge well)
- Default model used as fallback when router empty/unavailable
- No crash when CascadeRouter is None

## Do NOT
- Change the JudgeOracle trait
- Make the judge gate mandatory (it's opt-in via rung config)
- Use expensive models for judging by default (prefer haiku)
