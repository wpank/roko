# LERN_21: Wire Post-Gate Reflection Promotion to Playbook Candidates

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-21`](../ISSUE-TRACKER.md#lern-21)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.21
- Priority: **P3**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`PostGateReflectionStore` (at `post_gate_reflection.rs:191`) records post-gate reflections with `observe()` (line 230). It has a `ReflectionPromotionConfig` (at line 164) with `min_confidence`, `min_validations`, `min_consistency` thresholds.

Reflections accumulate but are never checked for promotion eligibility. Promoted reflections should become playbook candidates.

## Exact Changes

1. After calling `PostGateReflectionStore::observe()` in `LearningRuntime::record_completed_run()`, check if the reflection meets promotion thresholds from `ReflectionPromotionConfig`.
2. If eligible, create a `Playbook` from the reflection's action sequence using `extract_playbook_from_episode()` (from `playbook.rs`).
3. Add the playbook candidate to `PlaybookStore` via `store.add()` or equivalent.
4. Log promotions at INFO.

## Write Scope

- `crates/roko-learn/src/runtime_feedback.rs`
- `crates/roko-learn/src/post_gate_reflection.rs`

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

- [ ] After 5+ successful reflections for the same pattern, a playbook candidate is created
- [ ] Promoted playbooks appear in `PlaybookStore` and are queryable

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 5+ successful reflections for the same pattern, a playbook candidate is created
- Promoted playbooks appear in `PlaybookStore` and are queryable
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
