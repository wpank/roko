+++
id = "bug-739dcc"
kind = "bug"
title = "The Claude CLI adapter has no cancel path: dropping a run kills only the CLI process, and its tool subprocesses keep running"
status = "open"
triage = "unverified"
severity = "p2"
goal = "golden-path"
size = "S"
subsystem = ["roko-agent/claude_cli"]
created = 2026-09-30
updated = 2026-09-30
source = "tmp/cybernetic-harness/workstreams/PROGRESS.md"
discovered_from = "tmp/cybernetic-harness/workstreams/PROGRESS.md (wk-watchdog's report, checked on work/spec-a0403b at d5546dfc7)"
anchors = ["crates/roko-agent/src/claude_cli_agent.rs"]
lane = "rust-hot"
parent = "spec-edda86"
links = { depends_on = [], blocks = [], related = ["spec-a0403b"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cancelling_a_claude_run_kills_its_tool_subprocesses' crates/roko-agent/src/ && cargo test -p roko-agent --lib cancelling_a_claude_run_kills_its_tool_subprocesses"
+++

## Problem

The Claude CLI adapter spawns `claude` in its own process group with `kill_on_drop(true)` (`crates/roko-agent/src/claude_cli_agent.rs:666-667`). `kill_on_drop` kills only the direct child. When a run is dropped (the stall watchdog, a restart or a cancel), the tool subprocesses `claude` started, such as shells, test runners and builds, keep running. No cancel path signals the whole process group.

## Why it matters

Watchdog and supervision (epic spec-edda86): orphaned tool processes keep using CPU, hold locks (cargo's) and can keep editing the worktree after the attempt is settled.

## Where

The spawn and drop handling in `claude_cli_agent.rs`, next to `set_process_group`.

## Plan

1. On cancel or drop, signal the process group (SIGTERM, then SIGKILL after a grace period), and unregister the pid.
2. Add `cancelling_a_claude_run_kills_its_tool_subprocesses`.

## Done when

- [ ] Cancelling a Claude CLI run leaves no process of its group running.
- [ ] The `[[verify]]` command passes.
