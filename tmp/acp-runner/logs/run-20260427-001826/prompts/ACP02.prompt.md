# ACP Batch ACP02

Run id: run-20260427-001826
Attempt: 3
Model: gpt-5.4
Reasoning: high

## Previous attempt failure context

Cargo check failed: code does not compile.

Recent log tail:
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests roko_acp

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


codex
`crates/roko-acp/src/types.rs` was already in the correct completed state. I verified the inline ACP/JSON-RPC type catalog, serde tagging/renaming, protocol constants, and the three round-trip unit tests in [types.rs](/Users/will/dev/nunchi/roko/roko/.roko/worktrees/acp-run-20260427-001826/crates/roko-acp/src/types.rs:1). I did not need to change code.

The previous failure was not a compile issue in `roko-acp`; it was a malformed verification command that passed `' -p roko-acp'` as a single unexpected argument. With the correct commands, all checks pass:

- `cargo check -p roko-acp`
- `cargo clippy -p roko-acp --no-deps -- -D warnings`
- `cargo test -p roko-acp`
2026-04-26T22:22:09.003717Z ERROR codex_core::session: failed to record rollout items: thread 019dcbe2-4042-7960-a075-5a3e1f010ed5 not found
tokens used
22,004
`crates/roko-acp/src/types.rs` was already in the correct completed state. I verified the inline ACP/JSON-RPC type catalog, serde tagging/renaming, protocol constants, and the three round-trip unit tests in [types.rs](/Users/will/dev/nunchi/roko/roko/.roko/worktrees/acp-run-20260427-001826/crates/roko-acp/src/types.rs:1). I did not need to change code.

The previous failure was not a compile issue in `roko-acp`; it was a malformed verification command that passed `' -p roko-acp'` as a single unexpected argument. With the correct commands, all checks pass:

- `cargo check -p roko-acp`
- `cargo clippy -p roko-acp --no-deps -- -D warnings`
- `cargo test -p roko-acp`

=== Finished: 2026-04-27T00:22:09+02:00 ===
=== Duration: 34s ===
=== Exit code: 0 ===
[verify] diff_gate: 1 changed path(s)
[verify] required-term check: OK (pattern: JsonRpc|SessionUpdate|ContentBlock)
[verify] cargo check: CARGO_TARGET_DIR=/var/folders/bn/ks_s66191vb0zzvs08qcw_gh0000gn/T//roko-acp-check-wDYvR1 cargo check  -p roko-acp
error: unexpected argument ' -p roko-acp' found

Usage: cargo check [OPTIONS]

For more information, try '--help'.
[verify] cargo check: FAILED (exit 1)

Use that context to avoid repeating the same failure.

## Shared Context Pack

### 00-ACP-RULES

# ACP Runner Rules (read first)

## Core rules

1. **No prior chat** — This prompt is self-sufficient. Do not reference external conversations.
2. **Scope locked** — Only modify files under `crates/roko-acp/` unless explicitly told otherwise (ACP07 may touch `crates/roko-cli/src/main.rs`).
3. **Repo reality** — Use `rg` or `grep` to verify current state before editing. Never assume file contents.
4. **No external SDK deps** — All ACP protocol types are defined inline in `roko-acp/src/types.rs`. Do NOT add dependencies on external ACP/JSON-RPC SDK crates.
5. **stdout = protocol channel** — All logging MUST go to files or stderr. Any non-JSON output on stdout corrupts the protocol stream. Use `tracing` with a file appender.
6. **Commit message format** — `acp(ACPnn): <batch title>` (e.g., `acp(ACP01): Scaffold roko-acp crate + workspace wire`).
7. **Subagents OK** — Spawn workers with disjoint write scopes when beneficial.
8. **Substantive only** — No placeholder `todo!()` macros or `unimplemented!()` in public APIs unless the batch explicitly says to stub. Every function must have a real implementation or a clear `// Stubbed — wired in batch ACPnn` comment.
9. **No destructive git** — The runner handles branch lifecycle. Do not create branches, commit, or push.
10. **Reuse existing patterns** — Roko has established patterns for Substrate, ProcessSupervisor, StateHub, CostLens, etc. Wire into them, don't reinvent.

## Rust conventions

