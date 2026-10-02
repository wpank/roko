+++
id = "find-4b4344"
kind = "finding"
title = "9 gate/threshold closures wired into deleted Runner-v2"
status = "done"
triage = "verified"
severity = "p1"
size = "M"
goal = "learning"
subsystem = ["roko-gate"]
created = 2026-09-06
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "aec267cac"
source = "tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code"
discovered_from = "audit:tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code"
anchors = ["crates/roko-cli/src/runner/persist.rs::GateThresholds", "crates/roko-cli/src/graph_task_dispatch/gate_learning.rs::update_graph_gate_thresholds", "crates/roko-cli/src/graph_task_dispatch/retry_budget.rs::TaskRetryBudgets", "crates/roko-gate/src/ratchet.rs::GateRatchet", "crates/roko-gate/src/adaptive_threshold.rs::ThresholdProfile", "crates/roko-cli/src/graph_task_dispatch/feedback.rs::GraphFeedbackContext", "crates/roko-cli/src/graph_execution/plan_runner.rs", "crates/roko-learn/src/oracles/coding.rs::CodingOracle::predict_test_pass_rate"]
links = { depends_on = [], blocks = [], related = ["reg-c7ecf6", "find-34a4b5", "gap-d8c39a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "for s in observe_residual apply_profile should_skip_rung_for_temperament settle_step_regressions roko_gate_verdicts_total suggested_max_retries; do grep -rq \"$s\" crates/roko-cli/src/graph_task_dispatch.rs crates/roko-cli/src/graph_task_dispatch crates/roko-cli/src/graph_execution crates/roko-cli/src/runner/persist.rs || exit 1; done && grep -rqw 'fn graph_verify_feeds_gate_thresholds' crates/roko-cli/src/ && cargo test -p roko-cli graph_verify_feeds_gate_thresholds"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T13:44:22Z"
by = "coordinator (session 7622b882)"
size = "M"
claimed_at = "2026-10-01T09:06:21Z"
forced = false
evidence = "Batch 20e gate on 1a8aad603, re-checked with the compile fixes (cf722c1be, bed29287d), tiers' rustfmt (8a6c932ce) and the run-index scrub fix (d972959bd) on 32fe02384; MAIN aec267cac has the same crates: check --workspace --tests, nightly fmt and clippy -D warnings clean on roko-acp/agent/cli/core/dreams/fs/gateway/graph/learn/neuro/serve/std; lib tests roko-cli 3305, roko-agent 2294, roko-core 1962, roko-learn 1213, roko-serve 991, roko-graph 478, roko-fs 260, roko-neuro 239, roko-std 229, roko-acp 199, roko-dreams 100, roko-gateway 42 all pass; extras: golden_path_suite 2/2, all eight canaries pass (secret_canary 11/11 and C2 2/2 after the scrub fix), worktree_task_diff 2/2, plan_run_config_flag 1/1, default_engine 1, bin 429, routing crash loop 10/10, bench driver 18, including graph_verify_feeds_gate_thresholds and the gate_learning tests; P1-08/P1-36 already wired, P1-10/P1-11/P1-12/P3-15 wired, P2-22 emits metrics (registry gap filed as gap-d8c39a), P1-35/P1-39 dropped. Merged 351aa5c8f (work/find-4b4344 5bec5149f)."
+++

## Problem

The 2026-09-06 cybernetic audit closed nine gate-learning items by wiring them into the Runner-v2
event loop (`runner/event_loop.rs`) and the gate pipeline entry point `run_gate_once`
(`runner/gate_dispatch.rs`). Runner-v2 was deleted on 2026-09-06 (6b5da8616). `run_gate_once` is
still in the tree, but only tests reach it. The Graph engine, the only plan executor, re-attached
none of these nine.

As a result, a `roko plan run` updates per-rung pass-rate EMAs in `.roko/learn/gate-thresholds.json`
and nothing else. No residual tightening, no domain priors, no regression ratchet, no skip advisory,
no gate metrics and no retry recommendation.

The nine closures, with what each did:
- P1-08: after a gate verdict, call `observe_residual(rung, predicted - actual)` so that thresholds
  tighten when predictions are over-optimistic.
- P1-10: seed rung priors from a `ThresholdProfile` chosen by task role or domain (`apply_profile`).
- P1-11: `GateRatchet` records the highest rung passed per plan and warns when a lower rung later
  fails.
- P1-12: before a gate runs, consult `should_skip_rung_for_temperament(rung, temperament)`, and skip
  it after a long pass streak with periodic probes.
- P1-35: Symbol-gate rung oracles, by setting `source_roots` in `build_rung_execution_inputs`.
- P1-36: observe each inner gate's verdict under its own rung.
- P1-39: when a rung's EMA converges, record `Evolved` config provenance.
- P2-22: emit the `roko_gate_verdicts_total` counter and the `roko_gate_duration_seconds` histogram.
- P3-15: at plan completion, compare `suggested_max_retries` per rung with the configured retries and
  log a recommendation.

## Why it matters

Goal `learning` (Learning loops on the Graph path). Gate history is collected but never used, so
verification does not adapt to the workspace. Gate metrics on `/metrics` stay at zero during Graph
runs.

Related items:
- `find-34a4b5`: 16 learning and routing closures, the same failure mode;
- `reg-c7ecf6`: `learning.gate_threshold_flush_interval` is inert; the Graph path saves after every
  task;
- `gap-7a3527`: the `gates.ema_alpha` and adaptive retry keys are dead;
  `AdaptiveThresholds::from_gates_config` has no caller.

## Where

- `crates/roko-cli/src/runner/persist.rs::GateThresholds` (:278) is the threshold type the Graph
  path uses. It has:
  - `observe` (:337, alpha fixed at 0.1);
  - `observe_residual` (:363);
  - `apply_profile` (:397);
  - `suggested_max_retries` (:412);
  - `should_skip_rung_for_temperament` (:437).
  All of these except `observe` carry `#[allow(dead_code)]` with "production caller not yet
  connected".
- `crates/roko-gate/src/adaptive_threshold.rs` holds the second threshold type,
  `AdaptiveThresholds`, which has CUSUM/SPC, convergence and `Evolved` (:798-826). It also holds
  `ThresholdProfile` (:93), with `coding()`, `research()`, `security()` and `by_name()`. Graph does
  not construct `AdaptiveThresholds`.
- `crates/roko-gate/src/ratchet.rs::GateRatchet` provides `load_or_new`, `record_pass(plan_id,
  rung)`, `highest_pass`, `can_regress` and `save`. It is only re-exported (`roko-gate/src/lib.rs:184`).
- `crates/roko-cli/src/runner/gate_dispatch.rs` has the old metric emission (:1541-1552, as tracing
  `monotonic_counter.` and `histogram.` fields) and `build_rung_execution_inputs` (:1781). Both are
  reached only from `run_gate_once` (:876).
- `crates/roko-cli/src/graph_task_dispatch.rs` is the Graph verify loop:
  - the steps are mapped to rungs by `rung_for_gate_name`;
  - the `CodingOracle` observations are at :1963-1990;
  - the "P2-LRN-6 Loop 1" threshold update is at :2203-2260 (load, then `observe` per step, then
    save, then the TUI notice);
  - `GraphFeedbackContext` is at :1012.
- `crates/roko-learn/src/oracles/coding.rs::CodingOracle::predict_test_pass_rate` (:118) is a
  success prediction that exists on the Graph path.
- `crates/roko-cli/src/graph_execution/plan_runner.rs` builds `GraphFeedbackContext` (about
  :1052-1070) and completes plans (`plan_completed`, :1377, :1765).
- Entry point: `roko plan run <dir>`.

## Current state

- P1-36 is covered in spirit. Graph observes each verify step under its own rung
  (`graph_task_dispatch.rs:2230-2237`), so `gate-thresholds.json` gets separate rung entries.
- P1-08, P1-10, P1-11, P1-12 and P3-15: the helpers exist and are dead code on every production
  path.
- P2-22: the metric names are registered as standard metrics (`roko-core/src/obs/metrics.rs:871`),
  but only `run_gate_once` emits them.
- P1-35: Graph verify runs the tasks' own verify commands, not the rung pipeline, so the Symbol
  gate never runs on Graph.
- P1-39: `GateThresholds` has no convergence detection. `Evolved` exists only on
  `AdaptiveThresholds`.
- `suggested_max_retries` feeds only displays (`tui/dashboard.rs:3153`,
  `tui/dashboard_model.rs:959`, `commands/util.rs:1067`).
- The TUI parses `gate-thresholds.json` as `AdaptiveThresholds` (`tui/dashboard.rs:586`), while
  Graph writes `GateThresholds`. Keep the two shapes compatible if you add fields.

## Plan

The design choice is which threshold type Graph uses:
- (A) Keep `runner::persist::GateThresholds` and wire its existing helpers. This is small and uses
  code that already exists.
- (B) Move Graph to `roko_gate::AdaptiveThresholds`, which gives convergence, `Evolved` and
  `from_gates_config`. This is larger, and it changes the file format.
Recommend (A) now, and file (B) separately if P1-39 is wanted.

With (A), first move the threshold block at :2203-2260 into a testable function, for example
`update_graph_gate_thresholds(thresholds, task, step_outcomes, predictions, ratchet)`. Then:
1. P1-10: before observing, call `thresholds.apply_profile(&ThresholdProfile::by_name(domain))`,
   using the task's `domain` (`TaskDef::domain`) or role, defaulting to `coding()`.
2. P1-08: in the verify loop, read `oracle.predict_test_pass_rate().0` before `observe_test`, and
   keep it with the step. For test rungs, call `thresholds.observe_residual(rung, predicted -
   actual)`.
3. P1-11: load a `GateRatchet` from `.roko/learn/` (new file, for example `gate-ratchet.json`) and
   call `record_pass(plan_id, rung)` on each pass. When a failing rung is below `highest_pass`, log a
   regression warning and add a TUI notice. Save it next to the thresholds.
4. P2-22: for each verify step, emit the same two tracing metric fields that `gate_dispatch.rs`
   emits (:1541-1552).
5. P3-15: at plan completion in `plan_runner.rs`, load the thresholds and, for each rung, log
   `suggested_max_retries(rung)` against the task `max_retries` when they differ.
6. P1-12 is a decision. Skipping an authored verify command would let an unverified change pass,
   which goes against the fail-closed rule. Recommend advisory only: compute
   `should_skip_rung_for_temperament(rung, temperament)` from the dispatcher's daimon state, log
   "would skip" and count it, but always run the step. Actually skipping needs an explicit decision
   by the owner.
7. P1-35 and P1-39: there is no Graph equivalent under (A). Record them as not applicable, and
   remove `run_gate_once`-only claims from the docs if any remain.
8. Remove the `#[allow(dead_code)]` attributes that the wiring makes stale.

## Done when

- After a Graph run of a two-task plan with verify steps:
  - `gate-thresholds.json` reflects profile priors and residual tightening;
  - the ratchet file records the highest rung passed;
  - a regression warning is logged when a lower rung fails after a higher one passed;
  - `roko_gate_verdicts_total` is non-zero on `/metrics` when `roko serve` hosts the run;
  - a retry recommendation is logged at plan end when it differs.
- A unit test covers the extracted update function.
- Verify (suggested replacement: the old grep passes on a comment, and it forces skip-wiring):
  `for s in observe_residual apply_profile should_skip_rung_for_temperament GateRatchet roko_gate_verdicts_total suggested_max_retries; do grep -rq "$s" crates/roko-cli/src/graph_task_dispatch.rs crates/roko-cli/src/graph_execution || exit 1; done && grep -rqw 'fn graph_verify_feeds_gate_thresholds' crates/roko-cli/src/ && cargo test -p roko-cli graph_verify_feeds_gate_thresholds`

## Notes

- Do not change which verify steps run, or their pass/fail meaning. This item only adds learning,
  logging and metrics around them.
- Keep the per-task load-then-save pattern, or fix `reg-c7ecf6` in the same place. Do not add a
  second writer for `gate-thresholds.json`.
- `gap-7a3527` wants to wire `gates.ema_alpha` into `GateThresholds::observe`. Do these two items in
  sequence, not in parallel: both edit `runner/persist.rs` and the same block in
  `graph_task_dispatch.rs`.
- `graph_task_dispatch.rs` is large and heavily edited. Keep the diff local to the verify and
  feedback block, and to plan completion in `plan_runner.rs`.

### Status of each closure (2026-10-01, on work/find-4b4344)

The threshold block in `graph_task_dispatch/verification.rs` moved to
`graph_task_dispatch/gate_learning.rs`. `update_graph_gate_thresholds` is the testable function, and
`GraphTaskDispatcher::settle_gate_learning` does its file I/O, logging and dashboard notices. Verify runs keep the
load-then-save pattern on `gate-thresholds.json`, which still has one writer. A process-wide lock now serializes
that load-update-save and the ratchet's, so parallel tasks of a run no longer drop each other's updates.

- **P1-08, residual: wired** (ce3bdcbb8, before this branch). `GateThresholds::observe_verify_steps` feeds
  `observe_residual` with the CodingOracle forecast taken before the steps run.
- **P1-10, profile priors: wired.** `apply_profile` runs before each observation, with the task's profile:
  `research` for `domain = "research"` or a researcher, strategist or pre-planner role; `security` for a
  security-reviewer or security role; `coding` otherwise. These are Runner-v2's role mappings, plus the domain.
  `observe` replaces a rung's prior with its first observation, so the priors show only on rungs that have no
  observations yet. They change neither retry budgets nor skip advice, since both need at least five observations.
- **P1-11, ratchet: wired.** `.roko/learn/gate-ratchet.json`, beside the thresholds, records the highest rung passed
  per `plan_id/task_id`. The key is per task, not per plan, because Graph retries tasks and a plan's tasks have
  different verify steps. A failed rung below the highest one already recorded logs a warning and adds a
  `gate_regression` entry to the dashboard event log (`TuiBridge::gate_regression`). Regressions are judged
  against the ratchet as it stood before the attempt, then the attempt's passes are recorded. The ratchet only warns,
  and it persists across runs of the same plan.
- **P1-12, skip advisory: wired, advisory only.** `should_skip_rung_for_temperament` is read before the attempt's
  observations, with the role's configured temperament (`agent.temperament_for_role`, the one dispatch uses), not
  daimon affect, which is on hold (dec-e70592). The log names the rungs it would skip and how many of them failed.
  Every step still runs. Actually skipping an authored verify step needs the owner's decision (fail-closed).
- **P1-35, Symbol-rung oracles: dropped (not applicable).** Graph verify runs the tasks' own commands, not the
  rung pipeline, so `build_rung_execution_inputs` has nothing to feed.
- **P1-36, inner-gate thresholds: covered.** Each verify step is observed under its own rung.
- **P1-39, Evolved provenance: dropped (not applicable under option A).** `GateThresholds` has no convergence
  detection. Moving Graph to `AdaptiveThresholds` (option B) would bring it; nobody has asked for that, so no item
  was filed.
- **P2-22, gate metrics: tracing wired, `/metrics` blocked.** Each verify step, including the post-auto-fix
  re-run, emits the same `monotonic_counter.roko_gate_verdicts_total` and `histogram.roko_gate_duration_seconds`
  tracing fields as `run_gate_once`. No layer turns those fields into registry samples, and serve's
  `run_plan_on_local_runtime` drops its `MetricRegistry`, so `/metrics` stays at zero. Filed as gap-d8c39a.
- **P3-15, retry recommendation: wired, at plan load.** Live retry budgets (ce3bdcbb8) already follow the
  thresholds for tasks that do not author `max_retries`. For an authored budget, `TaskRetryBudgets::max_retries`
  now logs what the thresholds would set when it differs and its rung has at least five observations. This runs
  when the plan is loaded, where budgets are decided, rather than at plan completion, which would have needed a
  hook in the hot `plan_runner.rs`. The suggestion is the live budgets' own (`AdaptiveThresholds` within
  `[gates] adaptive_*_retries`). `GateThresholds::suggested_max_retries`, a dead copy with fixed bounds, was
  removed.
- The `#[allow(dead_code)]` markers on `apply_profile` and `should_skip_rung_for_temperament` are gone.

Checked statically only (grep part of the verify, `cargo +nightly fmt`). `cargo test -p roko-cli
graph_verify_feeds_gate_thresholds` has not run on this branch.

## Original notes

Checked done 2026-09-06 via runner/event_loop.rs or gate_dispatch.rs: P1-08 observe_residual, P1-10 domain profiles, P1-11 GateRatchet, P1-12 skip advisory, P1-35 symbol rung oracles, P1-36 inner-gate thresholds, P1-39 Evolved provenance, P2-22 Prometheus gate metrics, P3-15 retry alignment.

Imported without verification from:
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P1 -- Wire Existing Code`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P2 -- Add Missing Observability`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P3 -- Improve Existing Mechanisms`
- `tmp/archive/cybernetic-audit/30-master-checklist.md#P0 -- Close Broken Feedback Loops`

A source claims this was fixed; confirm against current code before closing.

How to verify: Confirm Graph gate path (graph_execution/ or roko-graph verify cells) calls these; check .roko/learn/gate-thresholds.json changes after a Graph run. / grep these symbols for non-test call sites reachable from Graph plan execution.

Merged 2 mined candidates: m3-031, m3-032.

Verified 2026-09-28: rung_for_gate_name is used on the Graph path (graph_task_dispatch.rs:2030). But GateThresholds::observe_residual (crates/roko-cli/src/runner/persist.rs:363), apply_profile (:397) and should_skip_rung_for_temperament (:437) have no production callers, and GateRatchet (roko-gate/src/ratchet.rs) is only re-exported. build_rung_execution_inputs and update_gate_threshold no longer exist, though CLAUDE.md still cites the former. suggested_max_retries feeds only displays (tui/dashboard.rs:3154, commands/util.rs:1072), not retry policy. The import warning was wrong: crates/roko-cli/src/runner/gate_dispatch.rs exists.

Checked 2026-09-29 at d9e79e9d8: unchanged. Correction: build_rung_execution_inputs still exists (crates/roko-cli/src/runner/gate_dispatch.rs:1781) but is called only from run_gate_once (:1192, :1335), which only tests reach. A grep of graph_task_dispatch.rs and graph_execution/ found no reference to Prometheus gate metrics, Evolved threshold provenance, symbol-rung oracles or inner-gate thresholds either.

Checked 2026-09-29: Partly fixed in ce3bdcbb8: residual observation (runner/persist.rs:401) and retry budgets (graph_task_dispatch/retry_budget.rs) are live on the Graph path. Still not reached from it: apply_profile, should_skip_rung_for_temperament, GateRatchet and roko_gate_verdicts_total. The verify now also searches runner/persist.rs and the graph_task_dispatch/ directory.

2026-10-01 (wk-gates, for gap-6dba88): P1-11 moved from the rung-ordered `GateRatchet` (`gate-ratchet.json`) to a
per-step history in the retry feedback book (`graph_task_dispatch/step_ratchet.rs`). The rung ratchet flagged a step
that had never passed whenever a higher rung had passed on an earlier attempt. The `[[verify]]` now greps
`settle_step_regressions` instead of `GateRatchet`.
