# TEST_20: SWE-bench proxy smoke tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-20`](../ISSUE-TRACKER.md#test-20)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.20
- Priority: **P0**
- Effort: 4 hours
- Depends on: `TEST_01` (source 15.1), `TEST_02` (source 15.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Existing SWE-bench proxy harness at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bench.rs` defines `SweBenchOptions`, `SweAgentMode`, `SweBenchReport`. The built-in smoke dataset has 2 tiny tasks. The `SweAgentMode::Gold` path applies gold patches and validates plumbing.

## Exact Changes

1. Test `SweAgentMode::Gold` (plumbing validation): apply gold patch, run tests, verify pass
2. Test `SweAgentMode::Empty` (negative control): apply empty patch, verify fail
3. Test `SweAgentMode::PredictionFile`: write a JSONL predictions file with a valid patch, verify parsing and patch application
4. Test scoring: pass/fail/error counts in `SweBenchReport` are correct
5. Test learning integration: verify episodes are written after bench run (check `.roko/episodes.jsonl` or learn dir)
6. Test batch execution: run 2 tasks, verify both produce `BenchInstanceResult`
7. Test cost tracking: verify `BenchInstanceResult.cost_usd` field is present (even if 0.0 for mock)

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

- [ ] 7 tests, all passing
- [ ] Gold mode passes, Empty mode fails (validates test integrity)
- [ ] JSONL prediction file parsing handles well-formed input

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 7 tests, all passing
- Gold mode passes, Empty mode fails (validates test integrity)
- JSONL prediction file parsing handles well-formed input
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
