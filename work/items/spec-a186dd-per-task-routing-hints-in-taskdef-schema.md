+++
id = "spec-a186dd"
kind = "spec"
title = "Per-Task Routing Hints in TaskDef Schema"
status = "open"
triage = "unverified"
severity = "p2"
subsystem = ["roko-cli"]
created = 2026-09-07
updated = 2026-09-28
source = "tmp/backlog/archive/207-per-task-routing-hints.md#207 — Per-Task Routing Hints in TaskDef Schema"
discovered_from = "audit:tmp/backlog/archive/207-per-task-routing-hints.md#207 — Per-Task Routing Hints in TaskDef Schema"
anchors = ["crates/roko-cli/src/task_parser.rs:56", "crates/roko-learn/src/model_routing.rs", "crates/roko-cli/src/plan_generate.rs", "task_parser.rs", "plan_generate.rs", "crates/roko-cli/src/task_parser.rs", "crates/roko-cli/src/runner/event_loop.rs", "TaskDef"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
routing quality; the cascade router has no per-task signal about task nature, reducing model selection accuracy. The cascade router selects models based on learned performance data and context signals. Currently, it receives the task tier (mechanical/focused/integrative/architectural) but no…

Imported without verification from:
- `tmp/backlog/archive/207-per-task-routing-hints.md#207 — Per-Task Routing Hints in TaskDef Schema`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`, `crates/roko-learn/src/model_routing.rs`.

How to verify: Check: `TaskDef` has `category: Option<TaskCategory>` and `research_before_edit: Option<bool>`; `TaskCategory` enum has 8 variants matching the documented values; Existing `tasks.toml` files without `category` parse without error [evidence: 00-STATUS-SUMMARY 3. Open / P2 -- Medium (Open): S | 5 |]
