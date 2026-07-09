# Runner 3: `agent-session-parity` — Granular Batch Specification

Date: 2026-04-28

Parent: [FULL-WORK-PLAN.md](./FULL-WORK-PLAN.md) Runner 3 section.

---

## Runner Goal (one sentence)

Make interactive `roko` and one-shot `roko "prompt"` use a real Mori-style agent session that
delegates to existing adapters instead of growing `dispatch_direct.rs`.

## Context Pack Files

```text
tmp/runners/agent-session-parity/
  README.md
  batches.toml
  context/
    00-RULES.md                     — universal + runner-specific anti-patterns
    ARCHITECTURE-CONTRACT.md        — single-owner map for this runner
    ANTI-PATTERNS.md                — forbidden patterns with repo examples
    ACCEPTANCE.md                   — proof commands including negative proofs
    FILE-OWNERSHIP.md               — batch → write path map
    ISSUE-MAP.md                    — batch → issue id map
    DISPATCH-AUDIT.md               — current dispatch paths and their gaps (Group 0 output)
    SESSION-CONTRACT.md             — ChatAgentSession spec (Group 0 output)
```

---

## Anti-Pattern Rules (00-RULES.md)

Include the universal rules from FULL-WORK-PLAN.md plus:

```markdown
# Agent-Session Anti-Patterns

CP-1. **One session struct.** `ChatAgentSession` is the sole owner of chat/one-shot session
      state. No other struct should hold model, effort, system prompt, tools, MCP config, or
      session_id for the interactive path.

      EXISTING ANTI-PATTERN (do not repeat):
      - `crates/roko-cli/src/chat_inline.rs:740-764` keeps `conversation`, `system_message`,
        and model/provider fields. None are sent through dispatch.
      - `crates/roko-cli/src/dispatch_direct.rs:140-143` builds a bare `claude` command with
        only `--print --output-format stream-json`. No model, effort, system prompt, tools,
        MCP, or resume.

CP-2. **Delegate to existing adapters.** Claude CLI turns delegate to `ClaudeCliAgent`
      (or its command builder). API turns delegate to existing provider adapters or
      `ModelCallService`. Do NOT hand-roll provider HTTP loops in the CLI layer.

      EXISTING ANTI-PATTERN (do not repeat):
      - `crates/roko-cli/src/dispatch_direct.rs:34-90` builds raw Anthropic API requests
        with hand-rolled JSON, system prompt is not included.
      - `crates/roko-cli/src/dispatch_direct.rs:93-137` builds raw OpenAI-compat requests
        with hand-rolled JSON.

CP-3. **One-shot uses the same session path.** `roko "prompt"` must go through
      `ChatAgentSession` in single-turn mode. It gets the same system prompt, tools, MCP,
      model selection, and workspace context as interactive chat.

      EXISTING ANTI-PATTERN (do not repeat):
      - The positional prompt path in `unified.rs` / `dispatch_direct.rs` has completely
        separate provider resolution, no tools, no MCP, and no workspace context.

CP-4. **Session id is captured and reused.** After a Claude CLI turn, extract `session_id`
      from the result. Pass it as `--resume` on the next turn. Multi-turn chat MUST use
      session continuity.

      EXISTING ANTI-PATTERN (do not repeat):
      - `dispatch_direct.rs:205-207` extracts `session_id` from the stream. `chat_inline.rs`
        never stores or reuses it on subsequent turns.
```

---

## Group 0: Contract Guardrails

### Z01 — Audit current dispatch and chat paths

**Type:** Context-only (no code changes)

**Goal:** Map exactly how interactive and one-shot dispatch currently work, line by line.

**Write scope:**
- `tmp/runners/agent-session-parity/context/DISPATCH-AUDIT.md`

**Read:**
- `crates/roko-cli/src/chat_inline.rs` (session state, dispatch calls, slash commands)
- `crates/roko-cli/src/dispatch_direct.rs` (all three provider paths)
- `crates/roko-cli/src/dispatch_v2.rs` (provider-neutral seam)
- `crates/roko-cli/src/unified.rs` (entry point routing, background serve)
- `crates/roko-cli/src/main.rs` (positional prompt routing)
- `crates/roko-agent/src/claude_cli_agent.rs:305-341` (correct command builder)
- `crates/roko-agent/src/provider/anthropic_api.rs` (existing tool loop)
- `crates/roko-agent/src/provider/openai_compat.rs` (existing tool loop)

