+++
id = "bug-3203c4"
kind = "bug"
title = "a_cancel_stops_the_post_merge_regression_check fails under machine load: the regression checkout's lock is still held when the test checks it (WouldBlock)"
status = "done"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-cli/tests"]
created = 2026-10-16
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "22b895d16"
source = "tmp/hermes-roko-v5/findings/roko-caps.md (roko-caps, 2026-10-16)"
discovered_from = "roko-caps verify run at load ~120"
anchors = ["crates/roko-cli/src/graph_execution/delivery.rs::a_cancel_stops_the_post_merge_regression_check"]
links = { depends_on = [], blocks = [], related = ["bug-7b62ff", "bug-8d8f98"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib a_cancel_stops_the_post_merge_regression_check"

[[verify]]
command = "grep -rqw 'fn the_regression_checkout_comes_free_when_its_holder_lets_go' crates/roko-cli/src && cargo test -p roko-cli --lib -- the_regression_checkout_comes_free_when_its_holder_lets_go"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T08:57:36Z"
commit = "22b895d16"
by = "roko-speed"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-21T03:56:52Z"
forced = false
evidence = "22b895d16. Cause: lock_regression_checkout returned the lock's File, and the delivery let go by dropping it, which only closes this process's descriptor. flock belongs to the open file description, and a process forked while the lock is held shares it until it execs (a gate that enters its own process group forks first, and the test binary's other tests fork too), so under load the lock stayed held past the cancel test's try_lock_exclusive (WouldBlock). Fix: RegressionCheckoutLock wraps the file and unlocks it (fs2 unlock, LOCK_UN) as it drops, before the file closes, so the checkout comes free whatever else shares the file. Test the_regression_checkout_comes_free_when_its_holder_lets_go: a sleep child holds a dup of the lock's file as stdin, the holder drops, and a fresh open of the lock file gets the lock (the old drop leaves it WouldBlock). Chain on main e43dfa53e's tree (main has moved on only in other files): fmt and clippy -D warnings clean; both tests pass; a_cancel_stops_the_post_merge_regression_check passed 20 of 20 more runs from the same binary at load 55-80, and again in the full roko-cli suite at load about 100; lib suites roko-agent 2396, roko-cli 3984 of 3985 (the one failure, plan_check_publishes_start_heartbeat_and_finish, is bug-8d8f98's load flake in a file this leaves alone; it passed alone 3 of 3), roko-core 2047, roko-learn 1393, roko-runtime 213, roko-serve 1081."
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
