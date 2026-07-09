# Tool Dispatch Broken

## Symptom

`roko research hdc` outputs raw JSON tool calls as text:
```
{"tool_uses":[{"recipient_name":"functions.get_file_contents",
"parameters":{"path":"/Users/will/dev/nunchi/roko/roko/.roko/prd/INDEX.md"}},
{"recipient_name":"functions.get_file_contents",
"parameters":{"path":"/Users/will/dev/nunchi/roko/roko/.roko/research/INDEX.md"}}]}
I'm sorry, but I couldn't access the repository files because the file-reading
tools aren't available here.
```

The agent emits tool call JSON, nothing executes them, and the raw JSON appears
in the output alongside the model's confusion about tools not working.

## Root Cause Analysis

### Code Path

1. **Research command** (`crates/roko-cli/src/commands/research.rs:469-479`):
   ```rust
   allowed_tools: Some("Read,Write,Edit"),  // Tools declared but...
   ```

2. **Agent spawn** (`crates/roko-cli/src/agent_spawn.rs`):
   - Creates `SpawnAgentSpec` with `bare_mode: true`
   - Passes to `ClaudeCliAgent`

3. **Command building** (`crates/roko-agent/src/claude_cli_agent.rs:306-325`):
   ```rust
   fn build_command(&self) -> Command {
       let mut cmd = Command::new(&self.program);
       cmd.arg("--print")
           .arg("--output-format")
           .arg("stream-json")
           .arg("--model")
           // ... NO TOOL FLAGS PASSED
   }
   ```
   The `bare_mode` boolean is stored but **never used**. The `allowed_tools`
   string is stored but **never passed** to the Claude CLI command.

4. **No tool loop** (`claude_cli_agent.rs:625-869`):
   - `run()` spawns the process
   - Collects stdout/stderr
   - `output_text()` extracts text from StreamJson events
   - Returns raw output — **no tool dispatch, no tool loop**

5. **Tool calls parsed but ignored** (`provider/claude_cli/stream.rs:134`):
   - StreamJson events correctly identify `ToolUse` blocks as `AgentRuntimeEvent::ToolCall`
   - These events are used for **progress reporting only** (line 703-708)
   - They're **NOT dispatched** for execution

### Why Other Paths Work

| Path | Backend | Tool Loop | Works? |
|------|---------|-----------|--------|
| `roko run` with Claude CLI | ClaudeCliAgent | Claude CLI owns loop internally | Yes (Claude handles tools) |
| `roko do` / `roko plan run` | ToolLoopAgent | roko dispatches tools | Yes |
| `roko run` with OpenAI | OpenAiCompatBackend | ToolLoopAgent wraps it | Yes |
| `roko research` with Claude CLI | ClaudeCliAgent | **No loop** | **BROKEN** |
| `roko research` with OpenAI/Gemini | ToolLoopAgent | Should work | Untested |

The fundamental issue: **research commands use `run_agent_capture_silent()` which
does a single-shot agent call with no tool loop wrapper.**

### The Model Factor

The screenshot shows `gpt54-mini` as the model. OpenAI models emit tool calls in
`functions.*` format. But the research path doesn't set up tool schemas for the
model, so the model hallucinates tool names (`functions.get_file_contents`) that
don't match roko's registered tools (`read_file`).

Even if tools were dispatched, the wrong tool name would cause a "tool not found" error.

## Scope: Tool Dispatch Audit

This isn't just a research bug. Here's the full audit of tool support across backends:

### Backends with Working Tool Loops

| Backend | Tool Loop | Format | Status |
|---------|-----------|--------|--------|
| Claude CLI (via `roko run`) | CLI owns loop | anthropic_blocks | Working |
| Anthropic API | ToolLoopAgent | anthropic_blocks | Working (opt-in) |
| OpenAI-compatible | ToolLoopAgent | openai_json | Working |
| Gemini native | ToolLoopAgent | gemini_native | Working |
| Perplexity | ToolLoopAgent | openai_json | Working |
| Ollama | ToolLoopAgent | openai_json | Working (stream:false contract) |

### Backends WITHOUT Tool Loops

| Backend | Issue |
|---------|-------|
| Claude CLI (via research) | No tool flags passed, no loop |
| Cursor ACP | No tool loop at all |

### Commands Missing Tool Dispatch

| Command | Uses | Problem |
|---------|------|---------|
| `roko research topic` | `run_agent_capture_silent()` | Single-shot, no tool loop |
| `roko research enhance-prd` | `run_agent_capture_silent()` | Same |
| `roko research enhance-plan` | `run_agent_capture_silent()` | Same |
| `roko research enhance-tasks` | `run_agent_capture_silent()` | Same |
| `roko prd draft new` | Likely similar | Needs audit |
| `roko prd plan` | Likely similar | Needs audit |

## Fix Plan

### Option A: Wire tool flags to Claude CLI (Quick fix, ~30 min)

Pass `--allowedTools` to the Claude CLI subprocess:
```rust
if let Some(tools) = &self.allowed_tools {
    cmd.arg("--allowedTools").arg(tools);
}
```

