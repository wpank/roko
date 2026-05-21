# ACP Integration PRD — Roko as an Agent Client Protocol Server

> **Document**: PRD + Implementation Plan
> **Date**: April 26, 2026
> **Status**: Draft
> **Scope**: Implement ACP (Agent Client Protocol) server mode in Roko so that Roko agents are usable from any ACP-compatible editor — JetBrains IDEs, Zed, Neovim, Emacs, Obsidian, Toad, and any future ACP client — with zero custom integration work per editor.

---

## 1. Context & Motivation

### 1.1 What ACP Is

The Agent Client Protocol is an open standard co-developed by Zed Industries and JetBrains (partnership announced October 2025) that standardizes communication between code editors and AI coding agents. It is the LSP (Language Server Protocol) equivalent for AI agents. LSP made it so any editor could support any language through a shared interface; ACP does the same for coding agents.

The protocol is JSON-RPC 2.0 over stdio. The editor spawns the agent as a subprocess (`roko acp`), and they exchange newline-delimited JSON messages bidirectionally over stdin/stdout. The editor handles all UI — rendering markdown, displaying diffs, showing permission dialogs, managing terminal panels. The agent handles all cognition — model routing, tool execution, planning, verification.

Protocol version: v0.11.0 (March 2026), pre-1.0 but core lifecycle is stable. Apache 2.0 licensed. 2.3k+ GitHub stars. Official SDKs in Rust, TypeScript, Python, Kotlin, Java.

### 1.2 What ACP Is Not

ACP is not A2A (Agent-to-Agent Protocol). A2A is Google/Linux Foundation's protocol for agents talking to other agents over HTTP. ACP is for agents talking to editors over stdio. They solve orthogonal problems and compose naturally:

- **MCP** — agent ↔ tools (vertical, Roko already has this)
- **ACP** — agent ↔ editor/IDE (Roko inside editors, this document)
- **A2A** — agent ↔ agent over network (cross-instance, separate effort)

ACP sits between the user and the agent. A2A sits between agents. MCP sits between an agent and external data/tools. All three are JSON-RPC 2.0.

### 1.3 Why This Matters for Roko

Roko currently has three user interfaces: the CLI (`roko run`), the TUI (ratatui dashboard), and the web dashboard (Nunchi). None of these are where most developers spend their time — they're in their editors. ACP puts Roko directly into the editor, right next to the code, without any editor-specific integration work.

The economics are compelling:

- **JetBrains**: 30M+ users across IntelliJ, PyCharm, WebStorm, GoLand, RustRover, CLion. ACP is native since 2025.3. No JetBrains AI subscription required to use ACP agents.
- **Zed**: Fast-growing editor, created the protocol, most complete ACP support.
- **Neovim**: Via CodeCompanion and avante.nvim plugins. Large power-user audience.
- **Emacs**: Community ACP plugin.
- **Obsidian**: Via obsidian-agent-client. Relevant to knowledge management workflow.
- **Toad**: Will McGugan's TUI supporting 18+ agents via ACP.

One implementation, all of these editors. Implement ACP once, reach everywhere.

The ACP Registry (launched January 28, 2026) provides one-click installation. Once Roko is in the registry, any JetBrains or Zed user can install it from a dropdown menu — no `pip install`, no manual config, no documentation reading. The registry handles downloading, runtime management, and configuration.

Current ACP registry members: Cursor, Gemini CLI, OpenAI Codex, goose (Block/Square), Augment Code, Kimi CLI (Moonshot AI), Kiro (AWS), Mistral Vibe, OpenCode, Claude Code (via adapter), Cline, OpenHands. Roko would sit alongside all of these as a peer.

### 1.4 What Makes Roko Different from Other ACP Agents

Every other agent in the ACP registry is essentially a single-loop LLM wrapper: prompt → model → tool calls → response. Roko brings capabilities that no other ACP agent has:

- **Gate pipeline**: Compile, test, clippy, and formal verification gates that objectively verify agent output. When a user sees "Gate: 147/147 tests passed" in their editor, they know the code works — not because the LLM said so, but because the EVM/compiler proved it.
- **Multi-phase execution**: Enriching → Implementing → Gating → Verifying → Reviewing → Merging. ACP's plan support renders this as a visual step-by-step pipeline in the editor.
- **Knowledge store**: Persistent knowledge entries (Engrams) with confidence scores, decay dynamics, and HDC fingerprints. The agent remembers and learns across sessions.
- **Daimon affect state**: PAD vectors modulate agent behavior. High arousal from repeated failures triggers strategy shifts visible to the user.
- **Session modes**: Not just "ask" vs "code" — Roko offers plan-first, research, review, and full autonomous modes that map naturally to ACP's session mode system.
- **Conductor watchers**: GhostTurn, ReviewLoop, CompileFailThreshold — autonomous corrective actions that run in the background during execution.

These show up as concrete UX advantages in the editor: plan steps with live status updates, gate results as structured tool call outputs, mode switching between architect and implement, and slash commands that expose Roko-specific capabilities.

---

## 2. Protocol Mapping — ACP ↔ Roko

### 2.1 Protocol Lifecycle Mapped to Roko Concepts

```
ACP Protocol                          Roko Internals
──────────────────                    ──────────────────────────────────
initialize                            Capability negotiation
  clientCapabilities.fs.readTextFile   → AcpFileSubstrate (replaces local FileSubstrate)
  clientCapabilities.fs.writeTextFile  → AcpFileSubstrate (diffs shown in editor)
  clientCapabilities.terminal          → AcpTerminal (replaces ProcessSupervisor for shell)
  agentCapabilities.loadSession        → KnowledgeStore + DaimonState persistence
  agentCapabilities.promptCapabilities → Image support depends on backend model

session/new                            Episode::new() + KnowledgeStore::open()
  → returns sessionId                  → maps to episode_id
  → returns modes                      → Roko session modes (code/plan/research/review)
  → triggers available_commands_update  → Roko slash commands

session/prompt                         CognitiveLoop::run_once()
  prompt[].text                        → user_prompt in PromptComposer
  prompt[].resource                    → file content injected into context

session/update notifications           Bus Pulse events mapped to ACP updates:
  agent_message_chunk                  ← TokenChunk from LLM streaming
  tool_call                            ← ToolCallStart (read file, write file, run cmd)
  tool_call_update                     ← ToolCallProgress / ToolCallComplete
  plan                                 ← PlanPhase transitions in PlanState
  available_commands_update            ← Dynamic command updates based on context

session/request_permission             SafetyLayer::check_pre_execution()
  → editor shows approval dialog       → maps to GateKeeper policy
  → user approves/rejects             → CancelToken or proceed

fs/read_text_file (agent → editor)     Replaces roko_fs::FileSubstrate::read()
fs/write_text_file (agent → editor)    Replaces roko_fs::FileSubstrate::write()
                                        Editor shows inline diff with accept/reject

terminal/create (agent → editor)       Replaces bardo_runtime::ProcessSupervisor
terminal/output                        Editor's terminal panel shows output
terminal/wait_for_exit                 Gate pipeline waits for compile/test results
terminal/kill                          CancelToken propagation

session/cancel                         CancelToken::cancel() propagated to all subsystems

session/load                           Episode::load() + KnowledgeStore::load()
                                        Restores full session state including learned knowledge

session/set_mode                       Switches Roko routing tier:
  "code"                               → T1/T2 standard coding
  "plan"                               → Strategist-first with plan DAG
  "research"                           → Deep research with knowledge store queries
  "review"                             → Reviewer role with gate emphasis
  "auto"                               → Full autonomous mode (all conductor watchers active)
```

