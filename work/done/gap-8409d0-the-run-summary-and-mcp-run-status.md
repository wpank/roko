+++
id = "gap-8409d0"
kind = "gap"
title = "The run summary and MCP run_status never say where a run's verified work is (roko/batch/<run_id> branch, commit, files, promotion), so chat hosts cannot report or merge it"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-serve/runs"]
created = 2026-10-09
updated = 2026-10-12
last_verified = 2026-10-12
last_verified_rev = "8f4f7f2ee"
source = "live hermes-plugin-roko e2e, 2026-10-09 (Hermes session 20261009_144646_a355c0)"
anchors = ["crates/roko-serve/src/routes/runs.rs::get_run_summary", "crates/roko-cli/src/graph_execution/batch.rs::BATCH_BRANCH_PREFIX"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_summary_reports_where_the_verified_work_is' crates/roko-serve/ && cargo test -p roko-serve run_summary_reports_where_the_verified_work_is"

[closed]
at = 2026-10-12
at_ts = "2026-10-12T12:23:01Z"
commit = "8f4f7f2ee"
by = "commit trailer"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-12T12:14:16Z"
forced = false
evidence = "8f4f7f2ee: an ended run's summary (and so MCP run_status) carries result: the batch branch, its tip commit, the base (run manifest workspace.base_commit, else a plan checkpoint roko.batch@1 record, else merge-base with HEAD), files_changed (first 50) and diffstat from that base, merged, and tag/promoted_into or merge_command; absent while active or without a batch branch; documented in docs/v3/26-HTTP-API.md 8.4. Verify passes in the worktree: run_summary_reports_where_the_verified_work_is; task_diff_reads_the_graph_task_result and the summary tests pass; clippy -p roko-serve --no-deps -D warnings clean. hermes-plugin-roko reads it (working tree, uncommitted)."
+++

## Problem

A run's summary says it `succeeded`, but not where its verified work is. roko never commits into the
operator's checkout: a run's accepted work lands on its batch branch `roko/batch/<run_id>` (and plan
branches `roko/plan/<plan>`), started at the operator's `HEAD`. Neither `GET /api/runs/{id}/summary` nor MCP
`run_status` names that branch, its tip commit, the files it changes, or whether it has been promoted or
merged. For a prompt run (`run_prompt`, `POST /api/run`) the plan is ephemeral, so
`GET /api/plans/{id}/reviews` and `/tasks/{t}/diff` return 404 as well.

Observed live on 2026-10-09 through hermes-plugin-roko: after a `succeeded` run the Hermes model ran
`find /` and other shell searches for about 3.5 minutes to locate the change. The plugin now works around
it by reading `refs/heads/roko/batch/<run_id>` with git when the workspace is on the same machine, which
does not work for a remote roko serve.

## Why it matters

The person can only accept or merge verified work they can find. Every chat host (Hermes, OpenClaw,
Claude Code over MCP) needs this, and only roko knows it for certain (delivery receipts, promotion tags).
Goal: the `/mcp` delegation surface from 9100/9112.

## Where

- `crates/roko-serve/src/routes/runs.rs::get_run_summary` and the summary builder it calls (9112).
- `crates/roko-cli/src/graph_execution/batch.rs`: `BATCH_BRANCH_PREFIX`, `RUN_TAG_PREFIX`, and the
  delivery receipts (`DeliveryReceiptStore`) that record what was merged where.

## Current state

The summary has a `delivery` milestone kind, but runs started over `/mcp` with the default settings emit
none. Branch names are deterministic (`roko/batch/<run_id>`), but whether the branch exists, what it changes
and whether it was promoted (`roko/run/<run_id>` tag) is not reported anywhere a host can read.

## Plan

1. Add `result` to the run summary: `{branch, commit, files_changed (bounded), diffstat, promoted_into?,
   tag?}`, read from the delivery receipt store and git plumbing. Omit it while the run is active.
2. `run_status` returns it unchanged, since it is the same summary.
3. Document it in `docs/v3/26-HTTP-API.md` §8.41 as part of the host contract.

## Done when

`run_summary_reports_where_the_verified_work_is` passes: a finished run's summary names its batch branch,
tip commit and changed files. The `[[verify]]` command checks this.

## Notes

hermes-plugin-roko (`../hermes-plugin-roko`, `hermes_plugin_roko/workspace.py`) can drop its git
workaround once this lands.
