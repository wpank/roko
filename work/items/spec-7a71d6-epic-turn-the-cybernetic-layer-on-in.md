+++
id = "spec-7a71d6"
kind = "spec"
title = "Epic: turn the cybernetic layer on in safe stages"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
size = "L"
subsystem = ["roko-cli/graph_execution", "roko-cli/runtime_feedback", "roko-learn/loop_audit", "roko-learn/homeostasis", "roko-learn/self_model", "roko-gate/audit", "roko-core/config"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#decision-2"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::build_graph_feedback_context", "crates/roko-cli/src/runtime_feedback/homeostasis.rs::for_workdir", "crates/roko-core/src/config/self_model.rs::SelfModelMode", "crates/roko-learn/src/loop_audit/mod.rs::beta_fields", "crates/roko-gate/src/audit/feedback.rs", "crates/roko-core/src/config/audit.rs::AuditConfig"]
lane = "rust-hot"
links = { depends_on = ["gap-d76b1b"], blocks = [], related = ["q-6b7cca", "gap-d76b1b", "q-394b7f", "spec-c6e21b", "gap-e464e6", "gap-0132c2"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_plan_run_emits_plan_completed_once_per_plan' crates/roko-cli/ && grep -rqw 'fn default_plan_run_registers_m1_in_shadow' crates/roko-cli/ && grep -rqw 'fn self_model_config_defaults_shadow' crates/roko-core/ && grep -rqw 'fn census_estimates_beta_once_n_beta_opportunities' crates/roko-learn/ && grep -rqw 'fn ladder_holds_below_the_minimum_effective_sample' crates/roko-gate/ && grep -rqw 'fn default_audit_config_is_enabled_at_the_decided_rate' crates/roko-core/"
+++

## Problem

The cybernetic layer is built but does not act on a default plan run (status-quo assessment 2026-10-15, HEAD
`7b6a5af44`; `tmp/status-quo/2026-10-15/NEW-FINDINGS.md` findings 10-21):

- Graph runs never emit `FeedbackEvent::PlanCompleted`, so no plan-completion sink fires (finding 20, q-6b7cca).
- M1 never registers: `HomeostasisSink::for_workdir` returns `None` without `.roko/policy/viability.toml`, and nothing
  writes that file (finding 11; `backlog/A09-81-m1-controller.md` §3).
- M3 is `off` by default (`crates/roko-core/src/config/self_model.rs`), never fitted, and its calibration is never
  exported or kept across runs (finding 14; `backlog/A07-61-m3-self-model.md` §4).
- M2 never estimates benefit β, so no loop can go live or be demoted (finding 10; `backlog/A06-51-m2-liveness.md` §4).
- M4 is off (`[audit] enabled = false`), and switching it on as-is would push every task type up the strictness
  ladder on sampling noise (finding 15, gap-d76b1b, q-394b7f; `backlog/A08-71-m4-deep-audits.md` §4).
- Loops on the plan path are half-closed: no playbooks are created on plan runs (16), error patterns leak across plans
  (6), the router learns nothing under the ladder (17), L-sec and the placebo never reach the auditor (18), a stale
  prompt block reads a file nothing writes (19), M4's audits never reach M1's E3 (12), B4's `knowledge_section` knob
  has no reader (13).

## Why it matters

Will's decision 2 (2026-10-15, `tmp/status-quo/2026-10-15/DECISIONS-2026-10-15.md`): turn the layer on in safe
stages; the papers stay as written. The papers describe these loops as running (paper 8-39 "the forecaster runs in
shadow first", 9-30 "the controller starts in shadow", 9-16/9-20 benefit and transitions;
`papers/D5-paper-07-09.md`, `papers/D7-paper-appendices.md` A.8-A.11), so each stage also closes a paper-vs-code gap.

## Where

The children's anchors. Entry point: `roko plan run` / `roko run plans/<dir>` through
`crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan_body` and `build_graph_feedback_context`.

## Current state

Every library piece exists and is unit-tested (A05-A09 count 35/35 M1 tasks, 33/35 M2 tasks DONE). What is missing is
emission, defaults, production estimation and the audit floor. No M1 state, viability policy, self-model state,
`predictions.jsonl` or `calibration.json` exists in this workspace, `~/.roko` or the live scratch workspaces (A07, A09).

## Plan

Stages, in order. Items within a stage can run in parallel; an item's `depends_on` names the earlier-stage items it
needs.

1. **Emit and wire** (no new behaviour switched on by default beyond the fixes):
   - gap-a960a1 emit `PlanCompleted` once per plan; keep only the daimon sink (q-6b7cca option 2).
   - bug-56e072 key error patterns by plan as well as task.
   - gap-89f985 create playbooks from verified plan tasks.
   - gap-6bb6d8 give B4's `knowledge_section` knob a reader.
   - bug-dd4524 retire the stale "Prompt section effectiveness" block.
   - gap-6f31c3 bring L-sec and L-placebo into the auditor's census.
2. **Shadow defaults** (observe, never act):
   - gap-cad57a a default S5 viability policy, so M1 runs in shadow on a default plan run.
   - gap-9aca5d `[self_model] mode` defaults to `shadow`.
3. **Measure**:
   - gap-03fcd0 estimate loop benefit β in the census.
   - gap-b38ed2 keep M3's calibration window across runs and export `calibration.json`.
   - gap-96547f `roko learn self-model fit` reads `predictions.jsonl` and reports the join share.
   - gap-618548 feed M4's audit verdicts into M1's E3.
   - gap-51d2ca let the cascade router learn on ladder-routed plan runs (decision 4111 option B, after M3 calibrates).
4. **Enforce**:
   - bug-1e97dc the audit strictness ladder ignores windows below a minimum effective sample (q-394b7f; T1 decides floor
     vs. pooled prior), then gap-d76b1b turns audits on at 10% (decision 7103).
   - Loop-audit `enforce = true` and M1 `on` stay off until Will says so; no item is filed for them yet.

## Done when

- Every child is closed (done, or wontfix/superseded with a reason), and gap-d76b1b is done.
- The `[[verify]]` exit check passes: the key children's named tests exist.
- One default `roko plan run` on a fresh `roko init` workspace (fake provider, $0) leaves a `PlanCompleted`-driven
  daimon save when affect is on, `learn/homeostat-state.json`, `learn/self-model/state-v1.json`,
  `learn/self-model/calibration.json`, `predictions.jsonl` rows, and `loop.health` rows for L-sec and L-placebo.

## Notes

- Stage 4's loop-audit enforce and M1 `on` are Will's calls (decision 2); do not flip them inside this epic.
- Dreams, affect, the conductor tick and LinUCB stay held (decision 12): gap-a960a1 keeps the daimon sink only for
  workspaces that turn affect on.
- The related open items: q-6b7cca, gap-d76b1b, q-394b7f, gap-e464e6 (L1 forecaster swings), spec-c6e21b (M2 phase
  epic), gap-0132c2 (C4 power).
