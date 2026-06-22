# ACPM_39: Implement Linear TrackerAdapter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-39`](../ISSUE-TRACKER.md#acpm-39)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.39
- Priority: **P3**
- Effort: 4 hours
- Depends on: `ACPM_36` (source 9.36)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_39 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Linear uses a GraphQL API. The adapter syncs issues bidirectionally.

## Exact Changes

1. Implement `LinearTrackerAdapter { api_key, team_id, state_mapping }`.
2. `fetch_active()`: call Linear GraphQL API to list issues in active states with team filter.
3. `update_state()`: transition the Linear issue to the mapped state via GraphQL mutation.
4. `create_task()`: create a new Linear issue with team assignment.
5. Default state mapping: `pending -> Backlog`, `in_progress -> In Progress`, `completed -> Done`, `failed -> Backlog` (with comment).

## Write Scope

_None — this is a documentation/verification-only batch._

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

- [ ] GraphQL query fetches active issues
- [ ] State transitions map correctly between Roko and Linear
- [ ] Missing `LINEAR_API_KEY` returns clear error

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_39 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- GraphQL query fetches active issues
- State transitions map correctly between Roko and Linear
- Missing `LINEAR_API_KEY` returns clear error
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_39 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