### 2.2 Session Modes

ACP supports advertising different operating modes that the user can switch between. Roko maps these to its existing routing tiers and agent roles:

```json
{
  "modes": {
    "currentModeId": "code",
    "availableModes": [
      {
        "id": "code",
        "name": "Code",
        "description": "Write and modify code with compile/test gate verification"
      },
      {
        "id": "plan",
        "name": "Plan",
        "description": "Create a task DAG before implementing — architect first, then build"
      },
      {
        "id": "research",
        "name": "Research",
        "description": "Deep research with knowledge store integration and synthesis"
      },
      {
        "id": "review",
        "name": "Review",
        "description": "Code review mode — analyze existing code, run gates, suggest improvements"
      },
      {
        "id": "auto",
        "name": "Autonomous",
        "description": "Full autonomous execution with conductor watchers and self-correction"
      }
    ]
  }
}
```

When the user switches from "plan" to "code" (or when the model calls a "switch mode" tool after planning), Roko transitions from the Strategist role to the Implementer role, changes the system prompt via `RoleSystemPromptSpec`, and activates the gate pipeline. ACP's built-in mode-switch permission dialog maps directly to this: the editor shows "Roko wants to switch from Plan to Code mode. The implementation plan is: [plan summary]. Allow?"

### 2.3 Slash Commands

ACP allows agents to advertise slash commands that users can invoke directly. Roko exposes its unique capabilities through these:

```json
{
  "availableCommands": [
    {
      "name": "plan",
      "description": "Create a multi-step implementation plan as a task DAG",
      "input": { "hint": "what to build or change" }
    },
    {
      "name": "gate",
      "description": "Run the gate pipeline (compile, test, clippy) against the current state",
      "input": { "hint": "optional: specific gate to run" }
    },
    {
      "name": "learn",
      "description": "Show what the agent has learned — knowledge entries with confidence scores",
      "input": { "hint": "optional: topic to filter by" }
    },
    {
      "name": "inspect",
      "description": "Drill into an episode, engram, or heuristic by ID",
      "input": { "hint": "entity ID or description" }
    },
    {
      "name": "replay",
      "description": "Replay a previous episode — show what the agent did and why",
      "input": { "hint": "episode ID or description" }
    },
    {
      "name": "heuristics",
      "description": "Show applicable heuristics for the current file or context",
      "input": { "hint": "optional: file path or topic" }
    },
    {
      "name": "status",
      "description": "Show agent status — PAD state, active watchers, knowledge stats"
    },
    {
      "name": "budget",
      "description": "Show remaining token/cost budget for this session"
    }
  ]
}
```

These commands are dynamic — `/plan` disappears when already in plan mode, `/gate` shows contextual gates based on the project type (cargo for Rust, forge for Solidity, etc.), and `/heuristics` adjusts based on what files are open.

### 2.4 Agent Plans

ACP has first-class support for rendering multi-step plans with status updates. This is where Roko shines compared to single-loop agents. When Roko creates a plan, the editor renders it as a visual checklist with live status:

```json
{
  "sessionUpdate": "plan",
  "entries": [
    {
      "content": "Analyze existing ERC-4626 vault implementation",
      "priority": "high",
      "status": "completed"
    },
    {
      "content": "Implement share price monotonicity invariant",
      "priority": "high",
      "status": "in_progress"
    },
    {
      "content": "Gate: compile check",
      "priority": "high",
      "status": "pending"
    },
    {
      "content": "Gate: run test suite (147 tests)",
      "priority": "high",
      "status": "pending"
    },
    {
      "content": "Gate: clippy lint pass",
      "priority": "medium",
      "status": "pending"
    },
    {
      "content": "Review: verify no rounding exploits",
      "priority": "medium",
      "status": "pending"
    }
  ]
}
```

As Roko progresses through its `PlanPhase` transitions (Enriching → Implementing → Gating → Verifying → Reviewing → Merging), it sends updated plan notifications with status changes. The user sees each step go from "pending" to "in_progress" to "completed" in real time. When a gate fails, the step shows "pending" again (retry) and a new corrective step appears ("Fix: resolve 3 failing tests").

This is the kind of visibility that no other ACP agent provides. Claude Code, Gemini CLI, and Codex are all opaque — you see tokens streaming, maybe some tool calls, and then a result. Roko shows the full cognitive pipeline.

### 2.5 Tool Calls — Gate Results as Structured Output

When Roko runs its gate pipeline, each gate produces a tool call update with structured results:

```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "gate_compile_001",
  "title": "Compile Gate",
  "kind": "other",
  "status": "completed",
  "content": [
    {
      "type": "text",
      "text": "## Compile Gate: ✓ PASSED\n\n- **Target**: `roko-orchestrator`\n- **Time**: 4.2s\n- **Warnings**: 0\n- **Errors**: 0"
    }
  ]
}
```

```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "gate_test_001",
  "title": "Test Gate",
  "kind": "other",
  "status": "completed",
  "content": [
    {
      "type": "text",
      "text": "## Test Gate: ✓ PASSED\n\n- **Passed**: 147/147\n- **Failed**: 0\n- **Ignored**: 3\n- **Time**: 12.8s\n\n### Coverage\n- Line: 82.4%\n- Branch: 71.2%"
    }
  ]
}
```

The editor renders these as collapsible tool call cards in the chat panel. The user sees objective verification results — not LLM self-assessment, but compiler and test harness output.

### 2.6 File System Bridge

When the editor declares `fs.readTextFile` and `fs.writeTextFile` capabilities during initialization, Roko uses the editor's file system instead of direct disk access. This is critical for two reasons: the editor mediates all file access (security), and file writes show as inline diffs that the user can accept or reject per-hunk (UX).

