+++
id = "gap-8f8544"
kind = "gap"
title = "Sandbox levels are enforced only in-process: no OS sandbox confines agent processes"
status = "open"
triage = "unverified"
severity = "p2"
goal = "release"
size = "L"
hold = "Decided 2026-09-29 (spec-ba7bea): no OS sandbox in v1; revisit before any hosted demo"
subsystem = ["roko-agent/safety"]
created = 2026-09-29
updated = 2026-09-29
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (14:53, wk-wp-matrix); docs/whitepaper/data/mechanisms.toml (row IS6)"
anchors = ["crates/roko-agent/src/safety/sandbox.rs::SandboxLevel", "crates/roko-agent/src/safety/mod.rs::pre_dispatch_check_with_context", "crates/roko-agent/src/safety/mod.rs::post_dispatch_check", "crates/roko-agent/src/claude_cli_agent.rs::build_settings_json"]
lane = "rust-cold"
parent = "spec-ba7bea"
links = { depends_on = [], blocks = [], related = ["bug-a66941", "gap-8be530", "bug-7de5df", "gap-70e445", "gap-b72761"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn isolate_level_confines_cli_agent_to_worktree' crates/roko-agent/ && cargo test -p roko-agent isolate_level_confines_cli_agent_to_worktree"
+++

## Problem

`SandboxLevel` (`crates/roko-agent/src/safety/sandbox.rs:14`) has five levels: `None`, `Observe`, `Restrict` (the
default), `Isolate` ("deny network and constrain filesystem access to the worktree") and `Quarantine` ("no filesystem,
network, subprocess, or git"). Only roko's own code enforces them:

- roko's file tools check each path (`safety/path.rs::canonicalize_with_policy`);
- `SafetyLayer::pre_dispatch_check_with_context` and `post_dispatch_check` (`safety/mod.rs:976-1107`) are called only
  by the ACP server (`roko-acp/src/bridge_events/mod.rs:625`, `:914`; `runner.rs:1615`, `:1643`, `:2130`). Plan runs
  attach a `SafetyLayer` to the agent's options (`agent_spawn.rs`) but never call these two checks.

A CLI-subprocess agent (Claude Code, Codex, Cursor or Gemini CLI) is a separate process, and its own tools never pass
through roko's path checks. The Claude CLI runs with permission prompts skipped (bug-a66941). So no level confines what
such an agent reads, writes or sends over the network. The
status matrix tags this PARTIAL with the verdict "fix" (row IS6), and the whitepaper's §9 lists it as work with no
item.

## Why it matters

A level called `Isolate` that doesn't isolate is a safety claim the code does not keep. That matters most for a hosted
demo, where untrusted plans would run next to the operator's keys and data. Epic spec-ba7bea.

## Where

- `crates/roko-agent/src/safety/sandbox.rs::SandboxLevel` and the `SafetyLayer` in `safety/mod.rs`.
- `crates/roko-agent/src/claude_cli_agent.rs::build_settings_json`: the Claude CLI's settings and hooks, where
  bug-a66941 adds deny rules for key files.
- The spawn paths of the other CLI agents (Codex, Cursor, Gemini).

## Current state

Checked at `4c0326dfc`: as described. **Decided 2026-09-29 (Will, spec-ba7bea Notes):** no OS sandbox in v1. The git
guard is best effort, the whitepaper and README document the gap as a limitation, and the question is revisited
before any hosted demo. For the hosted showcase, S11.T10 plans a uid split through `setpriv` (imported with
gap-25065c).

## Plan

After v1:

1. Choose a mechanism for CLI-subprocess agents at `Isolate` and above. Options: macOS `sandbox-exec` profiles, Linux
   bubblewrap or Landlock, one container per plan, or S11's uid split.
2. Map each level to that mechanism's rules (filesystem roots, network, subprocesses), and fail closed when the
   mechanism is not available.
3. Add the test `isolate_level_confines_cli_agent_to_worktree`: an agent process at `Isolate` cannot read a file
   outside its worktree.

## Done when

- [ ] At `Isolate` and above, a CLI-subprocess agent is confined by the OS, or refused when the platform has no
      mechanism.
- [ ] The `[[verify]]` command passes.

## Notes

- Held by Will's decision. Remove the `hold` when the hosted demo is scheduled.
- Running the existing pre- and post-dispatch checks on plan runs is not held. The attempt-diff epic (spec-9230a9:
  gap-b72761, gap-abbd22) should reuse `post_dispatch_check`, which already covers secret leaks in output, output
  size and changed-file path escapes, rather than building the same checks again. gap-70e445 (parked) tracks the
  broader safety differences between dispatch paths.
