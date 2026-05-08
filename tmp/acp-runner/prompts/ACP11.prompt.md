# Batch ACP11 — Permission bridge

## Goal

Implement the permission bridge that asks the editor for user approval before destructive operations.

## Target files

- `crates/roko-acp/src/permissions.rs` — Permission bridge implementation

## Implementation details

### AcpPermissionGate struct

```rust
pub struct AcpPermissionGate {
    /// Transport for sending permission requests to the editor
    transport: Arc<Mutex<StdioTransport>>,
    /// Session ID for this gate
    session_id: String,
    /// Permanent allows (from "allow_always" responses)
    permanent_allows: HashSet<String>,
    /// Permanent denies (from "reject_always" responses)
    permanent_denies: HashSet<String>,
}
```

### Methods

1. **`new(transport, session_id)`** — Create with empty allow/deny sets

2. **`request_permission(&mut self, tool_call: ToolCallPreview) -> Result<PermissionDecision>`**
   - Check `permanent_allows` / `permanent_denies` first
   - If not cached: send `session/request_permission` to editor
   - Parse response: `allow_once`, `allow_always`, `reject_once`, `reject_always`
   - Update permanent sets for `always` decisions
   - Return decision

3. **`check_destructive(&self, action: &str) -> bool`** — Quick check if an action category is permanently allowed/denied

### PermissionDecision enum

```rust
pub enum PermissionDecision {
    Allowed,
    Denied,
}
```

### JSON-RPC message

```json
{
    "jsonrpc": "2.0",
    "id": N,
    "method": "session/request_permission",
    "params": {
        "sessionId": "sess_...",
        "toolCall": {
            "toolCallId": "write_main_rs",
            "title": "Modify src/main.rs",
            "kind": "edit",
            "status": "pending",
            "content": [{"type": "diff", "path": "src/main.rs", "diff": "..."}]
        },
        "options": [
            {"optionId": "allow", "name": "Allow", "kind": "allow_once"},
            {"optionId": "allow_all", "name": "Allow all this turn", "kind": "allow_always"},
            {"optionId": "reject", "name": "Reject", "kind": "reject_once"},
            {"optionId": "reject_all", "name": "Reject all", "kind": "reject_always"}
        ]
    }
}
```

### Integration point

This maps to Roko's `SafetyLayer` pattern. Before executing a tool that modifies files or runs commands, the orchestrator should call `permission_gate.request_permission()`.

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- PermissionGate sends correct JSON-RPC permission requests
- Permanent allow/deny caching works
- All 4 permission kinds are handled
- Integration point with SafetyLayer is clear
