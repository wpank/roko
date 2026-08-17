# LERN_15: Wire Knowledge-Informed Model Routing

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-15`](../ISSUE-TRACKER.md#lern-15)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.15
- Priority: **P2**
- Effort: 4 hours
- Depends on: `LERN_09` (source 7.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`DreamRoutingAdvice` (at `roko-dreams/src/routing_advice.rs:19`) contains pattern-based model recommendations generated during dream cycles. `load_dream_routing_advice()` (at line 241) loads from `.roko/learn/dream-routing-advice.json`. `dream_advice_to_routing_bias()` (at line 154) converts advice to routing bias.

The dream advice file is written during `DreamCycle::run()` but never read at dispatch time.

## Exact Changes

1. At CascadeRouter initialization in `roko run`, call `load_dream_routing_advice(&workdir)`.
2. If advice exists, call `dream_advice_to_routing_bias(&advice)` to get bias values.
3. Apply bias to the routing context before `select_for_frequency_among()`: adjust alpha or modify candidate scoring.
4. After routing outcome (success/failure with selected model), this is already fed back through `FeedbackService` -> `CascadeRouter` observation (from Task 7.5). No additional wiring needed for the feedback direction.
5. Log when dream advice influences model selection.

## Write Scope

- `crates/roko-cli/src/run.rs`
- `crates/roko-learn/src/cascade_router.rs`

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

- [ ] Run `roko knowledge dream run` to generate advice, then run tasks -- verify advice file is loaded
- [ ] Router considers dream advice in model selection (visible in logs)
- [ ] Advice has no effect when the file is missing (graceful fallback)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run `roko knowledge dream run` to generate advice, then run tasks -- verify advice file is loaded
- Router considers dream advice in model selection (visible in logs)
- Advice has no effect when the file is missing (graceful fallback)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
