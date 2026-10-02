+++
id = "gap-f61823"
kind = "gap"
title = "PK10 Decision records and census: Exposure and content-decision record types, and the telemetry report reads them (+9 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 10
size = "L"
subsystem = ["roko-cli/graph-dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK10"
anchors = ["crates/roko-cli/src/commands/plan.rs", "crates/roko-cli/src/dispatch/prompt_builder.rs", "crates/roko-cli/src/graph_execution/run_manifest.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/feedback.rs", "crates/roko-cli/src/graph_task_dispatch/ladder.rs", "crates/roko-cli/src/graph_task_dispatch/reflex_credit.rs", "crates/roko-cli/src/graph_task_dispatch/wiring.rs", "crates/roko-cli/tests/learning_wiring_census.rs", "crates/roko-core/src/config/learning.rs", "crates/roko-learn/src/telemetry/mod.rs", "crates/roko-learn/src/telemetry/records.rs", "crates/roko-learn/src/telemetry/report.rs", "crates/roko-learn/src/telemetry/writer.rs", "crates/roko-neuro/src/knowledge_store/crud.rs"]
lane = "rust-hot"
parent = "spec-99d417"
links = { depends_on = ["gap-cc5051"], blocks = [], related = ["gap-644040"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn exposure_record_round_trips_s01_example' crates/roko-learn/src/ && cargo test -p roko-learn --lib exposure_record_round_trips_s01_example"

[[verify]]
command = "grep -rqw 'fn report_reads_route_and_content_decisions' crates/roko-learn/src/ && cargo test -p roko-learn --lib report_reads_route_and_content_decisions"

[[verify]]
command = "grep -q 'router_pick' crates/roko-learn/src/telemetry/records.rs && grep -rqw 'fn ladder_attempt_records_router_pick' crates/roko-cli/src/ && cargo test -p roko-cli --lib ladder_attempt_records_router_pick"

[[verify]]
command = "grep -rqw 'fn diagnostics_mark_items_of_dropped_sections_not_included' crates/roko-cli/src/ && cargo test -p roko-cli --lib diagnostics_mark_items_of_dropped_sections_not_included"

[[verify]]
command = "grep -rqw 'fn exposure_log_distinguishes_retrieved_from_included' crates/roko-cli/src/ && cargo test -p roko-cli --lib exposure_log_distinguishes_retrieved_from_included"

[[verify]]
command = "grep -rqw 'fn attempt_record_fills_exposures' crates/roko-cli/src/ && cargo test -p roko-cli --lib attempt_record_fills_exposures"

[[verify]]
command = "python3 -c \"import re,sys; s=open('crates/roko-cli/tests/learning_wiring_census.rs').read(); m=re.search(r'const EXPECTED_MISSING[^=]*=\\s*&\\[(.*?)\\];', s, re.S); sys.exit(0 if m and 'store.exposure_writer' not in m.group(1) else 1)\" && cargo test -p roko-cli --test learning_wiring_census graph_dispatcher_production_wiring_census"

[[verify]]
command = "grep -rqw 'fn content_decisions_list_retrieved_and_included_ids' crates/roko-cli/src/ && cargo test -p roko-cli --lib content_decisions_list_retrieved_and_included_ids"

[[verify]]
command = "grep -rqw 'fn record_access_counts_without_changing_half_life' crates/roko-neuro/src/ && cargo test -p roko-neuro --lib record_access_counts_without_changing_half_life"

[[verify]]
command = "grep -rqw 'fn included_knowledge_records_access' crates/roko-cli/src/ && cargo test -p roko-cli --lib included_knowledge_records_access"

[[verify]]
command = "python3 -c \"import re,sys; s=open('crates/roko-cli/tests/learning_wiring_census.rs').read(); m=re.search(r'const EXPECTED_MISSING[^=]*=\\s*&\\[(.*?)\\];', s, re.S); sys.exit(0 if m and 'store.record_access' not in m.group(1) else 1)\" && cargo test -p roko-cli --test learning_wiring_census graph_dispatcher_production_wiring_census"

[[verify]]
command = "grep -rqw 'fn plan_run_writes_census_report' crates/roko-cli/src/ && cargo test -p roko-cli --lib plan_run_writes_census_report"

[[verify]]
command = "grep -rqw 'fn frozen_learning_switch_reaches_config_and_manifest' crates/roko-cli/src/ && cargo test -p roko-cli --lib frozen_learning_switch_reaches_config_and_manifest"

[[verify]]
command = "grep -rqw 'fn frozen_run_registers_no_learning_sinks' crates/roko-cli/src/ && cargo test -p roko-cli --lib frozen_run_registers_no_learning_sinks"

[[verify]]
command = "grep -rqw 'fn frozen_attempt_writes_no_affect_reflex_or_access_state' crates/roko-cli/src/ && cargo test -p roko-cli --lib frozen_attempt_writes_no_affect_reflex_or_access_state"
+++

## Problem

This package delivers 10 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK10, slice 22xx, phase 2), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 2211 | M | p2 | Exposure and content-decision record types, and the telemetry report reads them | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2211-exposure-and-content-decision-record-types.md` |
| 2 | 2212 | S | p3 | The ladder verdict records the router's shadow pick | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2212-ladder-verdict-records-router-pick.md` |
| 3 | 2213 | M | p2 | Prompt diagnostics list every retrieved item and whether it reached the prompt | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2213-prompt-diagnostics-list-retrieved-items.md` |
| 4 | 2214 | M | p2 | Graph dispatch writes one exposure row per retrieved item and fills the verdict's exposure counts | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2214-graph-dispatch-writes-exposures.md` |
| 5 | 2215 | S | p2 | Graph dispatch writes one content decision per decision point, with learned-state digests | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2215-graph-dispatch-writes-content-decisions.md` |
| 6 | 2216 | S | p2 | Included knowledge records an access, without the held half-life spacing | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2216-included-knowledge-records-access.md` |
| 7 | 2217 | S | p3 | Each plan run writes census.json from the production wiring report | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2217-plan-run-writes-census-json.md` |
| 8 | 2219 | S | p2 | The frozen-learning switch, carried in config and recorded in the run manifest | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2219-frozen-learning-switch-and-manifest-flag.md` |
| 9 | 2220 | M | p2 | A frozen run registers no learning sinks and saves no learned state at run end | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2220-frozen-run-registers-no-learning-sinks.md` |
| 10 | 2221 | S | p2 | A frozen attempt writes no affect, reflex or knowledge-access state | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2221-frozen-attempt-writes-no-learned-state.md` |

## Why it matters

Phase 2: honest measurement. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2200-decision-records-exposures-census-and-freeze.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/plan.rs`, `crates/roko-cli/src/dispatch/prompt_builder.rs`, `crates/roko-cli/src/graph_execution/plan_runner.rs`, `crates/roko-cli/src/graph_execution/run_manifest.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt.rs`, `crates/roko-cli/src/graph_task_dispatch/decision_log.rs`, `crates/roko-cli/src/graph_task_dispatch/feedback.rs`, `crates/roko-cli/src/graph_task_dispatch/ladder.rs`, `crates/roko-cli/src/graph_task_dispatch/reflex_credit.rs`, `crates/roko-cli/src/graph_task_dispatch/wiring.rs`, `crates/roko-cli/tests/learning_wiring_census.rs`, `crates/roko-core/src/config/learning.rs`, `crates/roko-learn/src/telemetry/census.rs`, `crates/roko-learn/src/telemetry/mod.rs`, `crates/roko-learn/src/telemetry/records.rs`, `crates/roko-learn/src/telemetry/report.rs`, `crates/roko-learn/src/telemetry/writer.rs`, `crates/roko-neuro/src/knowledge_store/crud.rs`.

It also edits the hot file(s) `crates/roko-cli/src/graph_execution/plan_runner.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: PK09 (gap-cc5051).
- Existing work items this package covers or touches: gap-644040. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.

## Progress

Cargo verification of every task is deferred to the batch gate (no cargo on the worker).

- 2211: implemented at 2959afbd9.
- 2212: implemented at e955e3534.
- 2213: implemented at 799095ff4. Knowledge, episodes and playbooks all render into the canonical `domain_context` section, so one budget cannot drop the knowledge and keep a playbook: the test drops that section with a budget that fits only the critical sections, and checks the included case (with digests) under the default budget.
- 2214: implemented at 201e92451. The verdict's `exposures` counts retrieved content items; section items get rows but are not counted, or `included > 0` would hold for every attempt.
- 2215: implemented at 9eb733abf; test fix at f4c002395 (the 2216 access count makes the second attempt's knowledge digest racy).
- 2216: implemented at 91d049ecc.
- 2217: implemented at dd9877e09.
- 2219: implemented at 3b8daad5e, CLI reference at fe1007f5c. `--frozen-learning` rides a task-local scope (`plan_runner::with_frozen_learning`) instead of a `GraphPlanRunParams` field: `prd.rs`, `commands/do_cmd.rs` and `run.rs` build those params, and roko-7d is rewriting them. `[learning] frozen` covers `roko run`.
- 2220: implemented at 8699ca81d.
- 2221: implemented at 5007eaa22.
