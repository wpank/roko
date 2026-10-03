+++
id = "gap-3c3729"
kind = "gap"
title = "PK75 Domains and assistant: The citations rung: resolve every DOI, arXiv id and URL in the artefact (+5 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "features"
rank = 75
size = "L"
subsystem = ["roko-gate"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK75"
anchors = ["crates/roko-cli/src/commands/graph.rs", "crates/roko-cli/src/graph_task_dispatch/judge_step.rs", "crates/roko-cli/src/graph_task_dispatch/routing_context.rs", "crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/lib.rs", "crates/roko-core/src/config/gates.rs", "crates/roko-core/src/config/schema.rs", "crates/roko-gate/Cargo.toml", "crates/roko-gate/src/lib.rs", "crates/roko-gate/src/llm_judge_gate.rs", "crates/roko-graph/src/cells/mod.rs", "crates/roko-graph/src/engine.rs"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = ["gap-dff960", "gap-ce1d11"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn citation_rung_fails_unresolvable_doi' crates/roko-gate/ && cargo test -p roko-gate citation_rung_fails_unresolvable_doi"

[[verify]]
command = "grep -rqw 'fn judge_rung_without_quoted_evidence_gives_no_score' crates/roko-cli/ && cargo test -p roko-cli judge_rung_without_quoted_evidence_gives_no_score"

[[verify]]
command = "grep -rqw 'fn schema_rung_fails_invalid_json_artefact' crates/roko-gate/ && cargo test -p roko-gate schema_rung_fails_invalid_json_artefact"

[[verify]]
command = "grep -rqw 'fn profile_named_for_domain_sets_pack_and_tools' crates/roko-cli/ && cargo test -p roko-cli profile_named_for_domain_sets_pack_and_tools"

[[verify]]
command = "grep -rqw 'fn shell_exec_cell_runs_command_without_interpolating_payload' crates/roko-graph/ && cargo test -p roko-graph shell_exec_cell_runs_command_without_interpolating_payload"

[[verify]]
command = "grep -rqw 'fn graph_run_agent_task_cell_dispatches_a_gated_run' crates/roko-cli/ && cargo test -p roko-cli graph_run_agent_task_cell_dispatches_a_gated_run"
+++

## Problem

This package delivers 6 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK75, slice 91xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9122 | M | p2 | The citations rung: resolve every DOI, arXiv id and URL in the artefact | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9122-citations-rung-resolves-dois-arxiv-and-urls.md` |
| 2 | 9123 | M | p2 | The judge rung: an evidence-citing rubric judge over the artefact, cross-family, advisory until calibrated | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9123-judge-rung-over-the-artefact-with-evidence.md` |
| 3 | 9124 | M | p3 | The schema rung: JSON Schema and CSV table-schema checks of named artefacts | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9124-schema-rung-for-json-and-csv-artefacts.md` |
| 4 | 9125 | M | p3 | Domain packs as data: [profiles.<domain>] carries the pack, the tool profile and the role identity | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9125-profiles-as-domain-packs.md` |
| 5 | 9126 | M | p2 | A shell.exec cell and a verify.command cell for roko graph run | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9126-shell-exec-and-verify-command-cells.md` |
| 6 | 9127 | M | p2 | An agent.task cell: roko graph run and triggers can start a gated agent run | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9127-agent-task-cell-for-graph-run.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9100-domains-and-the-assistant.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/commands/graph.rs`, `crates/roko-cli/src/graph_entry_cells.rs`, `crates/roko-cli/src/graph_task_dispatch/judge_step.rs`, `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs`, `crates/roko-cli/src/graph_task_dispatch/routing_context.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`, `crates/roko-cli/src/lib.rs`, `crates/roko-core/src/config/gates.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-gate/Cargo.toml`, `crates/roko-gate/src/citation_gate.rs`, `crates/roko-gate/src/lib.rs`, `crates/roko-gate/src/llm_judge_gate.rs`, `crates/roko-gate/src/schema_gate.rs`, `crates/roko-graph/src/cells/mod.rs`, `crates/roko-graph/src/cells/shell_exec.rs`, `crates/roko-graph/src/engine.rs`.

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

- Waits on: PK53 (gap-dff960), PK74 (gap-ce1d11).
- Suggested model: opus.

## Progress

- 9122: implemented at b75ddcec5
- 9123: implemented at 70e181687
- 9124: implemented at d2d9fdb0b
- 9125: implemented at 92e31eb7b
- 9126: implemented at fc2c66f22
- 9127: implemented at e9769a555

The Rust tasks were checked statically (verify greps, hand formatting); cargo verification is left to the batch gate. 9124 adds no dependency: its JSON Schema check covers a draft 2020-12 subset and skips the rung on any other keyword.
