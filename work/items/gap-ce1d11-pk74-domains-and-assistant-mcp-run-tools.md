+++
id = "gap-ce1d11"
kind = "gap"
title = "PK74 Domains and assistant: /mcp run tools: run_prompt, plan_generate, plan_run and run_cancel, annotated so the… (+6 more)"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
rank = 74
size = "L"
subsystem = ["roko-serve/mcp"]
created = 2026-10-02
updated = 2026-10-02
last_verified = 2026-10-02
source = "tmp/backlog/2026-10-02-complete-and-wire PK74"
anchors = ["crates/roko-cli/src/graph_task_dispatch/verification.rs", "crates/roko-cli/src/main.rs", "crates/roko-cli/src/plan_generator.rs", "crates/roko-cli/src/run.rs", "crates/roko-cli/src/serve_runtime.rs", "crates/roko-core/src/config/gates.rs", "crates/roko-core/src/config/serve.rs", "crates/roko-serve/src/lib.rs", "crates/roko-serve/src/routes/plans/authoring.rs", "crates/roko-serve/src/routes/plans/run_control.rs", "crates/roko-serve/src/routes/run.rs", "crates/roko-serve/src/routes/runs.rs", "crates/roko-serve/src/runtime.rs"]
lane = "rust-hot"
parent = "spec-0b3a32"
links = { depends_on = ["gap-5e9292"], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn mcp_plan_run_returns_engine_run_id' crates/roko-serve/ && cargo test -p roko-serve mcp_plan_run_returns_engine_run_id"

[[verify]]
command = "grep -rqw 'fn mcp_run_without_budget_cap_is_refused' crates/roko-serve/ && cargo test -p roko-serve mcp_run_without_budget_cap_is_refused"

[[verify]]
command = "grep -rqw 'fn chat_origin_prompt_is_fenced_as_untrusted_data' crates/roko-cli/ && cargo test -p roko-cli chat_origin_prompt_is_fenced_as_untrusted_data"

[[verify]]
command = "grep -rqw 'fn run_completion_is_posted_to_the_host_once' crates/roko-serve/ && cargo test -p roko-serve run_completion_is_posted_to_the_host_once"

[[verify]]
command = "grep -rqw 'fn rung_kind_defaults_to_command' crates/roko-core/ && cargo test -p roko-core rung_kind_defaults_to_command"

[[verify]]
command = "grep -rqw 'fn research_task_runs_the_research_pack' crates/roko-cli/ && cargo test -p roko-cli research_task_runs_the_research_pack"

[[verify]]
command = "grep -rqw 'fn roko_run_domain_sets_the_task_domain' crates/roko-cli/ && cargo test -p roko-cli roko_run_domain_sets_the_task_domain"
+++

## Problem

This package delivers 7 tasks of the backlog `tmp/backlog/2026-10-02-complete-and-wire/` (package PK74, slice 91xx, phase 9), in this order. Each task's full specification (Problem, Why it matters, Where, Current state, Plan, Done when, Notes and its verify) is in its file: read each one completely before starting it.

| # | Task | Size | Sev | Title | File |
|---|---|---|---|---|---|
| 1 | 9115 | M | p2 | /mcp run tools: run_prompt, plan_generate, plan_run and run_cancel, annotated so the host asks for approval | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9115-mcp-run-tools-annotated-for-host-approval.md` |
| 2 | 9116 | M | p2 | Runs started over /mcp carry their origin and must name a budget cap | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9116-chat-origin-runs-carry-origin-and-budget-cap.md` |
| 3 | 9117 | M | p2 | Chat-origin runs: the request text is fenced as untrusted data, and the data-model boundary is required | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9117-chat-text-is-untrusted-data-boundary-required.md` |
| 4 | 9118 | M | p3 | Deliver the verdict to the host when a run ends: a signed completion webhook | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9118-signed-completion-webhook-to-the-host.md` |
| 5 | 9119 | S | p2 | [[gates.rungs]] gets a kind: command, citations, judge, schema, receipt or confirm | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9119-gate-rungs-get-a-kind.md` |
| 6 | 9120 | M | p2 | Verifier packs keyed by TaskDomain; the code pack is today's ladder | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9120-verifier-packs-keyed-by-task-domain.md` |
| 7 | 9121 | S | p3 | roko run --domain, and a domain on POST /api/run, so one-task plans get their pack and tool policy | `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9121-roko-run-domain-flag.md` |

## Why it matters

Phase 9: domains, assistant, held and parked work, cleanup, showcase and deploy. The slice's epic, with its goal and scope, is `/Users/will/dev/nunchi/roko/roko/tmp/backlog/2026-10-02-complete-and-wire/9100-domains-and-the-assistant.md`. The whole order is in `00-INDEX.md` and `PACKAGES.md` in the backlog folder; Will's decisions are in its `DECISIONS.md`.

## Where

Files the tasks change: `crates/roko-cli/src/graph_task_dispatch.rs`, `crates/roko-cli/src/graph_task_dispatch/pack_rungs.rs`, `crates/roko-cli/src/graph_task_dispatch/verification.rs`, `crates/roko-cli/src/main.rs`, `crates/roko-cli/src/plan_generator.rs`, `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/serve_runtime.rs`, `crates/roko-core/src/config/gates.rs`, `crates/roko-core/src/config/serve.rs`, `crates/roko-serve/src/delivery.rs`, `crates/roko-serve/src/lib.rs`, `crates/roko-serve/src/routes/mcp.rs`, `crates/roko-serve/src/routes/plans/authoring.rs`, `crates/roko-serve/src/routes/plans/run_control.rs`, `crates/roko-serve/src/routes/run.rs`, `crates/roko-serve/src/routes/runs.rs`, `crates/roko-serve/src/runtime.rs`.

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

- Waits on: PK73 (gap-5e9292).
- Suggested model: opus.

## Progress

- 9115: implemented at ae0c9fa02
- 9116: implemented at 9e3d31bd1
- 9117: implemented at 80110ed6f (test layout follow-up 31fdfbff4)
- 9118: blocked: decision 9106 says to confirm, before 9118, whether OpenClaw is an MCP client and whether Hermes can post to live sessions; neither answer is recorded
- 9119: implemented at 224bb3067
- 9120: implemented at ee8b14918
- 9121: implemented at 8e278e092

The Rust tasks were checked statically (verify greps, hand formatting); cargo verification is left to the batch gate.
