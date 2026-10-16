+++
id = "bug-2e887e"
kind = "bug"
title = "roko init leaves .roko/ untracked and out of .gitignore, so a newcomer's git add -A commits roko's state"
status = "done"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-cli/init"]
created = 2026-10-16
updated = 2026-10-16
last_verified = 2026-10-16
last_verified_rev = "c7f8f0e03"
source = "round v5 final dogfood, 2026-10-16 (team lead)"
anchors = ["crates/roko-cli/src/commands/util.rs::cmd_init"]
links = { depends_on = [], blocks = [], related = ["bug-4e336b", "bug-6af02b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn init_adds_roko_state_to_gitignore' crates/roko-cli/ && cargo test -p roko-cli --lib init_adds_roko_state_to_gitignore"

[closed]
at = 2026-10-16
at_ts = "2026-10-16T04:13:32Z"
commit = "c7f8f0e03"
by = "roko-init"
executor = "claude-agent"
forced = false
evidence = "c7f8f0e03 (on main 245bc3079). roko init adds '.roko/' (after a one-line comment) to the project's .gitignore, creating the file if needed and keeping what is there, and says so; the TUI's welcome init does the same. It changes nothing when git already ignores .roko/ (repository rules, .git/info/exclude, global excludes) or a rule names .roko, so roko-style negated rules (!/.roko/GAPS.md) are kept. Checks at c7f8f0e03: init_adds_roko_state_to_gitignore (the [[verify]] command) passes, covering create, append-and-keep, idempotence, roko's own rules and git excludes. All roko-cli lib/bin tests pass. All 73 roko-cli integration targets ran; the only failures are known items (bug-2a31bc x2, bug-c63cca smoke item_03, bug-ee2752). clippy -D warnings and fmt clean. CLI, fresh scratch git projects: before (old binary) git status listed '?? .roko/'. After: 'added .roko/ to .gitignore' (existing file kept) or 'created .gitignore with .roko/', git status no longer lists .roko/, a re-run changes nothing, and roko-style rules stay byte-identical. Evidence: tmp/hermes-roko-v5/roko-init/evidence/gitignore-{before,after}.out"
+++

## Problem

`roko init` creates `.roko/` (roko's state, logs and run records, and `.roko/.env`, a workspace's own secrets)
but leaves it untracked and out of `.gitignore`. After `roko init`, `git status` shows `?? .roko/`, so a
newcomer's `git add -A` commits roko's state, and any key in `.roko/.env`. Seen in the round v5 final dogfood
(2026-10-16).

## Why it matters

Committed run state bloats the repository and conflicts between machines. A committed `.roko/.env` leaks keys.
Agents that skip ignored files (bug-6af02b) only skip `.roko/` when an ignore rule covers it.

## Where

- `crates/roko-cli/src/commands/util.rs::cmd_init`: creates `.roko/` and writes `roko.toml`.
- `crates/roko-cli/src/commands/init.rs`: the init helpers. The TUI's welcome init writes the same workspace.

## Plan

1. `roko init` appends `.roko/` (with a one-line comment) to the project's `.gitignore`. It creates the file if
   needed, keeps what is there, and says what it did.
2. It skips that when git already ignores `.roko/` (the repository's rules, `.git/info/exclude`, global
   excludes), or when a rule in `.gitignore` already names `.roko`. A repository that tracks part of `.roko/`
   on purpose, as roko's own does (`!/.roko/GAPS.md`), keeps its rules.
3. Test: create, append (keeping the file), idempotent re-run, a repository's own `.roko` rules kept, and git's
   excludes counted.

## Done when

`init_adds_roko_state_to_gitignore` passes (the `[[verify]]` command), and in a scratch git project
`git status` after `roko init` no longer lists `.roko/`.
