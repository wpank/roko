+++
id = "bug-6c3491"
kind = "bug"
title = "The TUI inject key reports Injected for a directive nothing reads"
status = "open"
triage = "verified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "4309fc101"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "gap-f118b3"
anchors = ["crates/roko-cli/src/tui/app/actions.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["gap-f118b3"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --lib inject_key"
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