- `#[derive(Debug, Clone, Serialize, Deserialize)]` on all types
- `#[serde(rename_all = "camelCase")]` for ACP protocol types (JSON uses camelCase)
- `#[serde(tag = "sessionUpdate")]` for discriminated unions
- Use `thiserror` for error types
- Use `tokio` for async runtime
- Use `tracing` for structured logging
- All public items need doc comments (`///`)

## Dependency rules

- Allowed deps: tokio, serde, serde_json, tracing, tracing-subscriber (with file appender), uuid, chrono, thiserror, anyhow
- Allowed workspace deps: roko-core, roko-agent, roko-orchestrator, roko-compose, roko-gate, roko-fs, roko-runtime, roko-conductor, roko-learn, roko-neuro, roko-daimon, roko-primitives
- Do NOT add: any external JSON-RPC crate, any ACP SDK crate, tower, hyper, axum (this is stdio, not HTTP)

## File organization

```
crates/roko-acp/
├── Cargo.toml
└── src/
    ├── lib.rs              # Module declarations + re-exports
    ├── types.rs            # All ACP protocol types (JSON-RPC, sessions, updates)
    ├── transport.rs        # Stdio transport (read/write JSON-RPC messages)
    ├── handler.rs          # Main dispatch loop (method → handler)
    ├── session.rs          # Session state management
    ├── config.rs           # AcpConfig struct
    ├── config_options.rs   # 7 session config options
    ├── commands.rs         # 8 slash commands
    ├── elicitation.rs      # Structured form dialogs
    ├── permissions.rs      # Permission request/response bridge
    ├── bridge_fs.rs        # File system bridge (editor-mediated)
    ├── bridge_terminal.rs  # Terminal bridge (editor-mediated)
    ├── bridge_events.rs    # Cognitive event → session/update streaming
    ├── bridge_plan.rs      # Plan phase → plan notifications
    ├── bridge_gates.rs     # Gate results → tool call cards
    └── bridge_usage.rs     # Token/cost → usage notifications
```

### 01-ACP-PROTOCOL-PRIMER

# ACP Protocol Primer

## What is ACP?

Agent Client Protocol (ACP) is a JSON-RPC 2.0 protocol over stdio that lets AI agents communicate with editors (JetBrains, Zed, Neovim, VS Code). It's the standard way editors spawn and interact with coding agents.

## Transport

- **Newline-delimited JSON** over stdin/stdout
- Each message is a single line of JSON followed by `\n`
- Agent reads from stdin, writes to stdout
- All logging goes to a file (stdout is the protocol channel)

## Message Types

### Request (bidirectional)
```json
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {...}}
```

### Response
```json
{"jsonrpc": "2.0", "id": 1, "result": {...}}
```
or
```json
{"jsonrpc": "2.0", "id": 1, "error": {"code": -32600, "message": "..."}}
```

### Notification (no id, no response expected)
```json
{"jsonrpc": "2.0", "method": "session/update", "params": {...}}
```

## Lifecycle

1. **Initialize**: Client sends `initialize` → Agent responds with capabilities
2. **Session**: Client sends `session/new` → Agent responds with session ID + config options
3. **Prompt**: Client sends `session/prompt` → Agent streams `session/update` notifications → Agent sends final response
4. **Cancel**: Client sends `session/cancel` notification → Agent stops current prompt
5. **Config**: Client sends `session/config/update` → Agent responds with updated options

## Bidirectional Flow

The agent can also send requests TO the editor:
- `fs/read_text_file` — Read a file through the editor
- `fs/write_text_file` — Write a file through the editor
- `terminal/create` — Create a terminal session
- `terminal/output` — Get terminal output
- `terminal/wait_for_exit` — Wait for command completion
- `terminal/release` — Release terminal resources
- `session/request_permission` — Ask user to approve an action
- `elicitation/create` — Show a structured form

## Protocol Version

```rust
pub const ACP_PROTOCOL_VERSION: u32 = 1;
pub const ACP_SPEC_VERSION: &str = "0.12.2";
```

## Key Conventions

- All JSON field names use `camelCase`
- Session IDs are prefixed: `sess_` + UUID
- Tool call IDs are descriptive: `gate_compile_001`, `write_main_rs`
- The `session/update` notification is the workhorse — it carries all streaming data via the `sessionUpdate` discriminator field