Roko's `FileSubstrate` trait (L0) already abstracts file I/O. The ACP bridge implements this trait by sending JSON-RPC requests back to the editor:

```
Roko agent needs to read src/main.rs
  → Roko sends fs/read_text_file { path: "/abs/path/to/src/main.rs" }
  → Editor reads the file and returns contents
  → Roko receives file content, proceeds with analysis

Roko agent writes modified src/main.rs  
  → Roko sends fs/write_text_file { path: "/abs/path/to/src/main.rs", content: "..." }
  → Editor shows inline diff: green for additions, red for deletions
  → User reviews the diff, accepts or rejects individual hunks
  → Editor confirms write success back to Roko
```

### 2.7 Terminal Bridge

When the editor declares `terminal` capability, Roko runs shell commands through the editor's terminal instead of spawning processes directly. This means:

- `cargo test` output appears in the editor's terminal panel, not in a hidden subprocess
- The user can see exactly what commands the agent is running
- The editor can kill long-running commands via its own UI
- Terminal output is captured and returned to Roko for analysis

```
Roko needs to run tests:
  → Roko sends terminal/create { command: "cargo", args: ["test", "--lib"], cwd: "/project" }
  → Editor opens terminal tab, runs command, streams output
  → Roko sends terminal/output { terminalId: "term_001" }
  → Editor returns stdout/stderr and exit code
  → Roko analyzes test results, proceeds with gate evaluation
```

### 2.8 Permission Requests

ACP's `session/request_permission` maps to Roko's `SafetyLayer`. Before executing potentially destructive operations (writing files, running commands, modifying git state), Roko requests permission:

```json
{
  "method": "session/request_permission",
  "params": {
    "sessionId": "sess_001",
    "toolCall": {
      "toolCallId": "write_main_rs",
      "title": "Modify src/main.rs",
      "kind": "edit",
      "status": "pending",
      "content": [
        {
          "type": "diff",
          "path": "src/main.rs",
          "diff": "--- a/src/main.rs\n+++ b/src/main.rs\n@@ -42,7 +42,12 @@\n..."
        }
      ]
    },
    "options": [
      { "optionId": "allow", "name": "Allow", "kind": "allow_once" },
      { "optionId": "allow_all", "name": "Allow all edits this turn", "kind": "allow_always" },
      { "optionId": "reject", "name": "Reject", "kind": "reject_once" }
    ]
  }
}
```

The editor shows a native dialog with the diff preview and the three options. This is how Roko's gate-verified workflow looks in practice: the agent proposes a change, the gate pipeline verifies it, and then the user gets a one-click approval with full confidence that the code compiles and tests pass.

---

## 3. Architecture

### 3.1 New Crate: `roko-acp`

```
crates/roko-acp/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Public API: run_acp_server()
│   ├── transport.rs         # Stdio JSON-RPC transport (read/write/notify)
│   ├── handler.rs           # Method dispatch: initialize, session/*, etc.
│   ├── session.rs           # ACP session ↔ Roko Episode + KnowledgeStore
│   ├── bridge_fs.rs         # AcpFileSubstrate: impl Substrate via fs/* callbacks
│   ├── bridge_terminal.rs   # AcpTerminal: impl ProcessSupervisor via terminal/* callbacks
│   ├── bridge_events.rs     # Bus Pulse → session/update notification mapper
│   ├── bridge_plan.rs       # PlanPhase → ACP plan entries mapper
│   ├── bridge_gates.rs      # GateResult → ACP tool_call_update mapper
│   ├── modes.rs             # Roko routing tiers ↔ ACP session modes
│   ├── commands.rs          # Slash command definitions and dynamic updates
│   ├── permissions.rs       # SafetyLayer ↔ session/request_permission bridge
│   └── config.rs            # ACP-specific configuration
└── tests/
    ├── protocol_conformance.rs  # Validate against ACP spec
    ├── lifecycle.rs             # init → session → prompt → cancel flows
    └── bridge_integration.rs    # End-to-end with mock editor
```

### 3.2 Dependency Graph

```
roko-acp
├── agent-client-protocol   # Official Rust SDK from crates.io (types + transport)
├── roko-core                # L1: Engram, AgentRole, PlanPhase, Verdict, Budget
├── roko-compose             # L2: PromptComposer, RoleSystemPromptSpec
├── roko-gate                # L2: CompileGate, TestGate, ClippyGate
├── roko-agent               # L3: ClaudeCliAgent, ExecAgent, McpConfig
├── roko-orchestrator        # L4: PlanRunner, OrchestratorState, DaimonState
├── roko-fs                  # L0: FileSubstrate trait, RokoLayout
├── bardo-runtime            # L0: CancelToken, EventBus, ProcessSupervisor
├── tokio                    # Async runtime (already in workspace)
├── serde / serde_json       # Serialization (already in workspace)
└── tracing                  # Observability (already in workspace)
```

### 3.3 Entry Point: `roko acp`

New subcommand in `roko-cli`:

```rust
// crates/roko-cli/src/main.rs
#[derive(Subcommand)]
enum Commands {
    // ... existing commands
    
    /// Start Roko as an ACP agent server (stdio JSON-RPC)
    Acp {
        /// Working directory (defaults to cwd)
        #[arg(long)]
        workdir: Option<PathBuf>,
        
        /// Agent profile to use (defaults to "default")
        #[arg(long, default_value = "default")]
        profile: String,
        
        /// Config file path
        #[arg(long)]
        config: Option<PathBuf>,
        
        /// Log file path (stderr is protocol, must redirect logs)
        #[arg(long, default_value = "/tmp/roko-acp.log")]
        log_file: PathBuf,
    },
}
```

Critical implementation detail: **all logging must go to stderr or a file, never stdout**. Stdout is the JSON-RPC transport — any non-JSON output on stdout corrupts the protocol stream. The `--log-file` flag redirects tracing output.

### 3.4 Layer Placement

`roko-acp` sits alongside `roko-cli` as a presentation layer — it is NOT a new architectural layer. It is an alternative "harness" for the same L4 orchestrator that the CLI and TUI use:

```
┌──────────────────────────────────────────────────────┐
│                  Presentation Layer                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────┐ │
│  │ roko-cli │  │ roko-tui │  │ roko-acp │  │Nunchi│ │
│  │ (term)   │  │ (ratatui)│  │ (stdio)  │  │(web) │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──┬───┘ │
│       │             │             │            │      │
│  ┌────▼─────────────▼─────────────▼────────────▼───┐ │
│  │           L4: roko-orchestrator                  │ │
│  │    PlanRunner + OrchestratorState + DaimonState   │ │
│  └──────────────────┬──────────────────────────────┘ │
│                     │                                 │
│  ┌──────────────────▼──────────────────────────────┐ │
│  │  L3: roko-agent (ClaudeCliAgent, ExecAgent)     │ │
│  │  L2: roko-compose + roko-gate                    │ │
│  │  L1: roko-core                                   │ │
│  │  L0: roko-fs + bardo-runtime                     │ │
│  └─────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────┘
```

