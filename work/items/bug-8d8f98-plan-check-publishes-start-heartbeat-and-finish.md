+++
id = "bug-8d8f98"
kind = "bug"
title = "plan_check_publishes_start_heartbeat_and_finish fails under machine load: a 40 ms heartbeat can fire during the exit-3 step, which the test allows none"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16)"
discovered_from = "roko-caps verify run at load ~120"
anchors = ["crates/roko-cli/src/graph_execution/plan_verify.rs::plan_check_publishes_start_heartbeat_and_finish"]
links = { depends_on = [], blocks = [], related = ["gap-6b6ec1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib plan_check_publishes_start_heartbeat_and_finish"
+++

## Problem

`graph_execution::plan_verify::tests::plan_check_publishes_start_heartbeat_and_finish` fails on a loaded machine.
The check publishes a heartbeat every 40 ms (`with_heartbeat_every`). The test accepts any number of heartbeats
for step 1 (`sleep 0.4`), but none for step 2 (`exit 3`). When the machine is loaded, starting a shell for
`exit 3` takes longer than 40 ms, so step 2 publishes a heartbeat as well, and the exact row comparison fails.

Seen 2026-10-16 in roko-caps' `cargo test -p roko-cli --lib` run at load average ~120 (3707 passed, this one
failed). Run alone right afterwards from the same test binary, it passed 3 times out of 3. It also passed in the
same agent's earlier full run that day.

## Why it matters

The pre-commit `cargo test --workspace` fails for no code reason when the machine is busy, as it is in parallel
agent rounds.

## Where

- `crates/roko-cli/src/graph_execution/plan_verify.rs::plan_check_publishes_start_heartbeat_and_finish`

## Plan

Let any step's heartbeats through, the way step 1's are now. One way: compare the rows without heartbeats exactly,
and check that each heartbeat names a step that was running at the time.

## Done when

The test passes with step 2 delayed past the heartbeat interval (for example, `sleep 0.1; exit 3`), as well as under
load.
