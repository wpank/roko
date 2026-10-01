+++
id = "bug-f1f814"
kind = "bug"
title = "graph_task_dispatch's fake provider CLIs use a 5 s fixture timeout, so 5 to 51 tests per loop run time out under load"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "81d9379a2"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-settle's report, checked on work/bug-dfb28f-flake at d7aaafc48)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs", "crates/roko-cli/src/graph_task_dispatch/failover.rs", "crates/roko-cli/src/graph_task_dispatch/helper_calls.rs", "crates/roko-cli/src/graph_task_dispatch/served_model.rs"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["bug-779ae7", "bug-ea9959"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'timeout_ms: Some(5_000)' crates/roko-cli/src/graph_task_dispatch.rs"

[[verify]]
command = "grep -q 'Some(FIXTURE_PROVIDER_TIMEOUT_MS)' crates/roko-cli/src/graph_task_dispatch/failover.rs && grep -q 'Some(FIXTURE_PROVIDER_TIMEOUT_MS)' crates/roko-cli/src/graph_task_dispatch/helper_calls.rs && grep -q 'Some(FIXTURE_PROVIDER_TIMEOUT_MS)' crates/roko-cli/src/graph_task_dispatch/served_model.rs && ! grep -qF -e 'timeout_ms: Some(5_000)' -e 'timeout_ms: Some(15_000)' crates/roko-cli/src/graph_task_dispatch/failover.rs crates/roko-cli/src/graph_task_dispatch/helper_calls.rs crates/roko-cli/src/graph_task_dispatch/served_model.rs"
+++

## Problem

The graph_task_dispatch tests run fake provider CLIs configured with `timeout_ms: Some(5_000)`, `ttft_timeout_ms: Some(5_000)` and `connect_timeout_ms: Some(5_000)` (`crates/roko-cli/src/graph_task_dispatch.rs:1581-1583`, :1779-1780 on the settle branch). Under load (a parallel workspace test run, or other agents building), spawning the fake CLI takes longer than 5 s, and 5 to 51 of these tests fail per loop run with "timed out after 5000 ms" (wk-settle).

## Why it matters

Hygiene (epic spec-9a3131): a broad source of the flakes bug-779ae7 tracks one by one, and a reason CI and batch gates fail without a real defect.

## Where

The fake-provider fixtures in `graph_task_dispatch.rs`'s tests.

## Plan

1. Put the fixtures' timeouts in one constant with a load-tolerant value (for example 60 s), or remove them where the test doesn't exercise timeouts. Keep short timeouts only in the tests that test timeouts.
2. Run the module's tests in a loop under load to confirm.

## Done when

- [ ] The fixtures don't time out under load, and the module passes a 20-iteration loop under load.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-childenv): Implemented on `work/bug-f1f814` at `f1db380c6`; cargo verification deferred to the batch
  check, including the 20-iteration loop under load. The three fixtures in `graph_task_dispatch.rs` (`graph-cli`,
  `stream-cli` and `cli_provider`, which the submodule tests share) use `FIXTURE_PROVIDER_TIMEOUT_MS`, the existing
  `FIXTURE_HANG_GUARD_SECS` (120 s) in ms. No test relies on the 5 s limit: the timeout tests stop the provider
  with shorter attempt limits (`TIMEOUT_SECS_UNDER_LOAD`, at most 24 s). Not changed: submodule fixtures in
  `failover.rs`, `helper_calls.rs` and `served_model.rs` keep 15 s provider limits, and the watchdog fixture keeps 120 s.
- 2026-10-01 (wk-childenv): Extended at `81d9379a2` (on top of batch 20b, `7b7f92bf0`), because at load 110-140
  15 s is also within flake range. The five provider sites in the `failover.rs`, `helper_calls.rs` and
  `served_model.rs` fixtures (fake CLI, OpenAI-compatible mocks and never-called shadows) went from 15 s, with 5 s
  to connect, to `FIXTURE_PROVIDER_TIMEOUT_MS`. Their tasks' 30 s attempt limits would have capped that, so the six
  attempt-limit sites (`failover_cell`'s task and cell config, `fail_over_from_an_exhausted_cli`, and three
  `served_model.rs` tests) went to `FIXTURE_HANG_GUARD_SECS`. None of these tests tests a time limit. The watchdog
  fixture is unchanged (120 s, and 5 s to connect), because its tests test stalls. A second `[[verify]]` covers the
  three files. Cargo verification is deferred to the batch check, as for the first change.