The key architectural insight: `roko-acp` replaces the I/O substrate (file system and terminal) with editor-mediated versions, but the entire cognitive pipeline — prompt composition, model routing, gate verification, knowledge store, Daimon state — runs identically to the CLI. Same code, same quality, different I/O surface.

---

## 4. Implementation Details

### 4.1 Transport Layer (`transport.rs`)

The transport handles raw JSON-RPC 2.0 over stdin/stdout:

```rust
use agent_client_protocol::{JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcNotification};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct StdioTransport {
    reader: BufReader<tokio::io::Stdin>,
    writer: tokio::io::Stdout,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
        }
    }
    
    /// Read the next JSON-RPC message from stdin.
    /// Blocks until a complete line is available.
    pub async fn read_message(&mut self) -> Result<JsonRpcMessage> {
        let mut line = String::new();
        self.reader.read_line(&mut line).await?;
        if line.is_empty() {
            return Err(AcpError::ConnectionClosed);
        }
        let msg: JsonRpcMessage = serde_json::from_str(line.trim())?;
        Ok(msg)
    }
    
    /// Send a JSON-RPC response to stdout.
    pub async fn send_response(&mut self, id: serde_json::Value, result: impl Serialize) -> Result<()> {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(serde_json::to_value(result)?),
            error: None,
        };
        let json = serde_json::to_string(&response)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
    
    /// Send a JSON-RPC notification to stdout (no response expected).
    pub async fn send_notification(&mut self, method: &str, params: impl Serialize) -> Result<()> {
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params: Some(serde_json::to_value(params)?),
        };
        let json = serde_json::to_string(&notification)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }
    
    /// Send a JSON-RPC request to the CLIENT (bidirectional).
    /// Used for fs/*, terminal/*, session/request_permission.
    /// Returns the client's response.
    pub async fn send_request<R: DeserializeOwned>(
        &mut self,
        id: u64,
        method: &str,
        params: impl Serialize,
    ) -> Result<R> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: serde_json::Value::Number(id.into()),
            method: method.into(),
            params: Some(serde_json::to_value(params)?),
        };
        let json = serde_json::to_string(&request)?;
        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        
        // Wait for the response with matching id.
        // In practice, use a pending-requests map + channel.
        self.wait_for_response(id).await
    }
}
```

### 4.2 Handler (`handler.rs`)

The main event loop dispatches incoming methods:

```rust
pub async fn run_acp_server(config: AcpConfig) -> Result<()> {
    // Redirect all logging to file — stdout is the protocol channel
    let _guard = init_file_logging(&config.log_file)?;
    
    let mut transport = StdioTransport::new();
    let mut sessions: HashMap<String, AcpSession> = HashMap::new();
    let mut request_counter: u64 = 1000; // For outbound request IDs
    
    loop {
        let message = transport.read_message().await?;
        
        match message {
            JsonRpcMessage::Request(req) => {
                let result = match req.method.as_str() {
                    "initialize" => {
                        handle_initialize(&config, &req).await
                    }
                    "authenticate" => {
                        handle_authenticate(&config, &req).await
                    }
                    "session/new" => {
                        handle_session_new(&config, &mut sessions, &req).await
                    }
                    "session/load" => {
                        handle_session_load(&config, &mut sessions, &req).await
                    }
                    "session/prompt" => {
                        handle_session_prompt(
                            &config, &mut sessions, &mut transport, 
                            &mut request_counter, &req
                        ).await
                    }
                    "session/set_mode" => {
                        handle_set_mode(&mut sessions, &req).await
                    }
                    "session/list" => {
                        handle_session_list(&sessions, &req).await
                    }
                    _ => Err(AcpError::MethodNotFound(req.method.clone())),
                };
                
                match result {
                    Ok(value) => transport.send_response(req.id, value).await?,
                    Err(e) => transport.send_error(req.id, e).await?,
                }
            }
            JsonRpcMessage::Notification(notif) => {
                match notif.method.as_str() {
                    "session/cancel" => {
                        handle_session_cancel(&mut sessions, &notif).await?;
                    }
                    _ => {} // Ignore unknown notifications
                }
            }
            JsonRpcMessage::Response(resp) => {
                // Response to an outbound request (fs/*, terminal/*, permission)
                // Route to the pending request handler
                transport.resolve_pending(resp).await?;
            }
        }
    }
}
```

### 4.3 Initialization (`handler.rs`)

```rust
async fn handle_initialize(
    config: &AcpConfig,
    req: &JsonRpcRequest,
) -> Result<serde_json::Value> {
    let params: InitializeParams = serde_json::from_value(req.params.clone().unwrap_or_default())?;
    
    // Store client capabilities for later use
    let client_caps = params.client_capabilities;
    
    Ok(serde_json::to_value(InitializeResult {
        protocol_version: 1,
        agent_capabilities: AgentCapabilities {
            load_session: true,
            prompt_capabilities: PromptCapabilities {
                image: false,     // Enable when vision model backends are wired
                audio: false,
                embedded_context: true,
            },
            mcp_capabilities: McpCapabilities {
                http: true,       // Roko can connect to HTTP MCP servers
                sse: false,       // SSE transport deprecated in MCP spec
            },
        },
        agent_info: AgentInfo {
            name: "roko".into(),
            title: "Roko".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        auth_methods: vec![], // No auth for local stdio
    })?)
}
```

### 4.4 Session Creation (`session.rs`)

```rust
pub struct AcpSession {
    pub session_id: String,
    pub episode: Episode,
    pub knowledge_store: KnowledgeStore,
    pub daimon: DaimonState,
    pub plan_runner: PlanRunner,
    pub current_mode: SessionMode,
    pub cancel_token: CancelToken,
    pub client_caps: ClientCapabilities,
}

async fn handle_session_new(
    config: &AcpConfig,
    sessions: &mut HashMap<String, AcpSession>,
    req: &JsonRpcRequest,
) -> Result<serde_json::Value> {
    let params: SessionNewParams = parse_params(req)?;
    let session_id = format!("sess_{}", uuid::Uuid::new_v4().simple());
    
    // Initialize Roko subsystems for this session
    let knowledge_store = KnowledgeStore::open_or_create(&config.workdir)?;
    let daimon = DaimonState::default();
    let episode = Episode::new(&session_id);
    let plan_runner = PlanRunner::new(
        config.roko_config.clone(),
        knowledge_store.clone(),
        daimon.clone(),
    );
    
    let session = AcpSession {
        session_id: session_id.clone(),
        episode,
        knowledge_store,
        daimon,
        plan_runner,
        current_mode: SessionMode::code(),
        cancel_token: CancelToken::new(),
        client_caps: params.client_capabilities.unwrap_or_default(),
    };
    
    sessions.insert(session_id.clone(), session);
    
    Ok(serde_json::to_value(SessionNewResult {
        session_id,
        modes: Some(SessionModeState {
            current_mode_id: "code".into(),
            available_modes: roko_modes(),
        }),
    })?)
}
```