### 02-ROKO-ARCHITECTURE

# Roko Architecture (for ACP bridge authors)

## Where roko-acp fits

`roko-acp` is a **presentation layer** — it sits alongside `roko-cli` and `roko-serve` as an alternative frontend. The entire cognitive pipeline runs identically:

```
Editor ←stdio→ roko-acp ──→ roko-orchestrator ──→ roko-agent ──→ LLM
                   │              │                    │
                   │              ├──→ roko-gate        │
                   │              ├──→ roko-compose      │
                   │              ├──→ roko-learn        │
                   │              └──→ roko-conductor    │
                   │                                    │
                   └── bridges (fs, terminal, perms) ◄──┘
```

## Key crates to know

| Crate | What | You'll use it for |
|-------|------|-------------------|
| `roko-core` | Signal + 6 traits, types, config, tools, errors | `Engram`, `AgentRole`, config types |
| `roko-agent` | LLM backends, dispatch, tool loop | Agent spawning, response streaming |
| `roko-orchestrator` | Plan DAG, executor, merge queue | Plan execution, task management |
| `roko-compose` | Prompt assembly, 9 templates | System prompt building |
| `roko-gate` | 11 gates, 7-rung pipeline | Gate execution and results |
| `roko-fs` | FileSubstrate (JSONL), GC, layout | Signal/episode persistence |
| `roko-runtime` | ProcessSupervisor, event bus, cancellation | Process lifecycle, CancelToken |
| `roko-conductor` | 10 watchers, circuit breaker | Auto-correction, diagnosis |
| `roko-learn` | Episodes, playbooks, bandits, routing | Learning state queries |
| `roko-neuro` | Durable knowledge store | Knowledge queries |
| `roko-daimon` | Affect engine, somatic markers | PAD state |
| `roko-primitives` | HDC vectors, tier routing | Model tier types |

## Key types

- `roko_core::config::RokoConfig` — workspace configuration from `roko.toml`
- `roko_core::types::AgentRole` — agent role enum (Architect, Implementer, Reviewer, etc.)
- `roko_runtime::CancelToken` — cooperative cancellation
- `roko_gate::GateResult` — gate pass/fail with details
- `roko_learn::CascadeRouter` — model tier routing
- `roko_learn::CostLens` — token/cost accumulator

## Workspace layout

```
.roko/
├── roko.toml              # Config
├── signals.jsonl           # Signal log
├── episodes.jsonl          # Episode log
├── state/                  # Executor snapshots
├── learn/                  # Learning state (cascade-router.json, etc.)
├── prd/                    # PRD documents
└── research/               # Research artifacts
```

### 03-TYPE-REFERENCE

# ACP Type Reference

Complete catalog of all ACP protocol types to implement in `types.rs`.

## JSON-RPC Base Types

```rust
/// A JSON-RPC 2.0 message (request, response, or notification)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    Notification(JsonRpcNotification),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcRequest {
    pub jsonrpc: String, // Always "2.0"
    pub id: JsonRpcId,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    Number(u64),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: JsonRpcId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}
```

## Standard Error Codes

```rust
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;
pub const SESSION_NOT_FOUND: i32 = -32000;
pub const SESSION_BUSY: i32 = -32001;
```

