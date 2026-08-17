# ACP Tool Permission Gate

**Priority**: P1
**Size**: M (1-2 days)

---

## Problem

The ACP (Agent Client Protocol) path — used by IDE integrations like Cursor — has three
enforcement gaps that allow tools to execute without the correct safety checks applied.

**Gap 1: `check_plugin_tier` is never called from the MCP bridge.**
`crates/roko-agent/src/mcp/handler.rs` routes dynamic tool calls from external MCP servers
through `McpHandlerResolver`. The resolver dispatches to the remote MCP server but never
calls `check_plugin_tier` before doing so. The `PluginTier` attached to each
`RegisteredTool` (in `roko-plugin/src/tool_registry.rs`) is therefore decorative — a
`Sandboxed` or `Untrusted` tier MCP server can invoke write tools like `write_file` and
`edit_file` without any tier check blocking it.

**Gap 2: No per-slash-command tool denylist in the ACP bridge.**
`crates/roko-acp/src/bridge_events.rs` dispatches every ACP slash command (e.g. `/research`,
`/run`, `/chat`) through the same tool set. A `/research` session that should only need
read-only tools (`read_file`, `glob`, `grep`, `web_search`) gets access to the full tool
set including `write_file`, `edit_file`, and `bash`. The `ToolPermission` infrastructure
already tracks per-tool permission flags (`read`, `write`, `exec`, `git`, `network`), and
`derive_tool_permissions` in `crates/roko-acp/src/builtin_tools.rs` correctly assigns those
flags, but nothing enforces a per-command ceiling on what the tool set may contain.

**Gap 3: Unknown ACP roles produce an overly broad default contract.**
`SafetyLayer::with_defaults()` in `crates/roko-agent/src/safety/mod.rs` now constructs with
`AgentContract::hardened_default("default")` and `ToolPermissionPolicy::AllowExplicit` with
a `["*"]` wildcard list, meaning unknown roles that fall through without a matching bundled
YAML get a layer whose `tool_permission_list` allows every tool. The hardened contract
applies governance guardrails (max tool calls, cost ceiling), but `allowed_tools = None`
combined with the wildcard list means no tool is actually blocked at dispatch time unless
the TOML role-tools config adds explicit restrictions.

These gaps exist because the ACP path was built to route correctly for known roles and
known tools, but the "unknown" fallback paths were not tightened when the safety layer was
added.

---

### What already exists

| Component | Location | Status |
|---|---|---|
| `ToolPermission` struct | `crates/roko-core/src/tool/def.rs` | EXISTS — `read`, `write`, `exec`, `git`, `network` bool flags |
| `derive_tool_permissions()` | `crates/roko-acp/src/builtin_tools.rs:26` | EXISTS — maps tool names to `ToolPermission`; unknown tools fail closed |
| `compute_session_capabilities()` | `crates/roko-acp/src/builtin_tools.rs:55` | EXISTS — unions tool permissions into a session ceiling |
| `check_plugin_tier()` | `crates/roko-agent/src/safety/capabilities.rs:16` | EXISTS — exported from `roko_agent::safety`, rejects writes for `Sandboxed`/`Untrusted` tiers |
| `PluginTier` in `RegisteredTool` | `crates/roko-plugin/src/tool_registry.rs` | EXISTS — tier field present but not consulted at dispatch |
| `McpHandlerResolver` | `crates/roko-agent/src/mcp/handler.rs` | EXISTS — routes calls to MCP clients, no tier check |
| `AgentContract::restricted()` | `crates/roko-agent/src/safety/contract.rs:181` | EXISTS — `allowed_tools = Some(vec![])`, full governance guardrails |
| `AgentContract::hardened_default()` | `crates/roko-agent/src/safety/contract.rs:211` | EXISTS — governance guardrails, `allowed_tools = None` (wildcard) |
| `SafetyLayer::with_defaults()` | `crates/roko-agent/src/safety/mod.rs:412` | EXISTS — uses `hardened_default`, wildcard tool list |
| `PermissionAction` / `session/request_permission` | `crates/roko-acp/src/types.rs:1015` | EXISTS — mutation consent flow for `FileEdit`/`GitOperation` |

