+++
id = "bug-c83c84"
kind = "bug"
title = "A cancel can stop roko serve itself, and the agent's tool processes outlive it: the bash tool runs its command in roko's own process group and kills only its shell"
status = "open"
triage = "verified"
severity = "p1"
goal = "core"
size = "M"
subsystem = ["roko-std/tool"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-16
last_verified_rev = "71b7c73f8"
source = "tmp/hermes-roko-v5/findings/plugin-setup.md F1 (plugin-setup, 2026-10-15); filed by roko-checks"
discovered_from = "hermes-roko-v5 round: a cancelled run from Hermes"
anchors = ["crates/roko-std/src/tool/builtin/bash.rs::run", "crates/roko-std/src/tool/builtin/child_group.rs::output_within", "crates/roko-std/src/tool/builtin/run_tests.rs"]
links = { depends_on = [], blocks = [], related = ["bug-1d09e8", "bug-7b62ff"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn bash_commands_run_in_their_own_process_group' crates/roko-std/src && cargo test -p roko-std --lib bash_commands_run_in_their_own_process_group"
+++

## Problem

plugin-setup's run on 2026-10-15 (roko-main 7b6a5af44; `tmp/hermes-roko-v5/findings/plugin-setup.md`, F1; log
`tmp/hermes-roko-v5/plugin-setup/logs/serve-main-1.log`, lines 557–573):

- A run was cancelled over `/mcp` `run_cancel` while its agent (glm-4.7, roko's own tool loop) was in a tool call
  (`tool_loop: dispatching tool calls iteration=4`, 15:38:51).
- At 15:39:01 roko logged `plan run interrupted … signalled=0`, and at 15:39:04 `cancelled graph did not settle in
  time; stopping its attempts`.
- At 15:39:11, 10.8 s after the cancel, `roko serve` ran its Ctrl-C shutdown: `server shutdown initiated` (twice),
  then `server stopped`. A non-TUI `roko serve` shuts down only when it gets SIGINT (`commands/server.rs`,
  `tokio::signal::ctrl_c()`). No other process is known to have signalled it.
- A second run, cancelled during its verify step, left serve running.
- The cancelled attempt's tool command, a `cargo` build of about 12 rustc at nice 0, kept running in the attempt's
  worktree. It outlived both the cancel and the shutdown until it was killed by hand at 17:57.

Expected:
- a cancel ends the attempt's own processes and nothing else;
- nothing an agent runs can signal `roko serve`.

## Why it matters

In Will's setup, pressing Cancel on the roko page or in Hermes could take his whole `roko serve` down, and every
page that reads it goes dark. Separately, processes an agent started keep running after a cancel: here a
whole-workspace cargo build at nice 0, on a machine whose load already caused a kernel panic that day. Goal `core`.

## Where

- `crates/roko-std/src/tool/builtin/bash.rs::run`: the `bash` tool, which every API-model agent uses for shell
  commands. It spawns `bash -c <command>` with `kill_on_drop(true)` and no process group of its own.
- `crates/roko-agent/src/process/` (`group.rs::set_process_group`, `kill.rs::KillTreeOnDrop`): how the agent CLIs
  are spawned in their own group and killed as a tree.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::live_agent_process_trees`: what a stopping run signals.

## Current state

Read in the code at ced7fc3a5:

- **What a cancel does to a tool command.** Dropping a running `bash` tool call kills only its `bash` (tokio sends
  SIGKILL to that one pid). Its children are reparented and run on: cargo, rustc, test binaries, any server the
  agent started.
- **The shared process group.** The command runs in roko's own process group. Anything in it that signals its own
  group therefore signals `roko serve` too, for example `kill -INT 0`, a test harness or a tool that sends Ctrl-C to
  its group. serve takes the SIGINT as Ctrl-C and shuts down. This is the most likely route to F1's shutdown. It is
  not yet reproduced as the exact trigger.
- **What roko signals itself.** No roko signalling code targets serve's group:
  - `live_agent_process_trees` signals only registered descendants of serve;
  - `kill_process_group` and `KillTreeOnDrop` signal the child's own group id;
  - no roko code sends SIGINT.
- **Other spawners.** The agent CLIs (`claude_cli_agent.rs`, `exec.rs`, the harness runners) already start in their
  own process group. So do verify steps (`ShellGate`) and graph shell cells.

Fixed on `work/bug-c83c84`. `child_group::output_within`, used by both the `bash` and the `run_tests` tools, runs
the command as the leader of a process group of its own, with stdin closed as `Command::output` did. When the call
ends before the command does (dropped, or past its timeout), it SIGKILLs the whole group. A command that ends on its
own leaves its background processes alone, as before.

Live, zero cost (`tmp/hermes-roko-v5/roko-checks/serve-signal/run.sh`): a scratch `roko serve` started in its own
session, whose model is a scripted OpenAI-compatible server, so roko's tool loop runs the `bash` tool.

| Scenario | Main 7b6a5af44 | With the fix |
|---|---|---|
| The call runs `sleep 2; kill -INT 0` | serve shuts down 2 s later, with F1's exact log: the same tool hash, then `server shutdown initiated` twice and `server stopped` | serve keeps running and the run completes |
| The call runs `sleep 120 & …; wait`, and `/mcp` `run_cancel` cancels the run | the `sleep` child is still running 15 s after the cancel | the child is gone |

## Plan

1. Spawn the `bash` tool's command in its own process group. When the call is dropped or times out, kill that
   group and its descendants, as `KillTreeOnDrop` does for agent CLIs. A cancel then ends the agent's processes,
   and a signal to their group never reaches roko.
2. Check the other places roko runs a command an agent chose, such as MCP stdio servers and plugin tools, for the
   same.
3. Reproduce F1 on a scratch serve with a scripted OpenAI-compatible model whose tool call runs a long command,
   then cancel it, before and after.

## Done when

`[[verify]]` passes. A `bash` tool command runs in its own process group, so its signal to its own group does not
reach the test process. Dropping a running call kills the command's children.
