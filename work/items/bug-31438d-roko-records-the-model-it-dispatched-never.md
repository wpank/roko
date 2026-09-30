+++
id = "bug-31438d"
kind = "bug"
title = "Roko records the model it dispatched, never the model the provider reports serving"
status = "open"
triage = "verified"
severity = "p1"
goal = "truth"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch", "roko-core/usage"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (20:01, wk-bench-rokoarm's report on gap-b7ab99)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs:151", "crates/roko-core/src/usage.rs::UsageObservation", "crates/roko-agent/src/translate/openai.rs::parse_usage_observation", "crates/roko-cli/src/runtime_feedback/episodes.rs::EpisodeSink"]
lane = "rust-hot"
parent = "spec-b7303f"
links = { depends_on = [], blocks = [], related = ["bug-35379d", "gap-c4f364", "gap-b7ab99", "bug-55fd84"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn records_carry_the_provider_reported_model' crates/roko-cli/src/ && cargo test -p roko-cli --lib records_carry_the_provider_reported_model"
+++

## Problem

Every Graph record names the model Roko dispatched. `graph_task_dispatch/feedback.rs:151` takes `dispatch.target.model_slug` for the episode's `initial_model`, the cost row and the efficiency record. The episode's `model` is `outcome.model` (`runtime_feedback/episodes.rs:109`).

The provider's own answer is parsed, but never stored:

- `UsageObservation.model` (`roko-core/src/usage.rs`) is filled from the response's `model` field by `translate/openai.rs::parse_usage_observation` (test `parse_usage_observation_carries_provider_reported_model`), and by the Claude CLI and Codex adapters.
- No Graph record carries it.

wk-bench-rokoarm checked this at 33e107da1 with a loopback fake provider that answered as glm-4.7: every episode, cost row and efficiency record still said gpt-oss-120b (`benchmarks/viabilitybench/driver/run_roko.py:39-44` records this). Since then only three commits touched this code, and none of them concerns the model: 6cfc96d05 (reflex credit), a729fb911 (module split, no behaviour change) and 875482152 (a test).

## Why it matters

p1 for the epic "one settled record per attempt":

- A pin check can't see a substitution: not the benchmark's `model_mismatch`, and not S01's model fields.
- Cost is priced by the requested model, not the one that served.

bug-35379d is the case where Roko itself fails over to another model. This bug is the case where the provider does it: model aliases, routers, a gateway that falls back. Because of it, the benchmark leaves `model_reported` null for the Roko arm and needs the metering proxy (gap-e003ec) to learn the served model.

## Where

- The record writers in `graph_task_dispatch/feedback.rs` (episode event, cost row, efficiency record).
- `UsageObservation.model`.
- The dispatch outcome that carries usage from the provider to the feedback step.

## Current state

At BASE (4315add32), the reported model is parsed into `UsageObservation.model` and then dropped before any record is written.

## Plan

1. Carry the reported model from each provider response to the dispatch outcome, as `model_reported` next to the dispatched model (the last response's value, plus a flag when responses disagree).
2. Write it into episodes, cost rows and efficiency records.
3. Price the cost by the reported model, or mark the price unknown when the reported model has no price.
4. When the two differ, warn and record a field. When the model was pinned (`--model`), treat the difference as an error, like bug-35379d's pin.
5. Add `records_carry_the_provider_reported_model`: a scripted provider answers as another model, and the records carry both names.

## Done when

- [ ] Episodes, cost rows and efficiency records carry the model the provider reported, and a mismatch is visible.
- [ ] The `[[verify]]` command passes.

## Notes

- gap-c4f364 records reported models for the benchmark's Claude Code arm. This item is Roko's own records.
- Implemented on `work/bug-31438d` at `b0cf98b3d` (feedback.rs rows at `06bdb71be`); cargo verification deferred to the batch check. `records_carry_the_provider_reported_model` and the other tests it adds (targeted `cargo test` passed at the branch head). Root cause: `roko-agent/src/tool_loop/agent_wrapper.rs::attach_model` overwrote `usage_obs.model` with the configured slug, and the tool loop never read the response's `model`, so even the verdict's `executed.model_reported` was the dispatched model. Changes:
  - roko's tool loop reads each response's `model` (`ToolLoopTurnTrace.model`, `BackendResponse::extract_model`); `usage_obs.model` is the last one named, and the configured slug stays apart, on the output's `model` tag.
  - The verdict's `executed` gains `model_dispatched`, `models_reported` and `model_mismatch`.
  - Episodes (`extra.model_reported`, `extra.model_mismatch`) carry it, and so do cost and efficiency rows (`roko_learn::efficiency::ExecutedRow`).
  - A substitution is logged at WARN and priced by the model that served, unknown (0) when that model has no price. `served_model::same_model` treats dated snapshots and provider prefixes as the same model. Under `--model` a substitution fails the attempt with a non-retryable `model_substituted` error after it is recorded.
  - Not done: the CLI adapters (Claude CLI, Codex, Cursor, Gemini native) still fill `usage_obs.model` with the configured slug when the CLI names none. The streaming dispatch path warns and prices but does not fail a pin. The bridge's `model_call` rows still name only the dispatched slug.
