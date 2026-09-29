+++
id = "bug-55fd84"
kind = "bug"
title = "Episodes record turns = 1 for dispatches on providers that report no turn count"
status = "open"
triage = "unverified"
severity = "p2"
goal = "truth"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-agent/runtime_events"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-rokoarm's report on gap-b7ab99)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs:141", "crates/roko-cli/src/runtime_feedback/episodes.rs::EpisodeSink", "crates/roko-agent/src/runtime_events.rs"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-f9ae3e", "gap-5a6e01", "bug-31438d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn episode_turns_count_tool_loop_calls' crates/roko-cli/src/ && cargo test -p roko-cli --lib episode_turns_count_tool_loop_calls"
+++

## Problem

On the Graph path, an episode's `turns` comes from the dispatch's last `AgentRuntimeEvent::TurnCompleted { num_turns }`, and falls back to 1 when there is none (`graph_task_dispatch/feedback.rs:141-149`, commented "Agent turns as reported by the provider (the Claude CLI's `num_turns`)").

- Only the Claude CLI stream fills `num_turns` (`provider/claude_cli/stream.rs:208`).
- The Codex and Gemini CLI streams, and `runtime_events.rs` (:192, :220), set it to None.
- An OpenAI-compatible tool loop emits no count at all.

So every dispatch on those providers records `turns = 1`, however many model calls its tool loop made. The ViabilityBench Roko arm saw `turns=1` for a dispatch that made 3 calls (gpt-oss-120b on Cerebras). The same value feeds the efficiency record's `iteration` and `turn_number` (feedback.rs:333-334).

## Why it matters

One settled record per attempt (epic spec-b7303f): turn counts feed turn-cap tuning (gap-5a6e01), efficiency analysis and the benchmark's attempt records (`driver/run_roko.py` copies `episode.turns`). A constant 1 reads as a measurement, not as "unknown".

## Where

- The fallback: `feedback.rs:141`.
- The episode writer: `runtime_feedback/episodes.rs::EpisodeSink`, which sets `episode.turns`.
- The in-process tool loop in roko-agent (dispatcher), which knows how many calls it made.

## Current state

At BASE (4315add32) the fallback is `.unwrap_or(1)`. No non-Claude provider supplies a count.

## Plan

1. Count model calls in the in-process tool loop, and report them on `TurnCompleted` or on the dispatch result.
2. When no count exists (a CLI that doesn't report one), record the turns as unknown, not 1. For example, keep `turns = 0` and add `extra.turns_unknown = true`, or make the field optional; the efficiency record follows the same rule.
3. Add `episode_turns_count_tool_loop_calls`: a scripted OpenAI-compatible provider that makes 3 calls records `turns = 3`.

## Done when

- [ ] Episodes from OpenAI-compatible dispatches carry the real number of model calls, and a missing count is marked unknown.
- [ ] The `[[verify]]` command passes.

## Notes

- bug-f9ae3e covers the efficiency record's tool-call count, which is a different field with a different cause.
