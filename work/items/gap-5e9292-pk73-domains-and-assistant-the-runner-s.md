+++
id = "gap-5e9292"
kind = "gap"
title = "PK73 Domains and assistant: The runner's FeedRegistry is built empty and never read (+7 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
rank = 73
size = "L"
subsystem = ["roko-serve/routes"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK73"
anchors = ["crates/roko-cli/src/run.rs", "crates/roko-cli/src/runner/types.rs", "crates/roko-cli/src/serve_runtime.rs", "crates/roko-core/src/domain_profile.rs", "crates/roko-core/src/lib.rs", "crates/roko-serve/Cargo.toml", "crates/roko-serve/src/routes/mod.rs", "crates/roko-serve/src/routes/plans/run_control.rs", "crates/roko-serve/src/routes/plans/tests.rs", "crates/roko-serve/src/routes/route_permissions.rs", "crates/roko-serve/src/routes/run.rs", "crates/roko-serve/src/routes/runs.rs", "crates/roko-serve/src/runtime.rs", "crates/roko-serve/src/state.rs", "docs/v3/26-HTTP-API.md", "docs/v3/depth/05-agent/16-domain-profiles.md", "docs/v3/depth/05-agent/domain-profiles.md"]
lane = "rust-cold"
parent = "spec-0b3a32"
links = { depends_on = [], blocks = [], related = ["find-8872ad"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! grep -q 'feed_registry' crates/roko-cli/src/runner/types.rs"

[[verify]]
command = "! grep -rqw -e default_gate_rungs -e effective_gate_rungs crates/ --include='*.rs'"

[[verify]]
command = "grep -rqw 'fn api_run_without_gates_reports_unverified' crates/roko-serve/ && cargo test -p roko-serve api_run_without_gates_reports_unverified"

[[verify]]
command = "grep -rqw 'fn plan_status_reports_terminal_state' crates/roko-serve/ && cargo test -p roko-serve plan_status_reports_terminal_state"

[[verify]]
command = "grep -rqw 'fn second_plan_run_is_queued_not_refused' crates/roko-serve/ && cargo test -p roko-serve second_plan_run_is_queued_not_refused"

[[verify]]
command = "grep -rqw 'fn run_summary_reports_verdict_cost_and_milestones' crates/roko-serve/ && cargo test -p roko-serve run_summary_reports_verdict_cost_and_milestones"

[[verify]]
command = "grep -rqw 'fn api_run_reports_the_gated_runs_id_and_verdict' crates/roko-serve/ && cargo test -p roko-serve api_run_reports_the_gated_runs_id_and_verdict"

[[verify]]
command = "grep -rqw 'fn mcp_tools_list_returns_annotated_tools' crates/roko-serve/ && cargo test -p roko-serve mcp_tools_list_returns_annotated_tools"
+++

## Problem

This package delivers 8 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK73, slice 91xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9101 | S | p3 | The runner's FeedRegistry is built empty and never read | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9101-remove-the-runners-empty-feed-registry.md` |
| 2 | 9102 | S | p3 | Delete the unused DomainProfile enum, whose default_gate_rungs names gates that do not exist | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9102-delete-unused-domainprofile-enum-and-fake-gate-names.md` |
| 3 | 9103 | S | p2 | POST /api/run reports an ungated answer as completed and successful | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9103-api-run-reports-ungated-answer-as-unverified.md` |
| 4 | 9104 | M | p2 | The plan-run handle has no terminal status: GET /api/plans/{id}/status 404s once the run ends | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9104-plan-run-handle-keeps-terminal-status.md` |
| 5 | 9111 | M | p2 | Queue a second plan run instead of refusing it with 409 | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9111-queue-second-plan-run-instead-of-409.md` |
| 6 | 9112 | M | p2 | A run summary a host can post: GET /api/runs/{run_id}/summary with one verdict, its cost and milestones | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9112-run-summary-a-host-can-post.md` |
| 7 | 9113 | M | p2 | POST /api/run runs the prompt as a gated one-task plan under the id it returns, like roko run | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9113-api-run-runs-a-gated-one-task-plan.md` |
| 8 | 9114 | M | p2 | Serve POST /mcp: an MCP endpoint with run_status (bounded wait) and recall | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9114-serve-mcp-endpoint-with-run-status-and-recall.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9100-domains-and-the-assistant.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/runner/types.rs`, `crates/roko-cli/src/serve_runtime.rs`, `crates/roko-core/src/domain_profile.rs`, `crates/roko-core/src/lib.rs`, `crates/roko-serve/Cargo.toml`, `crates/roko-serve/src/routes/mcp.rs`, `crates/roko-serve/src/routes/mod.rs`, `crates/roko-serve/src/routes/plans/run_control.rs`, `crates/roko-serve/src/routes/plans/tests.rs`, `crates/roko-serve/src/routes/route_permissions.rs`, `crates/roko-serve/src/routes/run.rs`, `crates/roko-serve/src/routes/runs.rs`, `crates/roko-serve/src/runtime.rs`, `crates/roko-serve/src/state.rs`, `docs/v3/26-HTTP-API.md`, `docs/v3/depth/05-agent/16-domain-profiles.md`, `docs/v3/depth/05-agent/domain-profiles.md`.

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
- Existing work items this package covers or touches: find-8872ad. When its tasks are done, close those whose verify then passes.
- Suggested model: opus.

## Progress

Implemented on `work/gap-5e9292`; cargo verification deferred to the batch gate (static greps of every verify pass).

- 9101: implemented at c6a6e7134
- 9102: implemented at d59f3c8f4 (option a: the enum, `TypedContext` and both `lib.rs` lines deleted; both docs pages corrected)
- 9103: implemented at 01755a555 (adds `RunState`, the run-state words every run route shares)
- 9104: implemented at 3d7441310 (ended plan handles are kept an hour; active-plan counters count live runs only)
- 9111: implemented at 54c31ff46 (decision 9105 option a; queue of 8)
- 9112: implemented at f4ade3ffc
- 9113: implemented at f0c15452d
- 9114: implemented at 64bf22a36, test fix at 3c2a7e67f