**Required output:**
- For interactive chat: exact call chain from `main.rs` → chat_inline → dispatch_direct
- For positional prompt: exact call chain from `main.rs` → unified → dispatch_direct
- For `roko run`: exact call chain to see what's different
- What fields `chat_inline.rs` stores but never sends to dispatch
- What fields `ClaudeCliAgent` accepts that `dispatch_direct.rs` doesn't pass
- Where `session_id` is extracted and where it's dropped
- Current slash command parsing: what exists, what's a no-op

**DO NOT:** Change any source code.

---

### Z02 — Define ChatAgentSession contract

**Type:** Context-only (no code changes)

**Goal:** Write the precise session contract that all implementation batches will follow.

**Write scope:**
- `tmp/runners/agent-session-parity/context/SESSION-CONTRACT.md`

**Read:**
- `tmp/runners/agent-session-parity/context/DISPATCH-AUDIT.md` (from Z01)
- `crates/roko-agent/src/claude_cli_agent.rs` (what it accepts)
- `crates/roko-compose/src/system_prompt_builder.rs` (prompt assembly API)
- `crates/roko-agent/src/safety/` (tool policy API)
- `crates/roko-core/src/config.rs` (MCP config fields)
- FULL-WORK-PLAN.md Runner 3 section

**Required output:**
- Exact `ChatAgentSession` struct definition with field types
- Constructor signature: `fn new(config, workdir, auth, model_selection) -> Result<Self>`
- Turn method signature: `async fn send_turn(&mut self, prompt: &str) -> Result<TurnResult>`
- Slash command mutation signatures
- How it delegates to `ClaudeCliAgent` vs provider adapters
- What `TurnResult` looks like (text, tool_calls, session_id, usage, streaming_handle)

**DO NOT:** Change any source code.

---

## Group A: Session Core

### A01 — Add `ChatAgentSession` struct with constructor

**Goal:** One struct owns all interactive/one-shot session state.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs` (NEW FILE)
- `crates/roko-cli/src/lib.rs` (module declaration)

**Read:**
- `tmp/runners/agent-session-parity/context/SESSION-CONTRACT.md` (from Z02)
- `crates/roko-cli/src/model_selection.rs` (EffectiveModelSelection from Runner 2)
- `crates/roko-core/src/config.rs`

**Required behavior:**
```rust
pub struct ChatAgentSession {
    pub workdir: PathBuf,
    pub model_selection: EffectiveModelSelection,
    pub effort: String,
    pub system_prompt: String,
    pub allowed_tools_csv: String,
    pub mcp_config: Option<PathBuf>,
    pub session_id: Option<String>,         // Claude CLI session continuity
    pub api_history: Vec<ChatMessage>,       // API provider history
    pub http_client: reqwest::Client,        // shared, not per-request
    pub settings_json: Option<PathBuf>,      // safety/settings
    pub timeout: Option<Duration>,
}

pub struct ChatMessage {
    pub role: String,        // "user" | "assistant"
    pub content: String,
}
```
- Constructor takes `(config: &Config, workdir: PathBuf, model_selection: EffectiveModelSelection)`
- Constructor resolves system prompt via existing `SystemPromptBuilder` or prompt assembly service
- Constructor resolves tool policy from existing safety/tool contracts
- Constructor discovers MCP config from existing paths
- Constructor creates ONE `reqwest::Client` (not per-request)

**DO NOT:**
- Implement `send_turn` yet (that's A02/B01)
- Add provider-specific code to this struct
- Duplicate `EffectiveModelSelection` logic — import from Runner 2's module
- Create a second session struct for one-shot

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 5.1-5.5, MY-TAKE-SHORTEST-PATH.md, FINAL-SOLUTION.md

---

### A02 — Resolve system prompt from existing prompt services

**Goal:** Session gets a real workspace-aware system prompt, not an empty string.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Read:**
- `crates/roko-compose/src/system_prompt_builder.rs`
- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-cli/src/orchestrate.rs` (how run.rs currently builds system prompts)

**Required behavior:**
- Use `SystemPromptBuilder` or `PromptAssemblyService` to build the system prompt
- Include workspace context: project name, key files, language info
- Include role context: default read-oriented role for interactive chat
- Prompt is bounded in size (max ~4000 tokens for system prompt section)
- If prompt assembly fails or is too large, fall back to a minimal workspace-aware prompt

