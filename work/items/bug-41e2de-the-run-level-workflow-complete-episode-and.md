+++
id = "bug-41e2de"
kind = "bug"
title = "The run-level workflow_complete episode and plan-authoring episodes still record started_at equal to completed_at"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-learn/feedback_service", "roko-cli/agent_episode"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-24"
anchors = ["crates/roko-learn/src/feedback_service.rs::build_episode_from_workflow", "crates/roko-cli/src/agent_episode.rs::build_capture_episode", "crates/roko-learn/src/episode_logger.rs::Episode", "crates/roko-cli/src/runtime_feedback/episodes.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["gap-f548c1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn workflow_episode_starts_before_it_completes' crates/roko-learn/ && cargo test -p roko-learn workflow_episode_starts_before_it_completes"

[[verify]]
command = "grep -rqw 'fn capture_episode_starts_before_it_completes' crates/roko-cli/ && cargo test -p roko-cli capture_episode_starts_before_it_completes"
+++

## Problem

Backlog 2105 fixed the timings of per-task Graph episodes, but two other episode writers still record an episode
that starts and ends at the same instant (status-quo assessment 2026-10-15 at 7b6a5af44, finding 24;
`tmp/status-quo/2026-10-15/backlog/A02-21-22-honest-measurement.md`, row 2105). Both build the episode with
`Episode::new` (`crates/roko-learn/src/episode_logger.rs:320`), which sets `started_at = completed_at = now`, and then
set only `duration_secs`:

- `build_episode_from_workflow` (`crates/roko-learn/src/feedback_service.rs:970`): the run-level `workflow_complete`
  episode.
- `build_capture_episode` (`crates/roko-cli/src/agent_episode.rs:74`): the `plan:generate:<slug>` plan-authoring
  episodes (`plan_generate/pipeline.rs:829`) and other captured agent calls (`agent_exec.rs:362`).

42 rows in the live `.roko/episodes.jsonl` files have `started_at == completed_at` with `duration_secs > 0`. S01 SC5
(P0-5 timings) says no such rows exist.

## Why it matters

Goal `truth`: episode timings feed latency figures, the self-model's features and the papers' timing claims. A
zero-length episode with a positive duration is wrong whichever field a reader trusts.

## Where

- `crates/roko-learn/src/feedback_service.rs::build_episode_from_workflow` (called at `:761`).
- `crates/roko-cli/src/agent_episode.rs::build_capture_episode`.
- The 2105 fix to copy: `crates/roko-cli/src/runtime_feedback/episodes.rs:325-337` takes the settled attempt's start,
  else `completed_at` minus the duration.

## Current state

Per-task episodes are fixed: all 87 live T1-T5 rows have start < end. The package that closed 2105 (gap-f548c1)
covered only them.

## Plan

1. Pass the real start time to both builders where the caller knows it (the run's start; the agent call's start).
   Otherwise set `started_at = completed_at - duration`, as `runtime_feedback/episodes.rs` does.
2. Tests: `workflow_episode_starts_before_it_completes` (roko-learn: a workflow episode with a 1500 ms duration has
   `completed_at - started_at` of 1500 ms) and `capture_episode_starts_before_it_completes` (roko-cli: the same for
   `build_capture_episode`).

## Done when

Both `[[verify]]` commands pass, and a fresh `roko run --plan --dry-run` plus a one-task plan run write no episode
with `started_at == completed_at` and `duration_secs > 0`.

## Notes

Don't change `Episode::new`'s signature for every caller; the fix belongs in the two builders. Existing rows stay as
written.
