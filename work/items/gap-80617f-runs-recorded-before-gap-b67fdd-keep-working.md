+++
id = "gap-80617f"
kind = "gap"
title = "Runs recorded before gap-b67fdd keep working: serve's run result falls back to roko/batch/<run-id> for a run with no batch record (tested), and roko github status lists pull requests from run branches (roko/work/ and the old roko/batch/), not only roko/plan/"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "S"
subsystem = ["roko-serve", "roko-cli/commands"]
created = 2026-11-12
updated = 2026-11-12
last_verified = 2026-11-12
source = "round v8 lead's requirement on gap-b67fdd (old runs keep working everywhere)"
discovered_from = "agent:roko-recovery (round v8)"
anchors = ["crates/roko-serve/src/routes/run_result.rs::result_range", "crates/roko-cli/src/commands/github.rs::parse_plan_prs"]
links = { depends_on = [], blocks = [], related = ["gap-b67fdd"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn an_old_run_without_a_batch_record_names_its_old_branch' crates/roko-serve/src && grep -rqw 'fn status_lists_pull_requests_from_run_branches' crates/roko-cli/src && cargo test -p roko-serve --lib an_old_run_without_a_batch_record_names_its_old_branch && cargo test -p roko-cli --bin roko status_lists_pull_requests_from_run_branches"
+++

## Problem

gap-b67fdd (merged at b20766eda) names a new run's batch branch `roko/work/<slug>`. Runs recorded before it have
`roko/batch/<run-uuid>` branches, and the lead asked that they keep working everywhere: resume, merge, the portal,
the plugin, and `roko github status`.

- **Resume and merge:** they read the recorded branch, so old names work. An old record without its run id falls
  back to parsing `roko/batch/<run-id>`, and `batch_branch_is_created_at_head_and_continued_on_resume` tests that.
- **serve:** the run result falls back to `roko/batch/<run-id>` when a run's batch records name no branch, but no
  test covers a run with no batch record.
- **`roko github status`:** it lists only pull requests whose head is `roko/plan/…`. A pull request opened from a
  run's branch never appeared, under either name.

## Why it matters

Goal tooling. Existing runs must not vanish from the views that show where their work is.

## Where

- `crates/roko-serve/src/routes/run_result.rs::result_range` (the fallback) and the run summary tests in
  `routes/runs.rs`.
- `crates/roko-cli/src/commands/github.rs::parse_plan_prs`, which filters on `roko/plan/`.

## Plan

1. serve: test an old-style run, with a `roko/batch/<run-id>` branch and no batch record. Its summary names that
   branch and its merge command.
2. `roko github status`: list pull requests whose head is a plan branch (`roko/plan/`) or a run's branch
   (`roko/work/`, or the old `roko/batch/`), so an old run's pull request appears too. The JSON keeps its fields.
3. Update the GitHub integration doc.

## Done when

The two tests pass, and the `[[verify]]` passes.
