+++
id = "bug-8bafc0"
kind = "bug"
title = "roko doctor's git_dirty fix says git add -A && git commit, which would also commit unrelated untracked files such as work/telemetry/events/*.jsonl"
status = "open"
triage = "verified"
severity = "p3"
goal = "core"
size = "S"
subsystem = ["roko-execution/diagnostics"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
source = "hermes-roko round v5, dogfood-will run 2 (R2-7), 2026-10-16"
anchors = ["crates/roko-execution/src/diagnostics/checks.rs::check_git"]
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn git_dirty_names_the_paths_and_never_adds_everything' crates/ && cargo test -p roko-execution --lib git_dirty_names_the_paths_and_never_adds_everything"
+++

## Problem

`roko doctor`'s shared git check (`shared_git_dirty`) offers `git add -A && git commit` as the fix for any
uncommitted change. Seen 2026-10-16 in the hermes-roko round v5 dogfood (dogfood-will, run 2, finding R2-7): the
planner left an untracked plan folder (`plans/word-count-demo/`), and doctor printed

```
[warn] shared_git_dirty: working tree has 1 uncommitted change(s) — consider committing before plan run to avoid worktree merge conflicts - changed_count=1
    → fix: git add -A && git commit
```

In Will's repo that command would also commit his other untracked files (`work/telemetry/events/*.jsonl`), which
must not be committed.

## Why it matters

A person who copies a fix roko suggests should get exactly that fix. `git add -A` stages every untracked file in
the checkout, so the suggested commit can sweep in files nobody meant to commit, including local-only ones.

## Where

- `crates/roko-execution/src/diagnostics/checks.rs::check_git`: the finding and its remediation.
- Other `git add -A` / `git add .` text in the repo (grep 2026-10-16): `plans/demos/README.md` and `dev.sh` run it
  in a repo they create a line earlier with `git init`; the portal-programme harness and
  `tests/proof/mori-diffs/` scripts run it in scratch workspaces they make; `roko-agent`'s `safety/normalize.rs`
  holds it as a fixture. None of these is a hint about a person's own checkout, so they stay.

## Current state

Open at `1b8b62fb9`.

## Plan

Name the paths `git status --porcelain` finds. Up to three: the fix is `git add -- <those paths> && git commit`.
More: the finding says "commit or remove these" and lists them (ten at most, then how many more), with no command.
Never `-A`.

## Done when

`git_dirty_names_the_paths_and_never_adds_everything` passes (the `[[verify]]`), and `roko doctor` in a workspace
with an untracked plan folder suggests `git add -- plans/<dir>/ && git commit`.
