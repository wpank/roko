+++
id = "bug-4ba581"
kind = "bug"
title = "A failed GraphEventSink delivery is only logged, and a dropped event is not followed by a Gap event as the sink's docs say"
status = "open"
triage = "unverified"
severity = "p3"
goal = "visibility"
size = "S"
subsystem = ["roko-graph"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-runstate's report on reg-cbfff6)"
anchors = ["crates/roko-graph/src/events.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["reg-cbfff6"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_failed_sink_delivery_is_reported_and_followed_by_a_gap' crates/roko-graph/src/ && cargo test -p roko-graph --lib a_failed_sink_delivery_is_reported_and_followed_by_a_gap"
+++

## Problem

Since reg-cbfff6 the Graph engine emits node lifecycle events to its sink. When a delivery fails the engine logs it and carries on, and a Dropped delivery is not followed by a Gap event, although events.rs (about line 93) documents both.

## Why it matters

Visibility: a consumer that misses events cannot tell it missed them.

## Plan

Follow the documented contract: emit a Gap after a Dropped delivery, and decide whether a sink failure stops the run or is surfaced as an event. Add the test named in the verify.

## Done when

- [ ] Dropped deliveries are followed by a Gap event, and failures are surfaced
- [ ] The `[[verify]]` command passes.
