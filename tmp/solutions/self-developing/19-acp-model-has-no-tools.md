# 19: ACP Model Has No Tools — Can't Do Anything

## Problem

The ACP model in Zed is a **pure chat model with zero tools**. It cannot:
- Read files
- Write files
- Run commands
- Execute slash commands programmatically
- Chain operations

When a user says "run roko prd commands for all of these" or "create tasks from these docs", the model:
1. Talks about what it WOULD do
2. Asks "Can I write this to .roko/prd/plans/...?" — but it literally cannot
3. Generates text that looks like a plan but can't execute it
4. Hangs or produces no visible result

The user's reaction: "This isn't really working."

---

## Root Cause: Exact Code Path

The dispatch path routes through `handle_session_prompt_inner` in
`crates/roko-acp/src/bridge_events.rs`. Here is the exact sequence:

```
session/prompt  -->  handle_session_prompt()  [bridge_events.rs:934]
                -->  handle_session_prompt_inner()  [bridge_events.rs:957]
                    detect slash command  [line 970]
                    if is_slash_command:  run_slash_command()
                    elif pipeline_template:  run_with_workflow_engine()
                    else (DEFAULT PATH):
                        build_messages_array()  [session.rs:574]
                        --> run_anthropic_cognitive_task()  [bridge_events.rs:1458]
                            or run_openai_compat_cognitive_task()  [bridge_events.rs:1731]
```

### Where tools SHOULD be but aren't: `build_messages_array`

`crates/roko-acp/src/session.rs` line 574:

```rust
pub fn build_messages_array(
    &self,
    system_prompt: &str,
    current_prompt: &str,
) -> Vec<serde_json::Value> {
    let mut messages = Vec::with_capacity(self.conversation_history.len() + 2);
    messages.push(serde_json::json!({
        "role": "system",
        "content": system_prompt   // <-- TEXT ONLY, no tool_definitions
    }));
    // ... history turns (text only) ...
    messages.push(serde_json::json!({
        "role": "user",
        "content": current_prompt  // <-- TEXT ONLY
    }));
    messages  // <-- returned to both Anthropic and OpenAI dispatch paths
}
```

The messages array contains **only text content blocks**. There is no `tools` field,
no `tool_choice` field, no function definitions.

### Where the model call happens with no tools

`crates/roko-acp/src/bridge_events.rs` line 1536:

```rust
fn model_call_request_from_acp_messages(
    model_key: &str,
    messages: &[serde_json::Value],
) -> ModelCallRequest {
    ModelCallRequest {
        model: model_key.to_string(),
        messages: messages
            .iter()
            .filter_map(model_call_chat_message_from_acp)
            .collect(),
        caller: Some("acp".to_string()),
        ..Default::default()  // <-- tools field is None/empty
    }
}
```

`ModelCallRequest::Default` has no tools. The request hits the Anthropic/OpenAI API
as a plain chat completion. The model gets no function definitions.

### The MCP tool loop is already wired — but only for session-attached MCP servers

`run_openai_compat_cognitive_task` (line 1731) does check for tools, but only for
MCP servers the user explicitly attached at session creation:

```rust
if !mcp_servers.is_empty()
    && openai_compat_tool_loop_supported(resolved.provider_kind)
    && run_openai_compat_mcp_tool_loop(...)
    .await?
{
    return Ok(());
}
```

This code path:
1. Only runs when the user has attached MCP servers in `session/new`
2. Does not apply to the Anthropic API path at all
3. Has no builtin tools from `roko-std`

The `run_anthropic_cognitive_task` path (line 1458) has NO equivalent check at all —
it always goes to plain streaming with zero tools.

---

## Contrast: How the CLI Agent Gets Tools

The CLI orchestrator (`crates/roko-cli/src/orchestrate.rs`) gives agents a full
tool suite via `roko-std` and `roko-agent`:

```rust
// orchestrate.rs dispatch_agent_with():
let resolver = Arc::new(HandlerRegistry::new());  // roko-std all 16 handlers
let registry = Arc::new(StaticToolRegistry::default());  // all ToolDefs
let dispatcher = Arc::new(ToolDispatcher::new(registry, resolver));
// dispatcher is passed into the agent, which passes it to the LLM backend
// The LLM sees tool definitions, can request tool calls, gets results back
```

