+++
id = "gap-be7368"
kind = "gap"
title = "Resume preview converts plans to graphs with its own copy of run_one_plan's mapping"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-cli/graph_checkpoint", "roko-cli/graph_execution"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:w3b-fingerprint"
anchors = ["crates/roko-cli/src/graph_checkpoint.rs::convert_plan", "crates/roko-cli/src/graph_checkpoint.rs::preview_plan_resume", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_one_plan"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = 'grep -qE "graph_checkpoint::convert_plan|convert_plan\(" crates/roko-cli/src/graph_execution/plan_runner.rs'
+++

`roko plan run --dry-run --resume-plan` previews what a resume would skip by building the graph in `graph_checkpoint::convert_plan` and comparing fingerprints. `run_one_plan` builds the real graph separately (`plan_to_graph` or `ProductionPlanTopology`, with the same max-tasks and retry policy). The preview now receives `rich_topology` through `ResumeOptions`, but a later change to one mapping and not the other would make the preview's fingerprint disagree with the real run's, so the preview would promise a resume that then starts fresh, or the reverse.

Fix: have `run_one_plan` and the preview call one conversion function.

## Notes

- 2026-10-01 (wk-tamper): implemented on work/gap-7147bb; cargo verification deferred to the batch check.
  `graph_checkpoint` now holds the one mapping: `plan_task_infos` (tasks to `PlanTaskInfo`, with a retry-budget
  closure), `converted_max_parallel` and `convert_plan`. `run_one_plan`, the resume preview and the test helper all
  call them. The run passes its adaptive retry budgets and the preview passes `--max-retries` or the authored value;
  the budgets live in node configs, which the plan fingerprint leaves out (`retry_budgets_leave_the_preview_identity_unchanged`).
  The rich topology still converts in `run_one_plan` from the same task list; the preview refuses `--rich-topology`.
