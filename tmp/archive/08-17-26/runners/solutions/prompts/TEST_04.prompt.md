# TEST_04: Gate subsystem integration tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#test-04`](../ISSUE-TRACKER.md#test-04)
- Source: `tmp/solutions/roko/tasks/15-TESTING-VERIFICATION.md` — Task 15.4
- Priority: **P0**
- Effort: 6 hours
- Depends on: `TEST_03` (source 15.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: TEST_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

roko-gate has 38 source files with inline `#[cfg(test)]` modules but only 4 integration test files. Key types that need integration-level testing:

- `GateService` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_service.rs` (line 26) -- the main entry point, method `run_gates(config) -> GateReport`
- `ComposedGatePipeline` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_pipeline.rs` (line 328) -- sequential/parallel/voting/fallback composition
- `ParallelGate` / `VotingGate` / `FallbackGate` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/composition.rs` (lines 22, 152, 284)
- `BenchmarkRegressionGate` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/benchmark_gate.rs` (line 30) -- currently a stub that always passes
- `GateFeedback` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/feedback.rs` (line 53) -- structured error/warning/suggestion extraction
- `feedback_for_agent()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/feedback.rs` (line 202)

## Exact Changes

1. Test `GateService` with a passing `GateTestProject`:
   - Enable `["compile"]` -- verify 1 verdict, passed=true
   - Enable `["compile", "clippy"]` -- verify 2 verdicts, both passed
   - Enable `["compile", "clippy", "test"]` -- verify 3 verdicts, all passed
2. Test `GateService` with a broken project:
   - `break_compile()` + enable `["compile"]` -- verify passed=false, output contains "error"
   - `break_clippy()` + enable `["compile", "clippy"]` -- compile passes, clippy fails
   - `break_test()` + enable `["compile", "clippy", "test"]` -- compile+clippy pass, test fails
3. Test `GateService` with `["shell"]` and custom `ShellGateCommand`:
   - `program: "echo"`, `args: ["ok"]` -- passes
   - `program: "false"` -- fails with exit code 1
4. Test `GateService` with `["diff"]` -- verify DiffGate runs (needs git repo in project)
5. Test `GateService` ordering: rungs 0, 1, 2 execute in order; failure on rung 0 prevents rung 1
6. Test `GateReport::all_passed()` is true only when all verdicts pass
7. Test `GateReport::all_passed()` is false when any verdict fails
8. Test `ParallelGate` with 3 mock shell gates (all `true`) -- runs concurrently, minimum score
9. Test `VotingGate` with 3 mock gates: 2 pass, 1 fail -- passes at threshold 2/3
10. Test `VotingGate` with 3 mock gates: 1 pass, 2 fail -- fails at threshold 2/3
11. Test `FallbackGate` with primary=`false`, fallback=`true` -- tries primary, falls back
12. Test `ComposedGatePipeline` in Sequential mode with 2 gates
13. Test `ComposedGatePipeline` in Parallel mode with 2 gates
14. Test `ComposedGatePipeline` in Voting mode with 3 gates
15. Test `ComposedGatePipeline` in Fallback mode with 2 gates
16. Test `BenchmarkRegressionGate` stub behavior -- currently always passes, verify
17. Test `feedback_for_agent()` with compile error output -- verify structured `GateFeedback`

## Write Scope

- `crates/roko-gate/Cargo.toml`

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

- [ ] 17+ new tests, all passing
- [ ] Every gate type in `crates/roko-gate/src/` has at least one test
- [ ] Composition modes all tested

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: TEST_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- 17+ new tests, all passing
- Every gate type in `crates/roko-gate/src/` has at least one test
- Composition modes all tested
- No files outside the Write Scope are modified.
- Commit message contains `tracker: TEST_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