ACP has NONE of this wired for the single-agent chat path.

---

## The Full Builtin Tool List (roko-std)

`crates/roko-std/src/tool/builtin/mod.rs` exports 16 std tools + 17 chain tools + 4 ISFR tools = 37 total.

### The 16 std tools (all have `ToolHandler` implementations in `handlers.rs`):

| Name | Description | Permission | Category |
|------|-------------|-----------|----------|
| `read_file` | Read UTF-8 file from worktree | read-only | Read |
| `write_file` | Write file, create or replace | read+write | Write |
| `edit_file` | Edit file with old/new string replacement | read+write | Write |
| `multi_edit` | Edit multiple files in one call | read+write | Write |
| `glob` | File pattern matching | read-only | Read |
| `grep` | Ripgrep-based content search | read-only | Read |
| `bash` | Execute shell command via `bash -c` | read+exec | Exec |
| `ls` | List directory contents | read-only | Read |
| `web_fetch` | Fetch URL and extract content | network | Network |
| `web_search` | Search the web | network | Network |
| `notebook_edit` | Edit Jupyter notebook cells | read+write | Notebook |
| `todo_write` | Write structured todo list | read+write | Write |
| `task` (task_agent) | Spawn a sub-agent task | meta | Meta |
| `exit_plan_mode` | Exit plan mode, transition to implementation | meta | Meta |
| `apply_patch` | Apply a unified diff patch | read+write | Write |
| `run_tests` | Run test suite, return results | read+exec | Exec |

### Chain tools (17) and ISFR tools (4) are also in `ROKO_BUILTIN_TOOLS` but have no
`ToolHandler` implementations in `handlers.rs` — they are definition-only today.

---

## Option D: Hybrid Tools — Design

**Recommended approach**: expose a focused subset of 8 builtin tools for the
single-agent ACP path. This is enough for the model to read, write, search,
and run commands — covering 90% of real agentic tasks.

### The 8 tools to expose in the ACP hybrid set:

| Tool | Why | Safety tier |
|------|-----|-------------|
| `read_file` | Read any file — essential for context | Auto-approve |
| `write_file` | Write files — core agentic capability | Needs permission |
| `edit_file` | Targeted edits — less risky than write_file | Needs permission |
| `glob` | Find files by pattern — read-only | Auto-approve |
| `grep` | Search content — read-only | Auto-approve |
| `bash` | Run commands — powerful, needed for roko CLI | Needs permission |
| `ls` | List directories — read-only | Auto-approve |
| `web_fetch` | Fetch URLs — useful for research mode | Auto-approve |

This is exactly the set that Claude Code (the gold standard ACP agent) uses.

---

## How Tool Calls Flow in the ACP Protocol

The ACP protocol already has all the types needed. In `crates/roko-acp/src/types.rs`:

```rust
pub enum SessionUpdate {
    // Exists: shown as card in Zed
    ToolCall {
        tool_call_id: String,
        title: String,
        kind: ToolCallKind,         // Edit, Create, Terminal, Read, Search, etc.
        status: ToolCallStatus,     // Pending, InProgress, Completed, Failed
        content: Vec<ContentBlock>, // Rendered output shown in card
        locations: Option<Vec<ToolCallLocation>>, // File locations for Follow Agent
    },
    // Exists: updates an existing card
    ToolCallUpdate { ... },
}

pub enum ToolCallKind { Edit, Create, Delete, Terminal, Read, Search, Fetch, Think, Move, Other }
pub enum ToolCallStatus { Pending, InProgress, Completed, Failed }
```

The `CognitiveEvent` in `bridge_events.rs` already has the bridge types:

```rust
pub enum CognitiveEvent {
    ToolCallStart {
        tool_call_id: String,
        title: String,
        kind: ToolCallKind,
        locations: Option<Vec<ToolCallLocation>>,
    },
    ToolCallComplete {
        tool_call_id: String,
        status: ToolCallStatus,
        content: Vec<ContentBlock>,
    },
    // ...
}
```

These are already mapped to `SessionUpdate::ToolCall` and `SessionUpdate::ToolCallUpdate`
by `map_event_to_update()` in `bridge_events.rs`. The editor plumbing is complete —
the gap is only in the dispatch path not sending tools to the model and not
parsing tool calls from the response.

