+++
id = "bug-d85634"
kind = "bug"
title = "every_immune_denial_logs_its_reason fails whenever the immune_boundary tests run on their own at 4 threads: tracing-core caches Interest::never for the denial log line when another thread registers it while the test's scoped subscriber is the only dispatcher"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-agent/safety"]
created = 2026-10-26
updated = 2026-10-26
last_verified = 2026-10-26
source = "round v7 roko-effort, gap-1ba0ac's unit proof and the main baseline the lead asked for (2026-10-26)"
discovered_from = "agent:roko-effort (round v7, gap-1ba0ac)"
anchors = ["crates/roko-agent/src/immune_boundary.rs::every_immune_denial_logs_its_reason", "crates/roko-agent/src/immune_boundary.rs::denied_result"]
links = { depends_on = [], blocks = [], related = ["gap-1ba0ac"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn every_immune_denial_logs_its_reason' crates/roko-agent/src && for i in 1 2 3; do cargo test -p roko-agent --lib -- immune_boundary:: --test-threads=4 || exit 1; done"
+++

## Problem

`immune_boundary::tests::every_immune_denial_logs_its_reason` fails every time the `immune_boundary` module's tests
run on their own at 4 threads:

    cargo test -p roko-agent --lib -- immune_boundary:: --test-threads=4

It panics at `crates/roko-agent/src/immune_boundary.rs:2259` (main f50dc58fa) with `[]`: the test's subscriber
captured no log line at all, although the denial it checks happened (the result and `Done` assertions before it
pass).

- Main f50dc58fa: failed 6 of 6 runs this way, and passed 3 of 3 alone (round v7 roko-effort, 2026-10-26, the lead's
  ask: a detached checkout of f50dc58fa built with the same `-p` set).
- gap-1ba0ac's branch: failed 7 of 7 runs the same way. That change touches neither the test nor the denial path.
- The test passes alone (3 of 3 on each tree), with its module at `--test-threads=1` (3 of 3 on the branch), and in
  the full `cargo test -p roko-agent --lib` run (bug-29014c's suites, 2026-10-26), presumably because other
  modules' tests register the log callsite before this test installs its subscriber.

## Why it matters

A run of the module's tests, which an agent makes after touching `immune_boundary.rs`, reports a failure that isn't
there. A real regression in the denial log line could hide behind it. The full suite passes by luck of ordering.

## Where

- `crates/roko-agent/src/immune_boundary.rs::every_immune_denial_logs_its_reason`. It installs a thread-scoped
  subscriber with `tracing::subscriber::set_default(CapturedEvents)`.
- `ImmuneScreenedAgent::denied_result` (the same file). Its `tracing::warn!("provider result denied by immune
  boundary")` is the line the test looks for. Several sibling tests reach it too: the stream-limit, isolation and
  containment denials.

## Current state

The cause is tracing-core's callsite interest cache (tracing-core 0.1.36, `src/callsite.rs`, `mod dispatchers`).

- `register_dispatch` sets `has_just_one = dispatchers.len() <= 1`. With no global subscriber, the test's scoped
  `CapturedEvents` is the only registered dispatcher, so `has_just_one` becomes true.
- While it is true, a callsite registered for the first time uses `Rebuilder::JustOne`, which asks only the
  *registering thread's* default. A sibling test on another thread has no subscriber (`NoSubscriber`), so the
  callsite caches `Interest::never`.
- When a sibling test's denial registers the `warn!` callsite during this test's run, every later event at that
  callsite is skipped on all threads, and the test captures nothing.
- Running alone or serially, the test registers the callsite itself, under its own subscriber. In the full suite,
  some earlier test usually registers it first, before the scoped subscriber exists, and `register_dispatch`'s
  rebuild then recomputes it. So it passes there.

## Plan

Make the test's capture independent of which thread first reaches a callsite. Options:

1. **A global test dispatcher.** Install one `set_global_default` subscriber for the test binary whose
   `register_callsite` returns `Interest::sometimes()` and whose `enabled` defers to the thread's scoped capture. A
   once-guarded helper beside `CapturedEvents` would do. Two registered dispatchers keep `has_just_one` false, so
   every registration asks all of them. This is the robust fix.
2. **`tracing::callsite::rebuild_interest_cache()` right after `set_default`, and again just before the run.** This
   is cheaper, but a sibling can still register the callsite between the rebuild and the event. It narrows the window
   without closing it.
3. **Serialize the tests that install a subscriber** with a module-level mutex. This only works if every test that
   can register the callsite takes the mutex too, so it is not enough alone.

Option 1 is recommended. Keep the log assertion: the line is what an operator greps for (backlog 1103).

## Done when

The `[[verify]]` passes: the module's tests on their own at 4 threads, three runs in a row. Today it fails on the
first run.

## Notes

- Found while proving gap-1ba0ac, which doesn't touch this test or the denial path.
- Other tests that capture logs with a scoped subscriber have the same exposure when their test binary runs other
  tests in parallel:
  - `roko-cli/src/graph_execution/plan_runner.rs` (`with_default`, ~9462);
  - `roko-cli/src/main_tests.rs` (~3608);
  - `roko-agent/src/harness/acp_client.rs` (~1988, `EnableAll` around one call).
  Option 1 covers them all if it lives in a shared test helper.
