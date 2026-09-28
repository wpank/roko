+++
id = "gap-d31457"
kind = "gap"
title = "Plan Cost Budget Enforcement and Early Termination"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/runner"]
created = 2026-09-07
updated = 2026-09-28
last_verified = 2026-09-28
source = "tmp/backlog/archive/171-plan-cost-budget-enforcement.md#171 — Plan Cost Budget Enforcement and Early Termination"
discovered_from = "audit:tmp/backlog/archive/171-plan-cost-budget-enforcement.md#171 — Plan Cost Budget Enforcement and Early Termination"
anchors = ["crates/roko-cli/src/graph_execution/plan_runner.rs::resolve_budget_ceiling", "crates/roko-cli/src/main.rs:2066", "crates/roko-cli/src/graph_task_dispatch.rs::task_budget_ceiling_usd", "crates/roko-core/src/config/budget.rs::BudgetConfig"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }
+++
retry loops can burn unlimited tokens with no circuit breaker. When a task enters a retry loop due to gate failures, each retry dispatches a new agent that reads the codebase, investigates the failure, and attempts a fix — all costing tokens. There is no per-task or per-plan cost budget that…

Imported without verification from:
- `tmp/backlog/archive/171-plan-cost-budget-enforcement.md#171 — Plan Cost Budget Enforcement and Early Termination`

Some cited files are gone: `crates/roko-cli/src/runner/event_loop.rs`.

How to verify: Check: A task that exceeds its cost budget is stopped with a clear message.; A plan that exceeds its cost budget stops dispatching new tasks.; `--max-cost 2.0` on CLI limits total plan spend to $2. [evidence: 00-STATUS-SUMMARY 3. Open / P3 -- Low (Open): -- | -- |]

Verified 2026-09-28: Mostly in place: a config per-plan ceiling ([budget].max_plan_usd) blocks further dispatch at HEAD (resolve_budget_ceiling, plan_budget_snapshot().dispatch_blocked). Per-task and cumulative-retry caps (task_budget_ceiling_usd/admit_task_budget, max_task_retry_usd -> BudgetExceeded) exist only in the working tree. Still true: a CLI ceiling (`--budget-override <AMOUNT>`) only warns and execution continues (bypass_block), so the CLI cannot cap plan spend. Severity lowered p1 -> p2 (the source summary rated it P3).
