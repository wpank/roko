# ACP Single-Agent Chat: Tools Require Client Capability Declaration

**Priority**: P1
**Size**: M (1-2 days)

---

## Problem

The ACP (Agent Client Protocol) direct-chat path — used when an IDE user types a prompt
in Cursor/Zed without a slash command — gates all tool access behind
`ClientCapabilities.fs`, which is only set when the IDE explicitly declares filesystem
support at session initialization. When a client connects without that declaration (the
default for most clients), `derive_acp_tool_capabilities` returns an all-false
`ToolPermission`, which means:

- The tool loop is entered (tools are advertised to the model in the request)
- The model may call `read_file`, `bash`, etc. expecting results
- Every call is rejected by `ToolDispatcher::dispatch` with:
  `"read_file requires ToolPermission { read: true, ... }, role grants ToolPermission { read: false, ... }"`
- The model receives an error result and cannot proceed with file-aware tasks

The experience from the IDE is a model that responds as if it cannot see the codebase,
even though `tools_enabled` is `true` on the session and the 8 builtin tools are
registered. The underlying tool loop infrastructure is fully built and works correctly for
clients that do declare capabilities — the bug is in how the default state of an
unannounced client maps to an all-false permission floor.

Slash commands (`/do`, `/develop`, `/research`) do not suffer from this because they
dispatch through the runner or the workflow pipeline, which build their own `ToolContext`
independently of `derive_acp_tool_capabilities`.

### What already exists

| Component | Location | Status |
|---|---|---|
| `acp_builtin_tools()` | `crates/roko-acp/src/builtin_tools.rs:75` | EXISTS — returns the 8 tool defs (read_file, write_file, edit_file, glob, grep, bash, ls, web_fetch) |
| `derive_acp_tool_capabilities()` | `crates/roko-acp/src/bridge_events.rs:669` | EXISTS — intersects client capability declaration with role ceiling; all-false when client skips `fs` field |
| `run_anthropic_tool_loop()` | `crates/roko-acp/src/bridge_events.rs:2524` | EXISTS — full Anthropic tool loop with builtin handler dispatch |
| `run_openai_compat_builtin_tool_loop()` | `crates/roko-acp/src/bridge_events.rs:3318` | EXISTS — OpenAI-compat counterpart |
| `AcpBuiltinToolHandler` | `crates/roko-acp/src/bridge_events.rs:3850` | EXISTS — wraps `execute_acp_builtin_tool` in the `ToolHandler` trait |
| `execute_acp_builtin_tool()` | `crates/roko-acp/src/builtin_tools.rs` | EXISTS — dispatches to real filesystem/bash implementations |
| `session.tools_enabled` | `crates/roko-acp/src/session.rs:364` | EXISTS — default `true`, controlled by session init |
| `ClientCapabilities` | `crates/roko-acp/src/types.rs:134` | EXISTS — `fs: Option<FsCapabilities>`, `terminal: Option<bool>`, `mcp_servers: Option<bool>` |
| `ToolContext` capability enforcement | `crates/roko-agent/src/dispatcher/mod.rs:549` | EXISTS — checks `ctx.capabilities` against `def.permission` at dispatch |

### What is missing

The logic in `derive_acp_tool_capabilities` treats a missing `client_capabilities.fs`
field (i.e., `None`) as "no filesystem access granted" rather than "client did not
explicitly declare capabilities, apply the role-based default." This is the right policy
for high-trust environments where the client must opt in, but it produces a silent failure
mode: the IDE prompt appears to execute, the tool loop is entered, the model calls tools,
but every call fails permission-denied.

The fix requires choosing one of two approaches:

**Option A (Recommended): Default-open for direct chat, require declaration only for
privileged actions.** When `client_capabilities.fs` is `None`, grant `read: true` by
default for the `code`/`chat` modes (which are read-dominant). Keep `write` and `exec`
gated behind the client declaration or an always-allow grant. This matches the actual IDE
model: Cursor and Zed do not always declare `fs` capabilities in the ACP handshake
because they assume the server-side agent handles file operations natively.

