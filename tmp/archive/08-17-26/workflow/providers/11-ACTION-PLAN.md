# 11 — Prioritized Action Plan

## Dependency Graph

```
Config Resolution (P0)
  └→ Auth from Config (P0)
       └→ Dispatch Consolidation (P0)
            ├→ Unified Stream Parser (P0)
            ├→ Tool Output in All Paths (P0)
            ├→ Session Resume in Chat (P1)
            ├→ Per-Role Tools (P1)
            └→ CascadeRouter Wiring (P1)
                 └→ Provider Health (P2)

Token Limits Fix (P0) — independent
Cost Tracking Fix (P0) — independent
Stderr Classification (P1) — independent
DOA Detection (P1) — independent
```

---

## Phase 1: Make Chat Work Right (P0)

### 1A. Fix token limits
**Effort**: Small (< 1 hour)
**Files**: `dispatch_direct.rs`, `anthropic_api/tool_loop.rs`, `gateway.rs`
**What**: Replace hardcoded 4096/8192 with 16384 or configurable value.
**Why**: Current limits truncate complex responses. Mori uses USD budgets, not token caps.

### 1B. Unified stream-json parser
**Effort**: Medium (2-4 hours)
**Files**: New `roko-agent/src/translate/claude_stream.rs`
**What**: Port mori's `ClaudeStreamEvent` typed enum. Single parse function.
All Claude CLI paths call it. Remove duplicate parsers from:
- `translate/mod.rs:extract_text()` stream branch
- `chat.rs:extract_clean_text()` JSONL branch
- `dispatch_direct.rs:dispatch_claude_cli()` event loop
**Why**: Three separate parsers = three different behaviors. Must be one.

### 1C. Tool output in all Claude paths
**Effort**: Small (depends on 1B)
**Files**: All dispatch paths that spawn Claude CLI
**What**: After 1B, all paths automatically get tool output via shared parser.
Currently only `dispatch_direct.rs` captures tool outputs.
**Why**: User sees different results depending on which path is hit.

### 1D. Auth detection from config
**Effort**: Medium (2-4 hours)
**Files**: `auth_detect.rs`, `unified.rs`, `chat_inline.rs`
**What**: `detect_auth()` should accept loaded config. Check `[agent].default_backend`
first. Look up provider's `api_key_env`. Fall back to current env-var scanning.
**Why**: User's `default_backend = "zai"` and `default_model = "glm-5.1"` are ignored.
Config is loaded and thrown away.

### 1E. dispatch_direct uses config
**Effort**: Medium (depends on 1D)
**Files**: `dispatch_direct.rs`
**What**: `dispatch_prompt()` accepts resolved provider config. Uses config's
base_url, model, api_key_env instead of hardcoded values.
**Why**: dispatch_direct.rs is ~400 LOC duplicating roko-agent with different defaults.

### 1F. Cost tracking
**Effort**: Small (1-2 hours)
**Files**: `chat_inline.rs`, `dispatch_direct.rs`
**What**: Pass `total_cost_usd` from Claude Result events to CostMeter.
For API paths, compute cost from token counts * known pricing.
**Why**: Cost always shows $0.0000 currently.

---

## Phase 2: Consistency Across Paths (P1)

### 2A. Session resume in chat
**Effort**: Small (1-2 hours)
**Files**: `chat_inline.rs`, `dispatch_direct.rs`
**What**: Store `session_id` in `ChatSession`. Pass to next `dispatch_prompt()` call.
`dispatch_claude_cli()` passes `--resume <session_id>` if available.
**Why**: Each chat message is currently a fresh conversation with no history.

### 2B. Per-role tool enforcement
**Effort**: Medium (2-4 hours)
**Files**: `run.rs`, `dispatch_direct.rs`, tool handling across all paths
**What**: `claude_tool_allowlist()` applied in all paths, not just one.
Conductor is read-only. Effort-based filtering for Implementer.
**Why**: Agents currently have unrestricted tool access regardless of role.

### 2C. Wire CascadeRouter
**Effort**: Large (4-8 hours)
**Files**: `run.rs`, `dispatch_direct.rs`, `roko-learn/cascade_router`
**What**: Before dispatch, consult CascadeRouter for model selection.
After dispatch, record routing observation (success/failure, latency, cost).
**Why**: No adaptive model routing — always uses static defaults.

