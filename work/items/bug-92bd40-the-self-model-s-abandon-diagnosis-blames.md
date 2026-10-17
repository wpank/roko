+++
id = "bug-92bd40"
kind = "bug"
title = "The self-model's abandon diagnosis blames the forecast on a task's last attempt: 'forecasts at most 0.42 P(verified success) … below p_abandon 0.15', when it abandons only because no attempt is left"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "S"
subsystem = ["roko-cli/graph_task_dispatch", "roko-learn/self_model"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "round-v6-dogfood-will#R3-4"
discovered_from = "agent:dogfood-will (round v6 run 3, 2026-10-16)"
anchors = ["crates/roko-learn/src/self_model/cascade.rs::after_failure", "crates/roko-cli/src/graph_task_dispatch/self_model.rs::GraphTaskDispatcher::report_self_model_action"]
lane = "rust-hot"
parent = "spec-7a71d6"
links = { depends_on = [], blocks = [], related = ["bug-92cc56"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn abandon_with_no_attempt_left_does_not_blame_the_forecast' crates/roko-cli/ && cargo test -p roko-cli abandon_with_no_attempt_left_does_not_blame_the_forecast"
+++

## Problem

The self-model's abandon diagnosis contradicts its own numbers. In round v6's dogfood run 3 (2026-10-16, roko
47b3cdd20, `[self_model] mode = "shadow"`, `policy = "cascade"`), the dashboard's L2 ring and the event log showed, for
task SVF of plan pocket-synth:

> self-model: SVF is unlikely to pass on any rung · The self-model forecasts at most 0.42 P(verified success) for task
> `SVF` of plan `pocket-synth` on every rung, below p_abandon 0.15 (abandon).

0.42 is not below 0.15. That attempt's `predictions.jsonl` row (SVF attempt 6) has candidates at P(VS) 0.38
(gpt-oss-120b), 0.42 (glm-4.7) and 0.42 (gpt-5.4-mini), `decision.action = "abandon"` and `would_choose = null`. The
chat follow-up `913dc67f` shows the same at its attempt 4: P(VS) 0.31, 0.67 and 0.40, action abandon.

Both were the task's last permitted attempt. SVF had `max_retries = 5`, so attempt 6 was its last. The prompt run had
`max_retries = 3`, so attempt 4 was its last. `cascade::after_failure` returns `Abandon` when it has no option left
(`retries_left == 0`, so it can neither retry nor climb), whatever the forecasts say. `report_self_model_action` then
words every abandon as a forecast below p_abandon, with `best` taken over all candidates.

## Why it matters

Goal `cybernetic`: M3's diagnoses are the part of the self-model a person sees (the L2 ring, `diagnosis` events, the
event log). This one says the opposite of its own numbers. Its suggested action, "Split the task, replan it, or drop
it", is the wrong advice for a task that only used up its retries: SVF passed in 18 s on its next run, once its mid rung
worked. Related: bug-92cc56 and spec-7a71d6.

## Where

- `crates/roko-learn/src/self_model/cascade.rs::after_failure` returns `StepAction::Abandon` in two cases: when
  `options` is empty (no retry or climb left), and when the best option's P(VS) is below `P_ABANDON`. The action does
  not say which.
- `crates/roko-cli/src/graph_task_dispatch/self_model.rs::GraphTaskDispatcher::report_self_model_action` builds the
  `self_model_abandon` diagnosis, subject, detail and event-log message from `best`, the maximum P(VS) over every
  candidate.
- Policy (a) has the same shape: `crates/roko-learn/src/self_model/policy.rs::LcbAci::choose` abandons when no
  eligible arm is left above p_abandon.

## Current state

Shadow mode reports the forecast's action as a diagnosis on every attempt it would abandon. Evidence:
- the finding: `tmp/hermes-roko-v6/findings/dogfood-will.md`, R3-4;
- the predictions: `tmp/hermes-roko-v6/dogfood-will/evidence/run3/runs/ea0c32cb-7cde-4ec7-9ef7-1a9de68d7f1b/predictions.jsonl`
  (SVF attempt 6) and `…/runs/913dc67f-f835-45d0-a004-6163a132b70a/predictions.jsonl` (T1 attempt 4);
- the retry budgets: the `max_retries` of the attempt-open rows in each run's `attempts.jsonl`.

## Plan

1. Let the policies say why they abandon. Options: `StepAction::Abandon { reason }` with `NoAttemptLeft` or
   `BelowPAbandon`, or a separate exhausted action. Record the reason in the prediction's `decision`, as an optional
   `reason` field.
2. In `report_self_model_action`:
   - report a forecast-driven abandon as "unlikely to pass on any rung", with `best` taken over the options the policy
     weighed;
   - for an abandon caused by no attempt being left, either report nothing (the ladder already reports
     `ladder_exhausted`) or report "no attempt left", with no P(VS) wording.
3. Add the test `abandon_with_no_attempt_left_does_not_blame_the_forecast`: a task on its last attempt whose candidates
   forecast well above p_abandon publishes no "below p_abandon" diagnosis.

## Done when

- The `[[verify]]` passes.
- A shadow run's last-attempt abandon no longer prints a P(VS) above p_abandon as "below p_abandon".

## Notes

- Do not change what the policies decide; change only what they say and record.
- `roko learn self-model fit` and `replay` read the prediction rows, so a new `decision.reason` must be optional for
  old rows.
