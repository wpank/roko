# ACPM_40: Wire TrackerAdapter into Plan Execution

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-40`](../ISSUE-TRACKER.md#acpm-40)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.40
- Priority: **P2**
- Effort: 4 hours
- Depends on: `ACPM_36` (source 9.36), `ACPM_37` (source 9.37)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_40 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

When plan execution completes a task, the external tracker should be updated. Configuration lives in `roko.toml`.

## Exact Changes

1. Add `[tracker]` section to config schema:
   ```toml
   [tracker]
   kind = "github"  # or "linear", "sentry", "none"
   auto_sync = true

   [tracker.github]
   owner = "org"
   repo = "repo"
   label_filter = "roko"
   ```
2. Parse `TrackerConfig` in the config loader.
3. In the ACP runner, after each pipeline completes successfully, call `adapter.update_state()` with the completion state and a summary comment.
4. Add `--from-tracker` flag support: call `adapter.fetch_active()` to populate tasks from external issues.
5. When `auto_sync = false`, skip automatic updates.
6. When `kind = "none"` or config section absent, construct a no-op adapter.

## Write Scope

- `crates/roko-core/src/config/schema.rs`
- `crates/roko-acp/src/runner.rs`

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

- [ ] Task completion updates GitHub issue with comment
- [ ] `auto_sync = false` disables automatic updates
- [ ] Missing tracker config gracefully falls back to no-op

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_40 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Task completion updates GitHub issue with comment
- `auto_sync = false` disables automatic updates
- Missing tracker config gracefully falls back to no-op
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_40 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
