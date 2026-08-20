# 84 — Cascade Router Ignores Task Category in Stages 2 and 3

**Priority**: P2 — search-optimized models get selected for code-writing tasks after 50 observations accumulate
**Size**: M (1-2 days)
**Crates**: `crates/roko-learn` (paths: `src/cascade_router.rs`, `src/cascade/helpers.rs`, `src/cascade/types.rs`, `src/model_router.rs`)
**Depends on**: None

---

## Background

The cascade router selects which LLM model handles each task. It has three stages based on how many total observations have accumulated:

| Stage | Threshold | Selection Method |
|---|---|---|
| 1 (Static) | < 50 total observations | Role-table lookup with a research exception: sonar preferred for `TaskCategory::Research` |
| 2 (Confidence) | 50-200 observations | Confidence-weighted selection — ignores task category |
| 3 (UCB) | > 200 observations | LinUCB bandit — ignores task category |

Stage 1 correctly avoids selecting search-optimized models (like `sonar`) for code tasks. But once 50 total observations accumulate, stages 2 and 3 take over and consider only aggregate success rates — not what kind of task is being performed. A model that excels at research queries can accumulate a high confidence score from research tasks, then get selected for implementation tasks where it performs poorly.

This produces suboptimal code quality and can cause gate failures when a search-optimized model generates Rust code.

## Current State

1. **Stage 1 has task-category awareness** — `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` lines 2156-2173: in `route_static()`, if `ctx.task_category == TaskCategory::Research`, the router prefers a model whose `slug_family()` returns `"sonar"`. For all other categories, it uses the role table. This is the correct behavior but only applies when `total_observations < COLD_START_THRESHOLD`.

2. **Stage 2 (`route_confidence`) ignores task category** — Lines 2274-2297: `route_confidence()` calls `confidence_scores()` which returns scores based on `s.upper_bound()` — the beta distribution upper confidence bound from per-model success/failure counts. The `ctx.task_category` field is not consulted. The `confidence_stats` map (line 88: `confidence_stats: Mutex<HashMap<String, ModelStats>>`) stores global per-model counts with no partitioning by task category.

3. **Stage 3 (`route_ucb`) ignores task category** — Lines 2334-2359: `route_ucb()` calls `select_ucb_model()` which uses `ucb_scores()` (line 2563). The LinUCB feature vector is constructed from `ctx.to_features()` (which does encode `task_category` as a numeric feature in the context vector), but the UCB bandit's learned weights reflect all tasks combined. Category-specific patterns are diluted by cross-category observations.

4. **`COLD_START_THRESHOLD = 50`** — Defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/model_router.rs` line 65. The `stage_for_observations()` function in `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade/helpers.rs` line 542 determines the stage. `CONFIDENCE_TO_UCB_THRESHOLD = 200` is defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade/types.rs` line 383.

5. **`RoutingContext` has `task_category: TaskCategory`** — Defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/model_router.rs` line 131-133. `TaskCategory` is defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/task.rs` line 133 with variants: `Scaffolding`, `Implementation`, `Integration`, `Verification`, `Research`, `Refactor`, `Infra`, `Docs`.

6. **`observe_internal()` stores globally** — Lines 1536-1582: when an observation is recorded, `confidence_stats` is updated with `entry.trials += 1` and `entry.successes += 1` at lines 1552-1556. There is no partition by `task_category`. The LinUCB update at line 1582 does encode category via the context vector, which is why stage 3 has partial awareness — but it's still not as targeted as per-category statistics.

## Implementation Plan

There are two levels of intervention. Level 1 is a quick win with minimal code change. Level 2 is the complete solution.

### Level 1 (Quick Win) — Carry stage-1 category logic into stage 2

