# 12 — Provider Dispatch & Agent Output Audit

**Status**: open (critical)
**Scope**: `crates/roko-agent/`, `crates/roko-cli/src/run.rs`, `crates/roko-cli/src/dispatch_direct.rs`, `crates/roko-cli/src/chat_inline.rs`

## Problem Statement

The dispatch layer has three classes of critical issues:

1. **Output handling**: Claude outputs are wrong/incomplete compared to mori (original doc scope)
2. **Connection management**: Every API call creates a fresh HTTP client — no TLS reuse,
   no connection pooling, causing 3-7s overhead per request (see §Connection Latency)
3. **Chat context**: The chat REPL sends bare text with no system prompt, no tools, no
   conversation history, and no workspace context (see §Chat Context Gap)
4. **Cost metrics**: The "savings vs baseline" metric compares against hardcoded Opus pricing,
   producing misleading numbers (see §Savings Baseline)

---

## Critical Differences: Roko vs Mori

### 1. Tool Output Handling (THE PRIMARY ISSUE)

**Mori**: Receives `Tool` events from claude CLI's `--output-format stream-json`.
These carry the **full output** of tool calls (file contents, bash stdout/stderr).
Mori routes them to a separate `CommandOutput` panel in the TUI. Users see:
- Agent reasoning in the main panel
- Tool execution results (file reads, bash outputs) in a separate panel

**Roko**: `extract_text()` in `roko-agent/src/translate/mod.rs:169-181` **ignores
Tool events entirely**. It only collects text content deltas from the stream. Tool
outputs are silently discarded. The user sees only the agent's final text, with no
visibility into what tools did.

**Fix needed**: Parse Tool events from stream-json and either:
- Include them in the response text (inline)
- Emit them as separate events for TUI/chat display
- At minimum, log them to the episode record

### 2. Model Selection Hardcoding

**Mori**: Uses backend detection from model slug (`claude-*` → Claude CLI,
`cursor-*` → Cursor ACP, else → Codex). Model routing based on health tracking +
pass rates.

**Roko**: Has multiple hardcoded fallbacks:
- `run.rs:530` — `"claude-sonnet-4-6"` (was `"claude-opus-4-6"`, partially fixed)
- `run.rs:787` — `"claude-sonnet-4-6-20250514"` for Anthropic API path
- `dispatch_direct.rs:145` — `"claude-sonnet-4-6-20250514"` for API
- `dispatch_direct.rs:226` — `"gpt-4o"` for OpenAI-compat
- `resolved_model()` at run.rs:1107 — another hardcoded fallback chain

**Partially fixed**: Global config merge now brings `default_model` through, but
the fallback hierarchy still has Claude assumptions everywhere.

### 3. Dispatch Path Fragmentation

**Mori**: One spawn path per backend type (Claude CLI, Cursor ACP, Codex WS).
All go through the same agent lifecycle.

**Roko**: SIX separate dispatch paths in `dispatch_agent()` (run.rs:465-637):
1. Routing config exists → `spawn_agent_scoped()` with full provider adapter
2. Claude CLI + ANTHROPIC_API_KEY → `run_anthropic_api_tool_loop()` (HTTP)
3. Claude CLI (bare) → subprocess with hardcoded config
4. Ollama → `run_ollama_agentic_single()`
5. Known protocols (gemini, glm) → `synthesize_known_protocol_config()`
6. Generic subprocess → raw command execution

Each path has different:
- Tool handling
- Error reporting
- Response extraction
- Cost tracking
- Event emission

**Fix needed**: Consolidate to at most 2-3 paths that all go through the
provider adapter interface.

### 4. Response Text Extraction

**Mori** (`agent/connection.rs:3141-3243`):
- Parses `ClaudeStreamEvent` typed variants: `System`, `Assistant`, `Tool`, `Result`
- Text from `Assistant` events → agent output panel
- Tool outputs → command output panel
- `Result` event → session summary (cost, tokens, turns)

