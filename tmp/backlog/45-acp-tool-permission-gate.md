# 45 — ACP Tool Permission Gate

**Priority**: P1 — MCP tier checks and per-command tool filtering are safety enforcement gaps
**Size**: M (1-2 days)
**Crates**: `roko-agent` (`crates/roko-agent/`), `roko-acp` (`crates/roko-acp/`)
**Depends on**: None

---

## Background

The ACP (Agent Client Protocol) layer is how IDE integrations like Cursor talk to roko. When a user invokes a slash command (e.g. `/research <topic>`) or sends a prompt in Cursor's composer, the request travels through `crates/roko-acp/src/bridge_events.rs` and eventually dispatches to an agent with a tool set and a safety layer.

Three enforcement gaps allow tools to execute without the correct safety checks applied:

**Gap 1 — MCP tier not checked at dispatch time.** External MCP servers that are registered with the agent may have different `PluginTier` trust levels (`Trusted`, `Standard`, `Sandboxed`, `Untrusted`). The tier is stored in `RegisteredTool.tier` in `crates/roko-plugin/src/tool_registry.rs`. `McpHandlerResolver::resolve` in `crates/roko-agent/src/mcp/handler.rs` routes dynamic tool calls to the live MCP client but never checks the tier. A `Sandboxed` MCP server can therefore invoke `write_file` or `edit_file` with no block at the dispatch layer.

**Gap 2 — ACP slash commands share the full tool set.** `handle_session_prompt_inner` in `bridge_events.rs` calls `acp_builtin_tools()` at multiple points (lines 2505, 2589, 3083, 3347) to get the full ACP tool set. The full set includes `write_file`, `edit_file`, and `bash`. A `/research` session that should only read files and fetch web content gets the same tool set as a coding session. The `ToolPermission` flags (`read`, `write`, `exec`, `network`) are already assigned by `derive_tool_permissions()` in `crates/roko-acp/src/builtin_tools.rs:26`; they are just not used as a ceiling for slash commands.

**Gap 3 — Unknown ACP roles get wildcard tool access.** `SafetyLayer::with_defaults()` in `crates/roko-agent/src/safety/mod.rs:412` sets `tool_permission_policy: ToolPermissionPolicy::AllowExplicit` and `tool_permission_list: vec!["*".to_string()]`. The `"*"` wildcard means any tool passes the allowlist check. An unknown role that has no matching YAML bundle gets a safety layer that allows every tool. Only the governance guardrails (max tool calls, cost ceiling) apply; no individual tool is actually blocked.

These gaps exist because the ACP path was built to route correctly for known roles and known tools; the "unknown" fallback paths were not tightened when the safety layer was added.

## Current State

1. **`McpHandlerResolver<T>` in `crates/roko-agent/src/mcp/handler.rs:23`**: has fields `static_resolver: Arc<dyn HandlerResolver>`, `mcp_clients: HashMap<String, Arc<McpClient<T>>>`, `error_accumulator: Option<McpErrorAccumulator>`. The `resolve` method at line 62 extracts the server name from the prefixed tool name and returns an `Arc<McpToolHandler>`. There is no tier check and no `PluginTier` data in the struct.

2. **`check_plugin_tier` at `crates/roko-agent/src/safety/capabilities.rs:16`**: takes a `PluginTier` and a `&Capability` and returns `Ok(())` or an error string. `Capability::WritePath` is blocked for `Sandboxed` and `Untrusted`; `Capability::Exec` is blocked for both; `Capability::Network` is blocked for tiers that do not allow network.

3. **`derive_tool_permissions(tool_name)` at `crates/roko-acp/src/builtin_tools.rs:26`**: maps tool names to `ToolPermission { read, write, exec, git, network }` flags. `"read_file"`, `"glob"`, `"grep"`, `"ls"` → read-only. `"write_file"`, `"edit_file"` → read+write. `"bash"` → exec. `"web_fetch"`, `"web_search"` → read+network. Unknown tools get all-true permissions (fail open).

4. **`acp_builtin_tools()` at `crates/roko-acp/src/builtin_tools.rs:75`**: returns the full list of 8 ACP tools. Called at `bridge_events.rs` lines 2505, 2589, 3083, 3347 when assembling the tool set for a model call.

5. **`SafetyLayer::with_defaults()` at `crates/roko-agent/src/safety/mod.rs:412`**: sets `tool_permission_policy = AllowExplicit`, `tool_permission_list = vec!["*"]`. The `"*"` wildcard is documented as "allow all" when the policy is `AllowExplicit` (`safety/mod.rs:379`).

6. **`AgentContract::restricted("unknown")` at `crates/roko-agent/src/safety/contract.rs:181`**: sets `allowed_tools = Some(Vec::new())`, which means zero tools allowed. This is the correct fail-closed behavior for unknown roles, but `SafetyLayer::with_defaults()` does not use `restricted()`; it uses `hardened_default()` which sets `allowed_tools = None`.

7. **`run_slash_command` at `bridge_events.rs:4366`**: dispatches slash commands by mapping command names to CLI args and running them as subprocesses. The tool set for the session is assembled before entering this function.

## Implementation Plan

### Fix 1: Add tier check to `McpHandlerResolver::resolve`

`McpHandlerResolver` needs to know the tier of each MCP server. The simplest approach is to add a `server_tiers: HashMap<String, PluginTier>` field (keyed by server name, same as `mcp_clients`).

In `McpHandlerResolver::new`, accept an optional `server_tiers` parameter. Provide a builder method `with_server_tiers(HashMap<String, PluginTier>)`.

In `McpHandlerResolver::resolve` (line 63), after extracting `server_name`:

```rust
if let Some(tiers) = &self.server_tiers {
    let tier = tiers.get(server_name).copied().unwrap_or(PluginTier::Sandboxed);
    // Determine required capability from the tool name.
    let capability = capability_for_tool(remote_name);
    if let Err(reason) = check_plugin_tier(tier, &capability) {
        tracing::warn!(server = server_name, tool = name, %reason, "MCP tier check denied tool");
        return Some(Arc::new(DeniedToolHandler::new(reason)));
    }
}
```

Add a `DeniedToolHandler` struct (or return `None` from `resolve`) that returns `ToolError::PermissionDenied` when called.

Add a helper `capability_for_tool(tool_name: &str) -> Capability` that maps tool names to their primary capability (write tools → `WritePath`, bash → `Exec`, network tools → `Network`, others → `ReadPath`).

Estimated: ~60 lines in `handler.rs`.

### Fix 2: Per-slash-command tool ceiling in `bridge_events.rs`

Add a function `command_tool_ceiling(command: &str) -> Option<ToolPermission>` that returns a `ToolPermission` ceiling for known slash commands:

```rust
fn command_tool_ceiling(command: &str) -> Option<ToolPermission> {
    match command {
        "research" | "search" => Some(ToolPermission { read: true, write: false, exec: false, git: false, network: true }),
        "status" | "doctor" | "config" | "models" | "learn" => Some(ToolPermission { read: true, write: false, exec: false, git: false, network: false }),
        _ => None, // No ceiling for general commands
    }
}
```

In `handle_session_prompt_inner` and wherever `acp_builtin_tools()` is called with an active slash command context, filter the tool list:

```rust
let raw_tools = acp_builtin_tools();
let tools = if let Some(ceiling) = command_tool_ceiling(detected_command) {
    raw_tools.into_iter().filter(|tool| {
        let perm = derive_tool_permissions(&tool.name);
        // Keep tools whose permissions fit within the ceiling
        (!perm.write || ceiling.write) && (!perm.exec || ceiling.exec) && (!perm.network || ceiling.network)
    }).collect()
} else {
    raw_tools
};
```

The `is_slash_command` boolean is already computed at line 1687. Extract the command name from the prompt at the same point and pass it through.

Estimated: ~50 lines in `bridge_events.rs` and `builtin_tools.rs`.

### Fix 3: Tighten `SafetyLayer::with_defaults()` for unknown roles

In `crates/roko-agent/src/safety/mod.rs:439`, change `tool_permission_list` from `vec!["*".to_string()]` to `vec![]`:

```rust
// Before:
tool_permission_list: vec!["*".to_string()],

// After:
tool_permission_list: vec![], // deny-all default; callers must explicitly grant tools via with_role()
```

Any code that calls `SafetyLayer::with_defaults()` and expects tool access must be updated to call `with_role(known_role)` or `with_contract(explicit_contract)` afterward. Search `crates/` for `with_defaults()` callers and add the appropriate role or explicit allowlist. Tests using `with_defaults()` that expect tools to be permitted must use `SafetyLayer::permissive()` instead.

Note: `SafetyLayer::permissive()` at `safety/mod.rs:444` already uses `tool_permission_list: vec!["*"]` for test use. Callers of `with_defaults()` that need broad access should use `permissive()` or call `with_role("implementer")`.

Estimated: ~10 lines in `mod.rs`, plus updating existing callers.

## Acceptance Criteria

1. A `Sandboxed` or `Untrusted` tier MCP server attempting to invoke `write_file` or `edit_file` through `McpHandlerResolver::resolve` receives `ToolError::PermissionDenied` (or `None` causing a tool-not-found error) before the call reaches the remote server.
2. An ACP `/research` session's tool list (at `tools` variable assignment in `handle_session_prompt_inner`) contains only `read_file`, `glob`, `grep`, `web_fetch`, and `web_search`. `write_file`, `edit_file`, and `bash` are absent from the set passed to the model.
3. `SafetyLayer::with_defaults()` followed by no `with_role()` call results in `tool_permission_list` being empty (`[]`). Verified by unit test.
4. `cargo test -p roko-agent` and `cargo test -p roko-acp` pass with zero failures after changes.
5. `cargo clippy -p roko-agent -p roko-acp -- -D warnings` is clean.

## Verification Checklist

- [ ] Unit test: `McpHandlerResolver` with a `Sandboxed` server tier denies a write tool call
- [ ] Unit test: `/research` tool set from `handle_session_prompt_inner` excludes `write_file`, `edit_file`, `bash`
- [ ] Unit test: `SafetyLayer::with_defaults().tool_permission_list` is empty
- [ ] `cargo test -p roko-agent` passes
- [ ] `cargo test -p roko-acp` passes
- [ ] `cargo clippy -p roko-agent -p roko-acp -- -D warnings` is clean

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-agent/src/mcp/handler.rs` | Add `server_tiers: Option<HashMap<String, PluginTier>>` field to `McpHandlerResolver`; add `with_server_tiers` builder; add tier check in `resolve` before returning handler |
| `crates/roko-agent/src/mcp/dynamic_registry.rs` | Optionally expose a method to query per-server `PluginTier` so callers can pass tiers to `McpHandlerResolver` |
| `crates/roko-acp/src/bridge_events.rs` | Add `command_tool_ceiling(command)` function; filter `acp_builtin_tools()` output based on ceiling at the 4 call sites |
| `crates/roko-acp/src/builtin_tools.rs` | No structural changes needed; `derive_tool_permissions` already provides the per-tool permission flags |
| `crates/roko-agent/src/safety/mod.rs` | Change `tool_permission_list` in `with_defaults()` from `vec!["*"]` to `vec![]` (line 439); update downstream callers that relied on the wildcard default |
