+++
id = "bug-aa2044"
kind = "bug"
title = "Stalled attempts record no cost: the watchdog drops the provider before it reports usage"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch/watchdog"]
created = 2026-09-30
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "006bc97f8"
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

## Notes

- Implemented on `work/bug-739dcc` at `006bc97f8`; cargo verification deferred to the batch check. `a_stalled_attempt_records_the_usage_it_streamed` and `streamed_usage_sums_the_last_usage_of_each_model_call` (targeted `cargo test` passed). Changes:
  - The Claude CLI adapter sends the run's estimated usage so far after each `assistant` event that changed it, and the immune boundary forwards `Usage` and `Done` to the live sink.
  - `AttemptProgress` keeps each model call's last `Usage` (a `Done` ends one) and the call in flight, whose target and failover chain `run_bridge_with_failover` and the streaming path note as it starts.
  - A call the watchdog or the conductor cancels is accounted like any failed call: an unsuccessful dispatch carrying the streamed usage, marked estimated (unknown when nothing streamed), is settled, its spend reaches the task and plan ledgers, and `emit_feedback` writes its episode, cost and efficiency rows. Cost rows gain `cost_source`.
  - Not done: providers that stream no usage (Codex CLI, Gemini CLI, Cursor) still record a cancelled call's usage as unknown.
