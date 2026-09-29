+++
id = "gap-9ed15e"
kind = "gap"
title = "plan validate finds tasks.toml files with its own walker instead of discover_plans"
status = "open"
triage = "verified"
severity = "p3"
subsystem = ["roko-cli/plan_validate", "roko-cli/plan-discovery"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:e3-discovery"
anchors = ["crates/roko-cli/src/plan_validate.rs::collect_tasks_files_recursive", "crates/roko-cli/src/orchestrator/plan_discovery.rs::discover_plans"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = '! grep -q "fn collect_tasks_files_recursive" crates/roko-cli/src/plan_validate.rs'
+++

`plan_validate` walks directories with a private recursive walker (plan_validate.rs:280-303). Plan runs, serve and the TUI resolve plans through `orchestrator::plan_discovery::discover_plans`. Any difference in their rules means `roko plan validate plans/` can check a different set of plans than `roko plan run plans/` executes.

Fix: validate exactly the set that `discover_plans` returns.
