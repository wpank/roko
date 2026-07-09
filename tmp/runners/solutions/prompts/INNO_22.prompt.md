# INNO_22: Implement confidence scoring for tasks

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-22`](../ISSUE-TRACKER.md#inno-22)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.22
- Priority: **P2**
- Effort: 8 hours
- Depends on: `INNO_02` (source 11.2), `INNO_20` (source 11.20)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_22 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Confidence scoring enables the steering system to auto-proceed on high-
confidence tasks, suggest review on medium, and require approval on low.

## Exact Changes

1. Create `crates/roko-learn/src/confidence.rs`.
2. Define `ConfidenceScore` struct: `value: f64`, `components: Vec<(String, f64)>`.
3. Implement `compute_confidence(task_description: &str, memory: &MemoryInjection,
   thresholds: &AdaptiveThresholds) -> ConfidenceScore`:
   - Component 1: task complexity vs model capability
   - Component 2: similarity to past successes (from memory layer)
   - Component 3: expected gate pass probability (from adaptive thresholds)
   - Component 4: error pattern match (similar task failed before)
4. Weighted average of components (configurable weights).
5. Compare against `ConfidenceThresholds` to determine action:
   `AutoProceed`, `SuggestReview`, `RequireApproval`.
6. Add `pub mod confidence;` to `crates/roko-learn/src/lib.rs`.

## Write Scope

- `crates/roko-learn/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] A task with many similar past successes scores > 0.85
- [ ] A task touching unfamiliar code with no history scores < 0.5
- [ ] Confidence is logged per task in the episode data

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_22 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A task with many similar past successes scores > 0.85
- A task touching unfamiliar code with no history scores < 0.5
- Confidence is logged per task in the episode data
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_22 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
