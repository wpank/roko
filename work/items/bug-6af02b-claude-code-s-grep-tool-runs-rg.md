+++
id = "bug-6af02b"
kind = "bug"
title = "Claude Code's Grep tool runs rg --hidden, so a Grep at a workspace root reads .roko/.env unless .gitignore covers it; the guard's Grep rule doesn't apply the tree check"
status = "open"
triage = "unverified"
severity = "p1"
goal = "release"
size = "S"
subsystem = ["roko-agent/safety", "roko-std/sandbox"]
created = 2026-10-01
updated = 2026-10-01
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
