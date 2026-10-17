+++
id = "bug-f6b99f"
kind = "bug"
title = "An attempt whose model hits its output limit (finish_reason=length) settles as an infra-blamed provider error, so the ladder never counts it and the verify steps never judge its work"
status = "done"
triage = "verified"
severity = "p3"
goal = "golden-path"
size = "S"
subsystem = ["roko-agent/tool_loop", "roko-cli/graph_task_dispatch"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
last_verified_rev = "04e46172b"
source = "round-v6-dogfood-will#R3-6"
discovered_from = "agent:dogfood-will (round v6 run 3, 2026-10-16)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::provider_failure_outcome", "crates/roko-agent/src/tool_loop/mod.rs::ToolLoop::run_inner", "crates/roko-agent/src/provider/error_classify.rs::detect_turn_cap"]
lane = "rust-hot"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn output_limit_failure_counts_towards_the_climb' crates/roko-cli/ && cargo test -p roko-cli output_limit_failure_counts_towards_the_climb"

[closed]
at = 2026-10-17
at_ts = "2026-10-17T13:31:15Z"
commit = "04e46172b"
executor = "claude-agent"
via = "manual"
size = "S"
claimed_at = "2026-10-17T12:29:47Z"
forced = false
evidence = "04e46172b: error_classify gains OUTPUT_LIMIT_MARKER and detect_output_limit, which the tool loop's message uses (a blank cut reply stays empty_response). A closing turn cut at the output limit after tool work now ends the run as Stop, so the verify steps judge the work it left; a cut with no tool work still fails. provider_failure_outcome reads that failure as the model's own limit, like a turn cap: blamed on the agent, so two climb; its reason is classed output_limit, provider health counts it as the provider's success, and the next try is told its reply ran past the output limit and to write less per turn. output_limit_failure_counts_towards_the_climb (two such tries on cheap climb) and a_closing_turn_cut_at_the_output_limit_after_tool_work_is_stop pass. Checks at 04e46172b (on main 7a5dd6046): fmt --check, clippy -D warnings (roko-agent, roko-core, roko-cli, roko-serve), lib tests roko-agent 2371 passed, roko-cli 3774 passed with the known load flake plan_check_publishes_start_heartbeat_and_finish (bug-8d8f98: an extra heartbeat while the slot's tests ran; plan_verify.rs is untouched); the [[verify]] passes."
+++

## Problem

An attempt whose model runs out of output is blamed on infra, so the ladder never counts it.

In round v6's dogfood run 3 (2026-10-16, roko 47b3cdd20), the notch follow-up (`913dc67f`, cap $0.12) ran cheap
gpt-oss-120b three times.
- Attempt 2 did tool work: it wrote `synth/svf.py` and a test file, and ran the tests (1 failure). Then its last turn
  hit the output limit.
- The tool loop ended the attempt as `StopReason::BackendError("model hit output token limit (finish_reason=length)")`.
- `provider_failure_outcome` maps that message to `provider_error`, and the attempt settles with `Blame::Infra`.

As a result:
- the ladder did not count the failure, since only agent-blamed failures climb. Cheap got a third attempt ($0.032)
  before the climb to mid, and the plan's cap then stopped mid one model call later;
- the verify steps never ran on the work the attempt left in its tree.

The evidence is the verdict row for T1 attempt 2 in `runs/913dc67f…/attempts.jsonl` (outcome `provider_error`, blame
`infra`). The episode's `failure_reason` reads "provider: model hit output token limit (finish_reason=length)".

## Why it matters

Goal `golden-path`: the ladder exists for a model that can't do the task. Running out of output on it is that model's
limit, not the provider's. Each uncounted failure costs one more cheap attempt and delays the climb, here past the
plan's cap.

## Where

- `crates/roko-agent/src/tool_loop/mod.rs::ToolLoop::run_inner` sets the final turn's stop reason. With
  `hit_length_limit` it returns `StopReason::BackendError`, even after tool calls. A blank closing turn after tool work
  stays `Stop`, so that the verify steps judge the work.
- `crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::provider_failure_outcome` maps a provider failure message to
  `TurnCap`, `ProviderExhausted`, `Timeout` or `ProviderError` (infra).
- `crates/roko-agent/src/provider/error_classify.rs` holds `detect_turn_cap` and the other detectors.
- `crates/roko-cli/src/graph_task_dispatch/ladder.rs::GraphTaskDispatcher::note_ladder_outcome` counts only
  `Blame::Agent` failures.

## Current state

- `finish_reason=length` has no outcome of its own.
- The tool loop's log line says "increase max_output for this model", which a plan author cannot do per rung.

## Plan

1. Classify the output-limit failure as agent-blamed. Either add an `AttemptOutcome::OutputLimit` (blame Agent,
   learning label 0), or treat it as a turn cap. Detect it from the message with a `detect_output_limit` beside
   `detect_turn_cap`.
2. When the attempt made tool calls before the cut, end it as `Stop`, as a blank closing turn after tool work does.
   The verify steps then judge the work it left, and the length cut shows in the verdict's detail.
3. Add the test `output_limit_failure_counts_towards_the_climb`: two cheap attempts that end with
   `finish_reason=length` climb on the third, as two turn caps do.

## Done when

- The `[[verify]]` passes.
- An output-limit failure settles as agent-blamed (or reaches verify) and counts towards the climb.

## Notes

- Leave a blank answer with no tool call (`empty_response`, backlog 1102) as it is.
- The retry feedback for an output-limit failure should tell the next attempt to write less per turn.
- Evidence: `tmp/hermes-roko-v6/dogfood-will/evidence/run3/runs/913dc67f-f835-45d0-a004-6163a132b70a/attempts.jsonl`
  and `…/evidence/run3/episodes.jsonl`. The finding is R3-6 in `tmp/hermes-roko-v6/findings/dogfood-will.md`.
