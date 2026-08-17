# LERN_05: Attach CascadeRouter to FeedbackService in `roko run`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-05`](../ISSUE-TRACKER.md#lern-05)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.5
- Priority: **P0**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`dispatch_v2.rs` (at line 60-89) already creates a `FeedbackService` and attaches it to `ModelCallService` via `with_feedback_sink()`. But `run.rs` does not create a `FeedbackService` at all -- it uses `LearningRuntime::record_completed_run()` (at line 2680) which handles learning after the fact.

`FeedbackService::with_cascade_router()` (at `feedback_service.rs:133`) accepts `Arc<CascadeRouter>`. When attached, every `ModelCall` event automatically updates the router's bandit state.

`CascadeRouter` persists to `.roko/learn/cascade-router.json` via `CascadeSnapshot`.

## Exact Changes

1. In the `roko run` initialization path, load `CascadeRouter` from `.roko/learn/cascade-router.json` (use `CascadeRouter::load_or_new(path, model_slugs)` -- find the existing constructor).
2. Create `FeedbackService::from_roko_dir_with_episodes(&roko_dir).with_cascade_router(Arc::new(router))`.
3. Store the `FeedbackService` on the run context so it can be used for model call events.
4. After the agent dispatch returns, emit `FeedbackEvent::ModelCall` with the actual usage data from the `AgentResult`.
5. After gates complete, emit `FeedbackEvent::GateResult` for each verdict.
6. At run end, emit `FeedbackEvent::WorkflowComplete`.
7. Flush the service before returning.

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

- [ ] Run `roko run "test prompt"`, check `.roko/learn/cascade-router.json` observation count increases
- [ ] `efficiency.jsonl` has ModelCall + GateResult + WorkflowComplete records

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run `roko run "test prompt"`, check `.roko/learn/cascade-router.json` observation count increases
- `efficiency.jsonl` has ModelCall + GateResult + WorkflowComplete records
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
