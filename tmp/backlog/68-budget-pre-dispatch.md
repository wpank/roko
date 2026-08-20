# 68 — Budget Pre-Dispatch Admission Gaps

**Priority**: P2 — cost control: `roko run` currently ignores `max_plan_usd` entirely, and `BudgetPredictor` has been implemented but is never called
**Size**: M (2-3 days)
**Crates**:
- `crates/roko-cli/src/run.rs` — single-prompt `roko run` path (gap: no budget enforcement)
- `crates/roko-cli/src/serve_runtime.rs` — `dispatch_bench_prompt()` called by `run.rs`
- `crates/roko-cli/src/runner/event_loop.rs` — runner-v2 plan dispatch loop
- `crates/roko-compose/src/budget_predictor.rs` — `BudgetPredictor` (implemented, never called)
- `crates/roko-learn/src/budget.rs` — `BudgetGuardrail` routing actions
- `crates/roko-learn/src/cost_table.rs` — `CostTable` model pricing
- `crates/roko-agent/src/safety/spending.rs` — `SpendingLimiter` hook (tool-level)
- `crates/roko-graph/src/budget.rs` — graph-level `BudgetTracker` / `BudgetEnforcer`
- `crates/roko-serve/src/dispatch.rs` — serve dispatch anomaly checks (reference implementation)
- `crates/roko-core/src/config/budget.rs` — `BudgetConfig` schema

**Depends on**: None

---

## Background

Roko's budget enforcement lives at three distinct layers: the runner-v2 plan loop (`event_loop.rs`), the serve dispatch path (`roko-serve/src/dispatch.rs`), and the graph execution engine (`roko-graph/src/budget.rs`). However, the single-prompt `roko run` path has no budget enforcement at all, and a token-budget prediction system (`BudgetPredictor`) was fully implemented in `roko-compose` but never wired into any caller.

The runner-v2 plan loop already does rigorous budget enforcement. Before each task dispatch it checks cumulative `plan_spent` against `max_plan_usd` using a `BudgetGuardrail` (line 9009 of `event_loop.rs`), sets a sticky `budget_exhausted` flag, and calls `check_budget_post_dispatch()` after each completion (line 4179). The serve dispatch path (`roko-serve/src/dispatch.rs` lines 469–479) uses an anomaly detector session to check the cumulative session budget before dispatching. Both paths are covered.

What is missing: (1) the `roko run` single-prompt path has no budget guard at all, (2) the `BudgetPredictor` in `roko-compose/src/budget_predictor.rs` — which estimates optimal token counts from historical task data via EMA — is never loaded or queried at dispatch time, and (3) the runner's pre-dispatch block checks what has already been spent but never estimates what the next dispatch will cost using model pricing from `CostTable`.

---

## Current State

1. **`roko run` has no budget enforcement.** The entry point is `run_once()` in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/run.rs` (line 1140). It calls `dispatch_bench_prompt()` in `serve_runtime.rs` (line 856) without consulting `BudgetConfig`. There is no pre-dispatch check, no post-dispatch accumulation, and no `max_plan_usd` reference anywhere in `run.rs`.

2. **The serve dispatch pattern to follow** is in `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/dispatch.rs` lines 469–479. It reads `budget_limit = f64::from(effective_config.budget.max_plan_usd)`, calls `with_dispatch_anomaly_session(session_root, |session| session.detector.check_budget(budget_limit))`, and returns an error if `Anomaly::BudgetExhausted` is returned. The post-dispatch recording is in lines 484–508 via `record_post_turn_anomalies()`.

3. **`BudgetPredictor`** is fully implemented in `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/budget_predictor.rs`. Its public API:
   - `BudgetPredictor::predict(features: &TaskFeatures) -> u64` — returns estimated token count (line 146)
   - `BudgetPredictor::record(features: &TaskFeatures, actual_tokens: u64, success: bool)` — updates EMA (line 179)
   - `persist_predictor(predictor: &BudgetPredictor, learn_dir: &Path) -> io::Result<()>` — saves to `budget-predictor.json` (line 390)
   - `load_predictor(learn_dir: &Path) -> io::Result<Option<BudgetPredictor>>` — loads from `budget-predictor.json` (line 408)
   - `TaskFeatures { role: String, complexity: String, domain: String }` with `TaskFeatures::new()` constructor (line 60)

   The doc comment on line 16 says this is "meant to be called from the composition layer before assembling the prompt." No caller in `roko-cli` or `roko-serve` ever loads or calls it.

4. **`BudgetConfig`** fields are in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/budget.rs`:
   - `max_plan_usd: f32` — per-plan ceiling; `0.0` means unlimited (line 28)
   - `max_task_usd: f32` — per-task base ceiling; `0.0` means unlimited (line 31)
   - `max_turn_usd: f32` — per-turn ceiling; `0.0` means unlimited (line 34)
   - `prompt_token_budget: usize` — static default for prompt composition (line 37, default 10_000)
   - `task_limit_usd(tier: &str, model_hint: Option<&str>) -> f64` — applies tier multipliers (line 110)

