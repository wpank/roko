# ACPM_38: Implement Sentry TrackerAdapter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-38`](../ISSUE-TRACKER.md#acpm-38)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.38
- Priority: **P3**
- Effort: 4 hours
- Depends on: `ACPM_36` (source 9.36)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_38 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Sentry errors can be ingested as fix tasks. The adapter fetches unresolved issues with stack traces and converts them to `ExternalTask` descriptions.

## Exact Changes

1. Implement `SentryTrackerAdapter { org, project, token, state_mapping }`.
2. `fetch_active()`: call Sentry REST API to list unresolved issues with configurable filters (assignee, tag).
3. For each Sentry issue, construct `ExternalTask` with: `description` = stack trace + affected files + error count, `metadata` = error frequency, first/last seen.
4. `update_state("resolved")`: resolve the Sentry issue via API.
5. `create_task()`: no-op (Sentry issues are external-only).

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

- [ ] `fetch_active()` returns Sentry issues with stack traces in description
- [ ] `update_state("resolved")` resolves the issue in Sentry
- [ ] Missing `SENTRY_TOKEN` returns clear error

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_38 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `fetch_active()` returns Sentry issues with stack traces in description
- `update_state("resolved")` resolves the issue in Sentry
- Missing `SENTRY_TOKEN` returns clear error
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_38 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
