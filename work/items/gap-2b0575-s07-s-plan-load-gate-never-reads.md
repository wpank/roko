+++
id = "gap-2b0575"
kind = "gap"
title = "S07's plan-load gate never reads spec.refine_requested, so the self-model's refine signal has no consumer"
status = "open"
triage = "verified"
severity = "p3"
goal = "cybernetic"
size = "M"
subsystem = ["roko-gate/spec-quality", "roko-cli/graph-task-dispatch"]
created = 2026-10-04
updated = 2026-10-04
last_verified = 2026-10-04
source = "wave-10 follow-up reports 2026-10-04 (PK66 gap-414e56)"
discovered_from = "gap-414e56"
anchors = ["crates/roko-cli/src/graph_task_dispatch/self_model.rs", "crates/roko-gate/src/spec_quality.rs"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn s07_enforce_mode_acts_on_spec_refine_requested' crates/roko-gate/ && cargo test -p roko-gate s07_enforce_mode_acts_on_spec_refine_requested"
+++

## Problem

The self-model's post-pass step can write a `spec.refine_requested` event (`crates/roko-cli/src/graph_task_dispatch/self_model.rs:21,105,764,1605`,
`SPEC_REFINE_EVENT`) when it judges a task's spec itself needs refining rather than deeper verification. Nothing
on the consuming side reads it: S07's plan-load/spec-quality gate (`crates/roko-gate/src/spec_quality.rs`, S07
§4.2/§5) doesn't look for `spec.refine_requested` at all, so the event is logged and nothing acts on it.

## Why it matters

Goal: cybernetic, M3/S07 interface. A self-model that correctly diagnoses "this task's spec, not its
verification, is the problem" currently has no way to make that diagnosis change what actually happens next —
the signal is recorded for later analysis only, not consulted by the gate that could act on it (e.g. refusing to
proceed, or flagging the task for a spec rewrite, when running in an "enforce" mode that treats refine requests
as binding rather than advisory).

## Where

- `crates/roko-cli/src/graph_task_dispatch/self_model.rs` (the producer: `SPEC_REFINE_EVENT`, `post_pass_action`).
- `crates/roko-gate/src/spec_quality.rs` (the consumer that should read it; S07 owns the enforce-mode behavior).

## Current state

Confirmed: the event is written; nothing reads it on the gate side.

## Plan

1. Have S07's plan-load gate read `spec.refine_requested` events for a task and act according to whatever mode
   it's running in (log-only vs. enforce) — the exact enforce-mode behavior (block, warn, require a human) is
   S07's own design decision, not fixed here.

## Done when

- A task whose self-model post-pass wrote `spec.refine_requested` has that reflected in S07's gate behavior under
  enforce mode.
- The `[[verify]]` command passes.

## Notes

- Discovered during PK66's work (gap-414e56, done).
- Related but distinct from bug-78e5ce (the streaming dispatch path skips the self-model's post-pass step
  entirely, so it can't even produce this event there) — that item already has a dated note about this
  connection; this item is about the plan-load gate not consuming the event once it IS produced.

## Progress

- 2026-10-05 (wave 20): implemented at d52ae77bb on `work/gap-2b0575`; cargo verification deferred to the batch
  gate. roko-gate's `spec_quality::refine` reads `spec.refine_requested` records (`refine_requests`,
  `read_refine_requests` over `.roko/runs/*/spec.jsonl`), and `refine_verdict` decides per task. Under `enforce`,
  an open request from a self-model that routed (active, calibration gate eligible, breaker untripped) blocks
  the task: S07 §4.3 sends a weak spec to REFINE, and with no refiner in roko the task waits for its spec to be
  refined. Under `advise`, or from a self-model that only forecast (shadow mode, or before its calibration gate
  holds), the request is advice. A request stays open until the task's spec scores higher than the request's
  `spec_score`. `spec_gate::gate_plans` applies the verdicts (`apply_refine_requests`): a `refine_requested`
  finding that blocks, or `advice` on the decision, its `spec.gate` record and its event line; the holdout
  drops both. The producer now writes `acting`. Tests: `s07_enforce_mode_acts_on_spec_refine_requested`
  (roko-gate), `spec_gate_acts_on_the_self_models_refine_requests` (roko-cli), and 6133's producer test now
  reads its record with the gate's reader.
- Streaming: the request is written at forecast time (`report_self_model_action`, from `forecast_attempt`), not
  by the post-pass step, so the bug-78e5ce note's cause does not apply. Streaming calls `forecast_attempt`
  since bug-78e5ce, and reaches post-pass through `settle_task_verification` → `deepen_verification`, so both
  paths write the request.
- Left as is: `plan run`'s early pre-check (`spec_gate_before_run`) does not read requests; the plan-load gate,
  which every run passes before its first dispatch, does.
