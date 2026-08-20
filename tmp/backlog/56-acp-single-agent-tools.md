# 56 — ACP Single-Agent Chat: Tools Require Client Capability Declaration

**Priority**: P1 — IDE direct-chat sessions silently fail all tool calls when the client does not explicitly declare filesystem capabilities, even though `tools_enabled = true` and 8 built-in tools are registered
**Size**: M (1-2 days)
**Crates**: `crates/roko-acp/` (`roko-acp`), `crates/roko-agent/` (`roko-agent`)
**Depends on**: None

---

## Background

Roko exposes an ACP (Agent Client Protocol) server that IDE clients like Cursor and Zed connect to. When an IDE user types a prompt directly (not using a slash command), Roko handles it through the "direct chat" path. This path has a complete tool loop: 8 built-in tools are registered (`read_file`, `write_file`, `edit_file`, `glob`, `grep`, `bash`, `ls`, `web_fetch`), and the model is given access to them.

However, the tool loop has a gating mechanism. Before a tool call is executed, the dispatcher checks that the `ToolPermission` flags on the session allow the operation. For example, `read_file` requires `ToolPermission { read: true, ... }`. The `ToolPermission` is derived by `derive_acp_tool_capabilities()` at session start. This function intersects what the client declared at initialization with the role's permission ceiling.

The problem: `derive_acp_tool_capabilities()` grants `read: true` only when the client explicitly declares `fs: Some(FsCapabilities { read_text_file: true, ... })` at initialization. Many IDE clients (including common Cursor configurations) send `{"client_capabilities": {}}` or omit the field entirely because they assume the server handles file operations natively. In that case, `fs` is `None`, `read` is `false`, and every `read_file` call fails with `"read_file requires ToolPermission { read: true }, role grants ToolPermission { read: false }"`. The user sees the model behaving as if it cannot access the codebase.

The failure is silent: the model enters the tool loop, calls `read_file`, receives a permission error, and responds with a confused message. The IDE shows the model's confused response, not a diagnostic about the missing capability declaration.

---

## Current State

1. The `derive_acp_tool_capabilities()` function is at line 669 of `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`. The bug is in the `read` field computation on line 689:

   ```rust
   read: role.read && (fs.is_some_and(|caps| caps.read_text_file) || mcp),
   ```

   When `fs` is `None` and there is no session MCP, this evaluates to `false` regardless of the mode. The `write` and `exec` fields correctly fall back to `trusted_actions` when `fs` is `None`, but `read` has no such fallback.

2. `ClientCapabilities` is defined at line 134 of `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs`. It has three optional fields: `fs: Option<FsCapabilities>`, `terminal: Option<bool>`, and `mcp_servers: Option<bool>`. All default to `None`.

3. `PermissionAction` is defined at line 1015 of `types.rs`. Current variants: `FileEdit`, `FileCreate`, `FileDelete`, `TerminalCommand`, `NetworkRequest`, `GitOperation`. There is no `FileRead` variant.

4. The existing test that confirms the bug is at line 6285 of `bridge_events.rs`:

   ```rust
   let missing = derive_acp_tool_capabilities(
       "code",
       &ClientCapabilities::default(),
       false,
       &HashSet::new(),
   );
   assert_eq!(missing, ToolPermission::default()); // all false — this is the bug
   ```

5. `run_anthropic_cognitive_task()` is defined at line 2435 of `bridge_events.rs`. The tool loop is entered when `tools_enabled || !mcp_servers.is_empty()` (line 2475). `tool_capabilities` is derived at line 1954 and passed as a parameter.

6. `session.tools_enabled` defaults to `true` (line 429 of `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/session.rs`).

