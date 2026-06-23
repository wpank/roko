# STAB_31: Wire gate feedback_for_agent into GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-31`](../ISSUE-TRACKER.md#stab-31)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.31
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_31 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`feedback_for_agent()` at line 202 of `feedback.rs` is exported from `roko-gate` (line 156
of `lib.rs`) but called only from `orchestrate.rs` (dead code in V2 paths). The V2 paths
run gates but dump raw stderr into the retry context.

## Exact Changes

1. In `GateService`, after running gates, call `feedback_for_agent()` on any failed verdicts.
2. Add a `feedback: Option<GateFeedback>` field to the gate result/report struct.
3. When the pipeline state machine handles `GatesFailed`, extract feedback and inject into
   the retry prompt (replacing raw stderr).

## Design Guidance

Structured feedback (file, line, error message, suggestion) is far more valuable to the
retry agent than raw build output. The feedback should be under 1K tokens even for large
build failures.

## Write Scope

- `crates/roko-gate/src/feedback.rs`
- `crates/roko-gate/src/gate_service.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Task that fails compile gets structured feedback in retry prompt
- [ ] Feedback includes specific errors (not raw stderr)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_31 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Task that fails compile gets structured feedback in retry prompt
- Feedback includes specific errors (not raw stderr)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_31 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
