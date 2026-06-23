# LERN_17: Wire Knowledge Feedback Scoring Integration

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-17`](../ISSUE-TRACKER.md#lern-17)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.17
- Priority: **P2**
- Effort: 4 hours
- Depends on: `LERN_05` (source 7.5), `LERN_10` (source 7.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`FeedbackService` tracks knowledge provenance: on `ModelCall`, it remembers which `knowledge_ids` were used. On `GateResult`, it resolves provenance for the `run_id` and applies `KnowledgeOutcome` (+1/-1 score). Scores persist to `knowledge-scores.json`.

`FeedbackService::record_knowledge_usage()` (at `feedback_service.rs:300`) takes `run_id`, `knowledge_ids`, `passed`, `model`. But the caller needs to provide `knowledge_ids` -- these come from prompt assembly when knowledge entries are included.

## Exact Changes

1. During prompt assembly in `run.rs`, if knowledge entries are included in the prompt (from `roko-neuro` query), capture their IDs.
2. Pass `knowledge_ids` in the `FeedbackEvent::ModelCall.knowledge_ids` field.
3. After gates complete, `FeedbackService` automatically resolves provenance from the `run_id` and updates scores (this is already implemented in the `GateResult` handler).
4. On next prompt assembly, load `knowledge-scores.json` and use scores to influence knowledge retrieval ranking (higher-scored entries prioritized).
5. Log when a knowledge entry's score drops below 0 (consistently unhelpful).

## Write Scope

- `crates/roko-cli/src/run.rs`

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

- [ ] Add a knowledge entry, run tasks that use it, check `knowledge-scores.json` shows accumulating score
- [ ] Knowledge entries with negative scores are deprioritized in subsequent retrievals

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Add a knowledge entry, run tasks that use it, check `knowledge-scores.json` shows accumulating score
- Knowledge entries with negative scores are deprioritized in subsequent retrievals
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
