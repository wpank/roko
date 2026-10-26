+++
id = "gap-0cfa36"
kind = "gap"
title = "Retry feedback doesn't parse doctest failures: its failing-tests list and diagnosis name doctest's summary lines, not the failing CHECKs and their values (Undertow phase 2 run 82b7e069)"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-20
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "3dbd54d9e"
source = "round-v7-roko-ladder#doctest-feedback"
discovered_from = "agent:roko-ladder (round v7, run 82b7e069 analysis)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::GateFeedback::from_raw"]
links = { depends_on = [], blocks = [], related = ["bug-59ed90"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn doctest_failures_name_the_failing_checks' crates/roko-cli/ && cargo test -p roko-cli --lib -- doctest_failures_name_the_failing_checks"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T15:42:29Z"
commit = "3dbd54d9e"
by = "roko-exact (round v7)"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-26T14:54:44Z"
forced = false
evidence = "3dbd54d9e on top of bug-3e3d08 (b7b13f77f, which lists TEST CASE: and each failed C++ check once and never a summary line): from_raw appends what a failed check saw, doctest's values: line or Catch2's check with its with expansion: line (observed_values), entries up to 400 chars. Verify: doctest_failures_name_the_failing_checks passes (phase 2's shape: [TEST CASE: P2.04 ..., output_test.cpp:125: ERROR: CHECK( low >= 0.98 ) is NOT correct! (values: CHECK( 0.975951 >= 0.98 ))], no [doctest] line; Catch2 gives the REQUIRE and its expansion); dispatch::prompt_builder, retry_feedback and graph_task_dispatch::verification 97/97; nightly fmt and clippy -p roko-cli -D warnings clean."
+++

## Problem

The retry feedback's parsed failures don't read doctest output. For a failing doctest step, `GateFeedback`'s
`errors`/"Failing tests" list holds only the summary lines, for example "[doctest] test cases: 2 | 1 passed | 1
failed" and "[doctest] assertions: 19 | 15 passed | 4 failed", and the diagnosis names the summary as "the primary
error". The lines that say what failed are left out:

    output_test.cpp:125: ERROR: CHECK( low >= 0.98 ) is NOT correct!
      values: CHECK( 0.975951 >= 0.98 )

Undertow phase 2 (run `82b7e069`, the `retry_feedback` events of T2 and T3) is an example.

## Why it matters

The raw-output excerpt (bug-59ed90) still carries them when they fit in its 2 KB, as they did here. But the
structured list and the diagnosis, which lead the retry prompt and the dashboard, point at a summary. A longer output,
or one with build noise first, would push the CHECKs out of the excerpt. Undertow's C++ checks all run doctest
(`tools/spec.sh`).

## Where

`crates/roko-cli/src/dispatch/prompt_builder.rs::GateFeedback::from_raw`, which parses test, compile and warning
lines out of a step's output.

## Current state

`from_raw` knows cargo/rustc-style lines and some test runners. It doesn't know doctest's `ERROR: CHECK(...) is NOT
correct!` and `values:` pairs.

## Plan

Parse doctest (and Catch2's similar) failure blocks into `test_failures`: the test case name, the file:line, the
failing expression and its `values:` line. Name the first one in the diagnosis instead of the summary line.

## Design

roko-exact, 2026-10-26, after bug-3e3d08:
- bug-3e3d08 made `from_raw` list a doctest `TEST CASE:` and each C++ failed check once, never a runner's summary,
  and its fallback diagnosis now names each failing case.
- This item adds what each check saw. A failed check is listed with the line after it: doctest's
  `values: CHECK( … )`, or Catch2's check and its `with expansion:` line (`observed_values`). The entry is capped at
  400 characters, so a long path doesn't cut the values off.

## Done when

The `[[verify]]` test feeds `from_raw` a doctest failure output and gets the failing CHECKs with their values, not
the summary lines.

## Notes

Filed only (the lead, 2026-10-20, p3).