**Roko** (`chat.rs:386-489`, `translate/mod.rs:150-183`):
- `extract_clean_text()` tries 5 fallback strategies to extract text
- Stream-json parsing only looks for `/delta/text` and `/content_block/text`
- No handling for Tool events, Result events, or structured metadata
- Falls back to raw text if nothing matches

**Fix needed**: Port mori's typed `ClaudeStreamEvent` parsing to roko.

### 5. Max Tokens / Response Budget

**Mori**: Uses role-specific token budgets. Conductor gets less, Implementer gets more.

**Roko**:
- Anthropic API path: `max_tokens: 8192` (dispatch_direct.rs:150)
- Claude agent: `max_tokens: 4096` (claude_agent.rs:35)
- These are often too low for complex tasks

### 6. `--bare` Mode Usage

**Mori**: Uses `--bare` aggressively — reduces prompt overhead from ~55K to ~5K
tokens by skipping Claude Code's built-in context, plugins, and MCP auto-discovery.

**Roko**: Has `bare_mode` config option (`roko.toml` + global config) but it's
not consistently applied across all dispatch paths.

### 7. Tool Allowlist Per Role

**Mori**: Strict per-role tool restrictions:
- Conductor (read-only): `Read,Glob,Grep,WebFetch,WebSearch`
- Implementer: `Read,Glob,Grep,Edit,Write,Bash`
- Architect/Auditor: `Read,Glob,Grep,Bash,WebFetch,WebSearch`

**Roko**: Has `claude_tool_allowlist()` in run.rs but it's only used in the
Claude CLI subprocess path, not the routing path.

### 8. Fallback Model

**Mori**: Always passes `--fallback-model claude-haiku-4-5` to Claude CLI.
If the main model fails, it automatically falls back.

**Roko**: Has `fallback_model` config but it's only applied in one of the six
dispatch paths (Claude CLI bare path at run.rs:540-543).

### 9. Provider Health Tracking

**Mori**: Tracks consecutive failures per provider, with recovery windows and
pass rate history. Uses this for runtime model selection.

**Roko**: Has `CascadeRouter` with bandit-based routing but it's **never
consulted at runtime** (`wired-unproven` status). Model selection is static.

### 10. Session Resume / Thread ID

**Mori**: Captures `session_id` from Claude CLI `Result` event, passes it as
`--resume <session_id>` for next turn. Maintains conversation continuity.

**Roko**: Has resume support in some paths but it's inconsistent across the
six dispatch paths.

---

## The "Log Line" Problem

From the screenshot: `INFO roko_agent::provider: no explicit provider config — using defaults for known CLI model_key="claude-opus-4-6"`

This means:
1. The routing config found a model key `claude-opus-4-6`
2. But no explicit provider config existed for it
3. So it's using synthesized defaults (which may not match the user's intent)

The fix is to ensure the global config's providers and models are always
merged before model resolution.

---

## Immediate Action Items (P0)

### A. Fix Tool Output Visibility ✅ DONE
**Files**: `translate/mod.rs`, `dispatch_direct.rs`, `chat.rs`, `chat_inline.rs`
**What was done**:
- `BackendResponse::extract_text()` now includes Tool events inline with `[toolname]` prefix
- Added `extract_tool_outputs()` and `extract_session_id()` to `BackendResponse`
- `DispatchResult` now carries `tool_outputs: Vec<ToolOutput>` and `session_id: Option<String>`
- `dispatch_claude_cli()` captures Tool events (content/output field, 4KB truncation) and Result events (session_id)
- `extract_clean_text()` JSONL path handles `"tool"` event type
- Chat inline renders tool outputs above agent response with `⚙ toolname  preview (+N lines)` format
- One-shot mode (`roko "prompt"`) prints tool outputs to stderr
- 7 new tests covering tool output extraction, session_id, truncation, edge cases

### B. Unify Model Defaults
**File**: `crates/roko-cli/src/run.rs`
**What**: Replace all hardcoded model strings with `routing_config.agent.default_model`
fallback chain. Already partially done but needs completion.

### C. Fix dispatch_direct.rs for Chat Mode
**File**: `crates/roko-cli/src/dispatch_direct.rs`
**What**: The chat REPL uses `dispatch_direct` which has its own hardcoded defaults.
Should respect global config. Also needs tool output handling.

