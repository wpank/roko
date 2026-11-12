+++
id = "bug-232a67"
kind = "bug"
title = "roko research captures and ACP dispatch episodes still record started_at equal to completed_at"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/research", "roko-acp/bridge_events"]
created = 2026-10-27
updated = 2026-10-27
last_verified = 2026-10-27
source = "bug-41e2de worker, 2026-10-27"
discovered_from = "bug-41e2de"
anchors = ["crates/roko-cli/src/commands/util.rs::build_capture_episode", "crates/roko-cli/src/commands/research.rs", "crates/roko-acp/src/bridge_events/cost.rs::append_acp_episode"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["bug-41e2de"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn research_capture_episode_starts_before_it_completes' crates/roko-cli/ && cargo test -p roko-cli research_capture_episode_starts_before_it_completes"

[[verify]]
command = "grep -rqw 'fn acp_episode_starts_before_it_completes' crates/roko-acp/ && cargo test -p roko-acp acp_episode_starts_before_it_completes"
+++

## Problem

Two more episode writers record an episode that starts and completes at the same instant although it lasted a
while: they build it with `Episode::new` (`crates/roko-learn/src/episode_logger.rs`), which sets
`started_at = completed_at = now`, and then set only `duration_secs`.

- `roko research` captures: the binary's own copy of the capture builder,
  `commands::util::build_capture_episode` (`crates/roko-cli/src/commands/util.rs`), reached through
  `commands::util::persist_capture_episode` from the eleven call sites in `crates/roko-cli/src/commands/research.rs`.
  It is a near-verbatim duplicate of `crate::agent_episode::build_capture_episode`, which bug-41e2de fixed: the lib
  builder now takes the clock read as the work began.
- ACP dispatches: `append_acp_episode` (`crates/roko-acp/src/bridge_events/cost.rs`) gets `dispatch_started: Instant`
  and writes only `duration_secs = elapsed`, so its `acp-dispatch` and `acp-pipeline-*` episodes start as they
  complete.

The main checkout's `.roko/episodes.jsonl` shows the pattern: 2 of 2 `acp-dispatch` rows have
`started_at == completed_at` with `duration_secs > 0`.

## Why it matters

Goal `truth`: S01 SC5 (P0-5 timings) says no episode row has equal start and end with a positive duration. bug-41e2de
fixed the run-level `workflow_complete` episode and the plan-authoring captures; these two writers still break the
rule, so latency figures and the self-model's timing features read zero-length episodes from them.

## Where

- `crates/roko-cli/src/commands/util.rs::build_capture_episode` and `::persist_capture_episode` (binary crate), called
  from `crates/roko-cli/src/commands/research.rs` (each call site has a `started = Instant::now()`).
- `crates/roko-acp/src/bridge_events/cost.rs::append_acp_episode`, called from `bridge_events/mod.rs` with the
  dispatch's `Instant`.
- The fix to copy: `Episode::set_timing(started_at, duration_ms)` (bug-41e2de) places an episode at the clock read as
  its work began and completes it `duration_ms` later.

## Current state

bug-41e2de threaded a real start (`Utc::now()` read beside the `Instant`) into `FeedbackEvent::WorkflowComplete`,
`agent_episode::build_capture_episode`, `agent_exec::persist_capture_episode(_distilled_by)` and plan generation.
The binary's duplicate builder and the ACP writer were left as they were.

## Plan

1. Research: drop the binary's duplicate builder and call `roko_cli::agent_episode::build_capture_episode` (or give
   `commands::util::persist_capture_episode` a `started_at`), reading `Utc::now()` beside each `started` in
   `research.rs`. Do not back-compute the start from the duration.
2. ACP: read `Utc::now()` where `dispatch_started` is taken and pass it to `append_acp_episode`, which calls
   `episode.set_timing(started_at, elapsed_ms)`.
3. Tests: `research_capture_episode_starts_before_it_completes` (roko-cli) and `acp_episode_starts_before_it_completes`
   (roko-acp): the episode's `completed_at - started_at` equals its duration and `started_at` is the start passed in.

## Done when

Both `[[verify]]` commands pass, and a `roko research` capture and an ACP dispatch write no episode with
`started_at == completed_at` and `duration_secs > 0`.

## Notes

Existing rows stay as written. Keep `Episode::new`'s signature.
