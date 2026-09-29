+++
id = "find-570af2"
kind = "finding"
title = "When HOME is the workdir, the key-file policy refuses agents the whole .roko directory"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "S"
subsystem = ["safety"]
created = 2026-09-29
updated = 2026-09-29
last_verified = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (15:47, wk-guard's report on bug-a66941)"
anchors = ["crates/roko-core/src/child_env.rs::key_file_paths", "crates/roko-agent/src/safety/path.rs"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-a66941"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn key_file_policy_when_home_is_workdir' crates/roko-core/src/ crates/roko-agent/src/ && cargo test -p roko-core --lib key_file_policy_when_home_is_workdir"
+++

## Problem

When `HOME` equals the workdir, bug-a66941's key-file policy refuses agents all of `<workdir>/.roko`, not only the key files in it. roko-cli's end-to-end tests set `HOME=workdir`, but they use fake agents, so they never hit this (wk-guard, 2026-09-29).

## Why it matters

Containers and CI often run with `HOME` set to the project directory, and there agents would lose access to `.roko` plans and state. Epic spec-ba7bea.

## Where

`child_env::key_file_paths(home, workdir)` and the path policy that consumes it.

## Current state

The policy refuses the whole directory.

## Plan

1. Decide whether the policy refuses only the key-file names, or `~/.roko` as a whole. The recommended default is key-file names only.
2. Narrow the policy if that's the decision.
3. Add `key_file_policy_when_home_is_workdir`.

## Done when

- [ ] The behaviour with `HOME` equal to the workdir is decided and tested.
- [ ] The `[[verify]]` command passes.

## Notes

- Premise confirmed at `407ce30d5`, and wider than stated: plan worktrees live under `<workdir>/.roko/worktrees/`, so with `HOME` equal to the workdir the Claude guard refused `Read src/lib.rs` inside a plan worktree and any Bash command naming the worktree's absolute path, and the `Read(~/.roko/**)` / `Edit(~/.roko/**)` deny rules did the same.
- Decision: key-file names only (the recommended default). `.env`, `secrets.toml` and `credentials.json` are refused in any `.roko` directory, `~/.roko` included; the rest of `.roko` stays readable. `is_key_file(path)` drops its `home` argument; SafetyLayer's path policy, roko-std's `refuse_key_file`, the Claude deny rules and both guard hooks follow. The Bash hook also denies a glob directly in a `.roko` directory (`cat ~/.roko/*`).
- Trade-off: `~/.roko/config.toml`, which can hold `serve.auth.api_key`, is now readable, and a Bash command that reads a whole `.roko` directory (`grep -r`, `tar`) is no longer caught for `~/.roko`, as it never was for a workdir's.
- Implemented on `work/bug-f4e133` at `2b5dbde35`; cargo verification deferred to the batch check.
