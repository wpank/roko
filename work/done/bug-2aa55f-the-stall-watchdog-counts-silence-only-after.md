+++
id = "bug-2aa55f"
kind = "bug"
title = "The stall watchdog counts silence only after an assistant message, so a provider that streams only content_block_delta is never cancelled"
status = "done"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "8a3c530af"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tiers' report on gap-9eebcb, branch work/gap-9eebcb at 55ffa7074)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs"]
lane = "rust-hot"
parent = "spec-edda86"
links = { depends_on = [], blocks = [], related = ["gap-9eebcb", "spec-a0403b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_delta_only_stream_still_trips_the_watchdog' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_delta_only_stream_still_trips_the_watchdog"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T15:16:12Z"
by = "coordinator (session 7622b882)"
forced = false
evidence = "Batch 20f gate on 2ff1b7891 (MAIN has the same crates and portal): check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-agent/cli/core/graph/serve; lib tests roko-cli 3309, roko-agent 2296, roko-core 1963, roko-serve 992, roko-graph 480 pass; extras: all eight canaries + golden_path_suite + secret_canary 11/11 + C2 2/2 + worktree_task_diff + default_engine pass, bin 429, graph_task_dispatch suite at --test-threads=32 passed 10 of 10, including a_delta_only_stream_still_trips_the_watchdog and a_streaming_call_is_silent_from_its_start; a 30 s first-output grace keeps slow starts from being cancelled (the load flake found in the first 20f gate). Merged (work/gap-9eebcb-l7 9424434fd)."
+++

## Problem

The watchdog starts counting silence only after the stream's first `assistant` message. wk-tiers' first C7 fake printed only `content_block_delta` events and then went quiet; the watchdog never cancelled it.

## Why it matters

Supervision (epic spec-edda86): a provider can stall after streaming deltas, or before any assistant message, and the watchdog is the only thing that ends it short of the task timeout.

## Plan

1. Count silence from the attempt's start and reset it on any stream event that shows progress (deltas included), not only on assistant messages.
2. Add `a_delta_only_stream_still_trips_the_watchdog`.

## Done when

- [ ] A stream of deltas that then goes silent is cancelled after the stall threshold.
- [ ] The `[[verify]]` command passes.

## Notes

- 2026-10-01 (wk-tiers): implemented on `work/gap-9eebcb-l7` at `fe18ce50e`; cargo verification deferred to the batch check.
  - The Claude CLI adapter now forwards partial-message deltas, bare `content_block_delta` lines and `stream_event` wrappers alike, as text or reasoning deltas, so they reach the watchdog as progress.
  - Silence counts from the call's start only for the Claude CLI (`streams_as_it_goes`). Providers that may report only at the end (the Codex CLI, the Cursor CLI, and for now the API and ACP kinds) keep the first-event rule, so a long Codex run is not cancelled after 300 s without events. The coordinator agreed this boundary.
  - Tests: `a_delta_only_stream_still_trips_the_watchdog`, `a_streaming_call_is_silent_from_its_start`.
- 2026-10-01 (wk-tiers): the batch-20f gate showed a problem with the Claude-only "silent from its start" rule.
  Under load, it cancelled calls that were only starting: providers never launched, and attempts that talked were
  blamed on infra. Fixed at `fe853655e`:
  - Before its first event, a streaming call's silence counts only once `FIRST_OUTPUT_GRACE` (30 s) has passed.
    A call that never reports anything is still cancelled, after max(30 s, `task_stall_secs`).
  - After the first event, silence counts from the last event, as before.
  - Evidence: `cargo test -p roko-cli --lib graph_task_dispatch -- --test-threads=32` passed 10 of 10 runs (220 tests
    each) in a cloned target at load average 36-62.
