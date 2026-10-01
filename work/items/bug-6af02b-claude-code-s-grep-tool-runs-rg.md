+++
id = "bug-6af02b"
kind = "bug"
title = "Claude Code's Grep tool runs rg --hidden, so a Grep at a workspace root reads .roko/.env unless .gitignore covers it; the guard's Grep rule doesn't apply the tree check"
status = "open"
triage = "verified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-agent/safety", "roko-std/sandbox"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "9fd38f50f"
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report on bug-fa1537 and bug-bb3262, branch work/guard-l9 at 4a75e9965)"
anchors = ["crates/roko-agent/src/claude_cli_guard.py", "crates/roko-std/src/tool/builtin/sandbox/reads.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-fa1537", "bug-bb3262", "bug-77413c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn grep_tool_at_a_workspace_root_cannot_read_a_key_file' crates/roko-agent/src/ && cargo test -p roko-agent --lib grep_tool_at_a_workspace_root_cannot_read_a_key_file"
+++

## Problem

Claude Code's built-in Grep tool runs `rg --hidden` (checked in the CLI binary by wk-guard2), so a Grep over a workspace root also searches `.roko/.env` and other key files unless a `.gitignore` excludes them. The guard's Grep rule still checks only the named path, not the tree a recursive search reaches, so bug-fa1537's tree rule for shell reads does not cover it.

Known gap from the same work: a `.roko` directory more than two levels below the search root, and off the cwd's path, is not found by the tree rule.

## Why it matters

Secrets and git guard (epic spec-ba7bea): a provider key read by an agent can end up in its transcript, its output or a commit.

## Plan

1. Apply the tree rule to Claude Code's Grep (and Glob, if it can list key files): refuse a search whose tree contains a key file the search would read, with a `git check-ignore` exemption for files the repository ignores, since rg skips them only when the ignore file is honoured and `--hidden` does not override `.gitignore`.
2. Decide how deep to look for nested `.roko` directories, or walk the search tree for them when the tree is small. Record the bound in the notes.
3. Add `grep_tool_at_a_workspace_root_cannot_read_a_key_file` (Rust settings-hook test) plus a row in the shared case table for the Python guard.

## Done when

- [ ] A Grep at a workspace root with a key file present is refused unless the file is git-ignored.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise held at 9fd38f50f: the guard's Grep rule considered only secret-holding configs, so a Grep `{"pattern": "OPENAI"}` at a root holding `.roko/.env` exited 0. Read in the installed CLI: Claude Code runs Grep as `rg --hidden`, with `--glob !<dir>` for each VCS directory, `--type` for its type, and its glob split at whitespace, then at commas unless the word holds a brace.
- Rule: a Grep whose tree holds a key file or a secret-holding config (the same `sensitive_files` as for shell reads) is refused when rg would read the file, judged by `search_reads` for rg with `--hidden` and the Grep's type and globs. A file git ignores is exempt (`git check-ignore -q` in the file's repository), since rg honours `.gitignore`; rg's own `.ignore` and `.rgignore` files are not consulted, so a file only they leave out is still refused. The refusal tells the agent to narrow the path, or to add the glob `!.roko`.
- Glob, decided: no tree rule. Claude Code's Glob runs `rg --files --hidden --no-ignore`, so it can list `.roko/.env`, but only its name; reading it takes a Read or a Grep of that path, which the existing rules refuse.
- Depth bound, decided: below a searched root, two levels down always (at most 4096 directories a level), and deeper while fewer than 256 directories have been read in all, never into a hidden directory or through a symlink; the workspace's `.roko` above the command's or the call's directory and `~/.roko` count at any depth. Measured with the guard's walk: two levels took 0 to 18 ms at the roko root, its worktrees' parent, HOME and `crates/`; the extension to 256 directories 18 to 98 ms (`target/` the slowest); a 1024-directory budget took 0.6 s at the roko root, where build trees hold huge directories. roko-std's port walks the same way, so shell reads get the same bound.
- Tests: `grep_tool_at_a_workspace_root_cannot_read_a_key_file` (this item's verify) covers a Grep at the root, with a path, glob or type, the Glob and Read cases, and the git-ignore exemption after `git init`. The shared table gains 12 `Grep:` rows (177 in all), which roko-agent's table test sends to the file hook and roko-std's skips. `recursive_search_reaching_a_key_file_is_refused` now pins the bound: a small tree is searched whole, a wide one past two levels is not.
- Possible follow-up, not done: Claude Code also passes Grep `--iglob` exclusions taken from the settings' permission rules, so Read deny rules for the key files in the `--settings` payload would make Grep skip them itself.
- Checked without cargo: the guard passes the table, its other suites, and a suite mirroring the new test (git exemption, nested repositories, configs) under python 3.12 and 3.9. The port, built standalone from the real sources, passes the table's shell rows and its three tests and lints clean, and a further 4,000 fuzzed commands, each run with and without the key files, found no disagreement between guard and port.
- Implemented on `work/guard-l9` at `53b454e02`; cargo verification deferred to the batch check.
