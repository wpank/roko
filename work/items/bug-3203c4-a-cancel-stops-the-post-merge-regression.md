+++
id = "bug-3203c4"
kind = "bug"
title = "a_cancel_stops_the_post_merge_regression_check fails under machine load: the regression checkout's lock is still held when the test checks it (WouldBlock)"
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
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::a_cancel_stops_the_post_merge_regression_check"]
links = { depends_on = [], blocks = [], related = ["bug-7b62ff", "bug-8d8f98"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib a_cancel_stops_the_post_merge_regression_check"
+++

## Problem

`graph_execution::delivery::tests::a_cancel_stops_the_post_merge_regression_check` fails when the machine is heavily
loaded. A cancel stops the post-merge regression check. The test waits for the check's processes to exit, then
asserts that the regression checkout's lock file is already free (`try_lock_exclusive`). Under load the lock is still
held at that moment, and the assertion panics: "the regression checkout is free: … WouldBlock (Resource temporarily
unavailable)".

Seen 2026-10-16 in roko-caps' `cargo test -p roko-cli --lib` run at main 5e32264ce merged with bug-a877bf and
gap-130413, at a load average around 120 (3726 passed, this one failed). Run alone right afterwards from the same
test binary, it passed 3 times out of 3.

## Why it matters

The pre-commit `cargo test --workspace` fails for no code reason when the machine is busy, as it is in parallel
agent rounds.

## Where

- `crates/roko-cli/src/graph_execution/delivery.rs::a_cancel_stops_the_post_merge_regression_check`
- What releases the regression checkout's lock after a stopped check.

## Plan

Find what still holds the lock after the check's processes exit: a drop on another task, or a child that outlives
them. Either release it before the outcome is returned, or have the test wait, with a bound, for the lock to come
free. Prefer the first if the code path can hand the lock back late in production too.

## Done when

The test passes under load (run it with the machine busy, for example during another `cargo build`), as well as
alone.
