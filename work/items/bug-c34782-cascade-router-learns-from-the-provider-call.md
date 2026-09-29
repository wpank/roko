+++
id = "bug-c34782"
kind = "bug"
title = "Cascade router learns from the provider call's success flag before gates run"
status = "done"
triage = "verified"
severity = "p2"
size = "M"
goal = "learning"
subsystem = ["roko-cli/graph-dispatch", "roko-learn/cascade-router"]
created = 2026-09-28
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "89f4b09ea"
source = "local-audit-2026-09-26"
discovered_from = "audit:local-defect-review-2026-09-26 (untracked design notes)"
anchors = ["crates/roko-cli/src/graph_task_dispatch/feedback.rs::GraphTaskDispatcher::emit_feedback", "crates/roko-cli/src/graph_task_dispatch/turn_policy.rs::provider_failure_reason", "crates/roko-cli/src/runtime_feedback/mod.rs::FeedbackEvent", "crates/roko-cli/src/runtime_feedback/routing.rs::RoutingObservationSink", "crates/roko-graph/src/cells/task_executor.rs::TaskGateVerdict"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn routing_learns_only_from_gate_verdicts' crates/roko-cli/src/ && cargo test -p roko-cli --lib routing_learns_only_from_gate_verdicts && cargo test -p roko-cli --lib verified_outcome_drives_output_verdict_and_feedback"

[closed]
at = 2026-09-29
by = "coordinator (session 7622b882)"
evidence = "RoutingObservationSink learns from settled.learning_label, not the provider call's success flag (folded into gap-8f6206, 04c1da262; merged). The provider-bridge path is bug-07bc75. Batch 9 gate (dedicated target dir, batch tree = MAIN crates after the merges): cargo check --workspace --tests clean; nightly rustfmt clean; clippy -p roko-cli -p roko-learn -p roko-agent -p roko-std -p roko-core -p roko-daimon -p roko-neuro --no-deps -D warnings clean; lib tests roko-cli 3098, roko-agent 2254, roko-core 1925, roko-learn 1181, roko-neuro 239, roko-std 222, roko-daimon 100, 0 failed."
+++

## Problem

This bug had two parts.

The first part is fixed. The cascade router used to learn from the provider call's own `success` flag, before
any verify step ran. Since `725f21e05`, both Graph dispatch paths settle feedback after verification.

The second part is still open. Feedback is still settled as a single `succeeded: bool`, and that bool mixes up
four different outcomes:

- `verification.is_ok()` is true for all three `TaskGateVerdict` values:
  - `Passed`;
  - `Unverified`: the task has no `[[task.verify]]` steps;
  - `ForcedAccept`: verify failed, but the cheap-model judge accepted the result anyway.

  So the router gets a full-quality reward (`observe_multi_objective` with quality 1.0) for work that was never
  gate-checked, and for work that failed its gate and was force-accepted.
- A provider-side failure is settled as `succeeded = false`, and the router records it as a model-quality
  failure (`record_category_outcome(.., false)`, `record_confidence_outcome(.., false)`). This covers credit or
  billing exhaustion, transport errors and refusals, settled at `graph_task_dispatch.rs:3807` with a
  `provider_failure_reason`. A model whose account ran out of credit is taught to the router as a bad model.

Expected: the router learns only from real quality evidence. A gate pass is a success. A gate failure, and
possibly a turn-cap stop, is a failure. Unverified, force-accepted and infrastructure-failed attempts are not
quality observations.

## Why it matters

- Goal `learning` (learning loops on the Graph path). The router's bandit and confidence scores
  (`.roko/learn/cascade-router.json`) decide which model future tasks get. Rewarding unverified output and
  punishing a model for a billing error both steer routing the wrong way.
- The same bool feeds episodes (`runtime_feedback/episodes.rs`), playbook outcomes (`store.record_outcome`,
  ~line 1736), prompt-experiment settlement (`AssignmentSettlement::Observed`, ~lines 1762-1764), `costs.jsonl`
  (`success: succeeded`, ~line 1681) and efficiency records. Make the verdict explicit once, so each consumer can
  pick the rule it needs.
- Related:
  - `bug-35379d`: failover runs a different model. The router must credit the model that actually ran.
  - `gap-0f3980`: routing context fields.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`:
  - `GraphTaskDispatcher::emit_feedback` (line ~1458): takes `succeeded: bool` and `failure_reason`, and sends
    `FeedbackEvent::TaskCompleted` to the feedback facade. It also writes efficiency, `costs.jsonl`, playbook and
    experiment settlements.
  - Callers: ~line 3807 (batch, provider call failed: `false` plus `provider_failure_reason`), ~line 3891
    (batch, after `settle_task_verification`: `verification.is_ok()`), ~line 4453 (streaming:
    `matches!(verification, Some(Ok(_)))`).
  - `settle_task_verification` (line ~1807) returns `Result<TaskGateVerdict>`. The judge that can produce
    `ForcedAccept` is at ~line 2290.
  - `provider_failure_reason` (line ~765) classifies a failure as `turn_cap`, `provider_exhausted` or `provider`,
    using `roko_agent::provider::error_classify`. `attempt_failure_reason` (line ~752) builds `"<class>: <line>"`.
- `crates/roko-graph/src/cells/task_executor.rs::TaskGateVerdict` (line ~114): `Passed | Unverified |
  ForcedAccept`. Its doc says a `ForcedAccept` is "never a pass".
- `crates/roko-cli/src/runtime_feedback/mod.rs::FeedbackEvent::TaskCompleted` (line ~70): carries
  `succeeded: bool` and `failure_reason: Option<String>`.
- `crates/roko-cli/src/runtime_feedback/routing.rs::RoutingObservationSink::on_event` (lines ~66-129): turns
  `succeeded` into router observations. Overrides go through `record_override_outcome`.
- Entry point: `roko plan run` → Graph engine → `GraphTaskDispatcher::dispatch` (batch) or its streaming
  variant.

## Current state

- Fixed in `725f21e05`: a successful provider call is settled only after verification, on both paths.
- The test `verified_outcome_drives_output_verdict_and_feedback` (graph_task_dispatch.rs ~line 5633) pins the
  current behaviour. It currently asserts that an `Unverified` attempt is recorded as `"success"` in
  efficiency. After this fix it will need updating.
- Nothing distinguishes `ForcedAccept` from `Passed` for learning.
- Provider-side failures are recorded against the model by `RoutingObservationSink`. It never looks at
  `failure_reason`.
- The current `[[verify]]` (`cargo test -p roko-cli runtime_feedback`) passes whatever the behaviour is. The
  replacement proposed on 2026-09-29 (`verified_outcome_drives_output_verdict_and_feedback` plus a
  `! grep 'dispatch.result.success,$'`) already passes at HEAD, so it only checks the part that is fixed.

## Plan

1. Add an explicit attempt verdict for feedback, for example `AttemptVerdict` in `runtime_feedback/mod.rs`:
   `Passed`, `Unverified`, `ForcedAccept`, `GateFailed`, `TurnCap`, and `ProviderFailed { exhausted: bool }`
   (transport or exhaustion; not a quality signal).
2. Carry it through the event.
   - Add `verdict: AttemptVerdict` to `FeedbackEvent::TaskCompleted`. Keep `succeeded`, derived from the verdict,
     so the other sinks do not change behaviour in this item.
   - Update every place that builds `TaskCompleted`: `graph_task_dispatch.rs`, `runner/state.rs`, and tests in
     `tests/test_runtime.rs`, `tests/dispatch_feedback_projection_e2e.rs`, `tests/runner_facades_e2e.rs`, and the
     `runtime_feedback/{routing,episodes,knowledge}.rs` tests.
3. Compute the verdict at the three `emit_feedback` call sites. The info is already there: the
   `Result<TaskGateVerdict>` and the `provider_failure_reason` class. Pass the verdict to `emit_feedback`
   instead of a bare bool.
4. In `RoutingObservationSink::on_event`, learn only from quality evidence:
   - `Passed` → today's success path;
   - `GateFailed` and `TurnCap` → today's failure path;
   - `Unverified`, `ForcedAccept` and `ProviderFailed` → no bandit, category or confidence update.
   - Provider health is tracked elsewhere, by the failover health registry.

   The design choice is how to treat `TurnCap` and `Unverified`:
   - Option A (recommended): `TurnCap` counts as a failure, since the model did not finish inside its budget.
     `Unverified` is ignored, since there is no evidence.
   - Option B: record `Unverified` as a dampened observation via `record_override_outcome`. It is noisier, but
     plans without verify steps still teach the router something.
5. Add the verdict to the `costs.jsonl` record as a new optional field (`verdict`, with serde default). Leave the
   meaning of the existing `success` field alone, because `roko status` and `roko show costs` read it.
6. Tests in `graph_task_dispatch.rs` `mod tests`, using the existing `make_test_dispatcher` helpers:
   - `routing_learns_only_from_gate_verdicts`. Use a recording router or a `RoutingObservationSink` over a temp
     router, and assert that:
     - a passing verify gives one success;
     - a failing verify gives one failure;
     - no verify steps gives no observation;
     - a provider result containing an exhaustion message gives no observation.
   - Update `verified_outcome_drives_output_verdict_and_feedback` for the new efficiency outcome of the
     unverified attempt, if you change it.

## Done when

- The router's category and confidence counters do not move for unverified, force-accepted or
  provider-exhausted attempts.
- They do move for gate-passed attempts (as successes) and for gate-failed attempts (as failures).
- `FeedbackEvent::TaskCompleted` carries an explicit verdict, and `costs.jsonl` records it.
- Verify:
  `grep -rqw 'fn routing_learns_only_from_gate_verdicts' crates/roko-cli/src/ && cargo test -p roko-cli --lib routing_learns_only_from_gate_verdicts && cargo test -p roko-cli --lib verified_outcome_drives_output_verdict_and_feedback`

## Notes

- Persistence: `costs.jsonl` and `cascade-router.json` are append or learned state on users' machines. Add
  fields; do not rename or remove any. Existing `cascade-router.json` stays valid, because only what gets recorded
  changes.
- Keep the override path (`ModelChoiceSource::Override` → `record_override_outcome`) as it is, but give it the
  same verdict filter.
- Do not change retry or gate behaviour. This is only about what learning records.
- Parallel safety: it edits `emit_feedback` and its call sites in `graph_task_dispatch.rs`. Do not run it in
  parallel with `gap-0f3980` (routing context in the same file) or `bug-35379d` (failover attribution).
- 2026-09-29: folded into gap-8f6206, implemented on `work/gap-8f6206` at `04c1da262`; cargo verification deferred
  to the batch check (the targeted tests passed under that item's cargo exception). Option A: the routing sink
  reads the settled verdict's learning label. A pass is a success, and a gate failure, turn-cap stop or timeout
  after output is a failure. Unverified, force-accepted, provider-failed and harness-failed attempts update no
  counter, override or bandit. `costs.jsonl` rows add `outcome` and `learning_label`, and `success` keeps its
  meaning. `routing_learns_only_from_gate_verdicts` drives a pass, a gate failure, an unverified attempt, a
  transport error and exhausted usage through batch dispatch. The unwired receipt settler's routing row now skips
  `AttemptFailed` and `Cancelled` receipts.
- Still open: the provider bridge (`dispatch_v2::record_agent_dispatch_feedback`, feedback Path B) observes the
  persisted `cascade-router.json` from the provider's pre-gate `success` on every Graph dispatch, so this bug's
  headline survives there. The run-end save overwrites it after a clean Graph run, but not after a crash.

## Original notes

Both Graph dispatch paths call `emit_feedback(.., dispatch.result.success, ..)` straight after the provider call (`graph_task_dispatch.rs:1963` non-streaming, `:3318` streaming), before any verify or gate step.
The routing sink turns that flag into router observations, and `costs.jsonl` records the same flag as `success`, so a model that answers confidently but fails compile/test gates is rewarded.
Fix: settle routing feedback once per attempt after gates (including retries) with an explicit verdict (passed / failed / skipped / inconclusive) and have the router learn only from that settlement.

2026-09-29: the headline defect is fixed in 725f21e05. Routing, episodes and costs.jsonl now get the post-gate verified outcome on both Graph paths (graph_task_dispatch.rs:3891 batch, 4453 streaming). Remaining: the settlement is a bool, not an explicit passed/failed/skipped/inconclusive verdict. A provider-side failure (exhaustion, timeout, refusal) is settled at graph_task_dispatch.rs:3807 as succeeded=false, so the router still records it as a model failure.

The [[verify]] command is unsound (see the check notes). Proposed replacement, not yet validated: `cargo test -p roko-cli --lib verified_outcome_drives_output_verdict_and_feedback && ! grep -n 'dispatch.result.success,$' crates/roko-cli/src/graph_task_dispatch.rs`.
