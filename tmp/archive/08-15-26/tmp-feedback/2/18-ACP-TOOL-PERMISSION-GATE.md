# ACP Tool Permission Gate Missing

## Problem

ACP slash commands dispatch agents with full tool access — no permission gate for destructive
tools (`Bash`, `Write`, `Edit`). The Zed extension never asks for confirmation before file
writes or shell commands.

## Root Cause

**File:** `crates/roko-acp/src/bridge_events.rs`

`run_slash_command` builds `AgentExecOptions` and dispatches via `agent_exec::run()`. The
`allowed_tools` field is either `None` (all tools) or a CSV string like `"Read,Write,Edit"`.
There is no permission callback or confirmation gate between the ACP frontend and tool execution.

In the CLI path (`orchestrate.rs`), agents run in a subprocess (Claude CLI) which has its own
permission system. But in the ACP/HTTP path, tools execute directly in-process via `tool_loop`
with no user confirmation.

### What's missing:

1. **No tool approval callback** — `ToolDispatcher::dispatch()` executes immediately
2. **No tool denylist per slash command** — `/research` gets `Bash` even though it shouldn't need it
3. **No confirmation for file writes** — agent can overwrite any file without user seeing the path first
4. **`ToolContext::testing()`** used in some ACP code paths — bypasses all safety checks

### Evidence in code:

```rust
// bridge_events.rs — no permission gate
let result = agent_exec::run(&options).await?;
// tools execute directly, no user confirmation

// tool_loop.rs — dispatch is immediate
let result = self.dispatcher.dispatch(&tool_call).await?;
// no "are you sure?" for write_file or bash
```

## Fix

### Fix 1: Add tool approval callback to ACP dispatch (~30 min)

**File:** `crates/roko-agent/src/tool_loop.rs`

Add an optional `tool_approval: Option<Box<dyn Fn(&ToolCall) -> bool>>` to `ToolLoopConfig`.
When set, call it before dispatching write/bash tools. The ACP bridge can wire this to send
a JSON-RPC notification to Zed asking for user confirmation.

### Fix 2: Restrict tool sets per slash command (~15 min)

**File:** `crates/roko-acp/src/bridge_events.rs`

- `/research` → `"read_file,glob,grep,web_search,web_fetch"` (no write, no bash)
- `/analyze` → `"read_file,write_file,glob,grep"` (no bash)
- `/enhance-prd` → `"read_file,write_file,edit_file,glob,grep"` (no bash)
- `/do` → all tools (with approval gate)

### Fix 3: Remove `ToolContext::testing()` from production paths

**File:** `crates/roko-acp/src/bridge_events.rs`

Replace any `ToolContext::testing()` with `ToolContext::production()` or `ToolContext::acp()`.

## Priority

**P1** — Users expect that an IDE extension won't silently write files or run shell commands.
Currently the safety layer is permissive-default and the ACP has no confirmation UX.
