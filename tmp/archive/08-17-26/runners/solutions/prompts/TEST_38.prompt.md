# TEST_38: ACP telemetry integration tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-38`](../ISSUE-TRACKER.md#test-38)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.38
- Priority: **P1**
- Effort: 4 hours
- Depends on: `TEST_08` (source 15.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_38 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Extends existing `telemetry_integration.rs`. Verifies that ACP pipeline runs produce correct telemetry events (gate progress, token usage, phase transitions, file changes, cost tracking).

## Exact Changes

1. Test gate progress events: during a pipeline run, verify the client receives gate start, gate pass/fail, gate complete notifications
2. Test token usage events: verify the client receives token usage updates during agent streaming
3. Test phase transition events: verify the client receives events for each pipeline phase
4. Test file change events: verify the client receives file change notifications after agent writes
5. Test cost tracking events: verify the client receives cost updates with running totals
6. Test event ordering: verify events arrive in logical order (gate_start before gate_pass, phase_start before phase_end)

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

- [ ] Every event type is verified
- [ ] Event ordering is correct
- [ ] Token counts are non-negative
- [ ] Events are delivered as JSON-RPC notifications (not responses)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_38 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every event type is verified
- Event ordering is correct
- Token counts are non-negative
- Events are delivered as JSON-RPC notifications (not responses)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_38 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
