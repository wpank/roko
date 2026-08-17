# Batch ACP02 — ACP JSON-RPC types (inline, no SDK dep)

## Goal

Implement all ACP protocol types as Rust structs with serde in `types.rs`. No external JSON-RPC or ACP SDK dependencies.

## Target files

- `crates/roko-acp/src/types.rs` — Complete type definitions

## Implementation details

Implement every type listed in the context pack file `03-TYPE-REFERENCE.md`. This is the complete list:

### JSON-RPC base types
- `JsonRpcMessage` (untagged enum: Request | Response | Notification)
- `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcNotification`
- `JsonRpcId` (untagged enum: Number(u64) | String(String))
- `JsonRpcError`
- Error code constants: `PARSE_ERROR`, `INVALID_REQUEST`, `METHOD_NOT_FOUND`, `INVALID_PARAMS`, `INTERNAL_ERROR`, `SESSION_NOT_FOUND`, `SESSION_BUSY`

### Protocol constants
- `ACP_PROTOCOL_VERSION: u32 = 1`
- `ACP_SPEC_VERSION: &str = "0.12.2"`

### Initialize types
- `InitializeParams`, `InitializeResult`
- `ClientCapabilities`, `FsCapabilities`, `ClientInfo`
- `AgentCapabilities`, `PromptCapabilities`, `McpCapabilities`
- `AgentInfo`

### Session types
- `SessionNewParams`, `SessionNewResult`
- `McpServerConfig`, `McpTransport`
- `ModesInfo`, `ModeInfo`
- `SessionPromptParams`, `SessionPromptResult`
- `StopReason`
- `SessionCancelParams`
- `SessionListResult`, `SessionInfo`
- `SessionLoadParams`
- `SessionSetModeParams`

### Content & update types
- `ContentBlock` (tagged enum: Text | Resource | Diff)
- `ResourceRef` (tagged enum: File)
- `SessionUpdate` (tagged by `sessionUpdate` field): AgentMessageChunk, ThoughtMessageChunk, ToolCall, ToolCallUpdate, Plan, AvailableCommandsUpdate, ConfigOptionsUpdate, UsageUpdate, SessionInfoUpdate
- `ToolCallKind`, `ToolCallStatus`

### Config types
- `ConfigOption`, `ConfigOptionType`, `ConfigOptionValue`
- `ConfigUpdateParams`, `ConfigUpdateResult`

### Plan types
- `PlanEntry`, `Priority`, `PlanStatus`

### Command types
- `SlashCommand`, `CommandInput`

### Usage types
- `UsageInfo`, `CostInfo`

### Permission types
- `PermissionRequest`, `ToolCallPreview`, `PermissionOption`, `PermissionKind`
- `PermissionResponse`

### Elicitation types
- `ElicitationRequest`, `ElicitationResponse`

### Terminal types
- `TerminalCreateParams`, `TerminalCreateResult`
- `TerminalOutputParams`, `TerminalOutputResult`

### Key serde rules
- All ACP types use `#[serde(rename_all = "camelCase")]`
- `SessionUpdate` uses `#[serde(tag = "sessionUpdate")]` with `#[serde(rename_all = "snake_case")]`
- `ContentBlock` uses `#[serde(tag = "type")]`
- `JsonRpcMessage` uses `#[serde(untagged)]`
- `StopReason` uses `#[serde(rename_all = "snake_case")]`
- Optional fields use `#[serde(default, skip_serializing_if = "Option::is_none")]`

Add a few unit tests at the bottom:
- Test serializing/deserializing a JsonRpcRequest
- Test serializing/deserializing SessionUpdate::AgentMessageChunk
- Test serializing/deserializing ConfigOption

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- All types from 03-TYPE-REFERENCE.md are implemented
- All serde attributes are correct
- Unit tests pass
- `cargo clippy` is clean
