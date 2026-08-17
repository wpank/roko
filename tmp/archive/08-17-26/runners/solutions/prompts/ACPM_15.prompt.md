# ACPM_15: Add ReviewOnly Workflow Template

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-15`](../ISSUE-TRACKER.md#acpm-15)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.15
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_15 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

ReviewOnly is a read-only template: the agent reviews code but does not make changes. It never enters the Implementing phase.

## Exact Changes

1. Add `ReviewOnly` variant to `WorkflowTemplate`.
2. Transitions:
   - `Pending + Start` (ReviewOnly) -> `Reviewing` + `SpawnReviewer { diff_context }` (populated from git diff or prompt)
   - `Reviewing + ReviewApproved` -> `Complete` + `Done`
   - `Reviewing + ReviewRevise` -> `Complete` + `Done` (report findings, do NOT spawn implementer)
3. `has_strategy()` -> false, `has_review()` -> true.
4. Update `auto_select()`: prompts containing "review", "audit", "check" without implementation words trigger ReviewOnly.
5. Update `from_config()` to accept `"review_only"`.

## Write Scope

- `crates/roko-acp/src/pipeline.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Unit test: ReviewOnly template never enters Implementing phase
- [ ] Unit test: review findings are reported but no implementation spawned
- [ ] `auto_select("review the changes in this PR")` -> ReviewOnly

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_15 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: ReviewOnly template never enters Implementing phase
- Unit test: review findings are reported but no implementation spawned
- `auto_select("review the changes in this PR")` -> ReviewOnly
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_15 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