**DO NOT:**
- Build a new prompt builder — use the existing one
- Hardcode prompt text — resolve from templates/config
- Make the system prompt path different for one-shot vs interactive

**Verify:** `cargo check -p roko-cli`

---

### A03 — Resolve tool policy from existing safety/tool contracts

**Goal:** Session has a typed tool allowlist, not an empty string.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Read:**
- `crates/roko-agent/src/safety/` (AgentContract, tool policy)
- `crates/roko-core/src/config.rs` (tool-related config)
- `crates/roko-cli/src/orchestrate.rs` (how tools are currently resolved for plan runner)

**Required behavior:**
- Resolve tool policy from safety contracts or config
- Default: read-oriented tools (Read, Glob, Grep, Bash with restrictions)
- `allowed_tools_csv` formatted as Claude CLI `--tools` expects
- If no safety contract found: use the permissive default but log a warning

**DO NOT:**
- Hardcode a tool string — resolve from existing safety layer
- Create a new tool policy system
- Make tool policy different for one-shot vs interactive (same session path)

**Verify:** `cargo check -p roko-cli`

---

### A04 — Resolve MCP config from existing discovery paths

**Goal:** Session finds and passes MCP config if available.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Read:**
- `crates/roko-core/src/config.rs` (agent.mcp_config field)
- `crates/roko-cli/src/orchestrate.rs` (how MCP config is currently discovered)

**Required behavior:**
- Check `config.agent.mcp_config` first
- Check `.roko/mcp.json` as fallback
- Check `~/.claude/mcp-config.json` as further fallback
- Store as `Option<PathBuf>` — None is fine when no MCP config exists
- Pass through to Claude CLI as `--mcp-config <path> --strict-mcp-config`

**DO NOT:**
- Create a new MCP discovery mechanism
- Fail if no MCP config exists — it's optional
- Load or parse the MCP config contents (Claude CLI handles that)

**Verify:** `cargo check -p roko-cli`

---

### A05 — Slash command state mutation (unit tests only)

**Goal:** Prove `/system`, `/model`, `/effort`, `/reset` change session state.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs` (add mutation methods + tests)

**Required behavior:**
```rust
impl ChatAgentSession {
    pub fn handle_slash_command(&mut self, input: &str) -> SlashResult {
        // /system <text> → replace system_prompt
        // /model <model> → re-resolve model selection (calls resolve_effective_model)
        // /effort <level> → update effort
        // /reset → clear session_id, api_history, reset system_prompt to default
        // /tools <csv> → update allowed_tools_csv
        // /mcp <path> → update mcp_config
        // unknown → SlashResult::Unknown(input)
    }
}

