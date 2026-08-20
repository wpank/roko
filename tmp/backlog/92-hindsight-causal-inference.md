# 92 — Hindsight Regression Detection Uses Temporal Ordering, Not Data-Flow

**Priority**: P2 — False-positive regressions in parallel plans corrupt the learning signal; episodes are penalized for failures they did not cause
**Size**: M (2–3 days)
**Crates**: `crates/roko-learn/` (`src/hindsight.rs`, `src/episode_logger.rs`)
**Depends on**: None

---

## Background

Roko's learning system records episodes for every agent task and uses a "hindsight relabeler" to retroactively adjust episode outcomes. The relabeler's job is to detect regressions: situations where a task appeared successful but later evidence (a gate failure) reveals that something went wrong.

The current regression detector uses a purely temporal heuristic: it finds a later-timestamped failing episode that shares either the same task ID or at least one filename with the successful episode. This is an over-approximation. When a plan runs tasks in parallel, multiple independent tasks will frequently touch the same files (for example, `Cargo.toml`, `src/lib.rs`, or any shared module). If task A edits `src/lib.rs` and later task B also edits `src/lib.rs` and fails a gate, the hindsight system marks A as a regression — even though A and B may be completely independent changes and B's failure may be caused entirely by B's own code.

The `Episode` struct already has three signal-hash fields (`input_signal_hash`, `output_signal_hash`, `trigger_signal_hash`) that exist precisely to establish data-flow relationships between episodes, but the hindsight detector ignores them entirely. The fix is to consult these fields before declaring a causal link, and to exclude high-churn files that are touched by nearly every task from the overlap check.

## Current State

1. **Regression detection logic**: `crates/roko-learn/src/hindsight.rs`, lines 73–95 (the `if episode.success { ... }` block inside `HindsightRelabeler::scan`). Checks only temporal ordering and filename overlap.

2. **Signal hash fields on Episode**: `crates/roko-learn/src/episode_logger.rs`:
   - `input_signal_hash: String` — line 204
   - `output_signal_hash: String` — line 207
   - `trigger_signal_hash: String` — line 229
   These are populated by the runner when episodes are recorded. They default to empty strings when not set (lines 306–307, 313 in the `Episode::new` constructor).

3. **File overlap extraction**: `crates/roko-learn/src/hindsight.rs`, lines 153–167 (`episode_files` and `shared_files`). Reads filenames from `episode.extra["files"]`, which is an optional JSON array populated by some (but not all) task types. When empty or absent, the overlap check always produces an empty set.

4. **Existing test**: `crates/roko-learn/src/hindsight.rs`, lines 191–209 (`hindsight_relabels_a_later_regression`). Creates two episodes one minute apart sharing `"src/lib.rs"`, verifies regression is detected. This test must continue to pass.

5. **`shared_files` return type discrepancy**: The backlog's original description said `shared_files` returns `HashSet<String>`. The actual implementation (line 165) returns `Vec<String>`:
   ```rust
   fn shared_files(left: &HashSet<String>, right: &HashSet<String>) -> Vec<String> {
       left.intersection(right).cloned().collect()
   }
   ```
   The `is_empty()` check on line 78 still works with a `Vec`.

## Implementation Plan

### Step 1: Add a signal-hash chain check

In `crates/roko-learn/src/hindsight.rs`, add a helper function that checks whether a later episode has a data-flow dependency on an earlier episode:

```rust
/// Returns true if `later` has a documented data-flow dependency on `earlier`.
/// A dependency is established when the later episode's input or trigger hash
/// references the earlier episode's output hash.
fn has_signal_chain(earlier: &Episode, later: &Episode) -> bool {
    if earlier.output_signal_hash.is_empty() {
        return false;
    }
    later.input_signal_hash == earlier.output_signal_hash
        || later.trigger_signal_hash == earlier.output_signal_hash
}
```

### Step 2: Add a common-file exclusion list

Add a module-level constant for files that are touched by nearly every task and should not be treated as causal evidence:

```rust
/// Files that are routinely touched by many independent tasks.
/// Shared-file overlap on these paths does not imply causal dependency.
const NOISE_FILES: &[&str] = &[
    "Cargo.toml",
    "Cargo.lock",
    "src/lib.rs",
    "src/mod.rs",
    "mod.rs",
];

fn is_noise_file(path: &str) -> bool {
    NOISE_FILES.iter().any(|noise| {
        path == *noise || path.ends_with(&format!("/{noise}"))
    })
}
```

### Step 3: Modify the regression detection condition

In `HindsightRelabeler::scan` (lines 73–95), replace the current regression finder with one that requires either a signal-hash chain OR a non-trivial file overlap (excluding noise files). Temporal ordering is kept as a prerequisite filter, not a sufficient condition:

