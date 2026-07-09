# TEST_16: CLI error handling smoke tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-16`](../ISSUE-TRACKER.md#test-16)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.16
- Priority: **P0**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Addresses AP-UNREACHABLE: the `unreachable!()` in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/config_cmd.rs:197-209` for config MCP/experiments/plugins/secrets dispatch.

## Exact Changes

1. Test `roko run "hello"` without init returns meaningful error (not panic, not raw Rust error)
2. Test `roko plan run nonexistent/` returns "directory not found" or similar error
3. Test `roko config show` without roko.toml returns "not initialized" error
4. Test `roko prd list` without `.roko/prd/` returns empty list (not crash)
5. Test `roko knowledge stats` without `.roko/` returns error or empty
6. Test invalid subcommand returns usage help (exit code 2)
7. Test `roko run` without prompt argument returns argument error
8. Test `roko config mcp list` does NOT panic with `unreachable!()` (AP-UNREACHABLE)

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] 8 tests, all passing
- [ ] No test produces a Rust panic backtrace
- [ ] Every error has a human-readable message
- [ ] Tests complete in < 15 seconds total

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 8 tests, all passing
- No test produces a Rust panic backtrace
- Every error has a human-readable message
- Tests complete in < 15 seconds total
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
