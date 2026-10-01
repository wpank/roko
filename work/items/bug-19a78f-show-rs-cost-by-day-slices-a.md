+++
id = "bug-19a78f"
kind = "bug"
title = "show.rs cost_by_day slices a timestamp by bytes and panics on non-ASCII"
status = "open"
triage = "verified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "f4323cf9d"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-6c11d1"
anchors = ["crates/roko-cli/src/commands/show.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-6c11d1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn cost_by_day_never_slices_a_timestamp_by_bytes' crates/roko-cli/src/commands/show.rs && cargo test -p roko-cli --bin roko cost_by_day"
+++

## Problem

`show.rs::cost_by_day` takes `timestamp[..10]` by bytes, so a malformed timestamp with a multi-byte character before byte 10 panics `roko show costs`.

## Plan

Parse the date (or use `get(..10)`), and skip malformed rows. Add a test named `cost_by_day_*`.

## Done when

- `cargo test -p roko-cli --bin roko cost_by_day` passes.

## Notes

- Reported on 2026-10-01 by wk-tuiv, working on bug-6c11d1, during the evening close-out round.
- 2026-10-01 (wk-tuiv): implemented on work/bug-6c11d1; cargo verification deferred to the batch check.
  `cost_by_day` takes each event's UTC date from its parsed RFC 3339 timestamp (`event_time`); a timestamp that
  does not parse counts as `undated` rather than being sliced, so its cost stays in the breakdown instead of
  being skipped. Days are UTC dates now, where the old prefix used the timestamp's own offset. Test
  `cost_by_day_never_slices_a_timestamp_by_bytes`. The verify command gained a grep guard: a bare filter passes
  when no test matches.