### D. Max Tokens Too Low
**Files**: `dispatch_direct.rs:150`, `claude_agent.rs:35`
**What**: Increase to 16384 or make configurable. 4096/8192 is too low for
implementation tasks.

### E. Provider Config Always Merged
**File**: `crates/roko-cli/src/config.rs`
**What**: `merge_global_providers` already done but needs to also merge
`agent.command`, `agent.effort`, `agent.bare_mode` etc.

---

## Medium-Term Action Items (P1)

### F. Consolidate Dispatch Paths
Reduce 6 paths in `dispatch_agent()` to 2:
1. Provider adapter path (for all configured providers)
2. Fallback CLI subprocess path (when no config exists)

### G. Wire CascadeRouter
Make `CascadeRouter` actually consulted at dispatch time for model selection.
Record routing observations for learning.

### H. Inference Gateway
Create a shared `ModelCallService` that all code paths use for LLM calls.
This unifies cost tracking, caching, event emission, and provider health.

### I. Episode Quality
Fix episode records to include: actual model used, provider name, files changed,
per-turn token counts (not just per-task summaries).

---

## Comparison Table

| Capability | Mori | Roko | Gap |
|-----------|------|------|-----|
| Tool output visible to user | Yes (separate panel) | Yes (inline + summary) | ✅ Fixed |
| Model selection | Health-based routing | Hardcoded fallbacks | High |
| Dispatch paths | 3 (claude/cursor/codex) | 6 (fragmented) | High |
| Stream-json parsing | Typed enum variants | Ad-hoc JSON pointer | Medium |
| Provider health tracking | Active (3-strike + recovery) | Built but unused | High |
| Session resume | Consistent across backends | Inconsistent | Medium |
| `--bare` mode | Always used | Config option, inconsistent | Low |
| Tool allowlist per role | Strict per-role | Only one path | Medium |
| Fallback model | Always `--fallback-model haiku` | Only one path | Medium |
| Max tokens | Role-based | Hardcoded 4096/8192 | Medium |
| Cost tracking | Unified per-run | Scattered across paths | High |
| Error classification | Rate limit/auth/timeout/context | Basic pattern match | Medium |

---

## Related Documents

- `tmp/mori-diffs/01-AGENT-DISPATCH.md` — dispatch architecture gaps
- `tmp/mori-diffs/29-CURRENT-RUNTIME-GAP-LEDGER.md` — runtime gap ledger
- `tmp/mori-diffs/30-ARCHITECTURAL-SIDE-EFFECT-AUDIT.md` — side-effect audit
- `tmp/mori-diffs/21-FEATURE-PARITY-MATRIX.md` — full parity matrix
- `tmp/mori-diffs/23-HANDOFF-OPEN-ITEMS.md` — open items from handoff

---

## §Connection Latency — Fresh HTTP Clients Per Request

### Observed Behavior

A simple "hello" to GLM-5.1 (OpenAI-compat) takes **7.2 seconds**. A follow-up takes
**15.6s**. A third turn takes **43.7s**. The model itself is fast — the overhead comes
from how roko manages HTTP connections.

### Root Cause: `reqwest::Client::new()` on every request

`reqwest::Client` manages an internal connection pool. Creating a new one per request
discards all pooled connections, forcing fresh DNS resolution + TLS handshake every turn.

**Every dispatch path has this bug:**

| Location | Path | Affects |
|---|---|---|
| `dispatch_direct.rs:372` | `dispatch_openai_compat` | All OpenAI-compat |
| `dispatch_direct.rs:290` | `dispatch_anthropic_api` | Anthropic API |
| `openai_compat_backend.rs:318` | `send_turn_streaming` | OpenAI-compat streaming |
| `model_call_service.rs:1047` | `create_agent_for_model` per message | All via MCS |

The non-streaming path (`send_turn`) correctly uses `self.poster.post_json()` via
`ReqwestPoster` which holds a persistent client. But the streaming path — which is the
one the chat REPL actually uses — bypasses the poster entirely.

