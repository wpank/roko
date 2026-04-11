# 08 — CascadeRouter: Live Multi-Objective Routing on Every Path

> Phase 2 of `tmp/workflow/UNIFIED-IMPLEMENTATION-PLAN.md`. Cross-references audit `tmp/workflow/12-learning-feedback-audit.md` § 3.

---

## Status (2026-05-01)

**PARTIAL.** Router exists, persists, has rich features. Live observation only via simple `observe(...)` from `FeedbackService`. The richer `observe_multi_objective(...)` is referenced only from tests and feature-gated `orchestrate.rs`. Routing context is the legacy 17-feature vector inside `orchestrate.rs`; no simplified 6-feature replacement.

**What's done:**

- `roko_learn::cascade_router::CascadeRouter` — `crates/roko-learn/src/cascade_router.rs`
- LinUCB contextual bandit, persisted to `.roko/learn/cascade-router.json`
- `CascadeRouter::select_for_frequency_among(...)` — model selection
- `CascadeRouter::observe(...)` — simple success/failure update — called by `FeedbackService::observe_model_call`
- `CascadeRouter::observe_multi_objective(...)` — multi-objective LinUCB update — only fired from tests and `orchestrate.rs:~11018`
- `CascadeRouter::record_confidence_outcome(...)` — called by CLI `RoutingObservationSink` on `TaskCompleted`
- Knowledge-routing boost (`knowledge_routing_boost`) — only inside orchestrate

**What's not:**

- `RoutingContext` not simplified to 6 features (still uses legacy 17-feature shape internally)
- `TaskRequirements` matching (`needs_web_search`, `needs_thinking`, `min_context_window`, `max_cost`) — exists but not consulted by live paths
- No `force_backend` override mechanism in `ModelCallService` config flow
- Tier-based defaults (`mechanical → haiku`, `architectural → opus`) not surfaced as a reusable fallback policy
- `observe_multi_objective` not the canonical path
- `roko run` and `roko plan run` (default) and chat all bypass `CascadeRouter` — they use `resolve_model(config, model_key)` which reads the static `[models]` config table

---

## Goal

`CascadeRouter` is consulted **before** every model call (including chat, plan tasks, gate judge, distillation). Its observations come from **every** model call via `FeedbackService::observe_model_call` calling the multi-objective path. Routing context is reduced to 6 features. `TaskRequirements` filter candidates before scoring. Tier defaults serve as a fallback when bandit confidence is low.

---

## Why This Exists (Anti-Patterns Eliminated)

- **#5 Hardcoded Role Behavior** — current model selection is `if config.has_key("model.implementer") { ... } else { default }`
- **#6 Feedback as Afterthought** — router updates only on a fraction of runs
- **#10 God file** — orchestrate.rs is the only place the rich features (knowledge boost, conductor pressure) live

---

## Existing Code — Read These First

```rust
// crates/roko-learn/src/cascade_router.rs (sketch)
pub struct CascadeRouter {
    arms: HashMap<String, ArmStats>,             // model_id → stats
    linucb: LinUcbState,
    persist_path: PathBuf,
}

impl CascadeRouter {
    pub fn select_for_frequency_among(&self, candidates: &[ModelId], context: &RoutingContext) -> ModelChoice;
    pub fn observe(&mut self, model: &str, success: bool);
    pub fn observe_multi_objective(&mut self, obs: MultiObjectiveObservation);
    pub fn record_confidence_outcome(&mut self, model: &str, success: bool);
}
```

The legacy 17-feature `RoutingContext` lives inside `orchestrate.rs::cascade_routing_context()` (~line 2648). It has features like `knowledge_familiarity`, `crate_familiarity`, `daimon_pad_arousal`, etc. — most have negligible signal.

---

## Implementation Steps

### Step 1 — Define a simplified `RoutingContext`

```rust
// crates/roko-learn/src/routing_context.rs
#[derive(Debug, Clone)]
pub struct RoutingContext {
    pub task_tier: Tier,                   // mechanical | focused | integrative | architectural
    pub role: AgentRole,                   // from roko_core::agent
    pub attempt: u32,                      // retry count (0 for first try)
    pub budget_pressure: f64,              // 0.0 = flush, 1.0 = broke
    pub prior_failure: bool,               // last attempt for this task failed
    pub task_category: TaskCategory,       // crate-modifying | docs-only | research | etc.
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Tier { Mechanical, Focused, Integrative, Architectural }

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TaskCategory {
    CodeChange, Refactor, Docs, Research, Test, Debug, Architecture, Other,
}
```

These 6 features are observable from the request itself (no need to query knowledge store, conductor, etc., before routing — those live in pre-call enrichment).

The 17-feature legacy context is mapped to the 6-feature one inside the `LinUcbState` for backward compatibility with persisted bandit state. After 1000 observations on the new context, retire the legacy mapping.

### Step 2 — Add `TaskRequirements` filter

