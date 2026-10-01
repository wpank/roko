+++
id = "bug-919fe8"
kind = "bug"
title = "roko redirects any workdir under a .roko directory to the outer project, including per-task worktrees in .roko/worktrees"
status = "open"
triage = "verified"
severity = "p3"
goal = "tooling"
size = "S"
subsystem = ["roko-cli/main"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-10-01
last_verified_rev = "ebdc0f5d5"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (16:47, wk-evidence's report on gap-09e478)"
anchors = ["crates/roko-cli/src/main.rs::resolve_workdir"]
lane = "rust-cold"
parent = "spec-9a3131"
links = { depends_on = [], blocks = [], related = ["gap-4ec59f"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn resolve_workdir_keeps_a_workspace_nested_under_dot_roko' crates/roko-cli/src/ && cargo test -p roko-cli --bin roko resolve_workdir_keeps_a_workspace_nested_under_dot_roko"
+++

## Problem

`resolve_workdir` (`main.rs:4102`) canonicalizes the working directory (or `--repo`). If any ancestor is named `.roko`, it returns that ancestor's parent and prints "Auto-correcting: running from inside .roko/, using project root". Its comment gives the intent: avoid a nested `.roko/.roko/` data dir when someone runs roko from inside a project's own data directory. The check has no other condition, so it also fires for real workspaces that live under a `.roko` directory:

- per-task worktrees, which `--worktree-per-task` creates under `<workdir>/.roko/worktrees/` (`plan_runner.rs:1173`). A `roko` or `cargo run -p roko-cli --` command in a task's verify step runs against the main checkout instead of the worktree;
- scratch or fixture workspaces created under `.roko/`; wk-evidence hit this with an evidence fixture;
- an explicit `--repo` that points at such a directory.

## Why it matters

Hygiene (epic spec-9a3131), with a real risk for self-hosting: a verify command that calls roko inside a worktree checks, and may write to, the main checkout's `.roko/` state. The redirect is quiet apart from one line on stderr.

## Where

`crates/roko-cli/src/main.rs::resolve_workdir`, and its tests `resolve_workdir_uses_repo_flag`, `resolve_workdir_defaults_to_cwd` and `resolve_workdir_canonicalizes_existing_repo_flag` (about :5611-5640).

## Current state

Unchanged at BASE. Graph worktrees are opt-in (`--worktree-per-task`), which may be why plan runs haven't hit it.

## Plan

1. Redirect only when the directory is not itself a workspace. Walk up from the directory itself: stop at the first directory that has a `roko.toml`, a `.git` entry or its own `.roko/`, and use it. Redirect only if the walk reaches a `.roko` component first.
2. Never redirect an explicit `--repo`; warn instead.
3. Add `resolve_workdir_keeps_a_workspace_nested_under_dot_roko` (a git worktree at `.roko/worktrees/x` stays put), next to a test that `.roko/state` still redirects.

## Done when

- [ ] A workspace under `.roko/` is used as it is, and running from inside a project's own data directory still redirects.
- [ ] The `[[verify]]` command passes.

## Notes

- If gap-4ec59f makes per-task worktrees the default, raise this to p2.
- 2026-10-01 (wk-climain): implemented on work/bug-28c193; cargo verification deferred to the batch check.
  `resolve_workdir` now asks `enclosing_project_of_data_dir`, which walks up from the directory and stops without a
  redirect at the first directory with a `roko.toml`, a `.git` entry or its own `.roko/`; only a `.roko` component
  reached first redirects. An explicit `--repo` inside a `.roko/` is used as given, with a warning. Tests:
  `resolve_workdir_keeps_a_workspace_nested_under_dot_roko`, `resolve_workdir_still_redirects_from_the_data_dir`.
