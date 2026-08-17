# LERN_12: Wire Anomaly Detection to `roko run` and `roko chat`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-12`](../ISSUE-TRACKER.md#lern-12)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.12
- Priority: **P1**
- Effort: 3 hours
- Depends on: `LERN_02` (source 7.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`AnomalyDetector` (at `anomaly.rs:20`) has session-local state for 3 detection channels:
- `check_prompt(prompt_hash: u64)` -> `Option<Anomaly::PromptLoop>`: sliding window of 20 hashes, alert at 5+ repeats
- `check_cost(cost_usd: f64)` -> `Option<Anomaly::CostSpike>`: EWMA baseline, z-score > 3.0
- `check_quality(score: f64)` -> `Option<Anomaly::QualityDrift>`: recent 5 vs prior 10 window
- `check_budget(limit_usd: f64)` -> `Option<Anomaly::BudgetExceeded>`: total cost vs limit

`learning_helpers.rs` (at line 11) imports `AnomalyDetector` and defines helper functions that accept `&mut AnomalyDetector` but these are not called from `run.rs` or `chat_session.rs`.

## Exact Changes

1. In `roko run`, create `AnomalyDetector::new(now_unix_ms_i64())` at session start.
2. After each model call, call:
   - `detector.check_cost(cost_usd)` -> if `Some(Anomaly::CostSpike { .. })`, log at WARN
   - `detector.check_prompt(hash_of_prompt)` -> if `Some(Anomaly::PromptLoop { .. })`, log at WARN, consider aborting
3. In `roko chat`, create `AnomalyDetector::new(now_unix_ms_i64())` at session start.
4. After each turn, check cost spike and prompt loop.
5. On anomaly detection: emit a warning to the user via stderr/tracing, but do not abort by default. Add a config option `anomaly.abort_on_loop = false` for future use.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-cli/src/chat_session.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Repeat the same prompt 10 times in `roko chat`, verify prompt loop warning
- [ ] Send a very expensive prompt, verify cost spike warning
- [ ] Normal operation produces no anomaly warnings

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Repeat the same prompt 10 times in `roko chat`, verify prompt loop warning
- Send a very expensive prompt, verify cost spike warning
- Normal operation produces no anomaly warnings
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