---

### What is missing

1. **`check_plugin_tier` call in `McpHandlerResolver::resolve`**
   Before dispatching to a live MCP client, resolve the `PluginTier` of the tool from the
   `DynamicToolRegistry` and call `check_plugin_tier(tier, &required_capability)`. The
   required capability can be derived from the tool's `McpToolAnnotations` (is_read_only,
   is_open_world). If the check fails, return `None` from `resolve` or return a handler
   that immediately returns `ToolError::PermissionDenied`.

2. **Per-slash-command tool filter in `bridge_events.rs`**
   Define a `command_tool_ceiling` function (or inline match) that maps ACP slash command
   names to a `ToolPermission` ceiling. `/research` gets `read = true, network = true`,
   everything else false. When building the tool set for a session, filter out any tool
   whose `ToolPermission` flags exceed the command's ceiling. The tool set construction
   happens in `handle_session_prompt` and the functions it calls.

3. **Restrictive unknown-role fallback in `SafetyLayer::with_defaults`**
   Change `tool_permission_list` from `vec!["*"]` to `vec![]` (empty denylist for the
   `AllowExplicit` policy), so an unknown role gets no tools by default. Callers that
   need broad access must explicitly request it via `with_role(known_role)` or
   `with_contract(explicit_contract)`. Alternatively, pair the existing wildcard list with
   a mandatory role check so `with_defaults()` without a subsequent `with_role()` produces
   a compile-time or runtime warning.

---

## Where to make changes

| File | Change |
|---|---|
| `crates/roko-agent/src/mcp/handler.rs` | Add `check_plugin_tier` call in `McpHandlerResolver::resolve` before returning an MCP-backed handler |
| `crates/roko-agent/src/mcp/dynamic_registry.rs` | Expose a method to look up the `PluginTier` for a tool name so `handler.rs` can retrieve it |
| `crates/roko-acp/src/bridge_events.rs` | Add `command_tool_ceiling(command: &str) -> ToolPermission` and filter the tool set at session-prompt dispatch |
| `crates/roko-agent/src/safety/mod.rs` | Tighten `SafetyLayer::with_defaults()` so the default wildcard cannot silently allow all tools for unknown roles |

---

## Acceptance criteria

1. A `Sandboxed` or `Untrusted` tier MCP server attempting to invoke a write tool (`write_file`,
   `edit_file`, `apply_patch`) receives `ToolError::PermissionDenied` from
   `McpHandlerResolver::resolve` before the call reaches the remote server.
2. An ACP `/research` session's tool list contains only `read_file`, `glob`, `grep`,
   `web_fetch`, and `web_search`. Any write or exec tool is absent from the set sent to
   the model, not just blocked at call time.
3. `SafetyLayer::with_defaults()` followed by no `with_role()` call results in a layer
   where no tool passes the `tool_permission_list` check when
   `ToolPermissionPolicy::AllowExplicit` is in effect (i.e., an empty or wildcard-free list).
4. `cargo test -p roko-agent` and `cargo test -p roko-acp` pass with zero failures after
   changes.
5. `cargo clippy -p roko-agent -p roko-acp -- -D warnings` is clean.

---

## References

- `crates/roko-agent/src/mcp/handler.rs` — `McpHandlerResolver`, the dispatch site missing the tier check
- `crates/roko-agent/src/safety/capabilities.rs:16` — `check_plugin_tier` function
- `crates/roko-agent/src/safety/mod.rs:412` — `SafetyLayer::with_defaults()` wildcard list
- `crates/roko-agent/src/safety/contract.rs:165,181,211` — `permissive`, `restricted`, `hardened_default`
- `crates/roko-acp/src/builtin_tools.rs` — `derive_tool_permissions`, `compute_session_capabilities`
- `crates/roko-acp/src/bridge_events.rs` — `handle_session_prompt`, tool set construction
- `crates/roko-plugin/src/tool_registry.rs` — `RegisteredTool` with `PluginTier` field
