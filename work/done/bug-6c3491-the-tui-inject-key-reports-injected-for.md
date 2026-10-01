+++
id = "bug-6c3491"
kind = "bug"
title = "The TUI inject key reports Injected for a directive nothing reads"
status = "done"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "d1e3c5681"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-f118b3"
anchors = ["crates/roko-cli/src/tui/app/actions.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-f118b3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib inject_key"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T19:41:14Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T17:36:56Z"
forced = false
evidence = "Gate 6c on 4bacc9d88 and its fix-up 060b62813, merged as d1e3c5681 (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 14 crates; lib tests pass (roko-cli 3395, roko-agent 2269, roko-core 1977, roko-learn 1225, roko-serve 1010, roko-gate 697, roko-compose 562, roko-graph 487, roko-dreams 260, roko-execution 245, roko-neuro 239, roko-acp 211, roko-gateway 101); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, merge_proof, plan_conversion, job_lifecycle and job_runner_integration tests pass; bin 445; Cargo.lock unchanged; route inventory tests and --check-snapshot pass. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

The TUI's `i` key appends a `roko.inject.directive` signal to `.roko/signals.jsonl` and shows an "Injected" toast, but nothing reads that kind. This is the same false success that gap-f118b3 removed from `roko inject`.

## Plan

Fail closed as `roko inject` now does: show that inject is unavailable, and write nothing until the acknowledged transport (gap-f118b3) exists. Add a test named `inject_key_*`.

## Done when

- `cargo test -p roko-cli --lib inject_key` passes.

## Notes

- Reported on 2026-10-01 by the worker on gap-f118b3, during the evening close-out round.
- 2026-10-01 (wk-childenv): implemented on work/gap-1555ac; cargo verification deferred to the batch check.
  The `i` key (`TuiAction::StartInject`, tui/app/actions.rs) shows a warning ("inject is not available: no live
  command transport reaches the run yet") instead of opening inject mode, and `SubmitInject` writes nothing to
  `.roko/signals.jsonl`, clears the input and shows the same warning. The help modal and docs/v2 say inject is not
  available yet. Test: `inject_key_fails_closed_and_writes_nothing`.