## Initialize

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: u32,
    #[serde(default)]
    pub client_capabilities: ClientCapabilities,
    #[serde(default)]
    pub client_info: Option<ClientInfo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    #[serde(default)]
    pub fs: Option<FsCapabilities>,
    #[serde(default)]
    pub terminal: Option<bool>,
    #[serde(default)]
    pub mcp_servers: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FsCapabilities {
    #[serde(default)]
    pub read_text_file: bool,
    #[serde(default)]
    pub write_text_file: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: u32,
    pub agent_capabilities: AgentCapabilities,
    pub agent_info: AgentInfo,
    #[serde(default)]
    pub auth_methods: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    #[serde(default)]
    pub load_session: bool,
    #[serde(default)]
    pub prompt_capabilities: Option<PromptCapabilities>,
    #[serde(default)]
    pub mcp_capabilities: Option<McpCapabilities>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptCapabilities {
    pub image: bool,
    pub audio: bool,
    pub embedded_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCapabilities {
    pub http: bool,
    pub sse: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub name: String,
    pub title: String,
    pub version: String,
}
```

## Session Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNewParams {
    #[serde(default)]
    pub session_name: Option<String>,
    #[serde(default)]
    pub client_capabilities: Option<ClientCapabilities>,
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum McpTransport {
    Http { url: String },
    Stdio { command: String, args: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNewResult {
    pub session_id: String,
    pub config_options: Vec<ConfigOption>,
    #[serde(default)]
    pub modes: Option<ModesInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModesInfo {
    pub current_mode_id: String,
    pub available_modes: Vec<ModeInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModeInfo {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPromptParams {
    pub session_id: String,
    pub prompt: Vec<ContentBlock>,
    #[serde(default)]
    pub include_context: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPromptResult {
    pub session_id: String,
    pub stop_reason: StopReason,
    #[serde(default)]
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    Cancelled,
    Error,
}
```

## Content & Update Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ContentBlock {
    Text { text: String },
    Resource { resource: ResourceRef },
    Diff { path: String, diff: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ResourceRef {
    File { uri: String },
}

/// Session update notification payload — discriminated by `sessionUpdate` field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "sessionUpdate")]
pub enum SessionUpdate {
    AgentMessageChunk {
        content: ContentBlock,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        _meta: Option<serde_json::Value>,
    },
    ThoughtMessageChunk {
        content: ContentBlock,
    },
    ToolCall {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        title: String,
        kind: ToolCallKind,
        status: ToolCallStatus,
        #[serde(default)]
        content: Vec<ContentBlock>,
    },
    ToolCallUpdate {
        #[serde(rename = "toolCallId")]
        tool_call_id: String,
        status: ToolCallStatus,
        #[serde(default)]
        content: Vec<ContentBlock>,
    },
    Plan {
        entries: Vec<PlanEntry>,
    },
    AvailableCommandsUpdate {
        available_commands: Vec<SlashCommand>,
    },
    ConfigOptionsUpdate {
        config_options: Vec<ConfigOption>,
    },
    UsageUpdate {
        used: u64,
        size: u64,
        #[serde(default)]
        cost: Option<CostInfo>,
    },
    SessionInfoUpdate {
        session_id: String,
        #[serde(default)]
        session_name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallKind {
    Edit,
    Create,
    Delete,
    Terminal,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}
```

## Config, Command, Permission Types

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOption {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub option_type: ConfigOptionType,
    pub category: String,
    pub current_value: serde_json::Value,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<ConfigOptionValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigOptionType {
    Select,
    Toggle,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigOptionValue {
    pub value: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanEntry {
    pub content: String,
    pub priority: Priority,
    pub status: PlanStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input: Option<CommandInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandInput {
    #[serde(default)]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageInfo {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default)]
    pub thought_tokens: Option<u64>,
    #[serde(default)]
    pub cached_read_tokens: Option<u64>,
    #[serde(default)]
    pub cached_write_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostInfo {
    pub amount: f64,
    pub currency: String,
}

// Permission types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub session_id: String,
    pub tool_call: ToolCallPreview,
    pub options: Vec<PermissionOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallPreview {
    pub tool_call_id: String,
    pub title: String,
    pub kind: ToolCallKind,
    pub status: ToolCallStatus,
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionOption {
    pub option_id: String,
    pub name: String,
    pub kind: PermissionKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionResponse {
    pub selected_option: String,
}

// Elicitation types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationRequest {
    pub session_id: String,
    pub mode: String, // "form"
    pub message: String,
    pub requested_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationResponse {
    pub outcome: String, // "accept" | "reject"
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

// Terminal types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCreateParams {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub env: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalCreateResult {
    pub terminal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputParams {
    pub terminal_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputResult {
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
}

// Config update
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdateParams {
    pub session_id: String,
    pub option_id: String,
    pub new_value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigUpdateResult {
    pub config_options: Vec<ConfigOption>,
}

// Session cancel
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCancelParams {
    pub session_id: String,
}

// Session list
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListResult {
    pub sessions: Vec<SessionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub session_id: String,
    #[serde(default)]
    pub session_name: Option<String>,
    pub created_at: String,
}

// Session load
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLoadParams {
    pub session_id: String,
}

// Set mode (legacy)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSetModeParams {
    pub session_id: String,
    pub mode_id: String,
}
```

### 04-EXISTING-PATTERNS

# Existing Roko Patterns (for bridge implementations)

## Pattern 1: Substrate trait (for FS bridge)

The `Substrate` trait in `roko-core` handles persistent storage:

```rust
#[async_trait]
pub trait Substrate: Send + Sync {
    async fn write(&self, engram: &Engram) -> Result<()>;
    async fn read(&self, hash: &str) -> Result<Option<Engram>>;
    async fn query(&self, filter: &Filter) -> Result<Vec<Engram>>;
    async fn delete(&self, hash: &str) -> Result<()>;
}
```

The ACP FS bridge (`bridge_fs.rs`) should implement a similar interface but route reads/writes through the editor's `fs/read_text_file` and `fs/write_text_file` JSON-RPC methods. When the editor doesn't declare `fs` capability, fall back to direct filesystem I/O.

## Pattern 2: ProcessSupervisor (for terminal bridge)

`roko-runtime` provides `ProcessSupervisor` for managing child processes:

```rust
pub struct ProcessSupervisor {
    processes: HashMap<String, ProcessHandle>,
}

impl ProcessSupervisor {
    pub async fn spawn(&mut self, cmd: &str, args: &[String]) -> Result<String>;
    pub async fn output(&self, id: &str) -> Result<ProcessOutput>;
    pub async fn kill(&mut self, id: &str) -> Result<()>;
    pub async fn wait(&self, id: &str) -> Result<ExitStatus>;
}
```

The ACP terminal bridge (`bridge_terminal.rs`) should route commands through the editor's `terminal/*` JSON-RPC methods. When the editor doesn't declare `terminal` capability, fall back to `ProcessSupervisor`.

## Pattern 3: StateHub / TuiBridge (for event streaming)

The TUI uses a push-based event model:

```rust
pub enum DashboardEvent {
    AgentOutput(String),
    GateStarted { name: String },
    GateCompleted { name: String, passed: bool },
    PhaseTransition(PlanPhase),
    // ...
}
```

Events flow via `tokio::sync::watch` channels. The ACP bridge should use a similar channel-based approach: the cognitive loop sends `CognitiveEvent`s, and `bridge_events.rs` maps them to ACP `session/update` notifications.

## Pattern 4: CostLens (for usage bridge)

`roko-learn` provides `CostLens` for tracking costs:

```rust
pub struct CostLens {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_tokens: u64,
    pub total_cost_usd: f64,
}
```

The ACP usage bridge (`bridge_usage.rs`) should accumulate from `CostLens` and push `usage_update` notifications.

## Pattern 5: CancelToken (for session cancellation)

`roko-runtime` provides cooperative cancellation:

```rust
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self;
    pub fn cancel(&self);
    pub fn is_cancelled(&self) -> bool;
    pub async fn cancelled(&self); // Future that resolves when cancelled
}
```

Each ACP session should have its own `CancelToken`. When the client sends `session/cancel`, call `token.cancel()` to stop the cognitive loop.

## Pattern 6: GateResult (for gate bridge)

Gate results carry structured data:

```rust
pub struct GateResult {
    pub gate_name: String,
    pub passed: bool,
    pub duration: Duration,
    pub details: GateDetails,
}

pub enum GateDetails {
    Compile { warnings: u32, errors: u32 },
    Test { passed: u32, failed: u32, total: u32 },
    Clippy { warnings: u32 },
    // ...
}
```

The gate bridge (`bridge_gates.rs`) maps these to ACP `tool_call` and `tool_call_update` notifications with markdown content summaries.

## Delegation Requirement

You are authorized to use subagents. Prefer multiple parallel agents when
the target file set is large.

Required delegation behavior:

- Form a plan first — for each target file, decide (a) does it need changes,
  (b) how big, (c) is it self-contained.
- For large independent modules, spawn a worker per file with a disjoint
  write scope.
- Every subagent gets the same context pack.
- Do not wait idly for subagents if you can progress locally.
- If subagents are unavailable in this environment, continue locally.

Suggested parallel split for batch `ACP02`:

- worker: implement JSON-RPC base types and error codes
- worker: implement initialize + session types
- worker: implement content, update, config, command, permission, elicitation types
- worker: write serde unit tests

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
