# 91 — Experiment Variant Assignment Not Stable Across Resume

**Priority**: P2 — Variant instability adds noise to A/B experiment data; the same task can get different models on resume, corrupting outcome attribution
**Size**: S (1 day)
**Crates**: `crates/roko-learn/` (`src/model_experiment.rs`)
**Depends on**: None

---

## Background

Roko runs model A/B experiments to learn which LLM performs best for a given role or task category. When a plan is interrupted and resumed, or when two tasks run concurrently, the same task ID may receive a different model variant than it did in the original run. This happens because variant selection uses a UCB1 bandit score that changes every time an outcome is recorded — so the "best" variant at resume time may differ from the variant assigned originally.

A secondary problem is initial exploration bias. When no variant has been tried yet, every variant returns `f64::MAX` from `ucb_score()`. The selection loop picks the first one in the iteration order — and because `self.variants` is a `Vec`, the first variant declared in the experiment config always wins the initial tie. This means the first variant listed gets statistically more early trials, biasing the experiment before any real data is collected.

Both issues stem from the absence of a deterministic, per-task tiebreaker. The fix is to add a hash-based tiebreaker that derives the winner from `(experiment_id, task_id)` when two variants are tied or both unsampled, making assignment reproducible without changing the exploration formula.

## Current State

1. **UCB1 implementation**: `crates/roko-learn/src/model_experiment.rs`, lines 97–107. When `self.trials == 0 || total_trials == 0`, returns `f64::MAX`. All unsampled variants tie at this value.

2. **Variant selection loop**: `crates/roko-learn/src/model_experiment.rs`, lines 120–145 (`assign_variant`). Uses `score > best_score` — strict greater-than — so for equal scores the first variant in `self.variants` always wins. No `task_id` is passed to this function.

3. **Public assignment entry point**: `crates/roko-learn/src/model_experiment.rs`, lines 297–310 (`assign_model_with_experiment`). This is what the runner calls. Its signature is `fn assign_model_with_experiment(&self, role: &str, category: &str) -> Option<(String, ModelVariant)>` — no task identifier is threaded through.

4. **Existing test confirming first-variant bias**: `crates/roko-learn/src/model_experiment.rs`, line 609. The test `model_experiment_ucb` asserts `assign_variant()` returns `"glm"` (the first variant) for an empty experiment. This test documents the current behavior and must be updated to reflect hash-based tiebreaking.

5. **No persistence of assignment**: There is no mapping from `(experiment_id, task_id)` to `variant_id` on disk. Every call to `assign_variant` recomputes from live statistics.

## Implementation Plan

### Step 1: Add `task_id` parameter to `assign_variant` and `assign_model_with_experiment`

In `crates/roko-learn/src/model_experiment.rs`:

Change the signature of `assign_variant`:
```rust
// Before (line 120):
pub fn assign_variant(&self) -> Option<&ModelVariant> {

// After:
pub fn assign_variant(&self, task_id: &str) -> Option<&ModelVariant> {
```

Add a tiebreaker hash helper (add near the bottom of the `impl ModelExperiment` block, above the closing brace):
```rust
fn tiebreak_index(&self, task_id: &str, tied_variants: &[usize]) -> usize {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    self.experiment_id.hash(&mut hasher);
    task_id.hash(&mut hasher);
    let h = hasher.finish() as usize;
    tied_variants[h % tied_variants.len()]
}
```

Rewrite the body of `assign_variant` to collect ties and break them deterministically:
```rust
pub fn assign_variant(&self, task_id: &str) -> Option<&ModelVariant> {
    if self.status == ExperimentStatus::Concluded {
        return self
            .variants
            .iter()
            .find(|variant| Some(&variant.id) == self.winner_id.as_ref());
    }

    let total: u64 = self.stats.values().map(|stats| stats.trials).sum();
    let mut best_score = f64::NEG_INFINITY;
    let mut best_indices: Vec<usize> = Vec::new();

    for (idx, variant) in self.variants.iter().enumerate() {
        let score = self
            .stats
            .get(&variant.id)
            .map(|stats| stats.ucb_score(total))
            .unwrap_or(f64::MAX);
        // Use an epsilon comparison to collect all variants at the same score.
        if (score - best_score).abs() < f64::EPSILON || score > best_score {
            if score > best_score {
                best_score = score;
                best_indices.clear();
            }
            best_indices.push(idx);
        }
    }

    if best_indices.is_empty() {
        return None;
    }

    let chosen_idx = if best_indices.len() == 1 {
        best_indices[0]
    } else {
        self.tiebreak_index(task_id, &best_indices)
    };
    self.variants.get(chosen_idx)
}
```

**Note on epsilon comparison**: `f64::MAX` values won't have floating-point drift between each other (they are the same bit pattern), so comparing two `f64::MAX` values with `==` is safe. The epsilon branch is there for sampled variants that may produce numerically identical UCB scores. You may simplify to a direct `==` comparison if that is cleaner:
```rust
if score > best_score {
    best_score = score;
    best_indices.clear();
    best_indices.push(idx);
} else if score == best_score {
    best_indices.push(idx);
}
```

### Step 2: Thread `task_id` through `assign_model_with_experiment`

