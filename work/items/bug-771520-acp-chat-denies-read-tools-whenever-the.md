+++
id = "bug-771520"
kind = "bug"
title = "ACP chat denies read tools whenever the client declares no fs capability, although write and exec fall back to the user's trusted actions (old backlog 56, P1)"
status = "open"
triage = "verified"
severity = "p2"
goal = "hermes"
size = "S"
subsystem = ["roko-acp/bridge_events"]
created = 2026-10-15
updated = 2026-10-15
last_verified = 2026-10-15
source = "status-quo-2026-10-15#finding-44"
anchors = ["crates/roko-acp/src/bridge_events/cost.rs::derive_acp_tool_capabilities", "crates/roko-acp/src/types.rs::ClientCapabilities", "crates/roko-acp/src/types.rs::PermissionAction"]
lane = "rust-cold"
links = { depends_on = [], blocks = [], related = [], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn acp_read_without_fs_capability_follows_trusted_actions' crates/roko-acp/ && cargo test -p roko-acp acp_read_without_fs_capability_follows_trusted_actions"
+++

## Problem

Old backlog file `tmp/backlog/archive/56-acp-single-agent-tools.md` (P1 in the frozen summary) never became a work
item, and its bug is still at HEAD. `derive_acp_tool_capabilities` (`crates/roko-acp/src/bridge_events/cost.rs:504`)
computes a session's tool permissions from the ACP client's declared capabilities:

- `write` falls back to the user's trusted actions (`FileCreate`/`FileEdit`) when the client declares no `fs`;
- `exec` and `git` fall back to trusted `TerminalCommand`/`GitOperation` when it declares no `terminal`;
- `read` is `role.read && (fs.is_some_and(|caps| caps.read_text_file) || mcp)`: with no `fs` declaration and no
  session MCP it is always false, and `PermissionAction` has no `FileRead` to trust.

So an IDE client that declares no filesystem capability gets a session whose model may write and run commands but not
read a file: it calls `read_file`, gets a permission error and answers confusingly, with no diagnostic. Status-quo
assessment 2026-10-15 at 7b6a5af44, finding 44 (B's archive check).

## Why it matters

Goal `hermes` (Hermes and ACP integration): editors are a main way to use roko, and a session that can edit but not
read is broken in the least safe direction.

## Where

- `crates/roko-acp/src/bridge_events/cost.rs::derive_acp_tool_capabilities`.
- `crates/roko-acp/src/types.rs`: `ClientCapabilities`, `PermissionAction`.

## Current state

Unchanged since the archived file was written, apart from the move to `bridge_events/cost.rs` and the `mcp` clause.

## Plan

Options (recommended: 1, the conservative one under Will's 2026-10-15 decision 9):

1. Add `PermissionAction::FileRead` and let `read` fall back to it when the client declares no `fs`, as `write` falls
   back to `FileCreate`/`FileEdit`. Reads then happen only when the user trusted them, and the role ceiling still applies.
2. Allow reads whenever `role.read` holds and the client declares no `fs` (the agent reads the local worktree itself).
   Simpler, but it grants reads without any user decision.

Either way, when a session ends with `read = false`, say so to the client at session start instead of failing each
call. Test `acp_read_without_fs_capability_follows_trusted_actions`: no `fs` declared, `FileRead` trusted → read
allowed; not trusted → denied, with the diagnostic.

## Done when

The `[[verify]]` command passes.

## Notes

Key-file deny rules and the role ceilings stay as they are. The archived file has the original analysis.
