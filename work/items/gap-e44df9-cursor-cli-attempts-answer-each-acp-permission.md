+++
id = "gap-e44df9"
kind = "gap"
title = "Cursor CLI attempts answer each ACP permission request with roko's command guard instead of running with --force"
status = "open"
triage = "verified"
severity = "p2"
goal = "release"
size = "M"
subsystem = ["roko-agent/cursor-cli", "roko-agent/safety"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-5"
anchors = ["crates/roko-agent/src/cursor_cli_agent.rs::parse_cursor_notification", "crates/roko-agent/src/exec.rs::guarded_command_violation", "crates/roko-cli/src/graph_task_dispatch/failover.rs::UNGUARDED_CLI_KINDS"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["spec-ba7bea", "q-c423b6", "gap-8f8544", "gap-baab0a"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn cursor_permission_request_runs_rokos_guard' crates/roko-agent/ && cargo test -p roko-agent cursor_permission_request_runs_rokos_guard"

[[verify]]
command = "! grep -qF 'cmd.arg(\"--force\")' crates/roko-agent/src/cursor_cli_agent.rs"
+++

## Problem

A Cursor attempt runs every shell command and file read with no roko check. `CursorCliAgent` starts
`agent --force --approve-mcps --workspace <dir> --output-format json acp`
(`crates/roko-agent/src/cursor_cli_agent.rs:259-262`). `--force` approves every command, and when Cursor still sends
an ACP `session/request_permission`, roko logs "received permission request despite --force; ignoring" and drops it
(`cursor_cli_agent.rs::parse_cursor_notification`, `:219-222`). So a Cursor agent can run `git stash` or
`cat ~/.roko/.env`, which roko's guard denies to API models (the tool loop's safety layer), to the Claude CLI (its
PreToolUse hook, `claude_cli_guard.py`) and, once the command has started, to Codex (the stream broker,
`crates/roko-agent/src/exec.rs::guarded_command_violation`).

Decision 1214 (2026-10-02) chose option 1: Codex's broker checks command text (1215), and Codex, Cursor and Gemini run
only in per-task worktrees (1216, `graph_task_dispatch/failover.rs::UNGUARDED_CLI_KINDS`). It said native per-CLI
guards "are filed as gaps later". None was filed (A01 §4, `tmp/status-quo/2026-10-15/backlog/A01-11-12-safe-runs.md:124`).

## Why it matters

Goal `release` (epic spec-ba7bea, secrets and git guard). The worktree keeps git damage away from the operator's
checkout, but nothing stops a key-file read: `~/.roko/.env`, `.roko/secrets.toml` and the audit vault are outside the
worktree and readable. The papers say every tool call passes the seven-step guard (claims C0.4, C1.21, C3.12, C3.29:
`tmp/status-quo/2026-10-15/papers/D3-paper-00-03.md:76-81,222-224`; appendix A.1 in `papers/D7-paper-appendices.md:56`).
That is true only for API models and, by its hook, the Claude CLI.

## Where

- `crates/roko-agent/src/cursor_cli_agent.rs`: the spawn (`--force`), `parse_cursor_notification` (drops
  permission requests), and the connection's request loop, which would answer them.
- `crates/roko-agent/src/exec.rs::guarded_command_violation`: the key-file and git checks the Codex broker runs on
  a command's text (`refuse_key_file_in_command`, `check_git_command`).
- `crates/roko-cli/src/graph_task_dispatch/failover.rs::UNGUARDED_CLI_KINDS`: the kinds kept out of the shared checkout.

## Current state

Cursor speaks ACP to roko, and ACP has the hook roko needs: the agent asks `session/request_permission` (a server
request with an id, a `toolCall` and allow/reject `options`) before a tool runs, unless the client told it not to ask.
roko asks it not to (`--force`). Not known: whether Cursor's `toolCall` carries the command text or file path (in
`rawInput` or the title).

## Plan

1. Record one Cursor ACP session without `--force` (no paid call: a local `agent` login, a one-line prompt that runs
   `ls`), and check what `session/request_permission` carries.
2. If it names the command or path: drop `--force`, answer each request with the allow option unless
   `guarded_command_violation` (made `pub(crate)` and shared) or the key-file check refuses it, and then reject and
   log the refusal like the Codex broker's. Keep `--approve-mcps` (roko's own MCP servers have their own checks).
3. If it does not, use Cursor's native hooks instead (a `beforeShellExecution` / `beforeReadFile` hook written into the
   attempt's worktree config, like the Claude `--settings` payload), and record which in decision 1214's spec item.
4. Remove `CursorCli`/`CursorAcp` from `UNGUARDED_CLI_KINDS` only if the guard is pre-execution and covers reads.

## Done when

`cursor_permission_request_runs_rokos_guard` passes (the `[[verify]]` command): a scripted ACP peer asks permission
for `git stash` and for `cat ~/.roko/.env`, roko rejects both and allows `ls`. The second verify confirms that
`--force` is gone.

## Notes

The paper claim is the PK101 review's to correct (gap-ac1aed, decision 4, 2026-10-15). It is related to q-c423b6
(Codex `deny_read`) and gap-8f8544 (no OS sandbox; held). The Codex and Gemini half is filed separately (finding 5's
second item).