**Option B: Surface the missing-capability state to the IDE.** When `tools_enabled` is
`true` but all capabilities are false (no `fs`, no `terminal`), send a
`CognitiveEvent::Warning` or `SessionUpdate` explaining what the client must declare at
`initialize` time to enable tools. Currently the failure is invisible — the IDE shows the
model's confused response, not a diagnostic about missing declarations.

Both options are complementary, not exclusive. The spec covers Option A as the primary
fix plus Option B as a diagnostic aid.

---

## Root cause

`derive_acp_tool_capabilities` at `crates/roko-acp/src/bridge_events.rs:669`:

```rust
fn derive_acp_tool_capabilities(
    mode: &str,
    client: &ClientCapabilities,
    has_session_mcp: bool,
    trusted_actions: &HashSet<PermissionAction>,
) -> ToolPermission {
    let role = acp_role_for_mode(mode).tool_permissions();
    let fs = client.fs.as_ref();
    let mcp = client.mcp_servers == Some(true) && has_session_mcp;
    let write = fs.map_or_else(
        || {
            trusted_actions.contains(&PermissionAction::FileCreate)
                || trusted_actions.contains(&PermissionAction::FileEdit)
        },
        |caps| caps.write_text_file,
    );
    let exec = client
        .terminal
        .unwrap_or_else(|| trusted_actions.contains(&PermissionAction::TerminalCommand));
    ToolPermission {
        read: role.read && (fs.is_some_and(|caps| caps.read_text_file) || mcp),  // <-- BUG
        write: role.write && write,
        exec: role.exec && exec,
        git: role.git && client.terminal.unwrap_or_else(|| ...),
        network: role.network && mcp,
    }
}
```

The `read` field is `false` when `fs` is `None` and there is no session MCP. A client
that simply sends `{"client_capabilities": {}}` (or omits the field entirely) gets
`ToolPermission::default()` (all-false), confirmed by the existing test at line 6285:

```rust
let missing = derive_acp_tool_capabilities(
    "code",
    &ClientCapabilities::default(),
    false,
    &HashSet::new(),
);
assert_eq!(missing, ToolPermission::default());  // all false — confirmed
```

The `write` and `exec` fields correctly fall back to `trusted_actions` (always-allow
grants). The `read` field does not — it has no trusted-action fallback.

There is a separate test at line 6247 that confirms the correct behavior when `fs` is
explicitly declared, so the fix must not regress that path.

---

## Proposed fix

### Change 1: Add `PermissionAction::FileRead` trusted-action fallback for `read`

In `derive_acp_tool_capabilities`, align `read` with the pattern already used for
`write` and `exec`: fall back to a trusted-action check when `fs` is `None`, and also
grant `read: true` by default for `code`/`chat` modes where `fs` is undeclared.

```rust
// In derive_acp_tool_capabilities, replace the read line:
read: role.read && (
    fs.is_some_and(|caps| caps.read_text_file)
    || mcp
    // NEW: fall back to always-allow grant when client skips fs declaration
    || (fs.is_none() && trusted_actions.contains(&PermissionAction::FileRead))
    // NEW: grant read by default for code/chat modes when client makes no declaration
    || (fs.is_none() && matches!(mode, "code" | "chat" | "default"))
),
```

This grants `read` for the common case without requiring the IDE to change its
initialization payload. Write and exec remain gated. The `plan`, `architect`, and
`research` modes continue to require explicit `fs` declaration for their elevated
access patterns.

`PermissionAction::FileRead` may need to be added to `types.rs` if it does not exist.

### Change 2: Emit a diagnostic when tool loop is entered with all-false capabilities

In `run_anthropic_cognitive_task` and `run_openai_compat_cognitive_task`, before
entering the tool loop, check if `tools_enabled && tool_capabilities == ToolPermission::default()`.
When true, log a `warn!()` with the session ID and send a `CognitiveEvent::TokenChunk`
prefixed with `"[roko: tools are enabled but no client capabilities were declared — read_file and other tools will be denied. Ask your IDE to send fs/terminal capabilities at initialize time]\n\n"`.

This surfaces the failure mode to the IDE user rather than leaving them with a confused
model response.

### Change 3: Update the `derive_acp_tool_capabilities` test

The test at line 6285 asserts `missing == ToolPermission::default()` for
`ClientCapabilities::default()`. With Change 1, `code` mode with a missing `fs` should
now produce `read: true`. Update the test to:

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

