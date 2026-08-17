# TEST_11: Orchestrator DAG and executor tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-11`](../ISSUE-TRACKER.md#test-11)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.11
- Priority: **P1**
- Effort: 3 hours
- Depends on: `TEST_01` (source 15.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_11 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Key types:
- `ParallelExecutor` at `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/executor/mod.rs` (line 241)
- DAG module at `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/dag.rs` (2,557 LOC)
- TOML fence stripping at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs` (line 964, `extract_toml_payload`)

Only 1 existing integration test file (`lifecycle.rs`). Missing: DAG construction, topological ordering, cycle detection, parallel readiness, plan validation, state persistence.

## Exact Changes

1. Test DAG construction from a 5-task TOML with dependencies
2. Test topological ordering: tasks with no dependencies come first
3. Test parallel readiness: tasks A and B (no deps) are both ready; task C (depends on A) is not ready until A completes
4. Test cycle detection: circular dependencies (A->B->C->A) produce error
5. Test plan validation: missing required fields (`id`, `title`) produce specific errors
6. Test TOML fence stripping via `extract_toml_payload()`: input with markdown code fences (` ```toml ... ``` `) strips fences correctly
7. Test state persistence: executor saves progress to JSON, reloads, resumes from correct task
8. Test resume fingerprint: modified tasks.toml after snapshot produces fingerprint mismatch

## Write Scope

- `crates/roko-orchestrator/Cargo.toml`

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

- [ ] 8+ new tests, all passing
- [ ] DAG ordering, parallelism, cycle detection all covered
- [ ] State persistence roundtrip verified

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_11 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 8+ new tests, all passing
- DAG ordering, parallelism, cycle detection all covered
- State persistence roundtrip verified
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_11 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
