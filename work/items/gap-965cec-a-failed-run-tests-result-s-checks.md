+++
id = "gap-965cec"
kind = "gap"
title = "A failed run_tests result's checks[] lists only the checks that passed before the failing one; the failing check is named only in the summary's prose, so a reader judging by checks sees all green"
status = "open"
triage = "verified"
severity = "p3"
goal = "truth"
size = "S"
subsystem = ["roko-std/task_checks"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "round-v7 run-analyst (tmp/hermes-roko-v7/findings/run-analyst.md, §11 and the telemetry gaps T13-T16), the lead's ask"
discovered_from = "agent:run-analyst (round v7)"
anchors = ["crates/roko-std/src/tool/builtin/task_checks.rs::run", "crates/roko-std/src/tool/builtin/task_checks.rs::checks_json"]
links = { depends_on = [], blocks = [], related = ["gap-ece700"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_failed_run_lists_its_failing_check' crates/roko-std/src && cargo test -p roko-std a_failed_run_lists_its_failing_check"
+++

## Problem

When a check fails, `run_tests` answers with `status: failed` and a `summary` such as "check 2 of 5
(verify[1:test]) failed (exit 1) after 8.2 s". Its `checks` array then lists only the checks that passed before the
failing one; the failing check is missing from it.

A reader that judges the run by `checks` alone sees all passed. The run-analyst's extractor counted two of phase
4.3 T2#3's red runs as green until it checked `status` (`tmp/hermes-roko-v7/findings/run-analyst.md`, telemetry
gap T15).

## Why it matters

- The tool result is how a try's own test runs are recorded in the event stream. The failing check's name and time
  survive only as prose in `summary`.
- Goal: truth.

## Where

`crates/roko-std/src/tool/builtin/task_checks.rs::run`, in the `ChecksOutcome::Failed` branch. It sends
`"checks": checks_json(&passed)`. `checks_json` is in the same file.

## Plan

1. List the failing check in `checks` too, with `passed: false`, its exit and its seconds.
2. Then list the checks it never ran, with `skipped: true`, as the verdict's steps do.
3. Keep `status` and `summary` as they are.
4. Test: `a_failed_run_lists_its_failing_check`.

## Done when

The `[[verify]]` passes.