### The full flow once wired:

```
1. build_messages_array() includes `tools: [...]` field
2. Model responds with tool_calls in the response JSON
3. Roko parses tool_call blocks from the Anthropic/OpenAI response
4. For each tool_call:
   a. Emit CognitiveEvent::ToolCallStart  →  Zed shows "Reading src/lib.rs..."
   b. Execute the tool handler (from roko-std HandlerRegistry)
   c. Emit CognitiveEvent::ToolCallComplete  →  Zed updates card
   d. Append tool_result message to messages array
5. Send follow-up model call with tool results
6. Repeat until model stops calling tools
7. Emit CognitiveEvent::Complete
```

---

## How to Wire OpenAI Function-Calling Format

### Step 1: Add tools to `build_messages_array`

`crates/roko-acp/src/session.rs`:

```rust
pub fn build_messages_array(
    &self,
    system_prompt: &str,
    current_prompt: &str,
    tools: Option<&[ToolDef]>,   // NEW parameter
) -> Vec<serde_json::Value> {
    let mut messages = Vec::with_capacity(self.conversation_history.len() + 2);
    messages.push(serde_json::json!({
        "role": "system",
        "content": system_prompt
    }));
    // ... history turns ...
    messages.push(serde_json::json!({
        "role": "user",
        "content": current_prompt
    }));
    messages
}

// Separate fn: build OpenAI tool definitions from ToolDef
pub fn build_openai_tool_defs(tools: &[ToolDef]) -> Vec<serde_json::Value> {
    tools.iter().map(|def| serde_json::json!({
        "type": "function",
        "function": {
            "name": def.name,
            "description": def.description,
            "parameters": def.parameters.as_value()
        }
    })).collect()
}

// For Anthropic format (slightly different shape):
pub fn build_anthropic_tool_defs(tools: &[ToolDef]) -> Vec<serde_json::Value> {
    tools.iter().map(|def| serde_json::json!({
        "name": def.name,
        "description": def.description,
        "input_schema": def.parameters.as_value()
    })).collect()
}
```

### Step 2: Add `ModelCallRequest` tools field

`crates/roko-acp/src/bridge_events.rs` line 1536:

```rust
fn model_call_request_from_acp_messages(
    model_key: &str,
    messages: &[serde_json::Value],
    tools: &[ToolDef],   // NEW
) -> ModelCallRequest {
    ModelCallRequest {
        model: model_key.to_string(),
        messages: messages.iter().filter_map(model_call_chat_message_from_acp).collect(),
        caller: Some("acp".to_string()),
        // Wire tools through ModelCallRequest if it has a tools field,
        // or embed them directly in the messages as system context.
        // See ModelCallRequest definition for the right field.
        ..Default::default()
    }
}
```

### Step 3: Parse tool calls from the model response stream

In `forward_model_stream_event` (bridge_events.rs line 1635), extend
`ModelStreamEvent` handling to include tool-use content blocks.

Anthropic returns tool use like:
```json
{"type": "tool_use", "id": "toolu_01...", "name": "read_file", "input": {"path": "src/lib.rs"}}
```

OpenAI returns:
```json
{"tool_calls": [{"id": "call_123", "function": {"name": "read_file", "arguments": "{\"path\":\"src/lib.rs\"}"}}]}
```

### Step 4: Execute the tool and feed result back

After parsing a tool call, execute via `roko-std` handlers:

```rust
// In the cognitive task loop, after receiving a tool_use block:
let call = ToolCall {
    id: tool_use.id.clone(),
    name: tool_use.name.clone(),
    arguments: tool_use.input.clone(),
};

// Emit ToolCallStart to Zed
event_sender.send(CognitiveEvent::ToolCallStart {
    tool_call_id: call.id.clone(),
    title: format_tool_title(&call.name, &call.arguments),
    kind: tool_call_kind_for(&call.name),
    locations: extract_file_locations(&call.name, &call.arguments, workdir),
}).await;

// Execute
let handler = handler_for(&call.name).expect("known tool");
let ctx = ToolContext::new(workdir).with_permissions(ToolPermission::full());
let result = handler.execute(call, &ctx).await;

// Emit ToolCallComplete to Zed
event_sender.send(CognitiveEvent::ToolCallComplete {
    tool_call_id: tool_use.id.clone(),
    status: result_to_status(&result),
    content: vec![ContentBlock::Text { text: result.to_string() }],
}).await;

// Append tool result to messages and call model again
messages.push(tool_result_message(&tool_use.id, &result));
```

