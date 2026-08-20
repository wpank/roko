# 80 — Learning Subsystem Data Quality

**Priority**: P2 — corrupted learning data degrades model routing and knowledge retrieval over time
**Size**: M (2-3 days)
**Crates**: `crates/roko-learn` (paths: `src/cascade_router.rs`, `src/cascade/helpers.rs`, `src/model_router.rs`), `crates/roko-neuro`, `crates/roko-cli` (path: `src/doctor.rs`)
**Depends on**: None (item 78 for A7 context, but this item is independently actionable)

---

## Background

The cascade router, knowledge store, gate threshold tracker, and dream subsystem all accumulate data over time to improve decision-making. After a real dogfood plan run, that data has six quality problems: stale temp files, empty knowledge receipts, a cascade router that doesn't partition observations by per-model count, underpopulated gate threshold defaults, slow dream confidence progression, and a corrupted state snapshot. Taken together, the system is learning from noisy or wrong data, which compounds over time.

None of these issues are user-visible failures — the system runs correctly. But they degrade the quality of learned routing decisions, inflate the knowledge store with noise, and mean dream consolidation is not effectively promoting useful patterns to working memory.

## Current State

1. **Cascade stage transitions are globally-gated, not per-model** — `crates/roko-learn/src/cascade/helpers.rs` line 542 defines `stage_for_observations(obs: u64) -> CascadeStage`, which reads the total observation count across all models (from `LinUCB.total_observations()`). The threshold constants are `COLD_START_THRESHOLD = 50` (defined in `crates/roko-learn/src/model_router.rs` line 65) and `CONFIDENCE_TO_UCB_THRESHOLD = 200` (defined in `crates/roko-learn/src/cascade/types.rs` line 383). Once 50 total observations accumulate, the router transitions all models to stage 2 (Confidence). A model with 0 observations gets `s.trials == 0` in `confidence_scores()` at `crates/roko-learn/src/cascade_router.rs` line 2459, which produces a base score of 0.5 — making it competitive with well-observed models.

2. **Stale `.tmp` files in `.roko/learn/`** — The atomic file writer used by the cascade router creates `.tmp` files that are renamed on success. If the process crashes between creation and rename, the `.tmp` files remain. No cleanup path exists.

3. **Content-free execution receipts in the knowledge store** — `crates/roko-neuro/` knowledge queries scan all entries, including receipts that record "task ran" metadata without useful content. These dilute the signal-to-noise ratio in query results by pushing actual knowledge entries out of result sets.

4. **Gate threshold tracking covers only 3 of 7 rungs** — The adaptive threshold EMA in `.roko/learn/gate-thresholds.json` only has data for the compile, test, and clippy rungs. The format, LLM-judge, diff, and fact-check rungs start from defaults with no adaptive history when they're first activated.

5. **Dream entries stuck at lowest confidence levels** — If 84% of dream entries remain at Raw or Replayed confidence (0.10-0.30), the dream validation pipeline is stalling before promoting entries to Working (0.40) or higher. This is likely related to the deadlock documented in item 83.

6. **State snapshot validation error on every `roko show` command** — `.roko/state/state-snapshot.json` contains a `lifecycle` count that exceeds `total_tasks`. This produces a non-fatal validation error on every read but indicates data integrity drift from a previous plan run where `total_tasks` was not updated.

7. **`gate_passed` is incorrectly 0% for all models** — (root cause is in item 78). The cascade router receives `gate_passed = false` for every outcome, preventing it from learning which models produce gate-passing code. This is noted here for completeness; the fix belongs in item 78, but data remediation belongs here.

## Implementation Plan

### Step 1 — Add per-model minimum observation guard to stage 2 confidence scoring

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`

In `confidence_scores()` (line 2445), when `s.trials == 0`, instead of returning 0.5 (which makes zero-observation models competitive), return a lower score so they are selected for exploration rather than exploitation:

```rust
fn confidence_scores(&self, candidates: &[String], ctx: &RoutingContext) -> Vec<(String, f64)> {
    let stats = self.confidence_stats.lock();
    // ... existing setup ...

    let mut scores: Vec<(String, f64)> = candidates
        .iter()
        .map(|slug| {
            let s = stats.get(slug).cloned().unwrap_or_default();
            let base_score = if s.trials == 0 {
                // No observations for this model — use a low score to deprioritize
                // during exploitation, but not 0.0 so it still gets occasional
                // exploration turns from the hysteresis logic.
                0.2
            } else {
                s.upper_bound()
            };
            // ... existing tier_bonus logic ...
            (slug.clone(), base_score + tier_bonus)
        })
        .collect();
    // ...
}
```

This ensures models with zero observations don't get selected for important tasks just because their upper-confidence-bound defaults to 0.5.

### Step 2 — Add stale `.tmp` file cleanup to `roko doctor`

File: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs`

Add a new check function and register it in the check list. The check scans `.roko/learn/` for `.tmp` files older than 1 hour and `.corrupted` files:

```rust
fn check_stale_learn_files(workdir: &Path) -> Vec<DoctorCheck> {
    let learn_dir = workdir.join(".roko").join("learn");
    if !learn_dir.is_dir() {
        return vec![];
    }

    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(3600))
        .unwrap_or(std::time::UNIX_EPOCH);

    let mut stale = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&learn_dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if name.ends_with(".tmp") || name.ends_with(".corrupted") {
                if let Ok(meta) = entry.metadata() {
                    if meta.modified().map(|m| m < cutoff).unwrap_or(true) {
                        stale.push(path.display().to_string());
                    }
                }
            }
        }
    }

    if stale.is_empty() {
        return vec![];
    }

    vec![DoctorCheck {
        id: "stale_learn_files".to_string(),
        status: DoctorStatus::Warn,
        message: format!("{} stale file(s) in .roko/learn/", stale.len()),
        detail: Some(stale.join("\n")),
        path: Some(learn_dir.display().to_string()),
        url: None,
        fix: Some(format!("rm {}", stale.join(" "))),
    }]
}
```

Register this check in the main `run_checks()` function (around line 200-210 where other checks are collected).

### Step 3 — Filter content-free entries from knowledge queries

File: Look up the knowledge query function in `crates/roko-neuro/src/`. Find where `query()` returns a list of `KnowledgeEntry` values.

Add a filter that excludes entries where the `content` field is empty or consists only of whitespace:

```rust
// In the query() return path, add:
.filter(|entry| !entry.content.trim().is_empty())
```

This is a read-path-only change and does not affect writes. No migration needed. Alternatively, stop writing receipts without content to the knowledge store — write them only to the episode log. That is a write-path change in the runner.

### Step 4 — Pre-populate all gate threshold defaults

File: The gate threshold initialization in `crates/roko-cli/src/runner/` (wherever `gate-thresholds.json` is first written or its defaults are set).

Find where adaptive thresholds default to initial values. Ensure entries exist for all 7 rungs — not just compile, test, and clippy. The four missing rungs are: format, LLM-judge, diff, and fact-check. Add them with conservative starting thresholds (e.g., 0.85 pass rate for format, 0.70 for LLM-judge and diff, 0.90 for fact-check).

### Step 5 — Fix corrupted state snapshot

This is a one-time data fix, not a code change. The `.roko/state/state-snapshot.json` has `lifecycle.completed_count > total_tasks`. Two options:

**Option A (recommended)**: Delete the snapshot file and let it regenerate on the next plan run:
```bash
rm /path/to/workspace/.roko/state/state-snapshot.json
```

**Option B**: Edit the snapshot and set `total_tasks` to match the lifecycle count.

Regardless, add a validation step in the snapshot reader that catches this mismatch and either repairs it automatically or emits a clear error rather than silently corrupting state.

Find the snapshot loading code in `crates/roko-cli/src/runner/` and add:

```rust
if snapshot.lifecycle.completed_count > snapshot.total_tasks {
    tracing::warn!(
        "state snapshot is inconsistent: lifecycle.completed_count ({}) > total_tasks ({}); \
         resetting total_tasks",
        snapshot.lifecycle.completed_count,
        snapshot.total_tasks,
    );
    snapshot.total_tasks = snapshot.lifecycle.completed_count;
}
```

### Step 6 — After gate_passed fix (item 78), mark pre-fix efficiency entries

Once item 78 is resolved, locate the efficiency JSONL file at `.roko/learn/efficiency.jsonl`. Optionally add a one-time migration that rewrites historical `gate_passed: false` entries as `gate_passed: null` so the cascade router doesn't continue learning from the incorrect signal. This migration can be run as part of `roko learn all` or documented as a manual step.

## Acceptance Criteria

1. `roko learn route` shows per-model observation counts; models with 0 observations score lower than observed models in stage 2.
2. `roko doctor` reports any `.tmp` files older than 1 hour in `.roko/learn/` as a warning with a `rm` fix command.
3. `roko knowledge query "rust"` returns only entries with non-empty content fields.
4. `.roko/learn/gate-thresholds.json` after a fresh plan run has entries for all 7 rungs (compile, test, clippy, format, LLM-judge, diff, fact-check).
5. `roko show plans` produces no "lifecycle" validation error after the snapshot repair.
6. After item 78 is fixed, `roko learn efficiency` shows non-zero `gate_passed` rates.

## Verification Checklist

- [ ] Add an observation to a model via test, verify that a zero-observation model scores below 0.5 in `confidence_scores()` via unit test
- [ ] Create a stale `.tmp` file in `.roko/learn/` (older than 1 hour), run `roko doctor` — warning appears
- [ ] Run `roko knowledge query ""` — no empty-content entries in results
- [ ] Run a plan to completion, inspect `.roko/learn/gate-thresholds.json` — all 7 rungs have entries
- [ ] Delete `.roko/state/state-snapshot.json`, run a plan, then run `roko show plans` — no lifecycle error
- [ ] `cargo test -p roko-learn` passes

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs` | Change zero-observation base score from 0.5 to 0.2 in `confidence_scores()` (line 2459) |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs` | Add `check_stale_learn_files()` function and register it in the check list |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-neuro/src/` | Add content-empty filter in the `query()` return path |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/` | Pre-populate all 7 gate rung defaults; add lifecycle/total_tasks consistency repair in snapshot loader |