### 4.5 Prompt Handling — The Core Bridge (`handler.rs`)

This is the most important function — it connects the ACP prompt lifecycle to Roko's cognitive loop:

```rust
async fn handle_session_prompt(
    config: &AcpConfig,
    sessions: &mut HashMap<String, AcpSession>,
    transport: &mut StdioTransport,
    request_counter: &mut u64,
    req: &JsonRpcRequest,
) -> Result<serde_json::Value> {
    let params: SessionPromptParams = parse_params(req)?;
    let session = sessions.get_mut(&params.session_id)
        .ok_or(AcpError::SessionNotFound)?;
    
    // Extract user prompt from ACP content blocks
    let user_prompt = extract_prompt_text(&params.prompt);
    let attached_files = extract_resources(&params.prompt);
    
    // Check for slash commands
    if let Some(command) = parse_slash_command(&user_prompt) {
        return handle_slash_command(session, transport, request_counter, command).await;
    }
    
    // Send initial plan notification based on current mode
    let initial_plan = session.current_mode.initial_plan(&user_prompt);
    if !initial_plan.is_empty() {
        transport.send_notification("session/update", SessionUpdate {
            session_id: params.session_id.clone(),
            update: UpdateKind::Plan { entries: initial_plan },
        }).await?;
    }
    
    // Advertise dynamic slash commands based on context
    let commands = dynamic_commands(&session.current_mode, &user_prompt);
    transport.send_notification("session/update", SessionUpdate {
        session_id: params.session_id.clone(),
        update: UpdateKind::AvailableCommandsUpdate {
            available_commands: commands,
        },
    }).await?;
    
    // Build I/O bridges
    let fs_bridge = if session.client_caps.has_fs() {
        Box::new(AcpFileSubstrate::new(transport.clone(), request_counter))
            as Box<dyn Substrate>
    } else {
        Box::new(FileSubstrate::new(&config.workdir)) as Box<dyn Substrate>
    };
    
    let terminal_bridge = if session.client_caps.has_terminal() {
        Box::new(AcpTerminal::new(transport.clone(), request_counter))
    } else {
        Box::new(LocalTerminal::new())
    };
    
    let permission_bridge = AcpPermissionGate::new(
        transport.clone(), request_counter, params.session_id.clone()
    );
    
    // Create event channel for streaming updates back to editor
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel::<CognitiveEvent>(256);
    
    // Spawn the cognitive loop in a background task
    let cancel = session.cancel_token.clone();
    let run_config = build_run_config(
        &session, &user_prompt, &attached_files,
        fs_bridge, terminal_bridge, permission_bridge, event_tx,
    );
    
    let cognitive_task = tokio::spawn(async move {
        roko_orchestrator::run_cognitive_loop(run_config, cancel).await
    });
    
    // Stream events from the cognitive loop as ACP notifications
    let stop_reason = stream_events_to_editor(
        &params.session_id, transport, &mut event_rx, &session.cancel_token
    ).await?;
    
    // Wait for the cognitive task to complete
    let outcome = cognitive_task.await??;
    
    // Persist knowledge learned during this turn
    session.knowledge_store.flush()?;
    session.episode.record_turn(&user_prompt, &outcome);
    
    Ok(serde_json::to_value(SessionPromptResult {
        stop_reason: match stop_reason {
            StopReason::EndTurn => "end_turn",
            StopReason::MaxTokens => "max_tokens",
            StopReason::Cancelled => "cancelled",
            StopReason::Refused => "refusal",
        },
    })?)
}

/// Stream CognitiveEvents as ACP session/update notifications.
/// Returns when the cognitive loop completes or is cancelled.
async fn stream_events_to_editor(
    session_id: &str,
    transport: &mut StdioTransport,
    event_rx: &mut mpsc::Receiver<CognitiveEvent>,
    cancel: &CancelToken,
) -> Result<StopReason> {
    while let Some(event) = event_rx.recv().await {
        if cancel.is_cancelled() {
            return Ok(StopReason::Cancelled);
        }
        
        match event {
            // --- Streaming text from LLM ---
            CognitiveEvent::TokenChunk(text) => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::AgentMessageChunk {
                        content: ContentBlock::Text { text },
                    },
                }).await?;
            }
            
            // --- Thinking/reasoning tokens (shown as thought bubbles) ---
            CognitiveEvent::ThinkingChunk(text) => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ThoughtMessageChunk {
                        content: ContentBlock::Text { text },
                    },
                }).await?;
            }
            
            // --- Tool call lifecycle ---
            CognitiveEvent::ToolCallStart { id, name, args } => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ToolCall {
                        tool_call_id: id,
                        title: human_readable_tool_name(&name),
                        kind: tool_kind(&name),
                        status: "pending".into(),
                        content: None,
                    },
                }).await?;
            }
            
            CognitiveEvent::ToolCallComplete { id, result } => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ToolCallUpdate {
                        tool_call_id: id,
                        status: "completed".into(),
                        content: Some(vec![ContentBlock::Text {
                            text: result,
                        }]),
                    },
                }).await?;
            }
            
            // --- Gate pipeline results ---
            CognitiveEvent::GateStarted(gate) => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ToolCall {
                        tool_call_id: format!("gate_{}", gate.name),
                        title: format!("{} Gate", gate.name),
                        kind: "other".into(),
                        status: "in_progress".into(),
                        content: None,
                    },
                }).await?;
            }
            
            CognitiveEvent::GateCompleted(gate) => {
                let status_emoji = if gate.passed { "✓" } else { "✗" };
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::ToolCallUpdate {
                        tool_call_id: format!("gate_{}", gate.name),
                        status: "completed".into(),
                        content: Some(vec![ContentBlock::Text {
                            text: format!(
                                "## {} Gate: {} {}\n\n{}\n\n**Time**: {:.1}s",
                                gate.name,
                                status_emoji,
                                if gate.passed { "PASSED" } else { "FAILED" },
                                gate.summary,
                                gate.duration.as_secs_f64(),
                            ),
                        }]),
                    },
                }).await?;
            }
            
            // --- Plan phase transitions ---
            CognitiveEvent::PhaseTransition(phase) => {
                let plan = build_plan_from_phase(&phase);
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::Plan { entries: plan },
                }).await?;
            }
            
            // --- Conductor watcher triggers ---
            CognitiveEvent::WatcherTriggered { watcher, action } => {
                transport.send_notification("session/update", SessionUpdate {
                    session_id: session_id.into(),
                    update: UpdateKind::AgentMessageChunk {
                        content: ContentBlock::Text {
                            text: format!(
                                "\n> ⚡ **{}** triggered: {}\n",
                                watcher, action
                            ),
                        },
                    },
                }).await?;
            }
            
            // --- Session complete ---
            CognitiveEvent::Complete(outcome) => {
                return Ok(StopReason::EndTurn);
            }
            
            CognitiveEvent::MaxTokens => {
                return Ok(StopReason::MaxTokens);
            }
        }
    }
    
    Ok(StopReason::EndTurn)
}
```