// plan mode: no default read grant — requires explicit fs declaration
let plan_missing = derive_acp_tool_capabilities(
    "plan",
    &ClientCapabilities::default(),
    false,
    &HashSet::new(),
);
assert_eq!(plan_missing, ToolPermission::default()); // unchanged
```

---

## Where to make changes

| File | Change |
|---|---|
| `crates/roko-acp/src/bridge_events.rs:669` | `derive_acp_tool_capabilities` — grant `read: true` for `code`/`chat` modes when `fs` is `None`; add `PermissionAction::FileRead` fallback |
| `crates/roko-acp/src/bridge_events.rs:2473` | `run_anthropic_cognitive_task` — emit diagnostic when `tools_enabled && all capabilities false` |
| `crates/roko-acp/src/bridge_events.rs:3047` | `run_openai_compat_cognitive_task` — same diagnostic emit |
| `crates/roko-acp/src/types.rs` | Add `PermissionAction::FileRead` variant if missing |
| `crates/roko-acp/src/bridge_events.rs:6285` | Update `capabilities_reflect_session` test to assert `read: true` for `code` mode with default client caps |

---

## Acceptance criteria

1. An ACP session using `ClientCapabilities::default()` (no `fs`, no `terminal`) in
   `code` mode has `tool_capabilities.read == true` after `derive_acp_tool_capabilities`.
2. A model call through `run_anthropic_cognitive_task` or `run_openai_compat_cognitive_task`
   with `tools_enabled = true` and default client caps successfully executes `read_file`
   without returning `ToolError::PermissionDenied`.
3. `write_file` and `bash` remain gated — they require explicit `fs.write_text_file` or
   `terminal` declaration (or an always-allow grant).
4. An IDE session that declares `fs: Some(FsCapabilities { read_text_file: true, write_text_file: false })`
   continues to get `read: true, write: false` (the existing declared path must not regress).
5. When `tools_enabled = true` but all derived capabilities are false (e.g., `plan` mode
   with default client caps), a `warn!()` is emitted and a diagnostic message is sent to
   the client before the tool loop is entered.
6. `cargo test -p roko-acp` passes with zero failures after changes.
7. `cargo clippy -p roko-acp -- -D warnings` is clean.

---

## Verification

To verify end-to-end after the fix:

1. Start `roko serve` with an Anthropic provider configured.
2. Connect via a minimal ACP client that sends `initialize` with empty `client_capabilities`.
3. Send a prompt: `"Read the file src/main.rs and summarize its first 10 lines."`
4. Observe that the model calls `read_file`, receives the file content, and returns a
   coherent summary — not an error about permission denial.
5. Attempt `write_file` in the same session; confirm it is denied with permission-denied.

Alternatively, add an integration test in `crates/roko-acp/src/bridge_events.rs` (near
the existing `capabilities_reflect_session` test) that:
- Creates a session with `ClientCapabilities::default()`
- Calls `derive_acp_tool_capabilities("code", &client, false, &HashSet::new())`
- Asserts `result.read == true`
- Uses a mock `ToolDispatcher` to dispatch a `read_file` call against the result
- Asserts the call is not `ToolError::PermissionDenied`

---

## References

- `crates/roko-acp/src/bridge_events.rs:669` — `derive_acp_tool_capabilities`, the root cause
- `crates/roko-acp/src/bridge_events.rs:6285` — existing test confirming all-false for default caps
- `crates/roko-acp/src/bridge_events.rs:2473` — Anthropic single-agent dispatch entry point
- `crates/roko-acp/src/bridge_events.rs:3047` — OpenAI-compat single-agent dispatch entry point
- `crates/roko-acp/src/builtin_tools.rs:75` — `acp_builtin_tools()` returning the 8 tool defs
- `crates/roko-acp/src/session.rs:364` — `tools_enabled: bool`, default `true`
- `crates/roko-acp/src/types.rs:134` — `ClientCapabilities` struct
- `crates/roko-agent/src/dispatcher/mod.rs:549` — capability enforcement ("role grants" error)
- `tmp/backlog/45-acp-tool-permission-gate.md` — related: per-command tool ceiling spec
