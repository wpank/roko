+++
id = "bug-a5cd6b"
kind = "bug"
title = "roko prd plan regenerates every old-format plan in plans/ as a side effect"
status = "open"
triage = "verified"
severity = "p2"
goal = "core"
subsystem = ["roko-cli/prd"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "5fe2b64a5"
source = "plan:portal-programme/04-backend-plan-authoring#T15"
discovered_from = "plan:portal-programme/04-backend-plan-authoring#T15"
anchors = ["crates/roko-cli/src/prd.rs::regenerate_old_format_plans", "crates/roko-cli/src/prd.rs::generate_plan_from_prd_with_outcome", "crates/roko-cli/src/prd.rs::generate_plan_from_prd_with_model", "crates/roko-cli/src/commands/prd.rs:845", "crates/roko-cli/src/task_parser.rs::validate_modern_fields_content"]
links = { depends_on = [], blocks = [], related = ["bug-8b1bf8"], supersedes = [], duplicate_of = "" }

[[repro]]
command = "ls plans/ | wc -l && cargo run -p roko-cli -- prd plan <any-slug> && ls plans/ | wc -l"

[[verify]]
command = "grep -q 'fn test_prd_plan_does_not_regenerate_other_plans' crates/roko-cli/src/prd.rs && cargo test -p roko-cli --lib prd::tests::test_prd_plan_does_not_regenerate_other_plans"
+++

`roko prd plan <slug>` calls `regenerate_old_format_plans` as a side effect. This function scans every directory under `plans/`, finds plans using the old flat-file format (not `tasks.toml`), and regenerates each one by sending a separate LLM request per plan. In a workspace with many old-format plans this:
- Fires one LLM agent call per old plan without any user confirmation.
- Overwrites plan files in place.
- Burns real API budget unrelated to the requested plan.
- Causes intermittent failures when a regenerated plan references non-existent `read_files`.

The server path (`POST /api/plans/generate`) was fixed in plan 04 T10 to skip `regenerate_old_format_plans` via an isolated variant; the CLI `roko prd plan` path was deliberately left unchanged and keeps the existing behaviour.

Fix: either remove `regenerate_old_format_plans` from the `roko prd plan` code path (make it an explicit opt-in subcommand), or gate it on a `--regenerate-old` flag.

Rechecked 2026-09-29 at d9e79e9d8: still open. The call is now gated by a regenerate_old_plans parameter (prd.rs:1314, :1975). Every CLI entry point passes true: generate_plan_from_prd :1210, _with_model :1234, _with_failure_context :1248, and the promote path :1088. Only the serve-side generate_plan_from_prd_isolated passes false. The verify test prd::tests::test_prd_plan_does_not_regenerate_other_plans does not exist, and a cargo test filter that matches no test exits 0.

## Notes

- Implemented on `work/gap-2623b2` at `5fe2b64a5`, test fixed at `16a17b139`. Checked in this worker's own target at
  `7865621db`: check, clippy and the verify test pass, and so do 232 lib tests under `prd::`, `task_parser::` and
  `plan_*::`. The batch check re-runs them.
- Root cause: `TasksFile::validate_modern_fields` required a `model_hint`, and the generator strips every
  `model_hint`, so each generated plan counted as old-format. Every `roko prd plan` regenerated all of them, the plan
  it had just written included. A modern task no longer needs a `model_hint` (role and tier route it); the modern
  fields are `tier`, `context.read_files`, `verify` and `depends_on`.
- The refresh is opt-in: `roko prd plan <slug> --regenerate-old` runs `regenerate_old_format_plans` after the requested
  plan (not under `--dry-run`). No generate path runs it on its own: the wrapper's `regenerate_old_plans` parameter,
  which every CLI entry point and promote's auto-plan set to true, is gone.
- `prd::tests::test_prd_plan_does_not_regenerate_other_plans` runs the generator with a fake `claude_cli` planner over
  an old-format plan and a generated plan without a `model_hint`. Both keep their tasks.toml byte for byte, the planner
  is called once, and only the old-format plan counts as old.
- The `[[repro]]` command needs a live planner and a real slug; not run.