### 4.6 File System Bridge (`bridge_fs.rs`)

```rust
/// ACP-mediated file system that routes through the editor.
/// Implements the same Substrate trait as FileSubstrate,
/// so the cognitive loop is unaware of the transport difference.
pub struct AcpFileSubstrate {
    transport: StdioTransport,
    request_counter: Arc<AtomicU64>,
}

impl Substrate for AcpFileSubstrate {
    async fn read_file(&self, path: &Path) -> Result<String> {
        let id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let result: ReadTextFileResult = self.transport.send_request(
            id,
            "fs/read_text_file",
            ReadTextFileParams {
                path: path.to_string_lossy().to_string(),
            },
        ).await?;
        Ok(result.text)
    }
    
    async fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        let id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let _result: WriteTextFileResult = self.transport.send_request(
            id,
            "fs/write_text_file",
            WriteTextFileParams {
                path: path.to_string_lossy().to_string(),
                content: content.to_string(),
            },
        ).await?;
        Ok(())
    }
    
    async fn list_files(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        // ACP doesn't have a list_files method — fall back to local FS
        // This is fine because listing is read-only and non-destructive
        tokio::fs::read_dir(dir).await?
            .map(|entry| entry.map(|e| e.path()))
            .collect()
    }
}
```

### 4.7 Terminal Bridge (`bridge_terminal.rs`)

```rust
pub struct AcpTerminal {
    transport: StdioTransport,
    request_counter: Arc<AtomicU64>,
    active_terminals: HashMap<String, TerminalHandle>,
}

impl AcpTerminal {
    pub async fn run_command(
        &mut self,
        command: &str,
        args: &[&str],
        cwd: &Path,
    ) -> Result<CommandOutput> {
        let id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        
        // Create terminal in editor
        let create_result: TerminalCreateResult = self.transport.send_request(
            id,
            "terminal/create",
            TerminalCreateParams {
                command: command.to_string(),
                args: args.iter().map(|s| s.to_string()).collect(),
                cwd: Some(cwd.to_string_lossy().to_string()),
                env: None,
            },
        ).await?;
        
        let terminal_id = create_result.terminal_id;
        
        // Wait for command to exit
        let wait_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let _wait_result: TerminalWaitResult = self.transport.send_request(
            wait_id,
            "terminal/wait_for_exit",
            TerminalWaitParams {
                terminal_id: terminal_id.clone(),
            },
        ).await?;
        
        // Get output
        let output_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        let output_result: TerminalOutputResult = self.transport.send_request(
            output_id,
            "terminal/output",
            TerminalOutputParams {
                terminal_id: terminal_id.clone(),
            },
        ).await?;
        
        // Release terminal
        let release_id = self.request_counter.fetch_add(1, Ordering::SeqCst);
        self.transport.send_request::<()>(
            release_id,
            "terminal/release",
            TerminalReleaseParams { terminal_id },
        ).await.ok(); // Best-effort release
        
        Ok(CommandOutput {
            stdout: output_result.stdout,
            stderr: output_result.stderr,
            exit_code: output_result.exit_code,
        })
    }
}
```

### 4.8 MCP Passthrough

When the editor has MCP servers configured, it passes their configuration to the agent during session setup. Roko connects to these MCP servers directly — the editor provides the connection info, and Roko's existing `roko-agent::mcp::McpConfig` handles the connection:

```rust
async fn handle_session_new(/* ... */) -> Result<serde_json::Value> {
    let params: SessionNewParams = parse_params(req)?;
    
    // If the editor provides MCP server configs, merge them with Roko's own
    if let Some(mcp_configs) = params.mcp_servers {
        for mcp in mcp_configs {
            session.plan_runner.add_mcp_server(McpConfig {
                name: mcp.name,
                transport: match mcp.transport {
                    McpTransport::Stdio { command, args } => {
                        McpTransportConfig::Stdio { command, args }
                    }
                    McpTransport::Http { url } => {
                        McpTransportConfig::Http { url }
                    }
                },
            });
        }
    }
    
    // ... rest of session creation
}
```

This means if a user has a GitHub MCP server, a database MCP server, or any other tools configured in their Zed/JetBrains setup, Roko automatically gets access to them.

---

## 5. Editor-Specific Integration Details

### 5.1 JetBrains — Registry Installation

Once implemented, Roko is submitted to the ACP Registry. Users install it from inside their IDE:

1. Open the AI Chat tool window
2. Click the agent selector dropdown
3. Select "Install From ACP Registry"
4. Find "Roko" → click Install
5. The IDE downloads the `roko` binary, configures `acp.json`, and Roko appears in the agent selector

Alternatively, manual configuration via `~/.jetbrains/acp.json`:

```json
{
  "default_mcp_settings": {
    "use_custom_mcp": true,
    "use_idea_mcp": true
  },
  "agent_servers": {
    "Roko": {
      "command": "roko",
      "args": ["acp"],
      "env": {
        "ROKO_LOG_LEVEL": "info"
      }
    }
  }
}
```

With `use_idea_mcp: true`, Roko gets access to IntelliJ's built-in MCP server — which provides refactoring tools, code analysis, test running, and other IDE-native capabilities. This means Roko can use IntelliJ's own refactoring engine in addition to its own gate pipeline.

### 5.2 Zed — Native Integration

Zed settings (`~/.config/zed/settings.json`):

```json
{
  "agent_servers": {
    "Roko": {
      "type": "custom",
      "command": "roko",
      "args": ["acp"],
      "env": {}
    }
  }
}
```

Or use a keyboard shortcut to open a new Roko thread:

