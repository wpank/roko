+++
id = "bug-19a78f"
kind = "bug"
title = "show.rs cost_by_day slices a timestamp by bytes and panics on non-ASCII"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-cli/commands"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "bug-6c11d1"
anchors = ["crates/roko-cli/src/commands/show.rs"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = ["bug-6c11d1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "cargo test -p roko-cli --bin roko cost_by_day"
+++

## Problem

`show.rs::cost_by_day` takes `timestamp[..10]` by bytes, so a malformed timestamp with a multi-byte character before byte 10 panics `roko show costs`.

## Plan

Parse the date (or use `get(..10)`), and skip malformed rows. Add a test named `cost_by_day_*`.

## Done when

- `cargo test -p roko-cli --bin roko cost_by_day` passes.

## Notes

- Reported on 2026-10-01 by wk-tuiv, working on bug-6c11d1, during the evening close-out round.