### Additional Connection Issues

**No connect timeout** (`dispatch_direct.rs:371-389`):
The `ProviderConfig` schema has `connect_timeout_ms` and `ttft_timeout_ms` fields, but
the direct dispatch path ignores them. `reqwest::Client::new()` has no default connect
timeout — it waits indefinitely. A slow TLS handshake just hangs silently.

**Agent rebuilt per message** (`model_call_service.rs:1047`):
`ProviderCallCell::execute` creates a new agent for every message — new `ReqwestPoster`
(new `reqwest::Client`), new tool registry, new `ToolLoop`, new `ToolDispatcher`. All
rebuilt from scratch per turn.

**Config re-read per message** (`dispatch_direct.rs:74-107`):
`dispatch_via_model_call_service` re-reads `roko.toml` from disk and rebuilds the
`ModelCallService` on every call.

**Background serve awaited at startup** (`unified.rs:48`):
`spawn_background_serve()` is awaited before showing the first chat prompt, adding
~0.3-1s startup latency.

### Why GLM-5.1 Is Worst (But All Providers Are Affected)

GLM-5.1's endpoint is `https://open.bigmodel.cn` (China-hosted). TLS handshake from
outside China adds 2-4s per connection. With connection reuse this happens once; without
it, every single message pays the full cost.

US-based providers (Anthropic, OpenAI) still pay unnecessary overhead (~200-500ms per
fresh TLS handshake), but it's less visible.

### Anti-Patterns

1. **Throwaway clients**: `reqwest::Client` is designed to be created once and shared.
   Creating it per-request defeats connection pooling, HTTP/2 multiplexing, and TLS
   session resumption.
2. **Inconsistent reuse**: Non-streaming uses `ReqwestPoster` (persistent client);
   streaming bypasses it and creates a fresh client. Same operation, different behavior.
3. **Config fields not consumed**: `connect_timeout_ms` and `ttft_timeout_ms` exist in
   the schema but are ignored by the code that needs them most.
4. **Per-turn agent construction**: The model call service rebuilds everything from
   scratch each turn instead of maintaining a session-scoped agent.

### Root Cause Fix

1. **Single `reqwest::Client` per session** — created once at session start, configured
   with connect timeout and TLS settings, passed to all dispatch calls.
2. **Session-scoped agent** — `create_agent_for_model()` runs once per session (or per
   model switch), not per message.
3. **Provider config applied** — `connect_timeout_ms`, `ttft_timeout_ms` read during
   client construction and applied to `reqwest::ClientBuilder`.
4. **Streaming uses the poster** — streaming path uses the same `ReqwestPoster` / shared
   client as non-streaming.
5. **Config loaded once** — `roko.toml` read at startup and passed through.

### Checklist

- [ ] `dispatch_direct.rs`: Replace per-call `Client::new()` with session-scoped client
- [ ] `openai_compat_backend.rs`: Streaming path reuses `self.poster` client
- [ ] `dispatch_direct.rs`: Apply `connect_timeout_ms` from provider config
- [ ] `dispatch_direct.rs`: Apply `ttft_timeout_ms` from provider config
- [ ] `model_call_service.rs`: Support session-scoped agent reuse
- [ ] `dispatch_direct.rs`: Cache config instead of re-reading per message
- [ ] `unified.rs`: Non-blocking background serve startup
- [ ] Verify with latency measurements (before/after)

---

## §Chat Context Gap — No System Prompt, No Tools, No History

### Observed Behavior

When a user types "what is this repo?" in `roko` chat, the model responds with "please
share the URL" — it has no idea it's running inside a workspace. The model can't read
files, search code, or reference prior turns. This is because the chat REPL sends
**nothing but the raw user text** to the model.

### What's Missing

**No system prompt** (`chat_inline.rs:1295`):
`ChatSession.system_message` is initialized to `None`. The `/system` slash command can
set it (line 2134), but `dispatch_prompt()` at line 1457 **never reads it**. The field
is written and never consumed.