### 2D. Fallback model in all paths
**Effort**: Small (1-2 hours)
**Files**: All Claude CLI spawn sites
**What**: Always pass `--fallback-model` when spawning Claude CLI.
Currently only one path does this.
**Why**: No automatic fallback on primary model failure.

### 2E. Stderr classification
**Effort**: Medium (2-4 hours)
**Files**: New filter function, all subprocess spawn sites
**What**: Port mori's benign_stderr_summary concept. Classify known harmless
stderr lines. Only surface important errors.
**Why**: Chat gets flooded with unhelpful stderr messages.

### 2F. DOA detection
**Effort**: Small (1-2 hours)
**Files**: Subprocess spawn wrappers
**What**: If agent process exits within 2 seconds, classify as DOA.
Report specific error (binary missing, auth failed, etc.) instead of generic failure.
**Why**: Silent failures on misconfigured backends.

---

## Phase 3: Architecture (P0 but large)

### 3A. Consolidate dispatch paths
**Effort**: Large (1-2 days)
**Files**: `run.rs`, new `dispatch/mod.rs`
**What**: Reduce 6+ paths in `dispatch_agent()` to 2:
1. Provider adapter path (all configured providers via `create_agent_for_model()`)
2. Subprocess fallback path (unconfigured commands)
**Why**: Each path has different behavior for everything. Must be one path with one behavior.

### 3B. Inference gateway
**Effort**: Large (2-3 days)
**Files**: New `ModelCallService` shared across all entry points
**What**: Single boundary for all LLM calls: runner, chat, HTTP, research, dreams,
neuro, vision. Unified cost tracking, caching, provider health, credential isolation.
**Why**: 9+ separate code paths make LLM calls independently with no shared state.

### 3C. Config consolidation
**Effort**: Large (1-2 days)
**Files**: `roko-core/config/schema.rs`, `roko-cli/config.rs`
**What**: Single `ResolvedRuntimeConfig` produced once, passed everywhere.
Remove duplicate config models.
**Why**: Two config systems, neither authoritative. Runtime ignores loaded config.

---

## Status of Previous Fixes

| Item | Status | Doc |
|------|--------|-----|
| Tool output capture in dispatch_direct | Done | 03-TOOL-OUTPUT.md |
| Tool output rendering in chat_inline | Done | 03-TOOL-OUTPUT.md |
| session_id capture in dispatch_direct | Done | 05-SESSION-RESUME.md |
| extract_text() includes Tool events | Done | 03-TOOL-OUTPUT.md |
| extract_tool_outputs() method added | Done | 03-TOOL-OUTPUT.md |
| extract_session_id() method added | Done | 05-SESSION-RESUME.md |
| Global config merges default_model/default_backend | Done | 07-CONFIG-AUTH.md |
| Auth detection reordered (ZAI first) | Done | 07-CONFIG-AUTH.md |
| TOOL symbol added to symbols.rs | Done | 03-TOOL-OUTPUT.md |

---

## Cross-References

| Topic | Primary Doc | Also In |
|-------|------------|---------|
| Dispatch fragmentation | 01-DISPATCH-PATHS.md | tmp/mori-diffs/01-AGENT-DISPATCH.md |
| Model hardcoding | 02-MODEL-SELECTION.md | tmp/binary-issues/12-PROVIDER-DISPATCH-AUDIT.md |
| Tool output | 03-TOOL-OUTPUT.md | tmp/binary-issues/12-PROVIDER-DISPATCH-AUDIT.md |
| Streaming protocol | 04-STREAMING-PROTOCOL.md | tmp/mori-diffs/29-CURRENT-RUNTIME-GAP-LEDGER.md |
| Config bypass | 07-CONFIG-AUTH.md | tmp/mori-diffs/33-CONFIGURATION-PROVIDER-POLICY-AUDIT.md |
| Cost tracking | 06-TOKEN-LIMITS-COST.md | tmp/binary-issues/03-UNIFIED-CHAT-BLOCKERS.md |
| Provider health | 09-ERROR-HANDLING.md | tmp/mori-diffs/29-CURRENT-RUNTIME-GAP-LEDGER.md |
