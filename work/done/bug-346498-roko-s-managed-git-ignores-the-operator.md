+++
id = "bug-346498"
kind = "bug"
title = "roko's managed git ignores the operator's custom core.excludesFile (GIT_CONFIG_GLOBAL=/dev/null), so attempt acceptance commits files the operator's global gitignore hides, while ignored_dirs and the review diff read that file"
status = "done"
triage = "verified"
severity = "p2"
size = "S"
subsystem = ["roko-cli/worktree"]
created = 2026-10-20
updated = 2026-10-21
last_verified = 2026-10-21
last_verified_rev = "9f501ee5d"
source = "bug-ba0c40 worker, 2026-10-20"
discovered_from = "bug-ba0c40"
anchors = ["crates/roko-cli/src/orchestrator/worktree/git_ops.rs::sanitize_git_environment", "crates/roko-cli/src/orchestrator/worktree/acceptance.rs::commit_attempt", "crates/roko-cli/src/orchestrator/worktree/git_ops.rs::ignored_dirs", "crates/roko-cli/src/orchestrator/worktree/acceptance.rs::attempt_review_diff"]
links = { depends_on = [], blocks = [], related = ["bug-ba0c40"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acceptance_honours_the_operators_excludes_file' crates/roko-cli/src && cargo test -p roko-cli --lib acceptance_honours_the_operators_excludes_file"

[closed]
at = 2026-10-21
at_ts = "2026-10-21T05:21:45Z"
commit = "9f501ee5d"
by = "commit trailer"
executor = "claude-agent"
via = "work-batch"
size = "S"
claimed_at = "2026-10-21T01:31:05Z"
forced = false
evidence = "9f501ee5d resolves the operator's core.excludesFile with their own config (never the sanitized GIT_CONFIG_GLOBAL=/dev/null environment) and passes it explicitly to commit_attempt's `git add --all` as `-c core.excludesFile=<path>`, so it agrees with ignored_dirs and attempt_review_diff, which already read it with the operator's config. git_ops.rs (new): git_probe_output_with_operator_config_at is git_probe_output_at without sanitize_git_environment; resolved_operator_excludes_file runs `git config --path --get core.excludesFile` through it once and caches the result in WorktreeManager's new resolved_excludes_file field (mod.rs, mirrors resolved_git_executable; set_test_git_probe_environment clears it so a test's HOME override is honoured); operator_excludes_file_args turns a resolved path into the `-c core.excludesFile=...` pair, or nothing when the operator set none, since the sanitized environment's own default excludes file ($XDG_CONFIG_HOME/git/ignore) already matches the operator's git for that case, independent of GIT_CONFIG_GLOBAL (checked by hand). acceptance.rs's commit_attempt applies those args; attempt_review_diff and ignored_dirs are untouched, since they already ran with the operator's config. Checked through heavy.sh on the committed code (this machine's 3-slot cargo queue ran 5-9 deep most of the session, 15-45 min per command): cargo check -p roko-cli --lib clean; the verify test acceptance_honours_the_operators_excludes_file passes (1 passed, 0 failed) via cargo test -p roko-cli --lib acceptance_honours_the_operators_excludes_file -- --nocapture; nightly rustfmt leaves the diff unchanged; cargo clippy -p roko-cli --no-deps -- -D warnings (CLAUDE.md's mandated form) clean. A manual git 2.39.5 reproduction (outside Rust, in a throwaway scratch HOME, nothing committed) confirmed the underlying mechanism directly: sanitized `git add --all` stages a custom-ignored file without the -c override and excludes it with the override, matching the operator's own git status exactly in both the set and unset core.excludesFile cases; re-running the Rust test in a reverted state was skipped given the queue, since this direct reproduction already isolates the exact git behaviour the fix and test both rely on. cargo clippy -p roko-cli --no-deps --all-targets -- -D warnings (not the mandated form) surfaced 78 pre-existing, unrelated findings in roko-cli's test code across about 40 files; filed as gap-7bf92d rather than fixed here."
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
