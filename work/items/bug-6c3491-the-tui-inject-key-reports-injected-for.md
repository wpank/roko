+++
id = "bug-6c3491"
kind = "bug"
title = "The TUI inject key reports Injected for a directive nothing reads"
status = "open"
triage = "unverified"
severity = "p2"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/tui"]
created = 2026-10-01
updated = 2026-10-01
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
