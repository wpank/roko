# 01 — Dispatch Path Fragmentation

## The Problem

Mori has **3 clean dispatch paths** (Claude CLI, Codex, Cursor), all going through the same
agent lifecycle with identical event handling, cost tracking, and error reporting.

Roko has **6+ fragmented dispatch paths**, each with different behavior for tools, errors,
response parsing, cost, and events.

---

## Mori's 3 Dispatch Paths

All paths share:
- Same `AgentEvent` enum for all events (MessageDelta, CommandOutput, ToolCall, TokenUsage, TurnCompleted, Error)
- Same TUI rendering pipeline
- Same cost tracking (per-turn delta from `total_cost_usd`)
- Same session resume mechanism
- Same per-role tool allowlists
- Same `--bare` + system prompt injection

### Path 1: Claude CLI
**File**: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2373-2622`

Spawn: `claude --print --verbose --output-format stream-json --model <slug> --bare ...`
Stream parsing: Typed `ClaudeStreamEvent` enum with serde `#[serde(tag = "type")]`
Tool output: `Tool` variant → `AgentEvent::CommandOutput` → separate TUI panel
Resume: `--resume <session_id>` from prior `Result` event
Budget: Per-role USD limits with model multipliers (Opus 2x, Haiku 0.6x)

### Path 2: Codex (OpenAI)
**File**: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:1163-1482`

Spawn: `codex app-server` with JSON-RPC
Stream parsing: `parse_notification()` for JSON-RPC notifications
Tool output: `item/commandExecution/outputDelta` → `AgentEvent::CommandOutput`
Resume: `thread_id` from `thread/start` response (server-side state)

### Path 3: Cursor ACP
**File**: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:1826-1900`

Spawn: `agent --force --approve-mcps --output-format json acp`
Stream parsing: `parse_cursor_notification()` for session updates
Tool output: `tool_call_update` with `status=completed` → `AgentEvent::CommandOutput`
Resume: `session_id` from `session/new` response

---

## Roko's 6+ Dispatch Paths

### Path 1: `roko run` with routing config
**File**: `crates/roko-cli/src/run.rs:478-510` (`dispatch_agent()`)
- Triggered when: Providers/models defined in roko.toml
- Model: From `routing_config.agent.default_model`
- Calls: `spawn_agent_scoped()` with full provider adapter
- Tools: `claude_tool_allowlist()` applied
- Response: Through provider adapter's own parsing
- Cost: Via provider adapter
- **This is the "correct" path but rarely hit because most configs don't have routing**

### Path 2: Claude CLI + ANTHROPIC_API_KEY → HTTP
**File**: `crates/roko-cli/src/run.rs:511-513`
- Triggered when: command=claude AND ANTHROPIC_API_KEY set
- Calls: `run_anthropic_api_tool_loop()` — direct HTTP to api.anthropic.com
- Model: `"claude-sonnet-4-6-20250514"` hardcoded at `dispatch_direct.rs:208`
- Max tokens: **8192** hardcoded at `dispatch_direct.rs:212`
- Response: AnthropicResponse struct, content blocks
- Tool output: **Not captured** (no tool loop, single-turn)
- Cost: Basic token counts from response usage
- **Bypasses Claude CLI entirely. No --bare, no tools, no resume.**

### Path 3: Claude CLI subprocess
**File**: `crates/roko-cli/src/run.rs:514-568`
- Triggered when: command=claude (bare CLI)
- Model: Falls back to `"claude-sonnet-4-6"` at run.rs:530
- Fallback model: From config (run.rs:540-543), only path that uses it
- Resume: session_id from prior run (run.rs:536-539), only path that uses it
- Response: `extract_clean_text()` on JSONL
- Tool output: Now captured (dispatch_direct.rs) but only in chat dispatch, not this path
- Cost: Token counts from stream metadata

