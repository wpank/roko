+++
id = "bug-e3df7d"
kind = "bug"
title = "roko plan generate and the job runner's fallback plan still write new plans to .roko/plans/"
status = "done"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/plan-generate", "roko-serve/jobs"]
created = 2026-09-29
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "db778db9b"
source = "session:roko-b6 2026-09-29 portal close-out"
discovered_from = "q-4299a9 Answer (1e0073605): 'Still writing new plans to .roko/plans/'"
anchors = ["crates/roko-cli/src/commands/plan.rs::cmd_plan", "crates/roko-serve/src/job_runner.rs::synthesize_coding_plan", "crates/roko-serve/src/job_runner.rs::prepare_coding_plan", "crates/roko-fs/src/workspace_plans.rs::workspace_plans_dir"]
links = { depends_on = [], blocks = [], related = ["q-4299a9", "bug-9f340c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "! awk '/PlanCmd::Generate [{]/{f=1} /PlanCmd::Regenerate [{]/{exit} f' crates/roko-cli/src/commands/plan.rs | grep -qF -e '.roko/plans' -e 'join(\".roko\").join(\"plans\")' && sed -n '/async fn synthesize_coding_plan/,/^}/p' crates/roko-serve/src/job_runner.rs | grep -q workspace_plans_dir && grep -rqw 'fn synthesized_coding_plan_lands_in_the_workspace_plans_dir' crates/roko-serve/src && cargo test -p roko-serve --lib synthesized_coding_plan_lands_in_the_workspace_plans_dir"

[closed]
at = 2026-10-01
at_ts = "2026-10-01T18:45:28Z"
by = "coordinator (session 7622b882)"
executor = "claude-agent"
size = "S"
claimed_at = "2026-10-01T16:12:51Z"
forced = false
evidence = "Gate 6b on 9e32a0d64, merged as db778db9b (crates and Cargo.lock identical to the gated tree): cargo check --workspace --tests, nightly fmt and clippy -D warnings clean on 11 crates; lib tests pass (roko-cli 3375, roko-agent 2241, roko-core 1971, roko-learn 1216, roko-serve 1003, roko-gate 696, roko-compose 561, roko-graph 483, roko-execution 245, roko-acp 200); all eight canaries, golden_path_suite, secret_canary and C2 pass; roko-acp, gemini, dispatch-feedback, e2e_domain, run_serve_share, property, job_runner and plan_execute integration tests pass; bin 447; Cargo.lock unchanged; portal tsc clean and vitest 800/800. Implemented in this round; the item's 2026-10-01 note names the change and its test."
+++

## Problem

q-4299a9 (`1e0073605`) made `roko_fs::workspace_plans::workspace_plans_dir` the one rule for where new plans go:
`plans/`, or `.roko/plans/` only while that directory holds a plan and `plans/` does not exist. Its answer names two
writers it did not convert. Both still write to `.roko/plans/`:

- `roko plan generate` in its plain (`--from-file`) and `--from-notes` forms tells the agent to write "under
  .roko/plans/", and afterwards validates only `.roko/plans/*/tasks.toml`. `--from-backlog` already writes to
  `plans/`.
- The marketplace job runner's fallback plan, `synthesize_coding_plan`, writes `.roko/plans/<slug>/`.

In a workspace that has `plans/` (roko itself, and any workspace after its first portal or `prd plan` plan), these
plans land outside the plans directory, so `roko plan list`, serve's plan listing and plan discovery do not show them.
In a fresh workspace, the first such plan makes `.roko/plans/` the plans directory, which is the opposite of the rule.

## Why it matters

Goal `tooling`: generated plans should appear where every other command looks for them. The job runner's fallback
plans are invisible in the portal.

## Where

- `crates/roko-cli/src/commands/plan.rs::cmd_plan`, the `PlanCmd::Generate` arm: the comment at :669, the
  `--from-notes` prompt (:853), the plain prompt (:951), and the post-generation validation, which reads
  `workdir.join(".roko").join("plans")` (:986-990). `--from-backlog` uses `workdir.join("plans")` (:704).
- `crates/roko-serve/src/job_runner.rs::synthesize_coding_plan` (:655, writes at :661), and
  `prepare_coding_plan`, whose fallback `plans_root` is `.roko/plans` (:619).
- `crates/roko-fs/src/workspace_plans.rs::workspace_plans_dir` (:45): the rule to use.

## Current state

Checked at `d5c1dc6be` by reading the code. The `.roko/plans` lookups in `job_runner.rs`
(`resolve_plan_targets` :686-690, `plan_artifacts` :893-897) and in `commands/plan.rs::cmd_resume` (:1778-1782) are
legacy reads, and they are fine.

## Plan

1. `plan generate`: resolve `workspace_plans_dir(&workdir)` once, name that directory (relative to the workspace) in
   both prompts, and validate that directory afterwards. Fix the comment at :669.
2. `synthesize_coding_plan` and the fallback in `prepare_coding_plan`: use `workspace_plans_dir(workdir)`.
3. Add `synthesized_coding_plan_lands_in_the_workspace_plans_dir` (roko-serve lib): in a temp workspace with a
   `plans/` directory, the synthesized plan is written under `plans/<slug>/`.

## Done when

- New plans from `plan generate` and from the job runner's fallback land in the workspace plans directory and show
  up in `roko plan list`.
- The `[[verify]]` command passes.

## Notes

`.roko/plans/` stays readable for legacy workspaces (q-4299a9). Do not remove the legacy reads.

- 2026-10-01 (wk-taskdef): implemented on work/bug-e3df7d; cargo verification deferred to the batch check.
- `plan generate` (prompt, `--from-file` and `--from-notes`) no longer passes `.roko/plans` as the plans root, so
  `prd::generate_plan` writes and validates in `workspace_plans_dir`. `synthesize_coding_plan` and
  `prepare_coding_plan`'s fallback use `workspace_plans_dir`; `job_runner_integration.rs` now expects the fallback plan
  under `plans/`. `generate_plan` validates every plan in its plans root, so in a workspace with `plans/` a broken
  unrelated plan now makes `plan generate` exit 1, as it already marks `prd plan`'s outcome invalid.
