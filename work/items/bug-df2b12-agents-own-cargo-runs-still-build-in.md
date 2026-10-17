+++
id = "bug-df2b12"
kind = "bug"
title = "Agents' own cargo runs still build in an exported CARGO_TARGET_DIR that every attempt worktree shares, so an agent can see a sibling attempt's build of a crate"
status = "open"
triage = "verified"
severity = "p2"
goal = "tooling"
size = "M"
subsystem = ["roko-core/child_env"]
created = 2026-10-17
updated = 2026-10-17
last_verified = 2026-10-17
source = "bug-be2a53 worker 2026-10-17: the check-side fix leaves tool commands and provider CLIs on the shared dir"
discovered_from = "work:bug-be2a53"
anchors = ["crates/roko-core/src/child_env.rs::apply_gate_env", "crates/roko-cli/src/dispatch_v2.rs::shared_target_dir"]
links = { depends_on = [], blocks = [], related = ["bug-be2a53"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn tool_commands_keep_only_their_own_build_dirs' crates/roko-core/src/ && cargo test -p roko-core --lib tool_commands_keep_only_their_own_build_dirs"
+++

## Problem

bug-be2a53 made check commands drop an inherited `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR` or
`CARGO_BUILD_BUILD_DIR` that names a directory outside the check's own, so gates in different checkouts no longer
share builds. The agent's own builds still share them:

- the commands an agent runs through roko's tools (`bash`, `run_tests`, ACP's `bash`) get the gate policy
  (`child_env::apply_gate_env`), which keeps every `CARGO_*` variable;
- provider CLIs (Claude Code's own Bash tool, Codex, ...) keep their inherited environment minus credentials
  (`CredentialScrub`).

So with `CARGO_TARGET_DIR` exported, an agent's `cargo check` in attempt worktree B can take attempt A's build of a
crate as fresh, because cargo names a workspace crate's build alike in every checkout and B's copy is older. The
agent then sees a pass on code that does not compile, or a failure against A's API. Its builds also never warm the
worktree's own `target/`, which is where its gates now build.

## Why it matters

Agents act on that feedback. A false compile pass costs an attempt at the gate, and a false failure sends the agent
after a problem that is not in its tree. The FAST lane shares the canonical repository's target with agents on
purpose (`dispatch_v2.rs::shared_target_dir`, `ROKO_AGENT_SHARED_TARGET=1`), so it has the same hazard.

## Where

- `crates/roko-core/src/child_env.rs::apply_gate_env` (tool commands) and `CredentialScrub` (provider CLIs).
- `crates/roko-cli/src/dispatch_v2.rs::shared_target_dir` (the FAST lane's sharing).
- The check-side rule to reuse: `child_env.rs::build_dir_is_own` and `drop_shared_build_dirs` (bug-be2a53).

## Current state

Checks follow the rule since bug-be2a53. Tool commands and provider CLIs do not.

## Plan

1. Give tool commands and provider CLIs that run in an attempt worktree the checks' rule: keep a cargo build dir
   only when it is inside their directory. The agent's builds and the gates' then share the worktree's own target.
2. Decide whether the FAST lane keeps its deliberate sharing. If it does, document the hazard where it is enabled.

## Done when

- [ ] A tool command an agent runs in an attempt worktree does not inherit a `CARGO_TARGET_DIR` outside it, and a
  test pins it.
