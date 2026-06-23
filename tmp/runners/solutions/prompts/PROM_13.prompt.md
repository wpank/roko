# PROM_13: Record Section Inclusion in CognitiveWorkspace Audit

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-13`](../ISSUE-TRACKER.md#prom-13)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.13
- Priority: **??**
- Effort: 2-3 days | **Impact**: High (most common interactive entry points)
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: `roko chat` and `dispatch_direct.rs` bypass the builder
partially or entirely. `compact_history()` is ready but not wired. Long
chat sessions grow without bound.

## Exact Changes

1. Add `influence_weights_applied: HashMap<String, f64>` to `CognitiveWorkspaceInput` (currently at line 21)
2. Include the weights in the `CognitiveWorkspace` via an additional builder method or field
3. Populate during assembly with the actual weights that were used
4. Sections without influence data show weight 1.0

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After a dispatch, `CognitiveWorkspace` contains the influence weight for each section
- [ ] Sections without influence data show weight 1.0

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After a dispatch, `CognitiveWorkspace` contains the influence weight for each section
- Sections without influence data show weight 1.0
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