```json
[
  {
    "bindings": {
      "cmd-alt-r": [
        "agent::NewExternalAgentThread",
        {
          "agent": {
            "custom": {
              "name": "Roko",
              "command": {
                "command": "roko",
                "args": ["acp"]
              }
            }
          }
        }
      ]
    }
  }
]
```

### 5.3 Neovim — Via avante.nvim

```lua
-- In your init.lua or plugin config
require('avante').setup({
  acp_providers = {
    ["roko"] = {
      command = "roko",
      args = { "acp" },
      env = {}
    }
  }
})
```

### 5.4 Obsidian — Via obsidian-agent-client

```json
{
  "agents": {
    "Roko": {
      "command": "roko",
      "args": ["acp", "--workdir", "/path/to/vault"],
      "env": {}
    }
  }
}
```

This creates a loop between Obsidian (human knowledge surface) and Roko's knowledge store (agent knowledge surface). Research notes in Obsidian can inform Roko's context; Roko's learned heuristics can be surfaced as notes in Obsidian.

---

## 6. Roko-Specific ACP Extensions

ACP supports custom extensions via the `_meta` field and underscore-prefixed methods. Roko uses these to expose capabilities that the base protocol doesn't cover:

### 6.1 Knowledge Store Queries (Extension)

```json
{
  "method": "session/update",
  "params": {
    "sessionId": "sess_001",
    "update": {
      "sessionUpdate": "agent_message_chunk",
      "content": {
        "type": "text",
        "text": "Found 3 relevant knowledge entries..."
      },
      "_meta": {
        "_roko.knowledge_entries": [
          {
            "id": "engram_0x3f2a",
            "type": "Heuristic",
            "content": "ERC-4626 vaults must ensure share price monotonicity",
            "confidence": 0.91,
            "confirmations": 89
          }
        ]
      }
    }
  }
}
```

Editors that don't understand Roko's extensions safely ignore the `_meta` field. Editors that do (Nunchi, or a future Roko Zed extension) can render knowledge entries as rich cards.

### 6.2 Gate Pipeline Status (Extension)

```json
{
  "_meta": {
    "_roko.gate_pipeline": {
      "gates": [
        { "name": "compile", "status": "passed", "duration_ms": 4200 },
        { "name": "test", "status": "passed", "tests_passed": 147, "tests_total": 147 },
        { "name": "clippy", "status": "passed", "warnings": 0 }
      ],
      "overall": "passed"
    }
  }
}
```

### 6.3 Daimon State (Extension)

```json
{
  "_meta": {
    "_roko.daimon": {
      "pleasure": 0.72,
      "arousal": 0.45,
      "dominance": 0.81,
      "mood": "confident",
      "strategy_shifts": 0
    }
  }
}
```

---

## 7. Testing Strategy

### 7.1 Protocol Conformance Tests

Test against the ACP spec using the official Rust SDK's test utilities:

```rust
#[tokio::test]
async fn test_initialize_handshake() {
    let (mut client, mut server) = create_test_pair().await;
    
    let response = client.send_initialize(InitializeParams {
        protocol_version: 1,
        client_capabilities: ClientCapabilities {
            fs: Some(FsCapabilities { read_text_file: true, write_text_file: true }),
            terminal: Some(true),
        },
        client_info: Some(ClientInfo {
            name: "test-client".into(),
            version: "1.0.0".into(),
            title: None,
        }),
    }).await.unwrap();
    
    assert_eq!(response.protocol_version, 1);
    assert!(response.agent_capabilities.load_session);
    assert_eq!(response.agent_info.unwrap().name, "roko");
}

#[tokio::test]
async fn test_prompt_streams_gate_results() {
    let (mut client, mut server) = create_test_pair().await;
    initialize_and_create_session(&mut client).await;
    
    // Send a prompt that will trigger the gate pipeline
    let updates = client.send_prompt_and_collect_updates(
        "Fix the failing test in src/lib.rs",
    ).await.unwrap();
    
    // Verify gate results appear as tool call updates
    let gate_updates: Vec<_> = updates.iter()
        .filter(|u| matches!(u, UpdateKind::ToolCall { title, .. } if title.contains("Gate")))
        .collect();
    
    assert!(!gate_updates.is_empty(), "Gate results should be reported as tool calls");
}
```

### 7.2 Editor Integration Tests

Run Roko as a subprocess and communicate via stdio, simulating what each editor does:

```rust
#[tokio::test]
async fn test_end_to_end_stdio() {
    let mut child = Command::new("cargo")
        .args(["run", "--bin", "roko", "--", "acp", "--workdir", "/tmp/test-project"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut transport = TestTransport::new(stdin, stdout);
    
    // Full lifecycle test
    transport.send_initialize().await;
    let session_id = transport.send_session_new().await;
    let updates = transport.send_prompt(&session_id, "Hello, what can you do?").await;
    
    assert!(updates.iter().any(|u| matches!(u, UpdateKind::AgentMessageChunk { .. })));
    
    child.kill().await.unwrap();
}
```

### 7.3 Bridge Tests

Test the file system and terminal bridges independently:

```rust
#[tokio::test]
async fn test_fs_bridge_read_write() {
    let (mut client, mut server) = create_test_pair().await;
    let fs = AcpFileSubstrate::new(server.transport.clone(), Arc::new(AtomicU64::new(0)));
    
    // Mock the client responding to fs/read_text_file
    client.on_request("fs/read_text_file", |params| {
        Ok(ReadTextFileResult { text: "fn main() {}".into() })
    });
    
    let content = fs.read_file(Path::new("/test/main.rs")).await.unwrap();
    assert_eq!(content, "fn main() {}");
}
```

---

## 8. ACP Registry Submission

### 8.1 Registry Manifest

The ACP Registry requires a manifest describing the agent:

```json
{
  "name": "roko",
  "title": "Roko",
  "description": "Multi-agent coding runtime with gate-verified execution, knowledge store, and multi-phase planning",
  "version": "0.1.0",
  "homepage": "https://github.com/roko-project/roko",
  "license": "MIT",
  "platforms": {
    "darwin-arm64": {
      "command": "roko",
      "args": ["acp"]
    },
    "darwin-x64": {
      "command": "roko",
      "args": ["acp"]
    },
    "linux-x64": {
      "command": "roko",
      "args": ["acp"]
    },
    "win32-x64": {
      "command": "roko.exe",
      "args": ["acp"]
    }
  },
  "capabilities": {
    "loadSession": true,
    "promptCapabilities": {
      "embeddedContext": true
    }
  }
}
```

### 8.2 Distribution

The registry downloads the agent binary. Two options:

