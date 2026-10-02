+++
id = "gap-cc5051"
kind = "gap"
title = "PK09 Decision records and census: S01 becomes the one schema: merge the S03/S06 addenda, the ladder source and today's… (+7 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "cybernetic"
rank = 9
size = "L"
subsystem = ["roko-cli/dispatch"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK09"
anchors = ["crates/roko-cli/src/commands/learn.rs", "crates/roko-cli/src/dispatch/mod.rs", "crates/roko-cli/src/dispatch/model_routing.rs", "crates/roko-cli/src/graph_task_dispatch/attempt.rs", "crates/roko-cli/src/graph_task_dispatch/ladder.rs", "crates/roko-cli/src/graph_task_dispatch/streaming.rs", "crates/roko-cli/src/graph_task_dispatch/wiring.rs", "crates/roko-cli/tests/learning_wiring_census.rs", "crates/roko-learn/src/cascade_router.rs", "crates/roko-learn/src/routing_log.rs", "crates/roko-learn/src/telemetry/records.rs", "crates/roko-learn/src/telemetry/report.rs", "tmp/cybernetic-harness/specs/S01-instrumentation.md", "tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md", "tmp/cybernetic-harness/specs/S06-ultrastable-controller.md"]
lane = "rust-hot"
parent = "spec-99d417"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -q 'A-LOOP' tmp/cybernetic-harness/specs/S01-instrumentation.md && grep -q 'A-CTL' tmp/cybernetic-harness/specs/S01-instrumentation.md && grep -q 'A-DIST' tmp/cybernetic-harness/specs/S01-instrumentation.md && grep -Eq 'source ∈ \\{[^}]*ladder' tmp/cybernetic-harness/specs/S01-instrumentation.md && ! grep -q '\"loop-audit/1\"' tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md && ! grep -q '\"controller/1\"' tmp/cybernetic-harness/specs/S06-ultrastable-controller.md"

[[verify]]
command = "grep -rqw 'fn route_decision_row_round_trips_s01_example' crates/roko-learn/src/ && cargo test -p roko-learn --lib route_decision_row_round_trips_s01_example"

[[verify]]
command = "grep -rqw 'fn route_report_counts_masked_and_honest_routes' crates/roko-learn/src/ && cargo test -p roko-learn --lib route_report_counts_masked_and_honest_routes"

[[verify]]
command = "grep -rqw 'fn route_logged_writes_candidates_propensity_and_default' crates/roko-cli/src/ && cargo test -p roko-cli --lib route_logged_writes_candidates_propensity_and_default"

[[verify]]
command = "grep -rqw 'fn graph_route_writes_decision_row' crates/roko-cli/src/ && cargo test -p roko-cli --lib graph_route_writes_decision_row"

[[verify]]
command = "python3 -c \"import re,sys; s=open('crates/roko-cli/tests/learning_wiring_census.rs').read(); m=re.search(r'const EXPECTED_MISSING[^=]*=\\s*&\\[(.*?)\\];', s, re.S); sys.exit(0 if m and 'store.decision_writer' not in m.group(1) else 1)\" && cargo test -p roko-cli --test learning_wiring_census graph_dispatcher_production_wiring_census"

[[verify]]
command = "grep -rqw 'fn unconfigured_cascade_pick_is_not_labelled_router' crates/roko-cli/src/ && cargo test -p roko-cli --lib unconfigured_cascade_pick_is_not_labelled_router"

[[verify]]
command = "grep -rqw 'fn snapshot_digest_changes_iff_state_changes' crates/roko-learn/src/ && cargo test -p roko-learn --lib snapshot_digest_changes_iff_state_changes"

[[verify]]
command = "grep -rqw 'fn decision_records_carry_learned_state_digest' crates/roko-cli/src/ && cargo test -p roko-cli --lib decision_records_carry_learned_state_digest"

[[verify]]
command = "grep -rqw 'fn loop_census_routed_task_logs_fallback_decision' crates/roko-cli/tests/ && cargo test -p roko-cli --test learning_wiring_census loop_census_routed_task_logs_fallback_decision"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK09, slice 22xx, phase 2), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 2202 | M | p2 | S01 becomes the one schema: merge the S03/S06 addenda, the ladder source and today's census into it | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2202-merge-s03-s06-addenda-into-s01-schema.md` |
| 2 | 2204 | M | p2 | The route decision row records the learned proposal, eligibility and per-candidate propensity, and route-report counts masked routes | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2204-route-decision-row-learned-proposal-and-propensity.md` |
| 3 | 2205 | M | p2 | ModelRouter returns its route decision with candidates, proposals, source and propensity | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2205-model-router-returns-its-route-decision.md` |
| 4 | 2206 | M | p2 | Graph dispatch writes one route decision per attempt to decisions.jsonl | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2206-graph-dispatch-writes-route-decisions.md` |
| 5 | 2207 | S | p3 | A guard's fallback is labelled fallback, not router | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2207-guard-fallback-labelled-fallback.md` |
| 6 | 2208 | S | p3 | CascadeRouter digests its learned state | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2208-cascade-router-learned-state-digest.md` |
| 7 | 2209 | S | p3 | Route decisions carry the router's learned-state digest | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2209-route-decisions-carry-learned-state-digest.md` |
| 8 | 2210 | S | p2 | The loop-census fixture routes T4 through an ineligible cascade pick and checks its decision row | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2210-loop-census-t4-routes-through-ineligible-pick.md` |

## Why it matters

Phase 2: honest measurement. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/2200-decision-records-exposures-census-and-freeze.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/learn.rs`, `crates/roko-cli/src/dispatch/mod.rs`, `crates/roko-cli/src/dispatch/model_routing.rs`, `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/attempt.rs`, `crates/roko-cli/src/graph_task_dispatch/decision_log.rs`, `crates/roko-cli/src/graph_task_dispatch/ladder.rs`, `crates/roko-cli/src/graph_task_dispatch/streaming.rs`, `crates/roko-cli/src/graph_task_dispatch/wiring.rs`, `crates/roko-cli/tests/learning_wiring_census.rs`, `crates/roko-learn/src/cascade_router.rs`, `crates/roko-learn/src/routing_log.rs`, `crates/roko-learn/src/telemetry/records.rs`, `crates/roko-learn/src/telemetry/report.rs`, `tmp/cybernetic-harness/specs/S01-instrumentation.md`, `tmp/cybernetic-harness/specs/S03-loop-liveness-audit.md`, `tmp/cybernetic-harness/specs/S06-ultrastable-controller.md`.

It also edits the hot file(s) `crates/roko-cli/src/graph_task_dispatch.rs`, which are left out of this item's anchors so that two hot packages can run at once; the coordinator resolves any merge conflict.

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

- Waits on: nothing.
- Suggested model: opus.

## Progress

- 2202: implemented in the main checkout's untracked `tmp/cybernetic-harness/specs/` (S01 v1.3, S03 v1.2, S06 v1.2), so no commit carries it; its verify passes.
- 2204: implemented at d74b6b16e; cargo verification deferred to the batch gate.
- 2205: implemented at 8c479de36; cargo verification deferred to the batch gate.
- 2206: implemented at 1eb9ec8fa; cargo verification deferred to the batch gate.
- 2207: implemented at 7843ee713; cargo verification deferred to the batch gate.
- 2208: implemented at 514016ca6; cargo verification deferred to the batch gate.
- 2209: implemented at f73c70db7; cargo verification deferred to the batch gate.
- 2210: implemented at 28eb58e54; cargo verification deferred to the batch gate.