### Path 4: Ollama
**File**: `crates/roko-cli/src/run.rs:569-570`
- Triggered when: command matches ollama pattern
- Calls: `run_ollama_agentic_single()`
- Model: `"llama3.1:8b"` hardcoded at run.rs:657
- Response: OpenAI-compat JSON
- Tool output: Not captured
- Cost: From response usage

### Path 5: Known protocols (gemini, glm)
**File**: `crates/roko-cli/src/run.rs:571-603`
- Triggered when: command matches known protocol patterns
- Calls: `synthesize_known_protocol_config()` → `spawn_agent_scoped()`
- Model: From synthesized config
- Response: Through provider adapter
- Tool output: Depends on provider adapter

### Path 6: Generic subprocess
**File**: `crates/roko-cli/src/run.rs:604-636`
- Triggered when: nothing else matches
- Spawns raw command with prompt as stdin
- Response: Raw stdout text
- Tool output: None
- Cost: None

### Path 7: Chat direct dispatch (dispatch_direct.rs)
**File**: `crates/roko-cli/src/dispatch_direct.rs`
- Triggered by: `roko chat` inline (direct mode)
- Sub-routes by `AuthMethod`: Claude CLI subprocess, Anthropic API HTTP, OpenAI-compat HTTP
- Model: Hardcoded per sub-route
- Tool output: Captured for Claude CLI sub-route (recently fixed)
- Cost: Basic token extraction

### Path 8: Chat HTTP dispatch (chat_inline.rs)
**File**: `crates/roko-cli/src/chat_inline.rs:3187-3270`
- Triggered by: `roko chat` with serve backend
- Routes to: sidecar `/message` or serve `/api/agents/{id}/message`
- Response: `extract_clean_text()` on HTTP response body
- Tool output: Not captured (HTTP response is already processed)
- Cost: From HTTP response fields if present

### Path 9: Orchestrate.rs dispatch
**File**: `crates/roko-cli/src/orchestrate.rs:13906` (`dispatch_agent_with()`)
- Triggered by: `roko plan run` plan execution
- Model: Multi-level selection (override → task → tier → CascadeRouter → fallback)
- Response: Through full agent dispatch
- Tool output: Via agent event system
- Cost: Episode logger + efficiency events
- **This is the most feature-complete path but only used by plan execution**

---

## What Each Path Gets Wrong

| Feature | Path 1 (routing) | Path 2 (API) | Path 3 (CLI) | Path 4 (ollama) | Path 5 (known) | Path 6 (generic) | Path 7 (chat direct) | Path 8 (chat HTTP) | Path 9 (orchestrate) |
|---------|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| Model from config | Yes | No | Partial | No | Yes | No | No | N/A | Yes |
| Tool output captured | Depends | No | No | No | Depends | No | Partial | No | Yes |
| Session resume | Depends | No | Yes | No | No | No | Partial | No | Yes |
| Cost tracking | Yes | Basic | Basic | Basic | Yes | No | Basic | Basic | Full |
| Error classification | Depends | Basic | Basic | Basic | Depends | None | Basic | Basic | Full |
| --bare mode | Yes | N/A | Config | No | Yes | No | N/A | N/A | Yes |
| Tool allowlist | Yes | No | Yes | No | Depends | No | No | No | Yes |
| Fallback model | Depends | No | Yes | No | No | No | No | No | Yes |
| System prompt | Yes | No | Yes | No | Yes | No | No | No | Yes |

---

## Target: Consolidate to 2 Paths

### Path A: Provider Adapter (all configured providers)
Use `create_agent_for_model()` → provider adapter → unified response handling.
Covers: Claude CLI, Anthropic API, OpenAI-compat, Ollama, Gemini, Perplexity.

### Path B: Subprocess fallback (unconfigured commands)
Raw subprocess spawn for arbitrary commands.
Captures stdout/stderr, basic response extraction.

Everything else should go through Path A by resolving a provider from config.
