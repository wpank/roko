+++
id = "gap-831b17"
kind = "gap"
title = "Codex and Gemini CLI attempts get a guard that refuses a denied command before it runs (decision 1214, option 2)"
status = "open"
triage = "verified"
severity = "p3"
goal = "release"
size = "M"
subsystem = ["roko-agent/codex-cli", "roko-agent/gemini-cli", "roko-agent/safety"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-5"
anchors = ["crates/roko-agent/src/exec.rs::guarded_command_violation", "crates/roko-agent/src/provider/gemini_cli.rs", "crates/roko-cli/src/graph_task_dispatch/failover.rs::UNGUARDED_CLI_KINDS"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = ["spec-ba7bea", "q-c423b6", "gap-0714c4", "gap-8f8544"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn codex_command_is_refused_before_it_starts' crates/roko-agent/ && cargo test -p roko-agent codex_command_is_refused_before_it_starts"

[[verify]]
command = "grep -rqw 'fn gemini_command_is_refused_before_it_starts' crates/roko-agent/ && cargo test -p roko-agent gemini_command_is_refused_before_it_starts"
+++

## Problem

Two CLI agents still run commands before any roko check can refuse them:

- **Codex.** The stream broker checks each `command_execution` command's text against roko's key-file and git guards
  (`crates/roko-agent/src/exec.rs::guarded_command_violation`, task 1215), but it acts on `item.started`, after Codex
  has started the command. It stops a key file's contents reaching the model and stops the run. It cannot undo a
  `git stash` or keep a read from happening. Codex's `workspace-write` sandbox does not confine reads (gap-0714c4, done
  for the audit vault; q-c423b6 asks whether `deny_read` can).
- **Gemini CLI** (`crates/roko-agent/src/provider/gemini_cli.rs`) runs as an exec agent with the default safety layer
  and no command check at all.

Decision 1214 (2026-10-02) chose option 1 for now (the broker, plus per-task worktrees for both,
`graph_task_dispatch/failover.rs::UNGUARDED_CLI_KINDS`) and said option 2, "native hooks per CLI", would be filed per
CLI as gaps. It was never filed (A01 §4).

## Why it matters

Goal `release` (epic spec-ba7bea). This repo's `roko.toml` configures a `codex_cli` provider and a `codex` model
(`roko.toml:43-45,146-147`), and decision 3 (2026-10-15, dec-55532d) bills the Claude and Codex CLIs as
subscriptions, so Codex is a live executor choice: the condition decision 1214 set for this work. The papers' "every tool call
passes a seven-step guard" holds only for API models and the Claude CLI (D3 C0.4, D7 A.1).

## Where

- `crates/roko-agent/src/exec.rs::guarded_command_violation`: the checks to run before execution.
- `crates/roko-agent/src/provider/gemini_cli.rs`: how Gemini is launched (provider-level args).
- `crates/roko-cli/src/graph_task_dispatch/failover.rs::UNGUARDED_CLI_KINDS`.

## Current state

Per-task worktrees keep git damage out of the operator's checkout. Key-file reads by Gemini are possible and
undocumented outside the papers' audit. Nothing was checked about the current Codex or Gemini CLI versions'
pre-execution hook or policy support.

## Plan

1. For each CLI, check the installed version's pre-execution options (a hook that can refuse a command, an approval
   mode that roko can answer, an exec-policy file, or ACP permission requests as for Cursor). Record the findings in
   this item.
2. Where one exists, write it per attempt into config the agent cannot edit (outside the worktree, as the Claude
   `--settings` payload is passed on the command line), and have it call the same checks as
   `guarded_command_violation`.
3. Where none exists, leave the kind in `UNGUARDED_CLI_KINDS`, say so in `docs/` and in the papers' limitation
   (through gap-ac1aed), and close this item for that CLI as wontfix with the reason.

## Done when

For Codex and Gemini each, either a guarded test shows a `git stash` refused before it runs
(`codex_command_is_refused_before_it_starts`, `gemini_command_is_refused_before_it_starts`, the `[[verify]]`
commands), or the item records that the CLI offers no pre-execution hook and the limitation is documented.

## Notes

Probing a CLI's hook support needs a local login and no paid call; record the CLI versions. Decision 1214's spec
file: `tmp/backlog/2026-10-02-complete-and-wire/1214-decide-command-guard-for-codex-cursor-and-gemini.md`. The Cursor
half is the item filed first from finding 5.
