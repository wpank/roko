+++
id = "gap-460230"
kind = "gap"
title = "Verify-then-escalate: two failed attempts move a task one rung up the ladder"
status = "done"
triage = "verified"
severity = "p1"
goal = "golden-path"
size = "M"
subsystem = ["roko-cli/graph_task_dispatch", "roko-cli/dispatch"]
created = 2026-09-29
updated = 2026-09-30
last_verified = 2026-09-30
last_verified_rev = "90307ad5e"
source = "tmp/cybernetic-harness/workstreams/PLAN.md#e5"
discovered_from = "tmp/cybernetic-harness/tldr/04-FRONTIER-PLANS-CHEAP-EXECUTES.md (step 8, design rule 5); specs/S04-self-model-routing.md §4.4 (baseline H4-B1)"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher::dispatch", "crates/roko-cli/src/graph_task_dispatch/streaming.rs::dispatch_streaming", "crates/roko-cli/src/graph_task_dispatch/retry_feedback.rs::NextAttempt", "crates/roko-cli/src/graph_task_dispatch/retry_budget.rs::TaskRetryBudgets", "crates/roko-cli/src/dispatch/model_routing.rs::ModelRouter::route", "crates/roko-cli/src/graph_task_dispatch/ladder.rs::GraphTaskDispatcher::note_ladder_outcome"]
lane = "rust-hot"
parent = "spec-98f76d"
links = { depends_on = ["gap-9cbf35", "gap-96f7ed"], blocks = [], related = ["gap-b62e95", "bug-35379d"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn two_failed_attempts_escalate_one_rung' crates/roko-cli/src/ && cargo test -p roko-cli --lib two_failed_attempts_escalate_one_rung"

[[verify]]
command = "grep -rqw 'fn escalation_rung_survives_resume' crates/roko-cli/src/ && cargo test -p roko-cli --lib escalation_rung_survives_resume"

[closed]
at = 2026-09-30
by = "coordinator (session 7622b882)"
evidence = "Merged in ce12e86d8. Two agent-blamed failures move a task one runnable rung up the ladder, at most twice; pinned models never move; the standing survives resume in retry-feedback.json; each verdict records its ladder rung, step and reason; ladder_exhausted posts one diagnosis; unhinted tasks get a 5-retry floor. Batch 17 gate: first run on 2c4abe35b (check clean; lib tests roko-agent 2271, roko-cli 3244 (gate_rows writer flake, fixed by bug-779ae7), roko-core 1955, roko-fs 260, roko-learn 1204, roko-serve 988), then re-gated on 53feea92e (same code as MAIN 90307ad5e) after the coordinator's doc-paragraph and rustfmt fix on guard2's branch (3f3a7be84): nightly fmt clean; clippy -p roko-agent -p roko-cli -p roko-core -p roko-fs -p roko-learn -p roko-serve --keep-going -D warnings clean; roko-fs lib 260; --test secret_canary 11 passed; --test secrets_and_git_guard_canary 2 passed. Verify: its test passes in that run and its static checks pass on MAIN."
+++

## Problem

When a task fails its verify step, the Graph engine retries it on the same model, with the gate feedback:

- `dispatch()` passes the attempt number in `DispatchContext.attempt`, but `ModelRouter::route` never reads it.
- Runner-v2's `RetryWithEscalation` and fatigue escalation were deleted with it in `6b5da8616`. The leftovers have no
  Graph caller: `TaskComplexityBand::escalate`, `MODEL_ESCALATION_LADDER`, `ModelRegistry::upgrade_tier`.
- Provider failover (`run_bridge_with_failover`) moves sideways, and only on refusals.

## Why it matters

tldr/04 design rule 5: retry twice cheaply, then escalate one rung, then split or replan. Without it a cheap model
that cannot do a task spends the whole retry budget failing, and the benchmark's cheap arm cannot recover. S04 §4.4
uses this rule (start low, escalate only on failure, K_max = 2) as its baseline H4-B1. Step 5 of epic spec-98f76d.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`: `dispatch` numbers the attempt (`next_retry_attempt`, :3614) and builds
  the `DispatchContext` (:3650); `dispatch_streaming` sends `attempt: 0` and no gate feedback (:4403–4418).
- `graph_task_dispatch/retry_feedback.rs`: `NextAttempt` and the per-plan book `retry-feedback.json`.
- `graph_task_dispatch/retry_budget.rs::TaskRetryBudgets` and `dispatch/model_routing.rs::ModelRouter::route`.

## Current state

Checked at `41c7ffbd6`. Since `ce3bdcbb8` attempt numbers follow the engine's retry counter and survive
`--resume-plan`, with pending gate feedback kept in `retry-feedback.json`. Since `99adacd6d` and `41c7ffbd6`, a task
with no authored `max_retries` gets 3–5 retries. Since `d4be4e872` a timeout is retried with 1.5× the time and a
turn-cap stop with a raised cap. None of this changes the model.

## Plan

1. Count agent-blamed failures per task: a failed verify, a turn-cap stop or a timeout. Infrastructure failures
   (refusals, rate limits) never count; they stay with failover. Keep `rung` and `failures_on_rung` in the retry book,
   so they survive resume.
2. The next attempt's rung is the start rung (tier and role, or the task's `rung` hint once gap-dbf2a6 lands), plus one
   after every second counted failure: at most K_max = 2 escalations, never past the top. Pass it in
   `DispatchContext` on both dispatch paths; `ModelRouter::route` returns that rung's model.
3. Pins never escalate: `--model` and a slug `model_hint` keep their model, and the log says so.
4. With the ladder on, a task with no authored `max_retries` gets at least 2 × (K_max + 1) − 1 = 5 retries (today's
   `adaptive_max_retries`). An authored value is kept exactly, as in `99adacd6d`; the per-task USD ceiling still binds.
5. When a task fails on the top rung and runs out of retries, record `ladder_exhausted` as the reason and publish one
   `DashboardEvent::Diagnosis` suggesting a split or replan. No automatic split.
6. Record each attempt's rung, model and reason (`start`, `hint`, `escalated`, `pinned`) on gap-96f7ed's attempt
   record and in the dispatch log.
7. Tests: `two_failed_attempts_escalate_one_rung` (attempts 0–1 on rung 0 and 2–3 on rung 1; a pin never moves; an
   infrastructure failure does not count) and `escalation_rung_survives_resume`.

## Done when

- [ ] A task that fails verify twice runs its third attempt one rung up, on both dispatch paths; a pin never moves.
- [ ] Every attempt records its rung, model and reason.
- [ ] Both `[[verify]]` commands pass.

## Notes

- Waits for gap-9cbf35 (the ladder) and gap-96f7ed (E4.2: one settled outcome per attempt, with its failure class).
- gap-b62e95 follows and fills `RoutingContext.iteration` and `has_prior_failure` from the same counters.
- Out of scope (S04.T12): low-confidence escalation, D11's "deepen verification before escalating", a learned start.
- **Hot file:** `graph_task_dispatch.rs`.
- **Decided 2026-09-29 (Will):** while the ladder is on, tasks without an authored `max_retries` get 5 retries, so two escalations fit (D11).
- 2026-09-30 (wk-tiers): implemented on `work/gap-460230` at `af0cb7f42`: `graph_task_dispatch/ladder.rs`, with the standing in `retry_feedback.rs` and the retry floor in `retry_budget.rs`. Both `[[verify]]` tests pass in the worktree, and `two_failed_attempts_escalate_one_rung_when_streaming` covers the streaming path; workspace verification is deferred to the batch check. A task with a `model_hint` never climbs, so it keeps its own retry budget, and `--model` turns the floor off. Batch 16 (`c0be49aa8`, after merging gap-0f3980): an attempt that a `rung` hint (gap-dbf2a6) started records `hint`, and a `preferred_model` pins the model like a `model_hint`.
