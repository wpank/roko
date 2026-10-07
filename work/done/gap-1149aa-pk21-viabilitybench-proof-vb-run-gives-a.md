+++
id = "gap-1149aa"
kind = "gap"
title = "PK21 ViabilityBench proof: vb run gives a multi-model arm one endpoint and one proxy upstream per provider (+5 more)"
status = "done"
triage = "verified"
severity = "p1"
goal = "proof"
rank = 21
size = "L"
subsystem = ["benchmarks/viabilitybench/driver"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
last_verified_rev = "6a5a0a1cb"
source = "tmp/backlog/2026-10-02-complete-and-wire PK21"
anchors = ["benchmarks/viabilitybench/analysis/metrics.py", "benchmarks/viabilitybench/driver/planemit.py", "benchmarks/viabilitybench/driver/records.py", "benchmarks/viabilitybench/driver/run_roko.py", "benchmarks/viabilitybench/driver/vb.py", "benchmarks/viabilitybench/experiments/budget.toml"]
lane = "bench"
parent = "spec-fef7c5"
links = { depends_on = ["gap-5ebb4f"], blocks = [], related = ["gap-1cd676", "gap-f30b8e"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'def test_multi_provider_arm_gets_one_upstream_per_provider' benchmarks/viabilitybench/driver/test_vb_providers.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_vb_providers.py -k test_multi_provider_arm_gets_one_upstream_per_provider -q"

[[verify]]
command = "grep -qw 'def test_routed_attempts_accept_escalation_and_reject_failover' benchmarks/viabilitybench/driver/test_run_roko_routed.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko_routed.py -k test_routed_attempts_accept_escalation_and_reject_failover -q"

[[verify]]
command = "grep -qw 'def test_pilot_c_manifest_rehearses_offline' benchmarks/viabilitybench/experiments/test_pilot_c.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/experiments/test_pilot_c.py -k test_pilot_c_manifest_rehearses_offline -q"

[[verify]]
command = "test -d benchmarks/viabilitybench/.venv-msa && grep -qw 'def test_msa_runner_meters_through_the_proxy' benchmarks/viabilitybench/driver/test_run_msa.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_msa.py -k test_msa_runner_meters_through_the_proxy -q"

[[verify]]
command = "grep -qw 'def test_fr_claude_arm_prices_cli_usage_from_verdicts' benchmarks/viabilitybench/driver/test_fr_claude.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_fr_claude.py -k test_fr_claude_arm_prices_cli_usage_from_verdicts -q"

[[verify]]
command = "grep -qw 'def test_roko_plan_runner_records_planner_and_executor_costs' benchmarks/viabilitybench/driver/test_run_roko_plan.py && benchmarks/viabilitybench/.venv/bin/python -m pytest benchmarks/viabilitybench/driver/test_run_roko_plan.py -k test_roko_plan_runner_records_planner_and_executor_costs -q"

[closed]
at = 2026-10-02
at_ts = "2026-10-02T19:49:33Z"
commit = "6a5a0a1cb"
executor = "claude-agent"
via = "work-batch"
size = "L"
claimed_at = "2026-10-02T16:02:40Z"
forced = true
evidence = "Gate 4a (merged into main as 6a5a0a1cb, tree identical to work/backlog-batch-4a): all 6 [[verify]] commands pass in the batch worktree's bench venvs; verify 3 needs benchmarks/viabilitybench/.venv-msa, which is a local venv that exists only where run_msa.py set it up, hence --force here. The whole ViabilityBench suite passes, 501 passed and 7 skipped. Gate fix 0a8f7ea69 updates the budget tests to S09 v1.6 (BL14), and S09 4.6 is amended in place."
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK21, slice 33xx, phase 3), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 3311 | M | p1 | vb run gives a multi-model arm one endpoint and one proxy upstream per provider | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3311-vb-run-multi-provider-arms.md` |
| 2 | 3312 | M | p1 | The Roko arm's model check accepts a ladder's escalations and still rejects failover | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3312-roko-arm-accepts-ladder-escalations.md` |
| 3 | 3313 | S | p1 | The roko_ladder arm: arm file, budget line BL14 and the Pilot C manifest | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3313-roko-ladder-arm-budget-line-and-pilot-c-manifest.md` |
| 4 | 3317 | M | p2 | mini-swe-agent as the pinned candidate direct loop (S08 decision 3) | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3317-mini-swe-agent-candidate-direct-loop.md` |
| 5 | 3318 | M | p2 | The fr_claude arm: the Roko arm on claude-opus-5-5 through the Claude CLI | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3318-fr-claude-arm-roko-on-opus-through-claude-cli.md` |
| 6 | 3319 | M | p2 | The roko_plan arm runner: frontier planner, ladder execution and the whole-plan gate | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3319-roko-plan-arm-runner.md` |

## Why it matters

Phase 3: golden-path proof. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/3300-the-proof-viabilitybench-pilots-log1-and-head-to-head.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `benchmarks/viabilitybench/analysis/metrics.py`, `benchmarks/viabilitybench/arms/cheap_direct_msa.toml`, `benchmarks/viabilitybench/arms/fr_claude.toml`, `benchmarks/viabilitybench/arms/roko_ladder.toml`, `benchmarks/viabilitybench/arms/roko_plan.toml`, `benchmarks/viabilitybench/driver/planemit.py`, `benchmarks/viabilitybench/driver/records.py`, `benchmarks/viabilitybench/driver/run_msa.py`, `benchmarks/viabilitybench/driver/run_roko.py`, `benchmarks/viabilitybench/driver/run_roko_plan.py`, `benchmarks/viabilitybench/driver/test_fr_claude.py`, `benchmarks/viabilitybench/driver/test_run_msa.py`, `benchmarks/viabilitybench/driver/test_run_roko_plan.py`, `benchmarks/viabilitybench/driver/test_run_roko_routed.py`, `benchmarks/viabilitybench/driver/test_vb_providers.py`, `benchmarks/viabilitybench/driver/vb.py`, `benchmarks/viabilitybench/experiments/budget.toml`, `benchmarks/viabilitybench/experiments/pilot_c.toml`, `benchmarks/viabilitybench/experiments/test_pilot_c.py`, `benchmarks/viabilitybench/requirements-msa.lock`.

## Current state

The tasks were checked against `2c3ea9f73` on 2026-10-02. Re-check each task's anchors and premise at your base commit before implementing it, and report a task that is already done instead of redoing it.

## Plan

1. Work through the tasks in the order above. For each: read its file, implement its Plan, write the test it names, and make one commit per task whose message ends with `Backlog-Task: <task id>`, `Work-Item: <this item's id>` and `Executor: claude-agent`.
2. Follow `BUILD-RULES.md` in the backlog folder. Workers run no cargo: Rust is checked by the coordinator's batched gate. Python and doc checks you may run.
3. If a task cannot be done (a premise is false, a decision is missing, or its verify cannot pass), stop at that task, keep the earlier commits, and report it; do not skip ahead to tasks that depend on it.

## Done when

- Every task's verify command passes (this item's `[[verify]]` list, one entry per task), after the coordinator's batched gate.
- Each task's own "Done when" holds (see its file).

## Notes

- Waits on: PK20 (gap-5ebb4f).
- Existing work items this package covers or touches: gap-1cd676, gap-f30b8e. When its tasks are done, close those whose verify then passes.
- Suggested model: sonnet.

## Progress

- 3311: implemented at d80b3b3f7
- 3312: implemented at 61cc1153f
- 3313: implemented at 70f2daeee
- 3317: implemented at 712806f91
- 3318: implemented at 54849b08f
- 3319: implemented at bbbdc3ede