---

## How Other ACP Agents Handle Tool Use

### Claude Code (gold standard)

Claude Code ACP (the Anthropic first-party agent, which this ACP implementation
was modeled after) handles tools as follows:

1. **Registers all tools at session init** — the editor gets `tools` in the
   `session/new` response or via the first prompt response.
2. **Tool calls stream as `tool_call` session updates** — Zed renders each tool
   call as a collapsible card with `ToolCallKind` (edit, terminal, read, etc.).
3. **Permission requests for destructive tools** — using `session/request_permission`
   which Zed shows as a dialog (Allow / Always Allow / Reject). This protocol is
   already fully implemented in `bridge_events.rs:request_permission()`.
4. **Tool results go back into the context** — multi-turn loop continues until
   model stops calling tools.

### Codex (OpenAI)

Codex uses the same OpenAI function-calling format (`tools: [...]` in the request,
`tool_calls` in the response). The existing `run_openai_compat_mcp_tool_loop` in
`bridge_events.rs` already implements exactly this pattern for MCP tools. The
builtin tool wiring is a straightforward extension of that same loop.

The key difference is that Codex tools always go through the OpenAI format,
while roko needs to handle both Anthropic blocks format and OpenAI JSON format
depending on the resolved provider.

---

## Safety Considerations for Zed

The ACP permission system is already fully implemented in `bridge_events.rs`.
`PermissionAction` enum and `request_permission()` function exist and work.

### Tool safety tiers for ACP:

**Auto-approve (no dialog)** — read-only or network-read:
- `read_file`, `glob`, `grep`, `ls` — read-only, sandboxed to worktree
- `web_fetch` — outbound HTTP read, user expects this

**Require permission on first use** — destructive or exec:
- `write_file`, `edit_file`, `multi_edit`, `apply_patch` — mapped to `PermissionAction::FileEdit` / `FileCreate`
- `bash`, `run_tests` — mapped to `PermissionAction::TerminalCommand`

**Already wired in `PermissionAction`:**
```rust
pub enum PermissionAction {
    FileEdit,
    FileCreate,
    FileDelete,
    TerminalCommand,
    NetworkRequest,
    GitOperation,
}
```

The `always_allowed` set on `AcpSession` persists "Always Allow" decisions within
a session, and `save_workspace_trust()` persists them to
`.roko/trust/permissions.json` across sessions. This is already working.

The tool handler should call `request_permission()` before executing destructive
tools, using the existing transport reference. This means the permission dialog
naturally interrupts the tool loop, the user approves/rejects, and execution
continues or aborts.

---

## Complete Implementation Plan

### Files and line targets:

| File | Location | Change |
|------|----------|--------|
| `crates/roko-acp/src/session.rs` | line 574 | Add `tools: Option<&[ToolDef]>` param to `build_messages_array`; add `build_openai_tool_defs()` and `build_anthropic_tool_defs()` helpers |
| `crates/roko-acp/src/bridge_events.rs` | line 1038 | Instantiate ACP builtin tool set: `let acp_tools = acp_builtin_tools();` |
| `crates/roko-acp/src/bridge_events.rs` | line 1043 | Pass tools into `build_messages_array()` |
| `crates/roko-acp/src/bridge_events.rs` | line 1458 | Add tool definitions to Anthropic dispatch (`run_anthropic_cognitive_task`) |
| `crates/roko-acp/src/bridge_events.rs` | line 1731 | Add builtin tools to OpenAI-compat dispatch (extends the existing MCP tool loop) |
| `crates/roko-acp/src/bridge_events.rs` | line 1635 | Extend `forward_model_stream_event` to handle `ModelStreamEvent::ToolUse` |
| `crates/roko-acp/src/bridge_events.rs` | new fn | `execute_acp_tool_call()` — wraps `handler_for()`, emits CognitiveEvent ToolCallStart/Complete, checks permissions |
| `crates/roko-acp/src/bridge_events.rs` | new fn | `acp_builtin_tools()` — returns the 8-tool hybrid set as `Vec<ToolDef>` |