**No tools** (`dispatch_direct.rs`):
```
OpenAI-compat (line 366): {"model":..., "messages":[...], "max_tokens":8192}
Anthropic API (line 285): {"model":..., "max_tokens":8192, "messages":[...]}
Claude CLI (line 140):    claude --print --output-format stream-json
```
Zero tool definitions in any path. Grep for `"tools"` in `dispatch_direct.rs`: zero
matches. The 19+ builtin tools in `roko-std`, `ToolRegistry`, `claude_tool_allowlist()`
— none wired into the chat path.

**No conversation history** (`chat_inline.rs:1457`):
Each call to `dispatch_prompt` sends only the current turn's text. The
`session.conversation` vector records messages for display/export but is **never sent
to the model**. Each API call is a single-message request. For Claude CLI, each turn
spawns a new subprocess — no session continuity.

**No workspace context**:
The welcome banner shows the workspace path to the user but this is never included in
any message to the model.

### Provider Comparison

| Provider | System Prompt | Tools | History | Workspace |
|---|---|---|---|---|
| Claude CLI | Not sent | **Implicit** (CLI built-ins) | None (new process/turn) | None |
| Anthropic API | Not sent | None | None | None |
| OpenAI-compat | Not sent | None | None | None |

Claude CLI appears less broken only because it has its own built-in tools — but it still
gets no system prompt or workspace context from roko.

### Contrast With Orchestrator Path

| Feature | Orchestrator (plan run) | Chat REPL |
|---|---|---|
| System prompt | 9-layer `SystemPromptBuilder` | None |
| Tools | `ToolRegistry` + `AgentOptions.tools` | None |
| MCP config | Passthrough via `AgentOptions` | None |
| Conversation history | Managed per-agent | Stateless per turn |
| Workspace context | Git status, files, task context | None |
| Knowledge store | Queried for routing + enrichment | Not used |

### Anti-Patterns

1. **Dead field**: `session.system_message` is write-only. Set by `/system`, ignored by
   dispatch. Code smell — a field that participates in no logic.
2. **Two parallel dispatch worlds**: The orchestrator has rich dispatch (system prompt,
   tools, MCP, knowledge, feedback). The chat REPL has a bare shim. They should share
   infrastructure.
3. **Display vs model confusion**: The session records history for the UI but treats the
   model as stateless. Same data should serve both purposes.
4. **Works-by-accident masking**: Claude CLI's implicit tools make one path appear to
   work, hiding the fact that dispatch provides nothing.

### Root Cause Fix

1. **System prompt at session start** — lightweight `SystemPromptBuilder` invocation
   including workspace path, git branch, directory listing, available tools, role.
2. **Tools for all providers** — `ToolRegistry` builds definitions; Anthropic API includes
   `"tools"` in body; OpenAI-compat includes `"tools"` (function calling format); Claude
   CLI gets `--allowedTools` + `--system-prompt`.
3. **Conversation history per turn** — accumulate messages, send full array each call,
   with context windowing for token limits.
4. **Shared dispatch layer** — `dispatch_direct.rs` should call the same `AgentOptions` +
   provider adapter pipeline as the orchestrator.
5. **`/system` actually consumed** — field merged with default system prompt at dispatch.

### Checklist

- [ ] Build system prompt with workspace context at session start
- [ ] Wire `session.system_message` into dispatch
- [ ] Add tool definitions to Anthropic API requests
- [ ] Add tool definitions to OpenAI-compat requests
- [ ] Add `--allowedTools` and `--system-prompt` to Claude CLI dispatch
- [ ] Send conversation history with each turn
- [ ] Implement context windowing for long conversations
- [ ] Wire MCP config passthrough for chat
- [ ] Share dispatch infrastructure between chat and orchestrator
- [ ] Integration test: model receives system prompt
- [ ] Integration test: model can call tools
- [ ] Integration test: model sees prior conversation turns

---

## §Savings Baseline — Misleading "17x savings" Metric

### Observed Behavior

The session footer shows:
```
◆ session  3 turns  ·  $0.0112 total  ·  17.0x savings vs baseline
```

This number is real but misleading. It compares actual cost against a hypothetical "every
token processed by Claude Opus at full price with no caching."

