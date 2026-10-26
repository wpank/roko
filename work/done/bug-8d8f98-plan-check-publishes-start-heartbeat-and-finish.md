+++
id = "bug-8d8f98"
kind = "bug"
title = "plan_check_publishes_start_heartbeat_and_finish fails under machine load: a 40 ms heartbeat can fire during the exit-3 step, which the test allows none"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-10-16
updated = 2026-10-26
last_verified = 2026-10-26
last_verified_rev = "aaa06db5e"
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16)"
discovered_from = "roko-caps verify run at load ~120"
anchors = ["crates/roko-cli/src/graph_execution/plan_verify.rs::plan_check_publishes_start_heartbeat_and_finish"]
links = { depends_on = [], blocks = [], related = ["gap-6b6ec1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib plan_check_publishes_start_heartbeat_and_finish"

[closed]
at = 2026-10-26
at_ts = "2026-10-26T12:24:15Z"
commit = "aaa06db5e"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-26T10:26:22Z"
forced = false
evidence = "aaa06db5e (on main 15335e2f9, through the bug-2a31bc and bug-a66fef branches): graph_execution::plan_verify::tests::plan_check_publishes_start_heartbeat_and_finish no longer counts heartbeats exactly. It compares the rows other than heartbeats exactly: started 1, finished 1 true Some(0), started 2, finished 2 false Some(3). A pass over the events then asserts that each heartbeat comes between its own step's start and end, and that a step starts only after the one before it finished. Step 1 (sleep 0.4, ten 40 ms periods) must still get at least one heartbeat, and elapsed_ms > 0 and the never-run third step are still checked. Step 2 is now `exit $(sleep 0.1; echo 3)`: still named `exit`, still exit code 3. So every run checks a failing step's heartbeats (normally two), the case a loaded machine hit. Before: with only step 2 delayed that way, the old exact comparison failed, its rows showing \"heartbeat 2\", \"heartbeat 2\" before \"finished 2\", the shape this item and bug-e362a1 report. After: cargo test -p roko-cli --lib plan_check_publishes_start_heartbeat_and_finish passes, then 20 of 20 more runs in a row at load average ~38-42, and in env -i with a scratch HOME, CI=true and no keys. cargo +nightly fmt --all --check is clean, and so is cargo clippy -p roko-cli --no-deps -- -D warnings. Clippy over the lib's test cfg (--profile test, 79 diagnostics in other files) reports nothing in plan_verify.rs."
+++

## Problem

`graph_execution::plan_verify::tests::plan_check_publishes_start_heartbeat_and_finish` fails on a loaded machine.
The check publishes a heartbeat every 40 ms (`with_heartbeat_every`). The test accepts any number of heartbeats
for step 1 (`sleep 0.4`), but none for step 2 (`exit 3`). When the machine is loaded, starting a shell for
`exit 3` takes longer than 40 ms, so step 2 publishes a heartbeat as well, and the exact row comparison fails.

Seen 2026-10-16 in roko-caps' `cargo test -p roko-cli --lib` run at load average ~120 (3707 passed, this one
failed). Run alone right afterwards from the same test binary, it passed 3 times out of 3. It also passed in the
same agent's earlier full run that day.

Seen again 2026-10-17 (round v6) in roko-caps' `cargo test -p roko-cli --lib` runs. This time it also failed when
run alone, while the machine stayed loaded (all three heavy.sh cargo slots busy). So rerunning it on a busy machine
does not clear it. It passes once the load drops.

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
