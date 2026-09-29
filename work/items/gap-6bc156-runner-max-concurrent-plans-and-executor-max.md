+++
id = "gap-6bc156"
kind = "gap"
title = "runner.max_concurrent_plans and [executor] max_concurrent_plans do nothing and are not flagged as inert"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
subsystem = ["roko-core/config", "roko-cli/config"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "d9e79e9d8"
source = "dogfood:tmp/dogfood/2026-09-28-portal-programme-continuation.md"
discovered_from = "agent:d1-parallel-plans"
anchors = ["crates/roko-core/src/config/schema.rs:2464", "crates/roko-cli/src/config.rs:1960", "crates/roko-cli/src/config_cmd.rs:126", "crates/roko-cli/src/graph_task_dispatch/inert_settings.rs::graph_engine_inert_settings"]
links = { depends_on = [], blocks = [], related = ["gap-7a3527"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "(grep -q '\"runner.max_concurrent_plans\"' crates/roko-cli/src/graph_task_dispatch.rs || ! grep -q 'max_concurrent_plans' crates/roko-core/src/config/schema.rs) && (grep -q '\"executor.max_concurrent_plans\"' crates/roko-cli/src/graph_task_dispatch.rs || ! grep -q 'pub max_concurrent_plans' crates/roko-cli/src/config.rs)"
+++

Plan concurrency is set by `max_parallel_plans` (config, or the `--max-parallel-plans` flag). Two older knobs with the same meaning remain: `runner.max_concurrent_plans` (roko-core schema.rs:2464) and `[executor] max_concurrent_plans` (roko-cli config.rs:1960, settable with `roko config set executor.max_concurrent_plans`). The Graph runner reads neither, and `graph_engine_inert_settings` lists neither, so `roko config doctor` does not warn about them.

Fix: delete both (with a migration note), or add them to the inert-settings list.

Re-checked 2026-09-29 at d9e79e9d8: unchanged. Also, `roko config init` writes executor.max_concurrent_plans = 4 into new configs (crates/roko-cli/src/config_cmd.rs:126), so the inert key spreads to new workspaces.