```rust
// crates/roko-learn/src/task_requirements.rs
#[derive(Debug, Clone, Default)]
pub struct TaskRequirements {
    pub needs_web_search: bool,
    pub needs_code_execution: bool,
    pub needs_thinking: bool,
    pub min_context_window: Option<u32>,
    pub max_cost_usd: Option<f64>,
}

pub fn filter_candidates<'a>(
    candidates: &'a [ModelDescriptor],
    requirements: &TaskRequirements,
) -> Vec<&'a ModelDescriptor> {
    candidates.iter()
        .filter(|m| !requirements.needs_web_search || m.supports_web_search)
        .filter(|m| !requirements.needs_code_execution || m.supports_code_execution)
        .filter(|m| !requirements.needs_thinking || m.supports_thinking)
        .filter(|m| requirements.min_context_window.is_none() || m.context_window >= requirements.min_context_window.unwrap())
        .filter(|m| requirements.max_cost_usd.is_none() || m.estimated_cost_per_call <= requirements.max_cost_usd.unwrap())
        .collect()
}
```

`ModelDescriptor` lives in `roko-core::config::ModelDescriptor` (verify; may need to be added). Capabilities should come from the `[models.<id>]` config block.

### Step 3 — Wire `CascadeRouter::select_for_frequency_among` into `ModelCallService`

**File:** `crates/roko-agent/src/model_call_service.rs`

Today the service does `resolve_model(config, model_key)` which reads `[models.<key>]`. Add a router consultation step:

```rust
async fn resolve(&self, req: &ModelCallRequest) -> Result<ResolvedDispatch> {
    // existing static resolution
    let static_choice = resolve_model(&self.config, &req.model)?;

    // optional router-based selection
    let chosen = if let Some(router) = &self.router {
        let context = build_routing_context(req);
        let requirements = build_task_requirements(req);
        let candidates = self.candidates_for(&req.role).await?;
        let filtered = filter_candidates(&candidates, &requirements);
        if filtered.is_empty() {
            tracing::warn!("no candidates after filter; falling back to static");
            static_choice
        } else {
            router.lock().await
                .select_for_frequency_among(&filtered.iter().map(|m| m.id.clone()).collect::<Vec<_>>(), &context)
                .into_dispatch(static_choice)
        }
    } else {
        static_choice
    };

    Ok(chosen)
}
```

Add helpers:

- `build_routing_context(req: &ModelCallRequest) -> RoutingContext` — extracts tier from `req.model` or role default; reads attempt from `req.routing_hints` (carry as `"attempt:3"` style hint or add `attempt: u32` field to request)
- `build_task_requirements(req)` — heuristics: `task.contains("search")` → `needs_web_search`; `messages.iter().sum(|m| m.content.len()) > 50_000` → `min_context_window: 200_000`

### Step 4 — Make `observe_multi_objective` the live path

**File:** `crates/roko-learn/src/feedback_service.rs`

Per plan 03 § Step 5: replace the simple `router.observe(...)` call inside `observe_model_call` with the full `observe_multi_objective(...)`.

After this, every model call ingested by the feedback service updates the router with multi-objective signal (success + latency + cost + tokens).

### Step 5 — Implement `force_backend` override

```rust
// crates/roko-core::config (extend)
[runtime.routing]
force_backend = "claude-sonnet-4"           # bypasses router entirely
```

Implementation in `ModelCallService::resolve`:

```rust
if let Some(force) = &self.config.runtime.routing.force_backend {
    tracing::info!(model = force, "force_backend override; bypassing router");
    return Ok(ResolvedDispatch::for_model(force));
}
```

Record the override as an observation:

```rust
if forced {
    self.router.lock().await.record_override_outcome(force_model, success);
}
```

`record_override_outcome` exists in `CascadeRouter` (per audit doc 12) — wire it.

### Step 6 — Implement tier-based fallback

When the router has < `min_observations_per_arm` (default 5) for any candidate, fall back to tier defaults:

```rust
fn tier_default(tier: Tier) -> &'static str {
    match tier {
        Tier::Mechanical => "claude-haiku-4",
        Tier::Focused => "claude-sonnet-4",
        Tier::Integrative => "claude-sonnet-4",
        Tier::Architectural => "claude-opus-4",
    }
}
```

These defaults are configurable via `[runtime.routing.tier_defaults]`. The chosen model must still pass through `TaskRequirements` filter.

### Step 7 — Two-router-test proof

