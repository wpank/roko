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

- Correction: the original finding was wrong for roko's agents. Claude Code runs Grep as `rg --hidden`, but roko's settings have denied `Read(//**/.roko/<name>)` for every key file since bug-a66941 (9c0d7196b). Claude Code (2.1.282, read in the CLI) turns Read deny rules into `--iglob` exclusions for its own Grep and Glob: each rule is re-rooted at the searched directory and appended after the Grep's own globs, as `--iglob '!/**/.roko/.env'`. So a Grep never reads the key files. What this item fixes is the config case, which no deny rule covers.
- Config premise held at 9fd38f50f: the guard read a Grep's glob as one pattern, so a Grep with `glob: "*.rs *.toml"` or `"*.rs,*.toml"` reached a roko.toml holding a secret (exit 0). It also refused a Grep whose config git ignores.
- Rule (the coordinator chose option b): the guard refuses a Grep that would read a secret-holding roko config in its tree (`secret_configs`: the tree's roko.toml, the workspace's, the file `ROKO_CONFIG` names, the legacy one). It judges the Grep as rg `--hidden` with its type and its globs, split as Claude Code splits them (whitespace, then commas outside braces), and exempts a file git ignores (`git check-ignore -q` in the file's repository), since rg honours `.gitignore`; rg's own `.ignore` and `.rgignore` are not consulted. The key files are left to the deny rules and the Read hook; a Grep rooted at a `.roko` directory is still refused. A first version (53b454e02) also refused a Grep whose tree held a key file; b00a103b0 dropped that, since on this CLI it refused every root Grep that was already safe wherever `.roko` is not git-ignored, the normal state after `roko init`.
- Glob, decided: no tree rule. Claude Code's Glob runs `rg --files --hidden --no-ignore` plus the same deny-rule exclusions; it lists names only, and a Read of a key file it lists is refused.
- Depth bound, decided, for the shell tree rule (a Grep checks configs at known places and walks nothing): below a searched root, two levels down always (at most 4096 directories a level), and deeper while fewer than 256 directories have been read in all, never into a hidden directory or through a symlink; the workspace's `.roko` above the command's or the call's directory and `~/.roko` count at any depth. Measured with the guard's walk: two levels took 0 to 18 ms at the roko root, its worktrees' parent, HOME and `crates/`; the extension to 256 directories 18 to 98 ms (`target/` the slowest); a 1024-directory budget took 0.6 s at the roko root. roko-std's port walks the same way.
- Tests: `grep_tool_at_a_workspace_root_cannot_read_a_key_file` (this item's verify) checks the settings' deny rule for each key file; that a root Grep over key files runs while a Grep rooted at `.roko` and a Read of a key file do not; the config cases with paths, globs and types; and the git-ignore exemption after `git init`. The shared table's 12 `Grep:` rows (177 rows in all) follow the narrowed rule; roko-agent's table test sends them to the file hook and roko-std's skips them. `recursive_search_reaching_a_key_file_is_refused` pins the depth bound.
- gap-18d1a7 (Read deny rules for the key files) was closed as superseded by bug-a66941, whose rules are the ones above.
- Checked without cargo: the guard passes the table, its other suites and a suite mirroring the new test (configs, git exemption, key files left to the rules) under python 3.12 and 3.9. The port, built standalone from the real sources, passes the table's shell rows and its three tests, lints clean, and still agrees with the guard over 4,000 more fuzzed commands, each run with and without the key files.
- Implemented on `work/guard-l9` at `b00a103b0` (narrowing `53b454e02`); cargo verification deferred to the batch check.
