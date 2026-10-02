+++
id = "gap-a0043b"
kind = "gap"
title = "PK08 Attempt ledger: roko.toml's cheap-model rates match the dated price snapshot (+11 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "truth"
rank = 8
size = "L"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK08"
anchors = ["CLAUDE.md", "crates/roko-cli/src/commands/dashboard.rs", "crates/roko-cli/src/commands/diagnose.rs", "crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/commands/show.rs", "crates/roko-cli/src/config.rs", "crates/roko-cli/src/doctor.rs", "crates/roko-cli/src/graph_execution/disk_admission.rs", "crates/roko-cli/src/graph_task_dispatch/budget.rs", "crates/roko-cli/src/runner/persist.rs", "crates/roko-cli/src/runner/types.rs", "crates/roko-cli/src/tui/dashboard_model.rs", "crates/roko-core/src/config/budget.rs", "crates/roko-fs/src/layout.rs", "crates/roko-fs/src/lib.rs", "crates/roko-fs/src/log_rotation.rs", "crates/roko-fs/src/observability.rs", "crates/roko-fs/src/tool_metrics_sink.rs", "crates/roko-learn/src/run_metrics.rs", "crates/roko-serve/src/lib.rs", "crates/roko-serve/src/retention.rs", "crates/roko-serve/src/routes/status/gates.rs", "crates/roko-serve/src/telemetry_observer.rs", "roko.toml"]
lane = "rust-hot"
parent = "spec-99d417"
links = { depends_on = ["gap-198c9c", "gap-f548c1"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "python3 -c \"import tomllib as t;s={r['slug']:r for r in t.load(open('config/prices/2026-09-28.toml','rb'))['model']};m=t.load(open('roko.toml','rb'))['models'];p={'cerebras-gptoss':'gpt-oss-120b','glm-4-7':'glm-4.7','gpt-5-4-mini':'gpt-5.4-mini','kimi-k2-5':'kimi-k2.6'};f={'cost_input_per_m':'input','cost_output_per_m':'output','cost_cache_read_per_m':'cache_read'};b=[(k,a) for k,v in p.items() for a,c in f.items() if m.get(k,{}).get(a) is not None and abs(m[k][a]-s[v][c])>1e-9];print(b);raise SystemExit(bool(b))\""

[[verify]]
command = "grep -rq 'InboxCategory::BudgetAlert' crates/roko-cli/src/graph_task_dispatch/"

[[verify]]
command = "grep -rqw 'fn plan_budget_emits_threshold_events' crates/roko-cli/src/ && cargo test -p roko-cli plan_budget_emits_threshold_events"

[[verify]]
command = "grep -rqE 'RaiseBudget|raise_budget' crates/roko-cli/src/runner/types.rs"

[[verify]]
command = "grep -rqw 'fn raised_plan_ceiling_admits_the_next_task' crates/roko-cli/src/ && cargo test -p roko-cli raised_plan_ceiling_admits_the_next_task"

[[verify]]
command = "! grep -q 'gate-verdicts' crates/roko-serve/src/routes/status/gates.rs"

[[verify]]
command = "grep -rqw 'fn gate_history_reads_graph_gate_results' crates/roko-serve/src/ && cargo test -p roko-serve gate_history_reads_graph_gate_results"

[[verify]]
command = "! grep -q 'Canonical signal log' CLAUDE.md && grep -q 'attempts.jsonl' CLAUDE.md"

[[verify]]
command = "grep -rln 'run-metrics' crates/roko-cli/src/commands crates/roko-cli/src/tui | grep -q ."

[[verify]]
command = "grep -rqw 'fn show_costs_lists_recent_runs' crates/roko-cli/src/ && cargo test -p roko-cli show_costs_lists_recent_runs"

[[verify]]
command = "! grep -rq 'tool_metrics.jsonl' crates/roko-fs/src/observability.rs crates/roko-cli/src/graph_execution/plan_runner.rs"

[[verify]]
command = "! grep -rq 'telemetry_observations_path' crates/roko-serve/src/telemetry_observer.rs"

[[verify]]
command = "grep -q 'gate-gaming-alerts' crates/roko-cli/src/commands/diagnose.rs"

[[verify]]
command = "grep -rqw 'fn diagnose_lists_gate_gaming_alerts' crates/roko-cli/src/ && cargo test -p roko-cli diagnose_lists_gate_gaming_alerts"

[[verify]]
command = "! grep -q 'task-metrics.jsonl' crates/roko-cli/src/commands/dashboard.rs"

[[verify]]
command = "grep -rqw 'fn dashboard_headlines_come_from_attempt_verdicts' crates/roko-cli/src/ && cargo test -p roko-cli dashboard_headlines_come_from_attempt_verdicts"

[[verify]]
command = "! grep -q 'DISK_BUDGET_REMAINING_METRIC' crates/roko-cli/src/graph_execution/disk_admission.rs"

[[verify]]
command = "! grep -rq 'run-ledger' crates/roko-cli/src/runner/persist.rs crates/roko-serve/src"

[[verify]]
command = "! grep -rq 'gate_verdicts_path' crates/roko-fs/src/log_rotation.rs"
+++

## Problem

This package delivers 12 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK08, slice 21xx, phase 2), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 2112 | S | p2 | roko.toml's cheap-model rates match the dated price snapshot | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2112-roko-toml-cheap-model-rates-match-the-snapshot.md` |
| 2 | 2116 | M | p2 | The plan budget raises alerts at 50% and 80% of its ceiling, before it stops the run | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2116-plan-budget-raises-threshold-alerts.md` |
| 3 | 2118 | M | p2 | An operator can raise a running plan's budget ceiling from the CLI | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2118-operator-can-raise-a-running-plans-ceiling.md` |
| 4 | 2119 | S | p2 | Serve's gate history reads the gate results Graph runs write, not Runner-v2's dead log | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2119-serve-gate-history-reads-graph-gate-results.md` |
| 5 | 2121 | S | p2 | CLAUDE.md stops calling signals.jsonl the canonical log and names the records plan runs write | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2121-claude-md-names-the-records-plan-runs-write.md` |
| 6 | 2122 | S | p3 | roko show costs lists recent runs from run-metrics.jsonl, which nothing reads today | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2122-roko-show-costs-lists-recent-runs.md` |
| 7 | 2123 | S | p3 | Plan runs stop writing metrics/tool_metrics.jsonl, whose readers were deleted | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2123-stop-writing-tool-metrics-jsonl.md` |
| 8 | 2124 | S | p3 | Serve stops persisting Lens samples to telemetry-observations.jsonl, which nothing reads | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2124-serve-stops-persisting-telemetry-observations.md` |
| 9 | 2125 | S | p3 | roko diagnose lists the gate-gaming alerts the run recorded | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2125-roko-diagnose-lists-gate-gaming-alerts.md` |
| 10 | 2126 | M | p3 | The dashboard's task-metrics panel reads the attempt ledger instead of a file only the demo writes | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2126-dashboard-task-panel-reads-the-attempt-ledger.md` |
| 11 | 2127 | S | p3 | Disk admission stops pushing a disk_budget_remaining metric that no watcher reads | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2127-drop-the-unread-disk-budget-metric.md` |
| 12 | 2128 | S | p3 | Dead Runner-v2 store paths leave the code, and roko doctor lists orphan state files | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2128-remove-dead-runner-v2-store-paths.md` |

## Why it matters

Phase 2: honest measurement. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2100-the-attempt-ledger-counts-tokens-and-money.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `CLAUDE.md`, `crates/roko-cli/src/commands/dashboard.rs`, `crates/roko-cli/src/commands/diagnose.rs`, `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/commands/show.rs`, `crates/roko-cli/src/config.rs`, `crates/roko-cli/src/doctor.rs`, `crates/roko-cli/src/graph_execution/disk_admission.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/budget.rs`, `crates/roko-cli/src/runner/persist.rs`, `crates/roko-cli/src/runner/types.rs`, `crates/roko-cli/src/tui/dashboard_model.rs`, `crates/roko-core/src/config/budget.rs`, `crates/roko-fs/src/layout.rs`, `crates/roko-fs/src/lib.rs`, `crates/roko-fs/src/log_rotation.rs`, `crates/roko-fs/src/observability.rs`, `crates/roko-fs/src/tool_metrics_sink.rs`, `crates/roko-learn/src/run_metrics.rs`, `crates/roko-serve/src/lib.rs`, `crates/roko-serve/src/retention.rs`, `crates/roko-serve/src/routes/status/gates.rs`, `crates/roko-serve/src/telemetry_observer.rs`, `roko.toml`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK04 (gap-198c9c), PK07 (gap-f548c1).
- Suggested model: opus.

## Progress

Implemented on `work/gap-a0043b`; cargo verification deferred to the batch gate.

- 2112: implemented at 0186381fe
- 2116: implemented at b4dad76bd
- 2118: implemented at be3e254f4 (the CLI sends the raise over the plan-control socket, as pause and cancel do since 1209, so it prints the run's answer; control.json's `raise_budget` reaches the run too)
- 2119: implemented at e24587bd4
- 2121: implemented at 521e6e9ad
- 2122: implemented at 47aec45c9
- 2123: implemented at ec53fe3bb (the doc comment on `dispatch_v2.rs::with_observability_sinks` is left for 2114)
- 2124: implemented at e4179f28c
- 2125: implemented at bbb656745
- 2126: implemented at 1ef44eb93 (`demo_seed.rs` still writes `memory/task-metrics.jsonl`, which nothing reads now)
- 2127: implemented at 6225096d6
- 2128: implemented at fa2e992bb (roko-runtime's in-memory `RunLedger` module is kept: gap-5d3b82 anchors on it)
- 2026-10-03 (coordinator, gate 4c): all twelve tasks are merged in 2724386ea except task 2121's two CLAUDE.md rows. They were left out because the main checkout has an uncommitted CLAUDE.md edit (the Goal line) that isn't the batch's. The rows are saved as a patch in the coordinator's scratchpad (`pk08-claude-md.patch`) and land once that edit is committed; this item closes then.