5. **Runner-v2 pre-dispatch budget block** is in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` around line 9009. It reads `max_plan_usd` from `ctx.config.max_plan_usd` and `plan_spent` from `ctx.state.plan_cost(plan_id)`. Uses `BudgetGuardrail` from `roko_learn::budget`. The `budget_remaining_usd` field is populated at line 9599 but reflects historical spend only — no cost estimate for the upcoming dispatch.

6. **`CostTable`** is in `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cost_table.rs`. Struct at line 36; `lookup(slug: &str) -> Option<&ModelPricing>` (line 47); `ModelPricing` has `input_per_m` and `output_per_m` fields. Already used by `ModelCallService` internally at `model_call_service.rs` line 453 after dispatch has been approved.

7. **`SystemPromptBuilder::with_token_budget()`** is in `/Users/will/dev/nunchi/roko/roko/crates/roko-compose/src/system_prompt_builder.rs` line 319. Currently only called with hardcoded values (lines 2196, 2218). The runner uses it via `RunConfig.prompt_token_budget` but this is a static config value, never dynamically predicted.

---

## Implementation Plan

### Section A: Wire budget check into `roko run`

The goal is for `roko run` to respect `max_plan_usd` from `roko.toml`, mirroring the serve dispatch pattern.

**Step A1: Add budget pre-check in `serve_runtime.rs`**

`dispatch_bench_prompt()` in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/serve_runtime.rs` (line 856) is the actual dispatch point called by `run_once()`. Add budget enforcement here so all callers inherit it.

Import from roko-serve:
```rust
use roko_serve::dispatch::{with_dispatch_anomaly_session, Anomaly};
```

After building `model_config` (around line 892) and before the actual model call, add:
```rust
// Budget pre-dispatch check.
let budget_limit = f64::from(config.budget.max_plan_usd);
if budget_limit > 0.0 {
    let session_root = workdir.join(".roko").join("sessions").join("bench");
    if let Some(Anomaly::BudgetExhausted { used, limit }) =
        with_dispatch_anomaly_session(&session_root, |session| {
            session.detector.check_budget(budget_limit)
        })
    {
        return Err(anyhow::anyhow!(
            "roko run budget exhausted: ${used:.2} spent >= ${limit:.2} limit (max_plan_usd)"
        ));
    }
}
```

Note: `Config` in `run.rs` is the CLI-layer config (`crates/roko-cli/src/config.rs`), not `RokoConfig`. Verify it has a `budget` field referencing `BudgetConfig`. If not, read it from the roko.toml by constructing a `RokoConfig` from workdir the same way `event_loop.rs` does in `RunConfig::from_roko_config()`.

**Step A2: Add post-dispatch cost recording in `serve_runtime.rs`**

After the model call returns, record the turn cost to accumulate it for future invocations in the same workspace:
```rust
if let Ok(ref result) = dispatch_result {
    if budget_limit > 0.0 {
        // result.cost_usd is the turn cost; record it.
        with_dispatch_anomaly_session(&session_root, |session| {
            session.detector.record_cost(result.cost_usd);
        });
    }
}
```

**Step A3: Post-dispatch `max_turn_usd` warning**

After dispatch, if `config.budget.max_turn_usd > 0.0` and `result.cost_usd > max_turn_usd`, emit a `tracing::warn!` with the overage amount. The turn already completed, but this provides observability. Do not return an error — per-turn limits on completed turns are advisory only in `roko run`.