```rust
if episode.success {
    let files = episode_files(episode);
    // Only count files that are not high-churn noise.
    let meaningful_files: HashSet<String> = files
        .into_iter()
        .filter(|f| !is_noise_file(f))
        .collect();

    let regression = episodes[index.saturating_add(1)..].iter().find(|later| {
        if later.success
            || later.timestamp < episode.timestamp
            || !later.gate_verdicts.iter().any(|v| !v.passed)
        {
            return false;
        }

        // Prefer signal-hash evidence: direct data-flow dependency.
        if has_signal_chain(episode, later) {
            return true;
        }

        // Same task ID is strong evidence (same task re-ran and failed).
        if later.task_id == episode.task_id && !episode.task_id.is_empty() {
            return true;
        }

        // File overlap only counts if the shared files are non-trivial.
        let later_files: HashSet<String> = episode_files(later)
            .into_iter()
            .filter(|f| !is_noise_file(f))
            .collect();
        !shared_files(&meaningful_files, &later_files).is_empty()
    });

    if let Some(later) = regression {
        // ... push EpisodeAdjustment as before
    }
}
```

### Step 4: Update the existing test to verify it still passes

The existing test `hindsight_relabels_a_later_regression` (line 191) uses `"src/lib.rs"` as the shared file. Under the new code, `"src/lib.rs"` appears in `NOISE_FILES`, so the file overlap path would not fire for this specific test. You have two options:

**Option A** (recommended): Change the test to use a non-noise file like `"src/feature.rs"`:
```rust
original.extra.insert("files".into(), serde_json::json!(["src/feature.rs"]));
// ...
regression.extra.insert("files".into(), serde_json::json!(["src/feature.rs"]));
```

**Option B**: Remove `"src/lib.rs"` from `NOISE_FILES` (it is not always a noise file in smaller crates). A tighter definition of noise files might only include `"Cargo.toml"`, `"Cargo.lock"`, and files literally named `mod.rs`.

Choose whichever is more accurate for this codebase. Option A is simpler.

### Step 5: Add new tests

Add two tests to the `tests` module in `crates/roko-learn/src/hindsight.rs`:

```rust
#[test]
fn concurrent_episodes_on_noise_file_do_not_create_regression() {
    use crate::episode_logger::EpisodeGateVerdict;
    let mut task_a = Episode::new("a", "task-a");
    task_a.success = true;
    task_a.extra.insert("files".into(), serde_json::json!(["Cargo.toml"]));

    let mut task_b = Episode::new("b", "task-b");
    task_b.timestamp = task_a.timestamp + Duration::seconds(5);
    task_b.success = false;
    task_b.extra.insert("files".into(), serde_json::json!(["Cargo.toml"]));
    task_b.gate_verdicts = vec![EpisodeGateVerdict::new("compile", false)];
    // task_b has no signal-hash link to task_a.

    let found = HindsightRelabeler::new().scan(&[task_a, task_b], &[]);
    assert!(
        found.is_empty(),
        "noise-file overlap alone should not trigger a regression"
    );
}

#[test]
fn signal_hash_chain_triggers_regression_without_file_overlap() {
    use crate::episode_logger::EpisodeGateVerdict;
    let mut task_a = Episode::new("a", "task-a");
    task_a.success = true;
    task_a.output_signal_hash = "abc123".to_string();
    // No files in extra.

    let mut task_b = Episode::new("b", "task-b");
    task_b.timestamp = task_a.timestamp + Duration::seconds(10);
    task_b.success = false;
    task_b.input_signal_hash = "abc123".to_string(); // references task_a's output
    task_b.gate_verdicts = vec![EpisodeGateVerdict::new("test", false)];
    // No files in extra.

    let found = HindsightRelabeler::new().scan(&[task_a.clone(), task_b], &[]);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].original_episode_id, task_a.id);
    assert_eq!(found[0].adjustment_kind, AdjustmentKind::Regression);
}
```

## Acceptance Criteria

1. Two concurrent successful and failing episodes that share only a noise file (e.g. `Cargo.toml`) do not produce a regression adjustment.
2. Two episodes connected by a signal-hash chain (`earlier.output_signal_hash == later.input_signal_hash`) still produce a regression adjustment even with no file overlap.
3. Two episodes sharing a non-noise file (e.g. `src/feature.rs`) with temporal ordering still produce a regression adjustment (backward compatibility).
4. Existing test `hindsight_relabels_a_later_regression` passes (update to use a non-noise filename if needed).
5. New test `concurrent_episodes_on_noise_file_do_not_create_regression` passes.
6. New test `signal_hash_chain_triggers_regression_without_file_overlap` passes.

## Verification Checklist

- [ ] `cargo test -p roko-learn -- hindsight` passes
- [ ] `cargo clippy -p roko-learn -- -D warnings` passes
- [ ] Manually confirm `NOISE_FILES` list is appropriate for this codebase's common patterns
- [ ] Review that `has_signal_chain` correctly handles the case where both hashes are empty (must return `false` to avoid false positives)

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-learn/src/hindsight.rs` | Add `has_signal_chain`, `is_noise_file`, `NOISE_FILES`; rewrite regression detection condition; update existing test; add two new tests |