In `confidence_scores()` at line 2445, add a category-aware tie-breaking step. After computing base scores from confidence statistics, if `ctx.task_category == TaskCategory::Research`, apply a bonus to models known to be search-optimized (sonar family) and a penalty to models that have never handled a research task. If `task_category` is any code-producing category (`Implementation`, `Scaffolding`, `Integration`, `Refactor`), apply a penalty to sonar-family models.

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`

```rust
fn confidence_scores(&self, candidates: &[String], ctx: &RoutingContext) -> Vec<(String, f64)> {
    let stats = self.confidence_stats.lock();
    // ... existing setup ...

    let is_code_task = matches!(
        ctx.task_category,
        TaskCategory::Implementation
            | TaskCategory::Scaffolding
            | TaskCategory::Integration
            | TaskCategory::Refactor
            | TaskCategory::Verification
    );
    let is_research_task = ctx.task_category == TaskCategory::Research;

    let mut scores: Vec<(String, f64)> = candidates
        .iter()
        .map(|slug| {
            let s = stats.get(slug).cloned().unwrap_or_default();
            let base_score = if s.trials == 0 { 0.2 } else { s.upper_bound() };
            let tier_bonus = /* existing tier_bonus logic */;

            // Category-specific adjustment:
            // Penalize sonar-family for code tasks; boost for research tasks.
            let category_adj = if slug_family(slug) == Some("sonar") {
                if is_code_task { -0.3 } else if is_research_task { 0.2 } else { 0.0 }
            } else if is_research_task {
                // Non-research models get a slight penalty on research tasks
                // to encourage keeping sonar for what it's good at.
                -0.1
            } else {
                0.0
            };

            (slug.clone(), (base_score + tier_bonus + category_adj).clamp(0.0, 2.0))
        })
        .collect();

    apply_cache_affinity(&mut scores, ctx.previous_model.as_deref());
    scores
}
```

Apply the same category adjustment in `ucb_scores()` at line 2563 for stage 3.

### Level 2 (Complete Solution) — Per-category confidence statistics

Change `confidence_stats` from `HashMap<String, ModelStats>` to `HashMap<(String, String), ModelStats>` where the key is `(model_slug, category_label)`. Add a `"general"` key that stores cross-category aggregates for fallback.

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`

In `observe_internal()` at line 1536, record the observation under both the specific category key AND the general key:

```rust
// In observe_internal(), after getting the slug:
let category_label = ctx_category.label(); // passed in from record_observation()

let mut stats = self.confidence_stats.lock();
// Update category-specific stats:
let cat_key = format!("{slug}:{category_label}");
let cat_entry = stats.entry(cat_key).or_default();
cat_entry.trials += 1;
if success { cat_entry.successes += 1; }

// Update general stats (used for fallback):
let gen_entry = stats.entry(slug.clone()).or_default();
gen_entry.trials += 1;
if success { gen_entry.successes += 1; }
```

Note: This requires passing `task_category` through to `observe_internal()`. Currently `record_observation()` takes a `&RoutingContext` (which has `task_category`) but `observe_internal()` only takes a `context_vec: &[f64]`. The category information is encoded in the vector, but not available as a typed value. You need to add a parameter or restructure to pass category explicitly.

In `confidence_scores()`, look up the category-specific key first, fall back to general:

```rust
let category_key = format!("{slug}:{}", ctx.task_category.label());
let cat_stats = stats.get(&category_key).cloned();
let gen_stats = stats.get(slug).cloned();

let base_score = match (cat_stats, gen_stats) {
    (Some(cat), _) if cat.trials >= 5 => cat.upper_bound(), // enough category-specific data
    (_, Some(gen)) if gen.trials > 0 => gen.upper_bound(),  // fall back to general
    _ => 0.2, // cold start
};
```

This change requires updating the persisted state format in `.roko/learn/cascade-router.json`. Add a migration path: if the loaded snapshot has flat model keys, treat them as general stats and initialize category-specific keys as empty.

### Recommended approach

Start with Level 1. It requires no data migration, no format change, and the test coverage for confidence scoring is already in `crates/roko-learn/src/cascade/tests.rs`. Verify that Level 1 alone prevents sonar from being selected for implementation tasks. If it does, Level 1 may be sufficient. File Level 2 as a follow-up if Level 1 is not sufficient after a real dogfood run.

## Acceptance Criteria

1. With `sonar` and `claude-sonnet` both configured, a task with `task_category = Implementation` never selects `sonar` in stage 2 or stage 3 when `claude-sonnet` has a competitive confidence score.
2. With `sonar` and `claude-sonnet` both configured, a task with `task_category = Research` can select `sonar` in all three stages.
3. `roko learn route` output indicates the stage and (for Level 2) per-category observation counts.
4. The change is backward-compatible with existing `.roko/learn/cascade-router.json` state.
5. `cargo test -p roko-learn` passes.

## Verification Checklist

- [ ] Add a unit test in `crates/roko-learn/src/cascade_router.rs` that verifies `sonar` is not selected for `TaskCategory::Implementation` in stage 2 when claude-sonnet has observations
- [ ] Run a plan with mixed implementation and research tasks — check logs for model selections
- [ ] Verify `sonar` is selected for research tasks and `claude-sonnet` (or equivalent) for implementation tasks
- [ ] Check that existing stage-transition unit tests still pass after the confidence_scores change
- [ ] (Level 2 only) Verify that old `cascade-router.json` state is migrated correctly on load

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` | Add category-aware scoring in `confidence_scores()` (line 2445) and `ucb_scores()` (line 2563); (Level 2) add per-category partitioning to `observe_internal()` and `confidence_stats` lookup |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade/tests.rs` | Add test cases verifying sonar is not selected for implementation tasks in stage 2 |
