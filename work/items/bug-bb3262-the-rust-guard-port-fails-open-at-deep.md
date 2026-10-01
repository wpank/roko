+++
id = "bug-bb3262"
kind = "bug"
title = "The Rust guard port fails open at deep command nesting, and doesn't resolve git aliases"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "S"
subsystem = ["roko-std/sandbox"]
created = 2026-10-01
updated = 2026-10-01
last_verified = 2026-10-01
last_verified_rev = "49acd1711"
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
- Premise held at 49acd1711: `check_command` returned `Ok(())` past `MAX_COMMAND_NESTING` (8), so a search nested in nine `eval`s or `sh -c`s went unchecked, and `check_git` judged only `git grep`, whatever `git <alias>` runs.
- Nesting: a command line nested deeper than 8 is refused (`CommandNotAllowed`), as the guard blocks it.
- Aliases, decided: refused, not resolved. A git subcommand that is not one of git's commands (a static list, then `git --list-cmds=main,others`, cached) is refused, since an alias can run any read and the same command line can define one (`git -c alias.x='!cat roko.toml' x`, or `git config alias.x ...` then `git x`). Where `git` may be an argument of a wrapper (`timeout 5 grep git src`), it is let through, as in the guard. The guard resolves aliases instead, so it allows an alias that expands to an allowed command where the port refuses; the shared table runs outside a repository and has no alias case.
- Also failing closed, the same kind of cap: a brace expansion or a glob component past 4096 words is refused in the port (it cut at 1024), and the guard refuses a brace expansion past 4096 (it cut at 64), rather than checking a truncated list.
- Test: `deep_nesting_and_git_aliases_fail_closed` (this item's verify) refuses 9 nested `eval`s, 8192- and 5000-word brace expansions, `git st`, `git -c alias.x=... x`, `sudo git -C . x` and `git $sub`, and allows 8 nested `eval`s, 4096 words, `git status`, `git -C . log --oneline`, `git --version`, `timeout 5 grep git src` and `xargs grep git src`.
- Checked without cargo: built standalone from the real sources, the test passes and lints clean with the workspace's clippy settings.
- Implemented on `work/guard-l9` at `4a75e9965`; cargo verification deferred to the batch check.
