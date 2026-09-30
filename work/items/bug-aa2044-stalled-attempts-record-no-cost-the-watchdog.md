+++
id = "bug-aa2044"
kind = "bug"
title = "Stalled attempts record no cost: the watchdog drops the provider before it reports usage"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch/watchdog"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-watchdog's report, checked on work/spec-a0403b at d5546dfc7)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = ["spec-a0403b"], blocks = [], related = ["spec-a0403b", "bug-62e3f4"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_stalled_attempt_records_the_usage_it_streamed' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_stalled_attempt_records_the_usage_it_streamed"
+++

## Problem

On `work/spec-a0403b`, the stall watchdog cancels an attempt that has been silent for `[conductor] task_stall_secs` by dropping its dispatch future (`graph_task_dispatch/watchdog.rs:12`, :49-66). The provider never reports usage, so a stalled attempt records no cost, although it may have used many tokens before it went quiet.

## Why it matters

One settled record per attempt (epic spec-b7303f): stalls are exactly the expensive failures, and they would be recorded as free.

## Where

The watchdog's cancel path, and wherever streamed usage is accumulated during a dispatch.

## Plan

1. Accumulate usage as it streams (per message), and settle a cancelled attempt with what was seen, marked partial.
2. Add `a_stalled_attempt_records_the_usage_it_streamed`.

## Done when

- [ ] A cancelled stalled attempt records the usage it streamed, marked partial.
- [ ] The `[[verify]]` command passes.
