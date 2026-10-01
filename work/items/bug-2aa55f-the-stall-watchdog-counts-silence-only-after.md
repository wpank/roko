+++
id = "bug-2aa55f"
kind = "bug"
title = "The stall watchdog counts silence only after an assistant message, so a provider that streams only content_block_delta is never cancelled"
status = "open"
triage = "verified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "faa378453"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-tiers' report on gap-9eebcb, branch work/gap-9eebcb at 55ffa7074)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/watchdog.rs"]
lane = "rust-hot"
parent = "spec-edda86"
links = { depends_on = [], blocks = [], related = ["gap-9eebcb", "spec-a0403b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn a_delta_only_stream_still_trips_the_watchdog' crates/roko-cli/src/ && cargo test -p roko-cli --lib a_delta_only_stream_still_trips_the_watchdog"
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
