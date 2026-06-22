# ACPM_37: Implement GitHub Issues TrackerAdapter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-37`](../ISSUE-TRACKER.md#acpm-37)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.37
- Priority: **P2**
- Effort: 5 hours
- Depends on: `ACPM_36` (source 9.36)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_37 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

GitHub Issues is the most common tracker for Roko's target users. The `roko-mcp-github` crate already has rate-limit-aware HTTP calling logic that can be referenced.

## Exact Changes

1. Implement `GithubTrackerAdapter { owner, repo, token, state_mapping, label_filter }`.
2. `fetch_active()`: use `gh` CLI or GitHub REST API to list open issues with the configured label filter (default: `roko`).
3. `update_state()`: add a comment and optionally close the issue (when state maps to "closed").
4. `create_task()`: create a GitHub issue with labels.
5. Default state mapping: `pending -> open`, `in_progress -> open` (add "in-progress" label), `completed -> closed`, `failed -> open` (add "failed" label).
6. On task completion, post comment: "Completed by Roko. Changes: {summary}".
7. Reuse `roko-mcp-github`'s retry/rate-limit patterns where possible (reference, not dependency -- keep it lightweight).

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

- [ ] `fetch_active()` returns issues from a configured repo
- [ ] `update_state("completed")` closes the issue and adds comment
- [ ] Label-based state tracking works
- [ ] Missing `GITHUB_TOKEN` returns clear error

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_37 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `fetch_active()` returns issues from a configured repo
- `update_state("completed")` closes the issue and adds comment
- Label-based state tracking works
- Missing `GITHUB_TOKEN` returns clear error
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_37 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