### How It's Calculated

```
savings_ratio = naive_baseline / total_cost

naive_baseline = (input_tokens × $15/M) + (output_tokens × $75/M)   // hardcoded Opus
total_cost     = CostTable.calculate(actual_model, tokens)           // actual model rates
```

**Data flow:**
```
DispatchResult → naive_opus_cost() → CostMeter.naive_baseline (accumulates)
             → cost_from_result()  → CostMeter.total_cost (accumulates)
             → savings_ratio() = naive_baseline / total_cost → display if > 1.5x
```

### Key Files

| Function | Location | What |
|---|---|---|
| `naive_opus_cost()` | `chat_inline.rs:3516-3518` | Hardcoded $15/$75 per M tokens |
| `cost_from_result()` | `chat_inline.rs:3521-3531` | Looks up actual model pricing |
| `CostMeter::savings_ratio()` | `cost_meter.rs:74-81` | `naive_baseline / total_cost` |
| `CostMeter::record_run()` | `cost_meter.rs:46-53` | Accumulates both costs |
| `CostTable::with_defaults()` | `cost_table.rs:99-122` | 8 hardcoded model prices |
| Display logic | `chat_inline.rs:1170-1196, 1416-1439` | Shown if ratio > 1.5 |

### Hardcoded Pricing Table

| Model | Input $/M | Output $/M |
|---|---|---|
| claude-opus-4-6 | 15.00 | 75.00 |
| claude-sonnet-4-6 | 3.00 | 15.00 |
| claude-haiku-4-5 | 0.80 | 4.00 |
| glm-5.1 | 1.40 | 4.40 |
| glm-5 | 1.00 | 3.20 |
| kimi-k2.5 | 0.60 | 3.00 |
| gpt-5.2 | 2.00 | 8.00 |
| gpt-5.4 | 3.00 | 12.00 |

### Issues

1. **Misleading comparison**: Comparing GLM-5.1 ($1.40/$4.40) against Opus ($15/$75) is
   apples-to-oranges. No user was going to use Opus for this workload.
2. **Hardcoded pricing will go stale**: `naive_opus_cost()` hardcodes $15/$75. Pricing
   changes silently make this wrong.
3. **Silent failure for unknown models**: `CostTable::calculate()` at `cost_table.rs:37`
   returns `$0.00` for unknown model slugs. If the API returns `glm-5.1-0228` instead of
   `glm-5.1`, actual cost is $0 and the metric is silently hidden.
4. **No cache token accounting**: Baseline uses full-price input. Actual cost may benefit
   from caching, making the comparison even more skewed.
5. **Duplicated rendering**: Session summary appears at two locations in `chat_inline.rs`
   (lines 1170-1196 and 1416-1439) with identical logic.

### Anti-Patterns

1. **Hardcoded pricing in application code**: Should come from config or updatable source.
2. **Apples-to-oranges baseline**: Always comparing against the most expensive model
   inflates the metric.
3. **Silent zero fallback**: Unknown models produce $0, hiding the metric instead of
   flagging a data gap.
4. **Copy-pasted rendering**: Two identical code blocks for session summary.

### Root Cause Fix

1. **Baseline = user's configured default** or **just show absolute cost** — only show
   savings when CascadeRouter actively routes to cheaper models.
2. **Pricing from config**: Load from `roko.toml` or a TOML pricing file, hardcoded
   defaults as fallback only.
3. **Fuzzy model matching**: `CostTable::calculate()` tries prefix matching before $0.
4. **Cache-aware baseline**: Use cached pricing rates for a fairer comparison.
5. **Single rendering function**: Extract session summary into one function.

### Checklist

- [ ] Replace hardcoded `naive_opus_cost()` with configurable baseline model
- [ ] Load pricing from config with hardcoded fallback
- [ ] Add fuzzy/prefix model matching in `CostTable::calculate()`
- [ ] Account for cache tokens in baseline
- [ ] Deduplicate session summary rendering code
- [ ] Warn/log when model slug has no pricing entry
- [ ] Unit test for savings ratio edge cases