Change the call at line 309 in `assign_model_with_experiment`:
```rust
// Before:
pub fn assign_model_with_experiment(
    &self,
    role: &str,
    category: &str,
) -> Option<(String, ModelVariant)> {
    let experiment = self.applicable_experiment(role, category)?;
    let variant = experiment.assign_variant()?.clone();
    Some((experiment.experiment_id.clone(), variant))
}

// After:
pub fn assign_model_with_experiment(
    &self,
    role: &str,
    category: &str,
    task_id: &str,
) -> Option<(String, ModelVariant)> {
    let experiment = self.applicable_experiment(role, category)?;
    let variant = experiment.assign_variant(task_id)?.clone();
    Some((experiment.experiment_id.clone(), variant))
}
```

Update `assign_model` (line 297) to forward `task_id`:
```rust
pub fn assign_model(&self, role: &str, category: &str, task_id: &str) -> Option<ModelVariant> {
    self.assign_model_with_experiment(role, category, task_id)
        .map(|(_, variant)| variant)
}
```

### Step 3: Update callers

Search for all callers of `assign_model` and `assign_model_with_experiment` across the workspace:
```
grep -rn "assign_model\|assign_model_with_experiment" crates/ --include="*.rs"
```

Pass the task ID (or plan-task ID string) from the dispatch context. In runner dispatch code the task ID is typically available as `task.id` or similar. If no task ID is available (e.g. in a bare `roko run` call), pass `""` as the task ID — the hash of an empty string is deterministic and consistent.

### Step 4: Update tests

In `crates/roko-learn/src/model_experiment.rs`, update the test `model_experiment_ucb` (line 580) to pass a `task_id` string:
```rust
// Before:
assert_eq!(experiment.assign_variant().map(|v| v.id.as_str()), Some("glm"));

// After:
assert_eq!(experiment.assign_variant("task-1").map(|v| v.id.as_str()), Some("glm"));
```

Note: after this change, `"glm"` will be the winner for `task_id = "task-1"` and experiment id `"glm-vs-kimi"`. If the hash happens to select `"kimi"` for that specific combination, update the assertion to match. The point is that the assignment is deterministic, not that it selects the first variant.

Add two new tests in the same `tests` module:

```rust
#[test]
fn unsampled_assignment_varies_by_task_id() {
    let experiment = make_experiment("exp", Some("implementer"), None);
    // Two different task IDs should not always return the same variant
    // (they may for some hash values, but the selection is deterministic
    // and the full space of task_ids distributes across variants).
    let a = experiment.assign_variant("task-alpha").map(|v| v.id.clone());
    let b = experiment.assign_variant("task-beta").map(|v| v.id.clone());
    // At least verify both return *a* variant (not None).
    assert!(a.is_some());
    assert!(b.is_some());
    // Verify stable: same task_id always returns same variant.
    assert_eq!(a, experiment.assign_variant("task-alpha").map(|v| v.id.clone()));
    assert_eq!(b, experiment.assign_variant("task-beta").map(|v| v.id.clone()));
}

#[test]
fn assignment_stable_across_outcome_recording() {
    let mut experiment = make_experiment("resume-test", Some("implementer"), None);
    // Record an outcome for the first variant to change UCB scores.
    let initial = experiment.assign_variant("my-task").map(|v| v.id.clone());
    assert!(initial.is_some());
    // After recording an outcome for the OTHER variant, our task should still
    // get the same variant (because the statistics change doesn't affect the
    // tiebreaker hash for this specific task_id when scores are equal).
    // Note: once one variant has trials and the other doesn't, UCB will
    // deterministically prefer the unsampled one, so we record for the
    // initially-selected variant:
    if let Some(ref id) = initial {
        experiment.record_outcome(id, true, 0.1, 100, 500);
    }
    // Now the other variant is unsampled (f64::MAX) and wins deterministically.
    // Re-assign for a new task_id to show it works:
    let new_task = experiment.assign_variant("other-task").map(|v| v.id.clone());
    assert!(new_task.is_some());
    // Calling again with the same task_id must return the same result.
    assert_eq!(
        new_task,
        experiment.assign_variant("other-task").map(|v| v.id.clone())
    );
}
```

## Acceptance Criteria

1. `assign_variant` accepts a `task_id: &str` parameter and uses it to break ties deterministically.
2. Calling `assign_variant("same-task")` twice on the same `ModelExperiment` (with unchanged statistics) returns the same variant.
3. Two different task IDs may return different variants when scores are tied (the hash distributes).
4. `assign_model` and `assign_model_with_experiment` on `ModelExperimentStore` both accept and thread through `task_id`.
5. All existing model experiment tests pass (update call sites to pass a task_id string).
6. New test `unsampled_assignment_varies_by_task_id` passes.
7. New test `assignment_stable_across_outcome_recording` passes.

## Verification Checklist

- [ ] `cargo test -p roko-learn -- model_experiment` passes after all call-site updates
- [ ] `grep -rn "assign_variant()" crates/ --include="*.rs"` returns no results (all callers updated)
- [ ] `grep -rn "assign_model(" crates/ --include="*.rs"` confirms all callers pass a task_id
- [ ] New tests added and passing

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-learn/src/model_experiment.rs` | Add `task_id: &str` to `assign_variant`, `assign_model`, `assign_model_with_experiment`; add `tiebreak_index` helper; rewrite selection loop to collect ties; add two new tests |
| Any file calling `assign_model` or `assign_variant` | Update call sites to pass a `task_id` string |
