+++
id = "bug-e362a1"
kind = "bug"
title = "graph_execution::plan_verify::tests::plan_check_publishes_start_heartbeat_and_finish fails under load (an exit-3 step crosses the heartbeat interval and publishes heartbeats the test counts exactly) and passes alone"
status = "superseded"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/plan-verify"]
created = 2026-10-20
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "a86c648c3"
source = "round-v7-roko-planner#flake"
discovered_from = "agent:roko-planner (round v7, full suites of work/bug-72dc0d and work/plan-058)"
anchors = ["crates/roko-cli/src/graph_execution/plan_verify.rs::plan_check_publishes_start_heartbeat_and_finish"]
links = { depends_on = [], blocks = [], related = ["bug-5c77d6"], supersedes = [], duplicate_of = "bug-8d8f98" }

[closed]
at = 2026-10-26
at_ts = "2026-10-26T12:25:20Z"
commit = "aaa06db5e"
executor = "claude-agent"
via = "manual"
forced = false
evidence = "Duplicate of bug-8d8f98 (filed 2026-10-16): the same test failing the same way, its step 2 (exit 3) publishing \"heartbeat 2\" rows under load. Fixed by aaa06db5e. The test now compares the rows other than heartbeats exactly and checks each heartbeat against the step running when it came, and its step 2 outlasts two heartbeat periods on every run; bug-8d8f98's evidence has the before and after. This item's own done-when (the full roko-cli lib suite three times in a row under heavy.sh at high load) was not run: the single test passed 21 times in a row at load ~40, and the old assertion fails every time once step 2 is delayed."
+++

## Problem

`graph_execution::plan_verify::tests::plan_check_publishes_start_heartbeat_and_finish` failed twice in full
`cargo test -p roko-cli --lib` runs on 2026-10-20 at machine load ~120–180, and passed when run alone in between.

- 16:40, branch work/bug-72dc0d (merged 9f0870e9a): 3921 passed, 2 failed (this test and bug-5c77d6's neighbour
  in delivery). The rerun of both at 9f0870e9a passed.
- 21:30, branch work/plan-058: 3927 passed, 1 failed (this test).

Both failures have the same shape. The test's second step (`exit 3`) published two "heartbeat 2" events before
"finished 2", where the test expects none:

```
left:  [..., "started demo in_place 2/3 exit command cargo-default: exit 3", "heartbeat 2", "heartbeat 2", "finished 2 false Some(3) None"]
right: [..., "started demo in_place 2/3 exit command cargo-default: exit 3", "finished 2 false Some(3) None"]
```

Under load, `exit 3` takes long enough to cross the heartbeat interval. The test counts the exact heartbeats of a
command whose run time it does not control.

## Why it matters

A red roko-cli lib run costs every agent a rerun and a doubt. Neither branch touched plan_verify.rs.

## Where

- `crates/roko-cli/src/graph_execution/plan_verify.rs`: the test (line 349, the event-list assertion).

## Plan

Assert the order (start, at least the first step's heartbeats, finish 1, start 2, finish 2) and allow heartbeats of
step 2, or drive the heartbeat interval from a test clock.

## Done when

The test passes in the full suite three times in a row under heavy.sh at high load.

## Notes

- Evidence: `tmp/hermes-roko-v7/roko-planner/flake-plan-verify-heartbeat.log` (both failures and the passing rerun).
- Related: bug-5c77d6 (another load flake in the roko-cli lib suite).