```rust
// crates/roko-learn/tests/router_learns_from_real_runs.rs
#[tokio::test]
async fn router_picks_different_model_after_5_failures() {
    let temp = tempdir()?;
    let services = ServiceFactory::for_test(temp.path()).await?;
    let router = services.router_handle();

    // Force 5 failures on sonnet
    for _ in 0..5 {
        services.feedback_sink().record(FeedbackEvent::ModelCall {
            model: Some("claude-sonnet-4".into()),
            role: "implementer".into(),
            success: false, cost_usd: 0.05, latency_ms: 8000,
            input_tokens: 1000, output_tokens: 500,
            ..base_event()
        }).await?;
    }

    let context = RoutingContext {
        task_tier: Tier::Focused, role: AgentRole::Implementer, attempt: 0,
        budget_pressure: 0.5, prior_failure: false, task_category: TaskCategory::CodeChange,
    };
    let choice = router.lock().await.select_for_frequency_among(
        &["claude-haiku-4".into(), "claude-sonnet-4".into(), "claude-opus-4".into()],
        &context,
    );
    assert_ne!(choice.model_id, "claude-sonnet-4", "router should learn to avoid failing sonnet");
}

#[tokio::test]
async fn router_filters_candidates_by_requirements() {
    let req = TaskRequirements { needs_web_search: true, ..Default::default() };
    let candidates = vec![
        ModelDescriptor { id: "ollama-llama3".into(), supports_web_search: false, ..base() },
        ModelDescriptor { id: "perplexity-sonar".into(), supports_web_search: true, ..base() },
    ];
    let filtered = filter_candidates(&candidates, &req);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "perplexity-sonar");
}

#[tokio::test]
async fn budget_pressure_prefers_cheaper() {
    let context = RoutingContext { budget_pressure: 0.95, ..base_context() };
    let choice = router_with_seeded_observations().select_for_frequency_among(&[
        "claude-haiku-4", "claude-sonnet-4", "claude-opus-4",
    ], &context);
    assert_eq!(choice.model_id, "claude-haiku-4");
}
```

---

## Anti-Patterns To Avoid

| ID | Manifestation | How to avoid |
|---|---|---|
| #5 Hardcoded role behavior | `if role == "implementer" { return "claude-sonnet-4" }` | Routing decisions live in CascadeRouter; tier defaults serve as fallback only |
| #6 Feedback afterthought | Adding routing without wiring observation back | Router and FeedbackService land together |
| #10 God file | Putting routing context derivation in `model_call_service.rs` | `routing_context.rs` and `task_requirements.rs` are their own modules |

---

## Things NOT To Do

1. **Don't query the knowledge store synchronously inside the routing path.** `knowledge_routing_boost` from orchestrate is a 200ms+ disk hit. If you want it, do it once per task in pre-enrichment, then pass the boost as a hint in `RoutingContext`.
2. **Don't increase the feature vector beyond 6.** Each feature added increases the LinUCB exploration time linearly (more arms × features = more observations needed).
3. **Don't auto-update the router from chat episodes.** Chat is high-volume and noisy. Either add a config flag `[learn].chat_routes_router = false` (default), or weight chat observations down.
4. **Don't store API keys in routing context.** It serializes into the persistent bandit state.
5. **Don't make `force_backend` silent.** Always log `tracing::info!` so operators see the override is active.
6. **Don't break old `cascade-router.json` files.** Add a schema version check; on mismatch, rebuild from scratch (warn, don't error).
7. **Don't rely on `record_confidence_outcome` and `observe_multi_objective` being called from different sinks.** That's the current bug — two paths update the router with disagreeing signal. After this plan, only `observe_multi_objective` fires (from `FeedbackService::observe_model_call`).

---

## Tests / Proof Criteria

```bash
# 1. RoutingContext is 6 features
rg 'pub struct RoutingContext' crates/roko-learn/src/ --type rust
# verify the struct in routing_context.rs has exactly: task_tier, role, attempt, budget_pressure, prior_failure, task_category

# 2. observe_multi_objective is the live path
rg 'router\.\w*observe' crates/roko-learn/src/feedback_service.rs
# expected: only `observe_multi_objective` (not `observe`)

# 3. force_backend honored
rg 'force_backend' crates/roko-agent/src/model_call_service.rs
# expected: 1+ usage in resolve()

# 4. TaskRequirements filter wired
rg 'filter_candidates' crates/roko-agent/src/model_call_service.rs
# expected: 1+ usage
```

Functional proofs:

- [ ] All 3 unit tests above pass
- [ ] Run 5 sequential `roko run "fix bug"` (or any failing task) — `cascade-router.json` shows `claude-sonnet-4` count decreasing in the candidate distribution
- [ ] `[runtime.routing.force_backend = "claude-haiku-4"]` overrides routing — verify by checking episodes
- [ ] Task with `needs_web_search: true` does not get routed to local Ollama
- [ ] Tier `mechanical` defaults to `haiku` when no observations exist; promotes to `sonnet` after 10 successes

---

## Dependencies

- **Plan 01 (ModelCallService)** — `resolve` is where router is consulted
- **Plan 03 (FeedbackService)** — calls `observe_multi_objective`

Can start in parallel with 01-04 once `RoutingContext` shape is agreed.

---

## Estimated Effort

**M.** ~1 week.

- Step 1 (RoutingContext) — S (1 day)
- Step 2 (TaskRequirements) — S (1 day)
- Step 3 (router in ModelCallService) — M (2 days)
- Step 4 (multi-objective live) — S (half day; mostly plan 03 work)
- Step 5 (force_backend) — S (1 day)
- Step 6 (tier fallback) — S (1 day)
- Step 7 (proof tests) — S (1 day)