7. The capability enforcement that returns the "role grants" error is in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/dispatcher/mod.rs` at lines 556-563:

   ```rust
   if !def.permission.satisfied_by(&role_perms) {
       let err = ToolError::PermissionDenied(format!(
           "{} requires {:?}, role grants {:?}",
           call.name, def.permission, role_perms
       ));
   ```

8. `acp_builtin_tools()` is at line 75 of `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/builtin_tools.rs` and returns the 8 tool definitions.

---

## Implementation Plan

### Change 1: Fix `derive_acp_tool_capabilities` — grant read by default for `code`/`chat` modes

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs`, replace line 689:

```rust
// Before (line 689):
read: role.read && (fs.is_some_and(|caps| caps.read_text_file) || mcp),

// After:
read: role.read && (
    fs.is_some_and(|caps| caps.read_text_file)
    || mcp
    // Grant read by default for code/chat modes when client makes no fs declaration.
    // Write and exec remain gated. This matches IDE behavior where clients often
    // omit fs capabilities because they expect the server to handle file ops natively.
    || (fs.is_none() && matches!(mode, "code" | "chat" | "default"))
),
```

Modes `code`, `chat`, and `default` map to `AgentRole::Implementer` (see `acp_role_for_mode()` at line 658), which already has `role.read = true`. The change adds a second condition: when `fs` is undeclared, `code`/`chat` mode gets `read` for free. The `plan`, `research`, and `architect` modes are not in the match arm, so they keep the current strict behavior (require explicit `fs` declaration).

### Change 2: Add `PermissionAction::FileRead` variant (optional fallback)

In `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs`, add a `FileRead` variant to `PermissionAction` at line 1015:

```rust
pub enum PermissionAction {
    /// Reading a file.
    FileRead,   // NEW
    /// Writing or editing a file.
    FileEdit,
    // ... rest unchanged
}
```

Then in `derive_acp_tool_capabilities`, also grant `read` when `trusted_actions.contains(&PermissionAction::FileRead)`:

```rust
read: role.read && (
    fs.is_some_and(|caps| caps.read_text_file)
    || mcp
    || (fs.is_none() && matches!(mode, "code" | "chat" | "default"))
    || (fs.is_none() && trusted_actions.contains(&PermissionAction::FileRead))
),
```

This makes `read` consistent with how `write` and `exec` fall back to `trusted_actions` when `fs` is `None`.

### Change 3: Emit a diagnostic when tool loop enters with all-false capabilities

In `run_anthropic_cognitive_task()` at line 2475 of `bridge_events.rs`, before entering the tool loop, add:

```rust
if tools_enabled && tool_capabilities == ToolPermission::default() {
    warn!(
        session_id,
        "ACP tool loop entered with all-false capabilities — tools will be denied; \
         client should declare fs/terminal at initialize"
    );
    // Send a visible diagnostic to the client before the model's first response.
    let _ = event_sender.send(CognitiveEvent::TokenChunk(
        "[roko: tools are registered but no client capabilities were declared — \
         file tools will fail. Your IDE should send `fs` capabilities at \
         ACP initialize time.]\n\n".to_string()
    )).await;
}
```

Apply the same diagnostic in the OpenAI-compat path. Find the equivalent entry point by searching for `tools_enabled` around line 2588 of `bridge_events.rs` (the `run_openai_compat_cognitive_task` function starts at line 2527).

### Change 4: Update the broken test

At line 6285 of `bridge_events.rs`, update the test to assert the new behavior:

```rust
// code mode: read granted by default even with no client declarations
let missing = derive_acp_tool_capabilities(
    "code",
    &ClientCapabilities::default(),
    false,
    &HashSet::new(),
);
assert!(missing.read);    // granted by default for code mode
assert!(!missing.write);  // still gated
assert!(!missing.exec);   // still gated

// plan mode: still requires explicit fs declaration
let plan_missing = derive_acp_tool_capabilities(
    "plan",
    &ClientCapabilities::default(),
    false,
    &HashSet::new(),
);
assert_eq!(plan_missing, ToolPermission::default()); // unchanged
```

The existing test at line 6255 (for a client that explicitly declares `read_text_file: true`) must not regress.

---

## Acceptance Criteria

1. Calling `derive_acp_tool_capabilities("code", &ClientCapabilities::default(), false, &HashSet::new())` returns `ToolPermission { read: true, write: false, exec: false, git: false, network: false }`.
2. A session using default `ClientCapabilities` in `code` mode can execute a `read_file` tool call without receiving `ToolError::PermissionDenied`.
3. `write_file` and `bash` remain denied for a session with default `ClientCapabilities` and no `trusted_actions`.
4. A session that explicitly declares `fs: Some(FsCapabilities { read_text_file: true, write_text_file: false })` continues to get `read: true, write: false` — the explicit declaration path does not regress.
5. A session in `plan` mode with default `ClientCapabilities` still gets `read: false` (no default grant for elevated modes).
6. When `tools_enabled = true` but all derived capabilities are false (e.g., `plan` mode with default client), a `warn!()` is emitted to the log and a visible diagnostic string is prepended to the first response chunk.
7. `cargo test -p roko-acp` passes with zero failures.
8. `cargo clippy -p roko-acp -- -D warnings` is clean.

---

## Verification Checklist

- [ ] Start `roko serve` with an Anthropic provider configured.
- [ ] Connect via a minimal ACP client that sends `initialize` with `{"client_capabilities": {}}`.
- [ ] Send the prompt `"Read src/main.rs and summarize its first 10 lines."` — confirm the model calls `read_file` and returns a coherent summary, not a permission error.
- [ ] In the same session, attempt `write_file` — confirm it is denied with a "role grants" message.
- [ ] Switch to `plan` mode. Send the same prompt — confirm `read_file` is denied (no default grant for `plan` mode).
- [ ] Run `cargo test -p roko-acp` — zero failures.
- [ ] Run `cargo clippy -p roko-acp -- -D warnings` — clean.

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` | Line 689: add default read grant for `code`/`chat`/`default` modes when `fs` is `None`; lines 2469-2475: add diagnostic when `tools_enabled && all capabilities false` |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/bridge_events.rs` | Line 6285: update test to assert `read: true` for `code` mode with default client caps; add `plan` mode assertion |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-acp/src/types.rs` | Line 1015: add `FileRead` variant to `PermissionAction` |