pub enum SlashResult {
    Updated(String),         // confirmation message
    Error(String),           // e.g., unknown model
    Unknown(String),         // not a slash command, treat as user prompt
}
```
- Unit tests for each mutation:
  - `/system You are a Rust expert` → system_prompt changed
  - `/model claude-haiku-4-5` → model_selection updated
  - `/effort high` → effort changed
  - `/reset` → session_id cleared, history cleared
  - `regular text` → SlashResult::Unknown

**DO NOT:**
- Wire into the actual REPL yet (that's Group D)
- Call external services in tests — mock config/model resolution
- Add UI rendering

**Verify:** `cargo test -p roko-cli -- chat_session`

---

## Group B: Claude CLI Turn Execution

### B01 — Add `send_turn` using `ClaudeCliAgent`

**Goal:** A chat turn delegates to the existing correct command builder.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Read:**
- `crates/roko-agent/src/claude_cli_agent.rs` (command builder, execute method)
- `tmp/runners/agent-session-parity/context/SESSION-CONTRACT.md`

**Required behavior:**
- `send_turn(prompt)` builds a request with ALL session fields:
  - workdir, model (from model_selection.backend_slug), effort
  - system_prompt (via --append-system-prompt)
  - allowed_tools_csv (via --tools)
  - mcp_config (via --mcp-config + --strict-mcp-config)
  - session_id (via --resume when present)
  - timeout
- Calls `ClaudeCliAgent::execute()` or its command builder directly
- Returns a `TurnResult` with text, tool_calls, session_id, usage

**DO NOT:**
- Build a raw `claude` subprocess command — use `ClaudeCliAgent`
- Duplicate the command builder from `claude_cli_agent.rs:305-341`
- Add streaming yet (that's Group C)
- Handle API providers here (that's Group E, deferred)

**Verify:** `cargo check -p roko-cli`

**Evidence:** FINAL-SOLUTION.md "What ClaudeCliAgent Already Does Right"

---

### B02 — Capture session_id from turn result

**Goal:** Multi-turn chat uses session continuity.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Read:**
- `crates/roko-agent/src/claude_cli_agent.rs` (where session_id is in the response)

**Required behavior:**
- After `send_turn`, extract `session_id` from the Claude CLI result
- Store in `self.session_id`
- Next `send_turn` passes it as `--resume <session_id>`
- If session_id is absent (e.g., Claude CLI didn't return one): continue without resume
- `/reset` clears session_id, starting a fresh session

**DO NOT:**
- Fabricate session ids
- Store session_id in a separate global — it lives in `ChatAgentSession`
- Break single-turn mode (one-shot should NOT pass --resume)

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 5.3

---

### B03 — Capture tool output for display

**Goal:** Tool calls are visible to the user during chat.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Read:**
- `crates/roko-agent/src/claude_cli_agent.rs` (stream-json parsing for tool events)

**Required behavior:**
- During a turn, tool names and brief summaries are available for display
- `TurnResult.tool_calls: Vec<ToolCallSummary>` with tool name, abbreviated input, output status
- The caller (REPL/one-shot) can print tool calls as they complete
- Tool output is NOT suppressed in normal mode

**DO NOT:**
- Print directly to stdout from the session — return data for the caller to render
- Parse full tool call JSON — summaries are sufficient
- Break existing `ClaudeCliAgent` stream parsing

**Verify:** `cargo check -p roko-cli`

---

### B04 — Cancellation and timeout cleanup

**Goal:** Ctrl-C and timeouts don't leave orphaned Claude processes.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Read:**
- `crates/roko-agent/src/claude_cli_agent.rs` (child process handling)
- `crates/roko-runtime/src/` (ProcessSupervisor if applicable)

**Required behavior:**
- `send_turn` returns a handle or future that can be cancelled
- Cancellation sends SIGTERM to the child process, then SIGKILL after 5s
- Timeout (configurable) also triggers cancellation
- After cancellation: `TurnResult` has `cancelled: true`, no session_id update
- No zombie processes after Ctrl-C

**DO NOT:**
- Implement a new process supervisor — use existing runtime facilities
- Block on cancellation (should be async-friendly)
- Leave process groups unmanaged

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 5.5, MY-TAKE-SHORTEST-PATH.md §5

---

### B05 — Unit tests for turn execution

**Goal:** Prove the session correctly invokes ClaudeCliAgent and handles results.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs` (test module)
- OR `crates/roko-cli/tests/chat_session.rs`

**Required behavior:**
Tests (using mocked command builder or fake agent result):
- First turn: no `--resume` in command args
- Second turn after session_id captured: `--resume <id>` in command args
- `/reset` then turn: no `--resume` in command args again
- Turn with `/model X` applied: correct model in command args
- Turn with `/system Y` applied: correct system prompt in command args
- Timeout: returns cancelled result, no session_id change