### New dependency needed in `roko-acp/Cargo.toml`:

```toml
roko-std = { path = "../roko-std" }
```

Currently `roko-acp` does not depend on `roko-std`. It depends on `roko-agent`
(for `ToolDispatcher`, `HandlerResolver`, `VecToolRegistry`, `ToolLoop`) and
`roko-core` (for `ToolDef`, `ToolCall`, `ToolContext`, `ToolResult`). Adding
`roko-std` gives access to `handler_for()`.

Check for circular deps: `roko-std` → `roko-core` → no cycle. `roko-acp` →
`roko-std` → `roko-core` is fine.

---

## Concrete Rust Code Sketches

### 1. ACP builtin tool set (new fn in bridge_events.rs)

```rust
/// The 8-tool hybrid set exposed to the ACP single-agent dispatch path.
/// Read-only tools are always active; write/exec tools require permission
/// before execution via `request_permission()`.
fn acp_builtin_tools() -> Vec<ToolDef> {
    use roko_std::tool::builtin::{bash, edit_file, glob, grep, ls, read_file, web_fetch, write_file};
    vec![
        read_file::tool_def(),
        write_file::tool_def(),
        edit_file::tool_def(),
        glob::tool_def(),
        grep::tool_def(),
        bash::tool_def(),
        ls::tool_def(),
        web_fetch::tool_def(),
    ]
}
```

### 2. Tool kind mapping (new fn in bridge_events.rs)

```rust
fn tool_call_kind_for(tool_name: &str) -> ToolCallKind {
    match tool_name {
        "write_file" | "edit_file" | "multi_edit" | "apply_patch" => ToolCallKind::Edit,
        "bash" | "run_tests" => ToolCallKind::Terminal,
        "read_file" => ToolCallKind::Read,
        "grep" | "glob" | "ls" => ToolCallKind::Search,
        "web_fetch" | "web_search" => ToolCallKind::Fetch,
        _ => ToolCallKind::Other,
    }
}
```

### 3. Tool execution helper (new fn in bridge_events.rs)

```rust
async fn execute_acp_builtin_tool(
    call: ToolCall,
    workdir: &Path,
    event_sender: &mpsc::Sender<CognitiveEvent>,
) -> serde_json::Value {
    use roko_std::tool::handlers::handler_for;

    let tool_call_id = if call.id.is_empty() {
        format!("tool-{}", uuid::Uuid::new_v4())
    } else {
        call.id.clone()
    };

    // Emit start event to Zed
    let _ = event_sender.send(CognitiveEvent::ToolCallStart {
        tool_call_id: tool_call_id.clone(),
        title: format!("{}: {}", call.name, summarize_args(&call.arguments)),
        kind: tool_call_kind_for(&call.name),
        locations: extract_tool_locations(&call.name, &call.arguments, workdir),
    }).await;

    let result = if let Some(handler) = handler_for(&call.name) {
        let ctx = ToolContext::new(workdir)
            .with_permissions(ToolPermission::full());
        // N.B.: destructive tools (write_file, bash) should call
        // request_permission() here before executing; sketched separately.
        handler.execute(call, &ctx).await
    } else {
        ToolResult::err(ToolError::Other(format!("unknown tool: {}", call.name)))
    };

    let (status, content_text) = match &result {
        ToolResult::Ok(content) => (ToolCallStatus::Completed, content.text.clone()),
        ToolResult::Err(e) => (ToolCallStatus::Failed, e.to_string()),
    };

    // Emit complete event to Zed
    let _ = event_sender.send(CognitiveEvent::ToolCallComplete {
        tool_call_id,
        status,
        content: vec![ContentBlock::Text { text: content_text.clone() }],
    }).await;

    // Return OpenAI-format tool result for re-injection into messages
    serde_json::json!({
        "role": "tool",
        "tool_call_id": call.id,
        "content": content_text
    })
}
```

### 4. Anthropic tool loop (extends run_anthropic_cognitive_task)

