+++
id = "bug-f831b3"
kind = "bug"
title = "run_tests reads forge's summary as unittest's, so a forge run counts its suite line as a passed test and never counts a failure; pytest and go results carry no counts"
status = "done"
triage = "verified"
severity = "p3"
size = "S"
subsystem = ["roko-std/tools"]
created = 2026-10-21
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "bc99049a0"
source = "bug-83d420 fix (2026-10-21)"
discovered_from = "bug-83d420"
anchors = ["crates/roko-std/src/tool/builtin/run_tests.rs::read_counts"]
links = { depends_on = [], blocks = [], related = ["bug-83d420", "bug-1f8db7"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn forge_pytest_and_go_summaries_are_read' crates/roko-std/src && cargo test -p roko-std --lib forge_pytest_and_go_summaries_are_read"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T14:28:47Z"
commit = "bc99049a0"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-26T10:26:20Z"
forced = false
evidence = "crates/roko-std/src/tool/builtin/run_tests.rs: read_counts now chains forge_counts, pytest_counts and go_counts (new), and unittest_counts uses a new unittest_ran helper that requires `test`/`tests` then `in` after the number, so forge's `Ran N tests for …`/`Ran N test suite(s) in …` lines no longer count as unittest passes. New test forge_pytest_and_go_summaries_are_read covers a failing forge suite (1 passed/1 failed, not 3/0), three forge suites summing to 6 not 9, a pytest `N passed, N failed` line, and go's `--- PASS:`/`--- FAIL:`/`--- SKIP:` lines (top-level only, non-verbose runs left uncounted). `grep -rqw 'fn forge_pytest_and_go_summaries_are_read' crates/roko-std/src && cargo test -p roko-std --lib forge_pytest_and_go_summaries_are_read` passes at bc99049a0 (1 passed; 0 failed). `cargo +nightly fmt --all` and `cargo clippy --workspace --no-deps -- -D warnings` both clean."
+++

## Problem

`run_tests` reads a run's counts with `read_counts`, which tries cargo's summary (`test result: …`), then Node's
(`# pass N`), then `unittest`'s (`Ran N tests …`, `FAILED (failures=N)`). forge prints none of the first two, and two
of its own lines match the third:

```
Ran 2 tests for test/Counter.t.sol:CounterTest
[FAIL: assertion failed] test_Increment() (gas: 0)
[PASS] testFuzz_SetNumber(uint256) (runs: 256, μ: 30454, ~: 31288)
Suite result: FAILED. 1 passed; 1 failed; 0 skipped; finished in 7.62ms (7.13ms CPU time)

Ran 1 test suite in 9.89ms (7.62ms CPU time): 1 tests passed, 1 failed, 0 skipped (2 total tests)
```

`unittest_counts` takes `Ran 2 tests for …` and `Ran 1 test suite in …` as two `unittest` runs and adds them up, so
this run reports `passed: 3, failed: 0` beside `status: "failed"`: the suite line counts as a passed test, and no
forge failure is ever counted (forge has no `FAILED (failures=N)` line). With three suites of two tests each it
reports 9 passed instead of 6.

pytest (`===== 3 passed, 1 failed in 0.12s =====`) and `go test` (`--- FAIL: TestX`, `ok  example.com/m 0.002s`)
match none of the three readers, so their results carry no counts at all.

## Why it matters

An agent reads `passed`/`failed` to see whether its fix worked. A forge result that says `failed: 0` while
`status` is `failed` sends it looking in the wrong place; a missing count is less harmful but leaves the agent to
read the tail itself.

## Where

- `crates/roko-std/src/tool/builtin/run_tests.rs::read_counts` and `unittest_counts`: the readers, tried in order,
  for whichever runner printed the output.
- `outcome`: puts the counts in the result.

## Current state

bug-1f8db7 added the Node and `unittest` readers; bug-83d420 made each runner take its filter its own way. Nothing
reads forge's `Suite result: ok. N passed; N failed; N skipped` lines, pytest's `N passed, N failed` line or go's
`--- PASS`/`--- FAIL` lines.

## Plan

1. Read the counts with the runner that ran (`outcome` knows `build`), falling back to the current order only for
   `make` and `checks`, whose commands may run anything.
2. forge: sum `Suite result: … N passed; N failed; N skipped` lines; do not let `unittest_counts` read `Ran N tests
   for` or `Ran N test suite(s) in`.
3. pytest: read the final `N passed, N failed, N skipped, N error(s)` line.
4. go: count `--- PASS:`, `--- FAIL:` and `--- SKIP:` lines (top-level tests), or leave go without counts and say so.

## Done when

- [ ] The `[[verify]]` test passes: forge's, pytest's and go's summaries above read as their real counts, and a
  failing forge run reports its failure.

## Notes

- Found while fixing bug-83d420 (reading `read_counts` beside the forge filter); forge's output is the format
  Foundry prints since 2024 (`Ran N tests for <file>:<contract>`, `Suite result: …`, `Ran N test suites in …`).
