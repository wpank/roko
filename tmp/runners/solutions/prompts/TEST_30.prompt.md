# TEST_30: Agent crash recovery tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-30`](../ISSUE-TRACKER.md#test-30)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.30
- Priority: **P1**
- Effort: 4 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. Test agent SIGKILL: spawn a mock agent (shell script that sleeps), send SIGKILL, verify the runner detects crash and records a failure episode
2. Test agent timeout: configure 5-second timeout, spawn agent that sleeps 10 seconds, verify timeout fires and process is killed
3. Test agent stderr output: mock agent writes to stderr, verify error output is captured in the failure context
4. Test agent invalid output: mock agent produces malformed JSON (not valid stream-json), verify parser handles it without panic
5. Test agent exit code: mock agent exits with code 1, verify appropriate error message in output
6. Test partial output recovery: agent produces valid output then crashes mid-stream, verify partial output is preserved where possible

## Design Guidance

Use shell scripts as mock agents: `#!/bin/sh\nsleep 100` for timeout tests, `#!/bin/sh\nexit 1` for failure tests, `#!/bin/sh\necho 'not json'` for malformed output tests. Configure the mock agent via `roko.toml` `[agent] command = "/path/to/mock.sh"`.

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

- [ ] No panic in any crash scenario
- [ ] All crashes produce failure context (not silent swallow)
- [ ] Timeout is enforced (process killed, not hung)
- [ ] Partial output preserved when possible

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- No panic in any crash scenario
- All crashes produce failure context (not silent swallow)
- Timeout is enforced (process killed, not hung)
- Partial output preserved when possible
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