This lets Claude CLI handle the tool loop internally. Works for Claude models only.

**Limitation**: Only works with Claude CLI backend. Other backends (OpenAI, Gemini)
used for research would still be single-shot.

### Option B: Wrap research in ToolLoopAgent (Proper fix, ~2 hr)

Change `run_agent_capture_silent()` to use `ToolLoopAgent` for all backends:

1. Register tools with `ToolDispatcher` (Read, Write, Edit + any MCP tools)
2. Create `ToolLoopAgent` wrapping the chosen backend
3. Run multi-turn loop until model stops calling tools
4. Collect final text output

**Files to modify**:
- `crates/roko-cli/src/commands/research.rs:469-479` — use ToolLoopAgent
- `crates/roko-cli/src/agent_exec.rs` — add tool loop variant to `run_agent_capture_silent`
- `crates/roko-agent/src/claude_cli_agent.rs:306-325` — pass tool flags

### Option C: Hybrid (Recommended, ~1.5 hr)

- For Claude CLI: pass tool flags (Option A) — Claude handles its own tool loop
- For all other backends: wrap in ToolLoopAgent (Option B)
- This matches how `roko run` and `roko do` already work

**Key files**:
```
crates/roko-cli/src/commands/research.rs     — dispatch logic
crates/roko-cli/src/agent_exec.rs            — agent execution helpers
crates/roko-cli/src/agent_spawn.rs           — SpawnAgentSpec construction
crates/roko-agent/src/claude_cli_agent.rs    — command building
crates/roko-agent/src/tool_loop/mod.rs       — ToolLoopAgent wrapping
```

## Related Issues

- **Tool name mismatch**: OpenAI models may hallucinate `functions.get_file_contents`
  instead of `read_file`. Need to either:
  - Register tools with model-specific names
  - Add tool name aliasing in the translator layer
  - The `roko_core::tool::aliases` module already has `canonical_of_claude` / `claude_of_canonical` —
    extend this for OpenAI format

- **Ollama M21 contract**: When tools are present, must send `stream: false` to Ollama.
  Currently enforced in HTTP backend layer but not documented in research path.

- **Gemini thinking + tools**: Extended thinking with tool calls not yet exercised at scale.

## E2E Test Findings (2026-05-08)

### Tool Loop Tests: 3/5 Skipped

The IDE integration tool loop tests (`tmp/solutions/ide/tests/test-tool-loop.sh`) have
**3 of 5 tests skipped** because the Nunchi IDE bridge (`http://127.0.0.1:6678`) is not
running. The skipped tests are the most critical ones:

| Test | Status | Why |
|------|--------|-----|
| `multiple tool calls in one turn` | SKIP | Bridge/MCP unavailable |
| `tool call creates a tile` | SKIP | Bridge/MCP unavailable |
| `tool error propagates to model` | SKIP | Bridge/MCP unavailable |
| `usage_update reports context window` | PASS | No bridge needed |
| `context grows across multiple turns` | PASS | No bridge needed |

**Implication**: The only passing tool loop tests verify context/usage tracking — they
don't actually exercise tool dispatch. The 3 tests that would verify real tool execution
are all skipped. This means **there are zero passing e2e tests for ACP tool dispatch**.

### MCP Tool Discovery: Partially Broken

From MCP integration tests:

| Test | Status | Detail |
|------|--------|--------|
| `nunchi-mcp initialize response` | PASS | MCP binary responds |
| `MCP tool discovery (valid binary)` | SKIP | Bridge unavailable |
| `MCP tool returns real data` | SKIP | Bridge unavailable |
| `nonexistent MCP binary → structured error` | PASS | Warning returned correctly |
| `MCP binary that exits → structured error` | **FAIL** | Session succeeds despite dead MCP |
| `MCP with invalid bridge token` | WARN | No tool call attempted |

**Key finding**: When an MCP binary immediately exits after spawning, the ACP server
doesn't detect the failure. The session proceeds with builtin tools and the model
responds normally. The MCP failure is silently swallowed.

### What This Means for Tool Dispatch

1. **No e2e test proves tools work end-to-end via ACP** — all 3 real tool tests need the bridge
2. **MCP server health is not monitored** — a crashing MCP server is invisible to the IDE
3. **The bridge dependency makes tool tests non-self-contained** — tests should work with
   a mock or local tool handler, not require an external service
4. **Tool dispatch in non-ACP paths (research, prd) has zero test coverage** — see above

### Recommendations

1. **Add self-contained tool tests**: Use roko's builtin tools (Read, ls) instead of requiring
   the bridge. The ACP server has access to these tools natively.

2. **Add MCP process monitoring**: After spawning an MCP server, verify it's still alive
   before marking it as ready. Check process status after a short delay.

3. **Add research tool dispatch test**: New test that runs `roko research topic "test"` and
   verifies tool calls are actually dispatched (not emitted as raw JSON).

4. **Decouple from bridge**: Tool loop tests should work without `http://127.0.0.1:6678`.
   Use the ACP server's own tool dispatch capability.
