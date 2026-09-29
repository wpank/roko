+++
id = "q-4299a9"
kind = "question"
title = "Should a plan generated in a workspace without plans/ be written to plans/ rather than the legacy .roko/plans/?"
status = "done"
triage = "verified"
severity = "p3"
goal = "visibility"
subsystem = ["roko-serve/plans"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
last_verified_rev = "1e0073605"
source = "plan:portal-programme/09-acceptance#T04"
discovered_from = "plan:portal-programme/09-acceptance#T04"
anchors = ["crates/roko-serve/src/routes/plans.rs::plans_dir", "crates/roko-fs/src/workspace_plans.rs::workspace_plans_dir", "crates/roko-cli/src/plan.rs::plans_dir"]
links = { depends_on = [], blocks = [], related = ["bug-9f340c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -qw 'fn workspace_plans_dir' crates/roko-fs/src/workspace_plans.rs && grep -q 'workspace_plans::workspace_plans_dir' crates/roko-cli/src/plan.rs && grep -q 'workspace_plans::workspace_plans_dir' crates/roko-serve/src/routes/plans.rs && grep -qw 'fn legacy_workspace_lists_its_plans_and_new_plans_join_them' crates/roko-cli/src/plan.rs && cargo test -p roko-fs --lib workspace_plans && cargo test -p roko-serve --lib routes::plans::tests::plans_dir_ && cargo test -p roko-cli --lib plan::tests::"

[closed]
at = 2026-09-29
commit = "1e0073605"
evidence = "Option (a) with a legacy fallback, recorded under Answer. roko_fs::workspace_plans::workspace_plans_dir returns plans/, or .roko/plans only while it holds a plan and plans/ is absent; roko-cli plan::plans_dir and roko-serve routes/plans.rs::plans_dir call it. Checked: in a fresh roko init workspace POST /api/plans (roko serve) returned 201 {path: plans/hello-world-app} and GET /api/plans listed it; roko plan create wrote plans/my-first and roko prd idea did not create plans/; in a workspace with .roko/plans/old-plan, roko plan create wrote .roko/plans/new-one and plan list showed both. Tests: roko-fs workspace_plans (6), roko-cli plan::tests (new/legacy listing, holds_plans_agrees_with_plan_discovery), serve_runtime::tests::created_plans_land_in_the_workspace_plans_dir_and_are_listed, main resolve_plans_dir_*, roko-serve routes::plans::tests::plans_dir_* and resume_plan_runs_the_plan_directory, tests prd_pipeline_workspace (roko prd plan writes plans/<slug>) and prd_publish."
+++

## Problem

In a fresh `roko init` workspace, which is the path 09 T03 proves, the portal's first generated plan
is written to `.roko/plans/<slug>/tasks.toml`. The real run's `serve.log` reads "Wrote tasks.toml
(1290 bytes) to /private/tmp/roko-hello-JdC7dN/.roko/plans/a-rust-app-that-prints-hello-world"
(`tmp/portal-audit/evidence/hello-world-real-run1/serve.log`). `plans_dir` returns `plans/` only when it
already exists; otherwise it returns `.roko/plans`, which its own doc comment calls the legacy
location. `.roko/` is roko's runtime-state directory (checkpoints, logs, `runtime/serve.token`), so
the user's first plan sits beside runtime state rather than with their code. bug-9f340c (residual 2)
notes that plans under `.roko/plans` stop being reachable once a `plans/` directory exists.

## Why it matters

Goal `visibility`. This is the first thing a new user makes.

## Where

`crates/roko-serve/src/routes/plans.rs::plans_dir` (and the identical helper in roko-cli that it
mirrors).

## Plan

Options:

- (a) Create `plans/` for the first generated plan.
- (b) Keep `.roko/plans` and list both roots (bug-9f340c).
- (c) Keep it as is, and document it.

## Done when

Will decides. The chosen option then becomes a bug or gap item with a verify command.

## Answer

Decided 2026-09-29 by the supervisor, following Will's "a workspace folder with plans in it":
option (a), with a legacy fallback.

- New plans go to `plans/<slug>/`. The first one creates `plans/`.
- `.roko/plans/` stays readable for legacy workspaces. It receives new plans only while a workspace
  keeps its plans there: it holds at least one plan and `plans/` does not exist. The empty
  `.roko/plans/` that `roko init` creates does not count.
- Once `plans/` exists it is the plans directory, and plans left in `.roko/plans/` are not listed
  (bug-9f340c residual 2, option (b), was not chosen; `roko doctor` warns when both directories exist).

Implemented here rather than filed as a new item. One resolver,
`roko_fs::workspace_plans::workspace_plans_dir`, applies the rule. roko-cli's `plan::plans_dir` and
roko-serve's `routes/plans.rs::plans_dir` both call it, so `POST /api/plans`, portal generation
(`generate_plan_from_prd`), `roko prd plan`, `roko plan create`, plan listing and discovery all use the
same directory. `roko do`, `POST /api/prds/{slug}/plan` and the PRD-publish auto-plan now tell the agent
to write there instead of `.roko/plans/`. A missing plans directory lists as no plans, and the implicit
index rebuild after `roko prd` / `roko research` commands no longer creates `plans/` before a plan
exists.

Still writing new plans to `.roko/plans/`: `roko plan generate` (its prompt, and the check that
validates what the agent wrote, name `.roko/plans/`) and the marketplace job runner's fallback plan
(`crates/roko-serve/src/job_runner.rs::synthesize_coding_plan`).

## Notes

Raised in `tmp/dogfood/2026-09-28-portal-programme-continuation.md` (09:55 entry) and seen in the 09
real-model run. Filed by plan 09 T04 (see VERDICT).
