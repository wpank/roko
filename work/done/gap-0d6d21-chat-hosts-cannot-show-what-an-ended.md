+++
id = "gap-0d6d21"
kind = "gap"
title = "Chat hosts cannot show what an ended run changed: no route returns the diff of its verified work"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "M"
subsystem = ["roko-serve/runs"]
created = 2026-10-13
updated = 2026-10-13
last_verified = 2026-10-13
last_verified_rev = "4bc447702"
source = "hermes-plugin-roko UI push 2026-10-13 (diff viewer), roko 473d41ab9"
anchors = ["crates/roko-serve/src/routes/run_result.rs", "crates/roko-serve/src/routes/runs.rs"]
links = { depends_on = [], blocks = [], related = ["gap-8409d0"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn run_diff_returns_the_runs_patches' crates/roko-serve/ && cargo test -p roko-serve run_diff_returns_the_runs_patches"

[closed]
at = 2026-10-13
at_ts = "2026-10-13T13:22:29Z"
commit = "4bc447702"
by = "commit trailer"
executor = "claude-agent"
size = "M"
claimed_at = "2026-10-13T12:42:44Z"
forced = false
evidence = "4bc447702: GET /api/runs/{run_id}/diff returns each file an ended run's batch changes from its base (status, line counts, binary flag, unified patch or null, truncated) within max_files (200, 1-500) and max_patch_bytes (65536, 1024-262144, cut on a line boundary, streamed), with totals; 409 active, 404 no result, 400 bad limits; run_result.rs result_range shared with the summary; OpenAPI + docs 8.4. Verify passes in the worktree: run_diff_returns_the_runs_patches; summary, task-diff and OpenAPI coverage tests pass; clippy -p roko-serve --no-deps -D warnings clean."
+++

## Problem

A chat host can say where an ended run's verified work is (the summary's `result`, gap-8409d0) but cannot show
what it changed. The only diff routes are per task (`GET /api/plans/{id}/tasks/{t}/diff`), which need the
plan, so they answer 404 for prompt runs, and they diff against `main` rather than the batch's base.
hermes-plugin-roko reads the branch with local git, which does not work for a remote roko serve.

## Why it matters

Reviewing the change before merging it is the step every chat host's person takes after a run succeeds. A
diff viewer in Hermes' desktop app and web dashboard needs the run's files and patches from roko itself.

## Where

- `crates/roko-serve/src/routes/run_result.rs`: computes the batch branch, its tip and its base.
- `crates/roko-serve/src/routes/runs.rs`: the `/runs/{run_id}/...` routes.

## Current state

Open at `473d41ab9`.

## Plan

1. `GET /api/runs/{run_id}/diff` (read scope): each changed file from the run's base to its batch tip, with
   status, line counts, a binary flag and its unified patch, bounded by `max_files` (default 200, at most 500)
   and `max_patch_bytes` per file (default 65536, at most 262144), plus totals over every file. 409 while the
   run is active, 404 when it has no result.
2. Reuse run_result.rs's branch, tip and base, so the diff and the summary agree.
3. Test it on a git repository with a modified, an added, a deleted and a binary file.

## Done when

`run_diff_returns_the_runs_patches` passes (the `[[verify]]` command).