The Anthropic Messages API returns tool_use blocks in the assistant message content.
The loop:
1. Collect all `tool_use` blocks from the response
2. Execute each (serially or in parallel by concurrency flag)
3. Append `{"role": "assistant", "content": [<full assistant content blocks>]}`
4. Append `{"role": "user", "content": [{"type": "tool_result", ...}]}`
5. Re-call the model
6. Repeat until no tool_use blocks

### 5. OpenAI tool loop (extends run_openai_compat_cognitive_task)

The existing `run_openai_compat_mcp_tool_loop` in `bridge_events.rs` (line 1822)
already implements the exact multi-turn tool loop using `ToolLoop::run_messages_streaming`.
The only change needed is to pass `acp_builtin_tools()` to it in addition to (or instead
of requiring) MCP servers:

```rust
// In run_openai_compat_cognitive_task, replace the guard:
// OLD: if !mcp_servers.is_empty() && openai_compat_tool_loop_supported(...)
// NEW: always run tool loop for supported providers, using builtin tools + optional MCP

let all_tools: Vec<ToolDef> = acp_builtin_tools()
    .into_iter()
    .chain(mcp_tools_from_servers(mcp_servers).await)
    .collect();

if openai_compat_tool_loop_supported(resolved.provider_kind) && !all_tools.is_empty() {
    return run_openai_compat_tool_loop_with_tools(
        session_id, messages, &resolved, workdir, &all_tools, cancel_token, event_sender
    ).await.map(|_| ());
}
```

---

## The Anthropic API Path Gap (Distinct from OpenAI)

The `run_anthropic_cognitive_task` path (line 1458) hits the Anthropic Messages API
directly via `ModelCallService`. This path has no tool loop at all — it streams
text only. To add tools here:

1. `ModelCallRequest` needs a `tools` field that `ModelCallService` forwards to the API
2. The streaming response needs to handle `content_block_start` events with
   `type: "tool_use"` (Anthropic's streaming tool protocol uses
   `input_json_delta` events to stream the tool input JSON)
3. Once the stream ends, collect all tool_use blocks and execute them
4. Build a new `ModelCallRequest` with the tool results and re-stream

The `ModelStreamEvent` enum in `roko-core/src/foundation.rs` likely needs a
`ToolUse` variant to carry tool call information from the stream.

---

## Why the MCP Path Already Works (for Reference)

The session MCP tool loop at `run_openai_compat_mcp_tool_loop` (line 1822) works
because:

1. `setup_session_mcp_tools()` discovers tools from MCP servers
2. `VecToolRegistry::from_tools(mcp_state.tools.clone())` builds a registry
3. `AcpMcpHandlerResolver` maps tool names to `AcpMcpToolHandler` instances
4. `ToolDispatcher::new(registry, resolver)` wraps them
5. `ToolLoop::new(translator, dispatcher, backend).run_messages_streaming(messages, &tools, &ctx, chunk_sender)` runs the full multi-turn loop

The builtin tool path needs the same structure but using `roko-std`'s
`handler_for()` as the resolver and `ROKO_BUILTIN_TOOLS` (filtered to 8) as the registry.

The `ToolLoop` in `roko-agent` handles the entire turn-over-turn tool loop,
OpenAI message serialization, and streaming. It is the right abstraction to use
here — there is no need to reimplement the loop.

---

## Impact Assessment

Without tools, the ACP model is fundamentally limited to:
- Answering questions about code (it has some context via file resource blocks)
- Suggesting commands for the user to run manually
- Generating text that the user must copy-paste elsewhere

This defeats the purpose of an "agent" — it is just a chatbot.

With 8 builtin tools:
- Model can read any file before editing
- Model can write files directly (with Zed permission dialog)
- Model can run `roko prd idea "..."` and other CLI commands via `bash`
- Model can search and navigate the codebase
- Multi-turn loops work: read → edit → verify → commit

This closes the largest behavioral gap between the ACP model and Claude Code.

---

## Priority

**Critical** — this is the #1 reason the ACP feels broken. Users expect an agent,
they get a chatbot. The infrastructure (permission dialogs, tool call cards,
ToolLoop, VecToolRegistry, ToolDispatcher, all 16 ToolHandlers) is already built.
The wiring is a matter of connecting existing pieces.

Estimated effort: 2-3 days for a complete implementation including tests.

Minimal viable version (OpenAI-compat path only, 8 tools, no Anthropic streaming):
1 day.
