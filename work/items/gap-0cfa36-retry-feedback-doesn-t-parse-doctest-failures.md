+++
id = "gap-0cfa36"
kind = "gap"
title = "Retry feedback doesn't parse doctest failures: its failing-tests list and diagnosis name doctest's summary lines, not the failing CHECKs and their values (Undertow phase 2 run 82b7e069)"
status = "open"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "round-v7-roko-ladder#doctest-feedback"
discovered_from = "agent:roko-ladder (round v7, run 82b7e069 analysis)"
anchors = ["crates/roko-cli/src/dispatch/prompt_builder.rs::GateFeedback::from_raw"]
links = { depends_on = [], blocks = [], related = ["bug-59ed90"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn doctest_failures_name_the_failing_checks' crates/roko-cli/ && cargo test -p roko-cli --lib -- doctest_failures_name_the_failing_checks"
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

## Done when

The `[[verify]]` test feeds `from_raw` a doctest failure output and gets the failing CHECKs with their values, not
the summary lines.

## Notes

Filed only (the lead, 2026-10-20, p3).
