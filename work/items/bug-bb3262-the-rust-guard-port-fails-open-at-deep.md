+++
id = "bug-bb3262"
kind = "bug"
title = "The Rust guard port fails open at deep command nesting, and doesn't resolve git aliases"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-std/sandbox"]
created = 2026-10-01
updated = 2026-10-01
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-guard2's report, checked on work/bug-77413c at 49acd1711)"
anchors = ["crates/roko-std/src/tool/builtin/sandbox/reads.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = ["bug-77413c"], blocks = [], related = ["bug-77413c", "bug-66f5a1"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn deep_nesting_and_git_aliases_fail_closed' crates/roko-std/src/ && cargo test -p roko-std --lib deep_nesting_and_git_aliases_fail_closed"
+++

## Problem

On bug-77413c's branch, the Rust port of the guard (`crates/roko-std/src/tool/builtin/sandbox/reads.rs`) caps how deep it follows nested commands (`MAX_COMMAND_NESTING`, :34). Past the cap it stops checking, and lets the command through, instead of refusing it. It also doesn't resolve git aliases, which the Python guard does (bug-66f5a1), so `git <alias>` that expands to a forbidden command passes (wk-guard2).

## Why it matters

Secrets and guard (epic spec-ba7bea): "missing or unknown safety contracts fail closed" is the project's rule. A cap that fails open is a bypass that needs only enough nesting.

## Where

The nesting cap and the git handling in `reads.rs`.

## Plan

1. Refuse a command whose nesting exceeds the cap.
2. Resolve git aliases (from the repo and global config) before judging a git command, as the Python guard does.
3. Add `deep_nesting_and_git_aliases_fail_closed`.

## Done when

- [ ] Deep nesting and aliased git commands are refused, or checked like any other.
- [ ] The `[[verify]]` command passes.

## Notes

- Build on bug-77413c's branch.
