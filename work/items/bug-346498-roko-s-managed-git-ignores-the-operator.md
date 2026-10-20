+++
id = "bug-346498"
kind = "bug"
title = "roko's managed git ignores the operator's custom core.excludesFile (GIT_CONFIG_GLOBAL=/dev/null), so attempt acceptance commits files the operator's global gitignore hides, while ignored_dirs and the review diff read that file"
status = "open"
triage = "verified"
severity = "p2"
size = "S"
subsystem = ["roko-cli/worktree"]
created = 2026-10-20
updated = 2026-10-20
last_verified = 2026-10-20
source = "bug-ba0c40 worker, 2026-10-20"
discovered_from = "bug-ba0c40"
anchors = ["crates/roko-cli/src/orchestrator/worktree/git_ops.rs::sanitize_git_environment", "crates/roko-cli/src/orchestrator/worktree/acceptance.rs::commit_attempt", "crates/roko-cli/src/orchestrator/worktree/git_ops.rs::ignored_dirs", "crates/roko-cli/src/orchestrator/worktree/acceptance.rs::attempt_review_diff"]
links = { depends_on = [], blocks = [], related = ["bug-ba0c40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acceptance_honours_the_operators_excludes_file' crates/roko-cli/src && cargo test -p roko-cli --lib acceptance_honours_the_operators_excludes_file"
+++

## Problem

roko's managed git commands run with `GIT_CONFIG_GLOBAL=/dev/null` and `GIT_CONFIG_NOSYSTEM=1`
(`crates/roko-cli/src/orchestrator/worktree/git_ops.rs::sanitize_git_environment`). Git then never reads a
`core.excludesFile` that the operator set in `~/.gitconfig`. It still reads the default
`~/.config/git/ignore`. So attempt acceptance (`acceptance.rs::commit_attempt`, `git add --all`) stages files
that the operator's own git ignores, and commits them.

Other worktree code runs plain `git` with the operator's config and reads that excludes file:
`git_ops.rs::ignored_dirs`, which chooses the `node_modules` directories (gap-2da1ff) and the warm CMake build
trees (gap-6754ab, `build_trees.rs::warm_build_dirs`) that each new attempt checkout gets, and
`acceptance.rs::attempt_review_diff`, which shows the review hold's diff. The two views can disagree.

Reproduced on 2026-10-20 with git 2.39.5, in a scratch HOME. `~/.gitconfig` sets
`core.excludesFile = ~/.gitignore_global`, which lists `node_modules/` and `build/`. In a repository with
`node_modules/x/index.js` and `build/CMakeCache.txt`:

- `git status --porcelain --untracked-files=all` prints nothing;
- with `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1`, it prints both files as untracked (`??`);
- with the same lines in `~/.config/git/ignore` instead, both views ignore them.

## Why it matters

An operator who keeps `node_modules/`, `build/`, `.DS_Store`, `.idea/` or `.env` out of git only through a
custom global excludes file gets them committed onto the plan branch and delivered. Since bug-ba0c40, build
output under `node_modules/`, a `target/` or a CMake tree (`CMakeCache.txt`) that the task's `files` do not name
is left out anyway. Every other globally ignored file is still committed, and the review hold
(`attempt_review_diff`) shows a different change from the one acceptance commits.

## Where

- `crates/roko-cli/src/orchestrator/worktree/git_ops.rs::sanitize_git_environment`: the managed environment.
- `crates/roko-cli/src/orchestrator/worktree/acceptance.rs::commit_attempt`: `git add --all` in that environment.
- `crates/roko-cli/src/orchestrator/worktree/git_ops.rs::ignored_dirs`: plain `git`, with the operator's config.
- `crates/roko-cli/src/orchestrator/worktree/acceptance.rs::attempt_review_diff`: plain `git` as well.

## Plan

1. Resolve the operator's excludes file once, with the operator's own config (`git config --path --get
   core.excludesFile`, falling back to git's XDG default). Pass it to the managed commands that stage or list
   untracked files as `-c core.excludesFile=<path>`, and keep the rest of the sanitized environment.
2. Or run `ignored_dirs` and `attempt_review_diff` in the managed environment too, so that every view agrees.
   This is simpler, but it commits what the operator ignores globally.
3. Test: with a scratch HOME whose `~/.gitconfig` names an excludes file that ignores `scratch.log`, an attempt
   that leaves `scratch.log` commits only its declared change.

## Done when

- [ ] The `[[verify]]` test passes, and fails before the fix.

## Notes

- Found while fixing bug-ba0c40 (2026-10-20). The test must never write under the real home: use a temporary
  HOME and a temporary repository.