**DO NOT:**
- Spawn real Claude processes in unit tests
- Depend on network
- Test streaming (that's Group C)

**Verify:** `cargo test -p roko-cli -- chat_session`

---

## Group C: Streaming

### C01 — Parse Claude stream-json incrementally

**Goal:** Long answers appear as they're generated, not after exit.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`
- OR `crates/roko-cli/src/stream_handler.rs` (NEW FILE if too large for chat_session)

**Read:**
- `crates/roko-agent/src/claude_cli_agent.rs` (existing stream-json parsing)

**Required behavior:**
- `send_turn_streaming(prompt) -> StreamingTurn`
- `StreamingTurn` is a stream/channel that yields `StreamEvent`:
  ```rust
  pub enum StreamEvent {
      TextDelta(String),
      ToolStart { name: String },
      ToolEnd { name: String, success: bool },
      Result { session_id: Option<String>, usage: Option<Usage> },
      Error(String),
  }
  ```
- Lines from Claude CLI stdout are parsed as they arrive (line-by-line)
- The caller can consume events and render incrementally

**DO NOT:**
- Buffer all output until process exit
- Parse raw Claude API wire format — parse `stream-json` output format
- Create a new streaming framework — use tokio channels or async streams
- Make non-streaming mode unavailable (keep `send_turn` as blocking alternative)

**Verify:** `cargo check -p roko-cli`

---

### C02 — Forward text deltas to terminal

**Goal:** Streaming text appears incrementally in the user's terminal.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs` (or stream_handler.rs)

**Required behavior:**
- `TextDelta` events are printed to stdout immediately (no buffering)
- Final newline added after last text delta
- Tool events are printed to stderr as `[tool: <name>]`
- If `--no-stream` flag is passed: fall back to blocking `send_turn`

**DO NOT:**
- Add TUI rendering here (that's Runner 7)
- Require a terminal capability check — work in dumb terminals too
- Buffer text deltas until a complete sentence/paragraph

**Verify:** `cargo check -p roko-cli`

---

### C03 — Capture final result metadata from stream

**Goal:** Session id, tokens, and cost are not lost after streaming.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Read:**
- `crates/roko-agent/src/claude_cli_agent.rs` (result event parsing)

**Required behavior:**
- The `Result` stream event includes `session_id` and `usage`
- After stream completes: `self.session_id` updated from result
- After stream completes: `TurnResult.usage` populated
- If stream is interrupted (Ctrl-C): result may be partial, session_id may be absent

**DO NOT:**
- Require a second pass over the output to get metadata
- Lose metadata when streaming is enabled
- Store metadata in a global — it goes into the session

**Verify:** `cargo check -p roko-cli`

---

### C04 — Streaming integration proof

**Goal:** End-to-end proof that streaming works with a real (or mock) Claude CLI.

**Write scope:**
- `crates/roko-cli/tests/streaming.rs` OR test module in chat_session

**Required behavior:**
- Mock a Claude CLI process that emits stream-json lines with controlled timing
- Verify text deltas arrive incrementally (not all at once)
- Verify final result metadata is captured
- Verify cancellation stops the stream

**DO NOT:**
- Require real Claude CLI for the test
- Require network access
- Make the test flaky with timing assumptions — use channels, not sleeps

**Verify:** `cargo test -p roko-cli -- streaming`

---

## Group D: Wire into REPL and One-Shot

### D01 — Route interactive `roko` through ChatAgentSession

**Goal:** The no-args REPL uses the new session instead of dispatch_direct.

**Write scope:**
- `crates/roko-cli/src/chat_inline.rs` (or its replacement)
- `crates/roko-cli/src/unified.rs` (entry point)

**Read:**
- `tmp/runners/agent-session-parity/context/DISPATCH-AUDIT.md`

**Required behavior:**
- `roko` (no args) creates a `ChatAgentSession`
- REPL loop: read input → check slash commands → send_turn_streaming → print output
- Session state persists across turns (model, effort, system prompt, session_id)
- `/exit` or Ctrl-D exits the REPL
- Ctrl-C during a turn cancels the turn (not the REPL)

**DO NOT:**
- Remove `dispatch_direct.rs` yet — just stop routing interactive chat through it
- Change the `roko run` path (separate concern)
- Add background serve changes (Runner 6)

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 5.1

---

### D02 — Route `roko "prompt"` through ChatAgentSession in single-turn mode

**Goal:** One-shot prompts get the same session capabilities as interactive chat.

**Write scope:**
- `crates/roko-cli/src/unified.rs` (or wherever positional prompt is routed)
- `crates/roko-cli/src/main.rs` (positional prompt handler)

**Required behavior:**
- `roko "What files are here?"` creates a `ChatAgentSession` in single-turn mode
- Gets system prompt, tools, MCP, model selection, workspace context
- Does NOT pass `--resume` (single turn)
- Output goes to stdout
- Exit code: 0 on success, 1 on failure
- Same model selection as interactive chat for the same config

**DO NOT:**
- Keep using `dispatch_direct.rs` for one-shot after this batch
- Add a separate one-shot session struct (use ChatAgentSession with one turn)
- Make one-shot require TUI

**Verify:** `cargo check -p roko-cli`

**Evidence:** COMPREHENSIVE-ISSUES 6.1-6.2

---

### D03 — Deprecate dispatch_direct.rs happy path

**Goal:** Old dispatch path is no longer the primary route.

**Write scope:**
- `crates/roko-cli/src/dispatch_direct.rs`

**Required behavior:**
- Add a deprecation log at the entry point:
  `tracing::warn!("dispatch_direct is deprecated; use ChatAgentSession")`
- Keep the code compilable for now (other callers may still reference it)
- Do NOT delete the file — just make sure no happy path routes through it
- If other code still calls dispatch_direct functions: leave them working but logged

**DO NOT:**
- Delete the file
- Break callers outside chat/one-shot
- Add new functionality to dispatch_direct

**Verify:** `cargo check -p roko-cli`

---

### D04 — Full interactive proof

**Goal:** Prove the interactive chat path works end-to-end.

**Write scope:**
- `tmp/runners/agent-session-parity/context/PROOF-INTERACTIVE.md` (manual proof script)

**Required proof steps:**
1. `roko` enters chat
2. User asks: "What files are in this repository root?"
3. Agent uses tools, tool output is visible
4. User asks a follow-up depending on the previous answer
5. Next turn uses `--resume` (verify in verbose/debug output)
6. `/system You are a Rust expert` changes the next answer's character
7. `/effort high` changes the next Claude invocation
8. `/model claude-haiku-4-5` changes the model used
9. Long answers stream incrementally (text appears before process exits)
10. Ctrl-C cancels an in-flight turn without orphaned processes

**DO NOT:** Automate this proof if it would require spawning real Claude — document the manual steps.

---

### D05 — Full one-shot proof

**Goal:** Prove one-shot prompts work end-to-end.

**Write scope:**
- `tmp/runners/agent-session-parity/context/PROOF-ONESHOT.md` (manual proof script)

**Required proof steps:**
1. `roko "What files are in the current directory?"` → uses tools, lists files
2. `roko --model claude-haiku-4-5 "say hello"` → uses haiku (verify in output/debug)
3. `roko "What files are here?"` and `roko run "What files are here?"` → same model for same config
4. `roko "prompt"` exit code 0 on success, 1 on failure
5. `roko "prompt"` gets workspace context (system prompt mentions the project)

---

## Group E: API Provider Path (Deferred Stub)

### E01 — Stub API provider turn in session

**Goal:** Document the API turn path without implementing full tool loops.

**Write scope:**
- `crates/roko-cli/src/chat_session.rs`

**Required behavior:**
- If `model_selection.provider_kind` is NOT `claude_cli`:
  - Return `Err("API provider chat not yet implemented. Use Claude CLI: roko --model claude-sonnet-4-6")`
- This is an explicit, typed error — not a silent fallback to a broken path
- Log which provider was requested and why it's not available

**DO NOT:**
- Implement API provider tool loops (that's Runner 7 or a later milestone)
- Fall back silently to Claude CLI when an API provider is selected
- Return an empty success

**Verify:** `cargo check -p roko-cli`

---

## Batch Summary

| Group | Batches | Main scope |
|---|---:|---|
| 0: Contracts | 2 | dispatch audit, session contract |
| A: Session Core | 5 | ChatAgentSession struct, prompt, tools, MCP, slash cmds |
| B: Claude CLI Turn | 5 | send_turn, session_id, tool output, cancellation, tests |
| C: Streaming | 4 | stream parsing, text forwarding, metadata, proof |
| D: Wire + Proof | 5 | REPL routing, one-shot routing, deprecation, proofs |
| E: API Stub | 1 | explicit not-implemented error |
| **Total** | **22** | |

## Suggested Execution Waves

Wave 1: Z01, Z02 (context-only, parallel)
Wave 2: A01 (session struct + constructor)
Wave 3: A02, A03, A04 (prompt, tools, MCP — parallel, all in chat_session.rs)
Wave 4: A05 (slash command tests)
Wave 5: B01, B02 (send_turn + session_id)
Wave 6: B03, B04, B05 (tool output, cancellation, tests)
Wave 7: C01, C02 (streaming parse + forward)
Wave 8: C03, C04 (metadata capture + streaming proof)
Wave 9: D01, D02, D03 (wire REPL + one-shot + deprecate)
Wave 10: D04, D05, E01 (proofs + API stub)

## Acceptance Criteria

This runner is done when:

**Positive proofs:**
- `roko` enters an interactive session with system prompt, tools, MCP, and model
- Follow-up turns use `--resume` for session continuity
- `/system`, `/model`, `/effort`, `/reset` change session behavior
- Long answers stream incrementally
- Tool output is visible during turns
- `roko "What files are here?"` uses tools and workspace context
- One-shot and interactive report the same effective model for the same config

**Negative proofs:**
- `dispatch_direct.rs` is not the happy path for chat or one-shot
- No second session struct exists for one-shot
- No hand-rolled provider HTTP in CLI code
- API provider selection returns a typed error (not silent fallback)
- Ctrl-C leaves no orphaned child processes
- `roko "prompt"` without tools available does not silently succeed with generic knowledge
