# CONF_16: Wire `build_repo_context()` Into Plan Generation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-16`](../ISSUE-TRACKER.md#conf-16)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.16
- Priority: **P2**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`build_repo_context()` at `crates/roko-cli/src/repo_context.rs:282` gives agents
awareness of repository structure. It IS called from `prd draft new`
(`commands/prd.rs:383`) but NOT from `plan generate`, `plan regenerate`, or `prd plan`.

Generated plans propose greenfield crates that duplicate existing functionality.

## Exact Changes

1. Call `build_repo_context()` before agent dispatch in `plan generate`,
   `plan regenerate`, and `prd plan` handlers.
2. Include the repo context as a system prompt section or user context section.
3. For `plan regenerate`, also inject validation errors from the previous attempt
   into the regeneration prompt.

## Write Scope

- `crates/roko-cli/src/commands/plan.rs`
- `crates/roko-cli/src/prd.rs`
- `crates/roko-cli/src/repo_context.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Running `roko plan generate` on a workspace with 18 crates produces a plan that

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Running `roko plan generate` on a workspace with 18 crates produces a plan that
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