- **Cargo install**: Users need Rust toolchain. `cargo install roko`
- **Pre-built binaries**: GitHub releases with binaries for macOS (arm64/x64), Linux (x64), Windows (x64). The registry manifest points to these.

Pre-built binaries are strongly preferred for the registry — zero-friction installation is the point.

---

## 9. Implementation Roadmap

### Phase 1: Core Protocol (Week 1–2)

**Goal**: `roko acp` compiles, initializes, creates sessions, handles prompts with streaming.

- Create `roko-acp` crate with transport, handler, session management
- Implement `initialize`, `session/new`, `session/prompt`, `session/cancel`
- Implement `stream_events_to_editor` with `agent_message_chunk` and `thought_message_chunk`
- Wire `session/prompt` to `roko_orchestrator::run_cognitive_loop` with local file system (no bridges yet)
- Redirect all logging to stderr/file
- Protocol conformance tests for lifecycle

**Acceptance criteria**: Run `roko acp`, connect with a test harness, send a prompt, receive streamed response.

### Phase 2: Bridges (Week 3–4)

**Goal**: Editor-mediated file I/O, terminal, and permissions.

- Implement `AcpFileSubstrate` (bridge_fs.rs) — `fs/read_text_file`, `fs/write_text_file`
- Implement `AcpTerminal` (bridge_terminal.rs) — `terminal/create`, `terminal/output`, `terminal/wait_for_exit`, `terminal/kill`, `terminal/release`
- Implement `AcpPermissionGate` (permissions.rs) — `session/request_permission`
- Wire gate pipeline results as `tool_call` / `tool_call_update` notifications
- Wire plan phase transitions as `plan` notifications
- MCP passthrough from editor MCP configs to `roko-agent::McpConfig`

**Acceptance criteria**: Connect from Zed or JetBrains. File edits show as diffs. Terminal commands run in editor terminal. Gate results display as tool cards. Plans show as step-by-step checklists.

### Phase 3: Modes, Commands, Sessions (Week 5–6)

**Goal**: Full Roko-specific UX through ACP's mode/command/session features.

- Implement session modes (code/plan/research/review/auto) with mode switching
- Implement slash commands (/plan, /gate, /learn, /inspect, /replay, /heuristics, /status, /budget)
- Dynamic command updates based on context
- `session/load` with knowledge store and Daimon state persistence
- `session/list` for session history
- Roko-specific `_meta` extensions for knowledge entries, gate pipeline, Daimon state

**Acceptance criteria**: User can switch modes in editor UI. Slash commands autocomplete. Sessions persist and reload with learned knowledge.

### Phase 4: Registry & Distribution (Week 7–8)

**Goal**: One-click installation from JetBrains/Zed.

- Set up cross-compilation for macOS/Linux/Windows binaries
- Create GitHub Actions workflow for release binaries
- Write ACP registry manifest
- Submit to ACP registry
- Write user-facing documentation (README, getting started guide)
- End-to-end testing with JetBrains 2025.3+, Zed, Neovim (avante.nvim)
- Performance profiling (startup time, response latency, memory usage)

**Acceptance criteria**: User installs Roko from JetBrains ACP Registry dropdown. Works out of the box.

---

## 10. Risks & Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| ACP pre-1.0 breaking changes | Protocol methods change, types restructure | Pin `agent-client-protocol` crate version. Abstract all ACP types behind internal trait boundary. Monitor ACP changelog. |
| stdout pollution from dependencies | Any non-JSON output on stdout corrupts the protocol stream | All logging to file via `--log-file`. Set `tracing` subscriber to file/stderr only. Test with `RUST_LOG=off` as default in ACP mode. |
| Bidirectional JSON-RPC complexity | Deadlocks if agent and editor both waiting for responses simultaneously | Use a pending-requests map with `oneshot` channels. Separate reader and writer tasks. Timeout on all outbound requests (30s default). |
| Startup time for Roko binary | If `roko acp` takes >2s to start, editors feel sluggish | Lazy-load heavy subsystems (knowledge store, model backends). Only initialize when first `session/new` arrives. Profile and optimize startup path. |
| Memory usage per session | Knowledge store + Daimon state per session adds up with concurrent sessions | Session limit (default: 4 concurrent). LRU eviction for idle sessions. Share knowledge store read handle across sessions. |
| Editor-specific quirks | JetBrains and Zed may interpret ACP slightly differently | Test against both. Join ACP community Discord for spec clarifications. File issues on `agentclientprotocol/agent-client-protocol` for ambiguities. |

---

## 11. What This Unlocks

### 11.1 Immediate

- Roko agents usable from JetBrains, Zed, Neovim, Emacs, Obsidian, Toad — with zero per-editor integration work
- One-click install from ACP Registry for 30M+ JetBrains users
- Gate-verified coding in any editor (no other ACP agent does this)
- Persistent knowledge across sessions (no other ACP agent does this)
- Multi-phase plan visibility in the editor (no other ACP agent does this)

### 11.2 Strategic

- Nunchi shifts from "the only way to use Roko" to "the fleet management surface." Individual coding tasks happen in the IDE via ACP. Fleet orchestration, knowledge visualization, arena competition, and collective intelligence monitoring happen in Nunchi.
- Community/marketplace gains a distribution channel: Roko templates shared via the ACP Registry mean other developers can install your pre-configured Roko agent from their editor's dropdown.
- Roko's gate pipeline becomes a selling point in a crowded ACP registry. When every other agent is a single-loop LLM wrapper, "147/147 tests passed, verified by compiler" is a differentiation story that's visible in every tool call card.

### 11.3 Composition with A2A

ACP and A2A compose naturally. A user working in JetBrains via ACP asks Roko to "audit this contract using the remote audit agent." Roko receives this via ACP, discovers the remote agent via A2A, delegates the audit task over HTTP, streams A2A results back to the editor through ACP's `session/update` notifications. The user sees a seamless experience: they typed a prompt in their IDE and got back a gate-verified audit result, without knowing that two protocols and a remote agent were involved.

---

## 12. References

- ACP Specification: https://agentclientprotocol.com/protocol/overview
- ACP Rust SDK: https://crates.io/crates/agent-client-protocol
- ACP GitHub: https://github.com/agentclientprotocol/agent-client-protocol
- ACP Registry: https://agentclientprotocol.com/get-started/registry
- JetBrains ACP Documentation: https://www.jetbrains.com/help/ai-assistant/acp.html
- JetBrains ACP Registry Blog: https://blog.jetbrains.com/ai/2026/01/acp-agent-registry/
- Zed ACP Page: https://zed.dev/acp
- Kiro ACP Implementation: https://kiro.dev/docs/cli/acp/
- OpenCode ACP Implementation: https://open-code.ai/en/docs/acp