### Section B: Wire `BudgetPredictor` into the runner-v2 dispatch

**Step B1: Load predictor once per plan run**

In the runner-v2 event loop, `RunState` is the mutable state across ticks. In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs`, add a field to `RunState`:
```rust
/// Adaptive token-budget predictor, loaded from `.roko/learn/budget-predictor.json`.
pub budget_predictor: roko_compose::budget_predictor::BudgetPredictor,
```

In the runner initialization (where `RunState::new()` or equivalent is called at plan-run start), load the predictor:
```rust
use roko_compose::budget_predictor::load_predictor;
let predictor_path = workdir.join(".roko").join("learn");
let budget_predictor = load_predictor(&predictor_path)
    .unwrap_or(None)
    .unwrap_or_default();
```

**Step B2: Query predictor before each dispatch**

In `event_loop.rs`, inside the dispatch action handler (function `dispatch_action` at line 8602), after the task and role are resolved, construct `TaskFeatures` and call `predict()`:
```rust
use roko_compose::budget_predictor::{BudgetPredictor, TaskFeatures};

let features = TaskFeatures::new(
    task.role.as_deref().unwrap_or("implementer"),
    task.tier.as_deref().unwrap_or("standard"),
    task.domain.as_deref().unwrap_or("code"),
);
let predicted_tokens = ctx.state.budget_predictor.predict(&features);
```

Pass `predicted_tokens` into the `SystemPromptBuilder` via `with_token_budget(predicted_tokens as usize)` instead of using `ctx.config.budget.prompt_token_budget` as the static default.

**Step B3: Record outcome and persist**

After dispatch completes (in the post-dispatch block where `check_budget_post_dispatch` is called, around lines 4172–4179 in `event_loop.rs`), call:
```rust
let actual_tokens = dispatch_result.usage.total_tokens();
let success = dispatch_result.success;
ctx.state.budget_predictor.record(&features, actual_tokens, success);

// Persist after every update.
use roko_compose::budget_predictor::persist_predictor;
let learn_dir = ctx.paths.roko_dir.join("learn");
if let Err(e) = persist_predictor(&ctx.state.budget_predictor, &learn_dir) {
    tracing::warn!("failed to persist budget predictor: {e}");
}
```

### Section C: Cost-table pre-flight estimate before runner dispatch

**Step C1: Estimate upcoming dispatch cost**

In the pre-dispatch budget check block in `event_loop.rs` around line 9031 (after `max_plan_usd > 0.0` check), after getting `plan_spent` and the predicted token count from Section B, compute an estimated cost:
```rust
use roko_learn::cost_table::CostTable;

// Rough split: 75% input, 25% output (adjust based on empirical data).
let est_input = (predicted_tokens as f64 * 0.75) as u64;
let est_output = (predicted_tokens as f64 * 0.25) as u64;

