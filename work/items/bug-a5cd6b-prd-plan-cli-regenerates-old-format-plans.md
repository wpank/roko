+++
id = "bug-a5cd6b"
kind = "bug"
title = "roko prd plan regenerates every old-format plan in plans/ as a side effect"
status = "open"
triage = "verified"
severity = "p2"
subsystem = ["roko-cli/prd"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "plan:portal-programme/04-backend-plan-authoring#T15"
discovered_from = "plan:portal-programme/04-backend-plan-authoring#T15"
anchors = ["crates/roko-cli/src/prd.rs::regenerate_old_format_plans"]
links = { depends_on = [], blocks = [], related = ["bug-8b1bf8"], supersedes = [], duplicate_of = "" }

[[repro]]
command = "ls plans/ | wc -l && cargo run -p roko-cli -- prd plan <any-slug> && ls plans/ | wc -l"

[[verify]]
command = "cargo test -p roko-cli -- prd::tests::test_prd_plan_does_not_regenerate_other_plans"
+++

`roko prd plan <slug>` calls `regenerate_old_format_plans` as a side effect. This function scans every directory under `plans/`, finds plans using the old flat-file format (not `tasks.toml`), and regenerates each one by sending a separate LLM request per plan. In a workspace with many old-format plans this:
- Fires one LLM agent call per old plan without any user confirmation.
- Overwrites plan files in place.
- Burns real API budget unrelated to the requested plan.
- Causes intermittent failures when a regenerated plan references non-existent `read_files`.

The server path (`POST /api/plans/generate`) was fixed in plan 04 T10 to skip `regenerate_old_format_plans` via an isolated variant; the CLI `roko prd plan` path was deliberately left unchanged and keeps the existing behaviour.

Fix: either remove `regenerate_old_format_plans` from the `roko prd plan` code path (make it an explicit opt-in subcommand), or gate it on a `--regenerate-old` flag.
