# INNO_52: Implement force_backend override learning (UX34)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-52`](../ISSUE-TRACKER.md#inno-52)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.52
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_52 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`force_backend` appears in 13 files across the codebase. CascadeRouter does
not learn from manual overrides (CLAUDE.md item 15: "UX34: force_backend
override learning").

## Exact Changes

1. In the dispatch path, detect when `force_backend` is set.
2. Record the override as a strong observation in CascadeRouter: the user
   explicitly chose this model for this task type.
3. Weight override observations 3x compared to automatic observations
   (configurable multiplier).
4. After accumulating 5+ overrides for the same task category, adjust
   CascadeRouter's static routing table to prefer the user's choice.
5. Add `roko learn tune routing --show-overrides` to display learned overrides.

## Write Scope

- `crates/roko-learn/src/cascade_router.rs`
- `crates/roko-cli/src/dispatch/model_routing.rs`

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

- [ ] After 5 `--force-backend cerebras` overrides on "simple fix" tasks, CascadeRouter routes "simple fix" tasks to Cerebras by default
- [ ] Override learning is visible in `cascade-router.json` observations
- [ ] `roko learn tune routing --show-overrides` lists learned preferences

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_52 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 5 `--force-backend cerebras` overrides on "simple fix" tasks, CascadeRouter routes "simple fix" tasks to Cerebras by default
- Override learning is visible in `cascade-router.json` observations
- `roko learn tune routing --show-overrides` lists learned preferences
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_52 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
