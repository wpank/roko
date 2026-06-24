# TEST_37: ACP session recovery tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-37`](../ISSUE-TRACKER.md#test-37)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.37
- Priority: **P1**
- Effort: 4 hours
- Depends on: `TEST_08` (source 15.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_37 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`AcpSession` at `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs` (line 237). The `TestHarness` pattern from `protocol_conformance.rs` uses `DuplexStream` for in-process testing.

## Exact Changes

1. Test session persistence: create session, add 5 turns of history, save, reload from disk, verify history intact
2. Test session resume after server restart: create session, create new `TestHarness`, load session by ID, verify context preserved
3. Test session cleanup: cancel session, verify resources are released, session ID is no longer loadable
4. Test session listing: create 3 sessions, list, verify all 3 present with correct metadata
5. Test session limit: if a `max_sessions` config exists, verify exceeding it returns appropriate error
6. Test stale session detection: create session, verify that sessions have a TTL or staleness marker

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

- [ ] Session state survives simulated server restarts
- [ ] History is preserved with exact content
- [ ] Cancelled sessions are cleaned up
- [ ] Session listing is consistent

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_37 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Session state survives simulated server restarts
- History is preserved with exact content
- Cancelled sessions are cleaned up
- Session listing is consistent
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_37 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