let estimated_cost = if let Some(pricing) = cost_table.lookup(&selected_model) {
    (est_input as f64 / 1_000_000.0) * pricing.input_per_m
        + (est_output as f64 / 1_000_000.0) * pricing.output_per_m
} else {
    0.0 // unknown model; skip pre-flight estimate
};
```

If `plan_spent + estimated_cost > max_plan_usd`, set `budget_exhausted = true` and return early with a budget-exceeded result rather than dispatching. Log the estimate so it is observable:
```rust
if estimated_cost > 0.0 && plan_spent + estimated_cost > max_plan_usd {
    tracing::info!(
        plan_spent = plan_spent,
        estimated_cost = estimated_cost,
        limit = max_plan_usd,
        "pre-flight budget check: estimated cost would exceed plan ceiling; skipping dispatch"
    );
    ctx.state.budget_exhausted = true;
    // return the same early-exit path as BudgetAction::Block
}
```

The `CostTable` should be loaded from config at plan-run start and stored in `RunContext` or passed into the dispatch action. Use `CostTable::from_config(&roko_config.model_profiles)` (see `cost_table.rs` line 262 for the pattern).

---

## Acceptance Criteria

1. `roko run "<prompt>"` respects `max_plan_usd` from `roko.toml`. When the session-accumulated cost meets or exceeds the ceiling, the command exits with a clear error message like `"roko run budget exhausted: $X.XX spent >= $Y.YY limit"` instead of dispatching.

2. `roko run` records turn cost to the anomaly detector session in `.roko/sessions/bench/` so that repeated `roko run` invocations in the same workspace accumulate cost against the ceiling.

3. When `max_turn_usd > 0.0` and a completed turn exceeds the per-turn limit, a `tracing::warn!` is emitted (no error — the turn already happened).

4. `BudgetPredictor` is loaded from `.roko/learn/budget-predictor.json` at the start of a runner-v2 plan run and held in `RunState`.

5. For each task dispatch in runner-v2, `BudgetPredictor::predict()` is called with a `TaskFeatures` derived from the task's role, tier, and domain. The result is passed to `SystemPromptBuilder::with_token_budget()` instead of using the static config default.

6. After each runner-v2 dispatch completes, `BudgetPredictor::record()` is called with actual token count and success/failure, and `persist_predictor()` is called to write `.roko/learn/budget-predictor.json`.

7. The runner-v2 pre-dispatch block uses `CostTable` pricing and the predicted token count to estimate the upcoming dispatch cost. If `plan_spent + estimated_cost > max_plan_usd`, the dispatch is blocked with `budget_exhausted = true` before sending any request to the model.

8. All existing runner-v2 budget tests pass: `cargo test -p roko-cli`.

9. `cargo test --workspace` passes. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

---

## Verification Checklist

- [ ] Set `max_plan_usd = 0.01` in `roko.toml`. Run `roko run "hello"` twice. Second invocation should exit with budget-exhausted error.
- [ ] Remove or zero out `max_plan_usd`. Run `roko run "hello"`. Should complete without budget error.
- [ ] Run `roko plan run plans/ --engine runner-v2` with `max_plan_usd = 0.01`. First task should dispatch; second should be blocked by pre-flight estimate.
- [ ] After a successful plan run, check that `.roko/learn/budget-predictor.json` exists and contains entries with `ema_tokens > 0`.
- [ ] Run the same plan run twice. On the second run, verify that `predicted_tokens` in the trace/log differs from the static `prompt_token_budget` config value (i.e., the predictor is being consulted).
- [ ] `cargo test -p roko-cli` passes.
- [ ] `cargo test -p roko-compose` passes (includes budget predictor persistence tests).
- [ ] `cargo clippy --workspace --no-deps -- -D warnings` is clean.

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/serve_runtime.rs` | Add budget pre-check and post-dispatch cost recording around `dispatch_bench_prompt()` (line 856) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/types.rs` | Add `budget_predictor: BudgetPredictor` field to `RunState` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | (B2) Query predictor at dispatch, pass to `with_token_budget()`; (B3) record outcome and persist; (C1) pre-flight cost estimate using `CostTable` |

## Files NOT to Modify

| File | Why |
|---|---|
| `crates/roko-compose/src/budget_predictor.rs` | Already complete; only call it, do not change it |
| `crates/roko-learn/src/budget.rs` | `BudgetGuardrail` is already correct; use it as-is |
| `crates/roko-graph/src/budget.rs` | Graph-level enforcer; already wired for graph execution |
| `crates/roko-agent/src/safety/spending.rs` | Tool-level hook; not in scope |

---

## Not in Scope

- **Per-agent sidecar budget enforcement**: the `roko-agent-server` sidecar has its own dispatch path. Adding budget checks there is a separate task.
- **ACP runner budget checks**: the ACP runner (`roko-acp/src/runner.rs`) has its own safety pipeline. Wiring that is separate scope.
- **`roko chat` budget enforcement**: the interactive chat path tracks cost per turn but does not enforce a ceiling. That is a UX decision and separate scope.
- **Cross-session daily budget tracking**: `BudgetTracker` in `roko-agent/src/lifecycle.rs` tracks daily cost per agent instance but does not persist across CLI invocations. A persistent daily spending ledger is separate scope.
- **`SectionInfluence` wiring**: the `SectionInfluence` scorer in `budget_predictor.rs` tracks per-section success rates. Wiring it into prompt composition section prioritization is separate scope.
- **Graph-level `BudgetEnforcer`**: already wired for graph execution in `roko-graph/src/budget.rs`. No changes needed.
