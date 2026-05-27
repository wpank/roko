# Full Context: Roko Audit, Diagnosis, and Fix Plan

**Date**: 2026-04-28
**Author**: Claude (multi-session investigation)
**Purpose**: Give a fresh agent everything it needs to make roko's interactive chat work
as well as mori's. Read this before touching any code.

---

## Table of Contents

1. [What Is Roko](#1-what-is-roko)
2. [What Is Mori (The Reference)](#2-what-is-mori-the-reference)
3. [How We Got Here](#3-how-we-got-here)
4. [The Core Problem (Simple Version)](#4-the-core-problem-simple-version)
5. [The Evidence: Audit Trail](#5-the-evidence-audit-trail)
6. [What Mori Does (That Works)](#6-what-mori-does-that-works)
7. [What Roko Does (That's Broken)](#7-what-roko-does-thats-broken)
8. [The 11 Systemic Problems](#8-the-11-systemic-problems)
9. [The Fix (Shortest Path to Mori Parity)](#9-the-fix-shortest-path-to-mori-parity)
10. [Source Directories (Where Everything Lives)](#10-source-directories-where-everything-lives)
11. [Solution Files in This Folder (History)](#11-solution-files-in-this-folder-history)
12. [What NOT to Do](#12-what-not-to-do)
13. [Key Files to Read](#13-key-files-to-read)
14. [After Mori Parity: What's Next](#14-after-mori-parity-whats-next)

---

## 1. What Is Roko

Roko is a Rust toolkit for building agents that build themselves. 18 crates, ~177K LOC.
It reads PRDs, generates implementation plans, executes tasks via Claude agents, validates
with gates, and persists results. The plan-execute-gate-persist loop works end-to-end for
automated plan execution (`roko plan run`).

**The problem is the interactive experience.** When a user types `roko` to chat with an
agent (like running `claude` to use Claude Code), things are fundamentally broken:
- No system prompt sent to the model
- No tools available to the model
- No conversation history (each turn is stateless)
- No streaming (entire response appears at once after 7-60 seconds)
- No workspace context (model doesn't know what repo it's in)

**Workspace root**: `/Users/will/dev/nunchi/roko/roko/`
**Branch**: `wp-arch2`

---

## 2. What Is Mori (The Reference)

Mori is the original orchestrator that roko replaces. 108K LOC, single binary. It works.
Roko's goal is to be as effective, reliable, and featured as mori for interactive agent use.

**Mori source**: `/Users/will/dev/uniswap/bardo/apps/mori/`
**Key file**: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs`
  - Lines 2444-2620: Agent subprocess spawn (the reference implementation)
  - Lines 2659-2731: Stream-json parsing and event emission
  - Lines 3141-3243: `parse_claude_event()` function

Mori's dispatch is ~300 lines that correctly spawn a `claude` subprocess. That's the
baseline roko needs to match.

---

## 3. How We Got Here

This document is the product of a multi-session investigation that went through these phases:

### Phase 1: Latency Diagnosis
User showed roko chat REPL with 7.2s latency on GLM-5.1 (a Chinese-hosted model).
Investigation found `reqwest::Client::new()` created per API call, discarding the
connection pool. Fresh DNS + TLS handshake every request.

### Phase 2: Missing Context Diagnosis
User asked why the model has no access to the repo. Investigation found the chat path
(`dispatch_direct.rs`) sends bare `{"messages":[{"role":"user","content":"..."}]}` with
no system prompt, no tools, no history. The rich infrastructure (9-layer prompt builder,
tool registry, MCP, knowledge store) exists in the orchestrator path but was never wired
into chat.

### Phase 3: Full Codebase Audit
Launched 5 parallel agents to audit chat lifecycle, plan execution, config/errors,
serve/security, and safety/learning. Created docs 16-20 in `tmp/binary-issues/`.

### Phase 4: Master Index
Organized ~90 individual bugs into 11 systemic root causes (S1-S11). Created
`tmp/binary-issues/MASTER-INDEX.md`.

### Phase 5: Effort Estimation
Estimated ~50-65 hours to fix everything. Wrote to `tmp/solutions/effort-estimates.md`.

### Phase 6: Cross-Directory Analysis
Analyzed 4 audit directories (binary-issues, mori-diffs, subsystem-audits, converge-runner).
Found massive overlap — all describe the same ~11 problems from different angles.

### Phase 7: Over-Engineered Solutions (Wrong Direction)
Created 6 solution proposals (A, B, C, 1, 2, 3) proposing increasingly complex architectures:
InferenceGateway structs, Service Triads, Cell/Graph engines. All wrong direction — building
infrastructure instead of wiring what exists. These are preserved in this folder as history.

### Phase 8: Mori Comparison (The Answer)
Actually read mori's dispatch code. Discovered the difference is **4 CLI flags** that roko
doesn't pass to the claude subprocess. Created `solution-ACTUAL.md`.

### Phase 9: Demo App Workflow Audit
Ran the current `demo/demo-app` product surface end-to-end through browser routes, browser
workflows, direct CLI scenario commands, and backend API probes. Created
`DEMO-APP-WORKFLOW-AUDIT.md`. This added a key implementation constraint: the demo UI cannot be
trusted as proof until silent fallback, terminal scraping, API contract drift, ambiguous workdirs,
and fake success states are fixed.

---

## 4. The Core Problem (Simple Version)

**Mori spawns claude with:**
```
claude --print --stream-json \
  --append-system-prompt "You are an implementer..." \
  --tools "Read,Glob,Grep,Edit,Write,Bash" \
  --mcp-config ".mori/mcp-config.json" \
  --resume "session_abc123"
```

**Roko spawns claude with:**
```
claude --print --stream-json
```

**That's it.** No system prompt. No tools. No MCP. No session continuity.

Claude CLI handles conversation history (`--resume`), tool execution (built-in), and
streaming (stream-json) **for free**. Roko just doesn't ask it to.

For the Anthropic API and OpenAI-compat paths, the same problem exists differently:
the HTTP request body has no `system` field, no `tools` array, and only one message
(no conversation history).

**Everything needed to generate these values exists in the codebase:**
- System prompt: `roko-compose/src/prompt_assembly_service.rs` (9-layer builder, works today)
- Tool restrictions: `roko-agent/src/safety/contract.rs` (role-based, works today)
- MCP config: `.roko/mcp-config.json` auto-discovery (works today for plan runs)
- Session ID: Already parsed from stream-json output (just never stored/reused)

---

## 5. The Evidence: Audit Trail

All audit documentation lives in `tmp/`. Here's what exists and what it covers:

### `tmp/binary-issues/` (20 files)

User-facing bugs organized by what breaks when you use roko.

| File | What it covers |
|---|---|
| `00-AUDIT.md` | Overview, the 6-things-that-should-be-1, anti-patterns |
| `01-11` | Pre-existing UX docs (design, edge cases, chat blockers, etc.) |
| `12-PROVIDER-DISPATCH-AUDIT.md` | Connection latency (reqwest per-call), chat context gap (no system prompt/tools/history), savings calculation (hardcoded Opus pricing) |
| `16-CHAT-SESSION-LIFECYCLE.md` | 30 issues: startup, auth, slash commands (6 no-ops), stateless turns, no streaming, error handling |
| `17-PLAN-EXECUTION-WORKFLOW.md` | 24 issues: no codebase context, hardcoded permissions, two execution engines, no real-time feedback |
| `18-CONFIG-AND-ERRORS.md` | 17 issues: two init paths, empty env vars, 18+ `.ok()` calls silently swallowing errors |
| `19-SERVE-AND-SECURITY.md` | **CRITICAL**: unauthenticated terminal PTY, auth disabled by default, CORS permissive, public gists |
| `20-SAFETY-LEARNING-KNOWLEDGE.md` | LinUCB weights not persisted, dream triggers no consumer, episode compaction never triggered, cost tracking returns $0 |
| `MASTER-INDEX.md` | **THE CANONICAL TRACKER**: All ~90 issues organized into 11 systemic root causes (S1-S11) with checklists |

### `tmp/mori-diffs/` (40 files)

Architecture-level parity gaps between mori and roko.

| Key files | What they cover |
|---|---|
| `00-OVERVIEW.md` | Index of all gap clusters |
| `01-AGENT-DISPATCH.md` | How mori dispatches vs roko (the core gap) |
| `21-FEATURE-PARITY-MATRIX.md` | 1,253-item checklist, ~33% done |
| `18-MASTER-AUDIT.md` | Full audit of what's real vs phantom |
| GAP-01 through GAP-12 | 12 gap clusters covering dispatch, streaming, tools, sessions, cost, etc. |

### `tmp/subsystem-audits/` (17 subdirectories, ~90 files)

Per-subsystem deep dives with AUDIT.md, GOALS.md, ISSUES.md, PLAN.md each.

| Subdirectory | Key findings |
|---|---|
| `gateway/` (9 files) | Complete spec for `roko-gateway` crate: transparent HTTP proxy with routing, caching, 7 phases, 12 novel innovations |
| `inference-dispatch/` (4 files) | 13 separate call sites, 4 duplicate stream-json parsers, CascadeRouter never called from live paths, PlanRunner (21K LOC) is DEAD CODE |
| `prompt-assembly/` (4 files) | 9-layer SystemPromptBuilder works but only `roko plan run` and `roko run` use it; chat/ACP/sidecar bypass it entirely |
| `ux/` (8 files) | Board/Epic/Task hierarchy, 5 duplicate chat event loops, Symphony integration analysis, TrackerAdapter trait |
| `acp-protocol/` (5 files) | Clean FSM + Effect Driver for editor integration; missing CascadeRouter, episodes, safety, SystemPromptBuilder |
| `converge-runner/` (6 files) | 55 open issues (6 critical), most blocked on "no single dispatch path" |

### `tmp/unified/` (31 files) and `tmp/unified-depth/` (40 directories)

The target architecture spec. 5 primitives (Signal, Pulse, Cell, Graph, Protocol),
9 protocols, 2 fabrics (Store, Bus), 4 universal patterns. This is the long-term
vision for how roko should be structured, but it's NOT what needs to be built right
now to achieve mori parity.

### `tmp/solutions/` (this folder)

Solution proposals generated during this investigation, in chronological order:

| File | What it proposed | Why it's wrong |
|---|---|---|
| `effort-estimates.md` | Time estimates per systemic problem | Useful reference, still accurate |
| `solution-A-surgical.md` | 7 independent patches, ~35h | Treats symptoms, doesn't fix root cause |
| `solution-B-architectural.md` | InferenceGateway struct, ~55h | Over-engineered a local-process problem |
| `solution-C-phased-migration.md` | Grow dispatch_direct into gateway, ~50h | Right direction, too much infrastructure |
| `solution-1-service-triad.md` | 3 services (Gateway, Session, Prompt), ~80h | Massively over-scoped |
| `solution-2-cell-graph-engine.md` | Full Cell/Graph engine from unified spec, ~130h | Building a platform before the product works |
| `solution-3-hybrid-engine-first.md` | Thin engine + cells, ~90h | Still building infrastructure first |
| **`solution-ACTUAL.md`** | **Wire 4 CLI flags, ~25h** | **THE CORRECT ANSWER** |
| `E2E-ROKO-DOGFOOD-AUDIT.md` | Fresh end-to-end CLI dogfood | Shows execution contracts are broken under chat/PRD/plan |
| `REVISED-BEST-SOLUTION-AFTER-DEMO.md` | Refined first milestone after demo artifact audit | Adds grounded PRD/plan artifacts |
| `FINAL-SOLUTION.md` | Current shortest-path recommendation | Adds M0-0 execution contract repair |
| `DEMO-APP-WORKFLOW-AUDIT.md` | Full demo-app route/workflow/API/CLI audit | Adds UI proof-surface repair plan |

**Read `FINAL-SOLUTION.md` first for the implementation direction, then
`DEMO-APP-WORKFLOW-AUDIT.md` before touching the demo UI.** `solution-ACTUAL.md` is still the
important historical pivot, but later audits showed that execution contracts and demo proof
surfaces must be repaired before the product can be called Mori-like.

---

## 6. What Mori Does (That Works)

Mori's interactive agent dispatch (`connection.rs:2445-2622`):

1. **Spawns `claude` CLI subprocess** with:
   - `--bare` (skip Claude Code's 30K-token built-in prompt, use only the appended one)
   - `--print --verbose --output-format stream-json` (streaming JSON output)
   - `--model <slug>` (explicit model selection)
   - `--append-system-prompt <text>` (role-specific prompt with workspace context)
   - `--tools <whitelist>` (role-based tool restrictions)
   - `--mcp-config <path>` (MCP server for code intelligence)
   - `--resume <session_id>` (conversation continuity across turns)
   - `--settings <path>` (agent-specific hooks)

2. **Writes message to stdin**, then closes stdin (EOF signals end of input)

3. **Reads stdout line-by-line**, parses `stream-json` events:
   - `ClaudeStreamEvent::Assistant` → emit `MessageDelta` (streaming text to UI)
   - `ClaudeStreamEvent::Tool` → emit `CommandOutput` (tool results)
   - `ClaudeStreamEvent::Result` → emit `TurnCompleted` (extract `session_id` for next `--resume`)

4. **Event loop in orchestrator** consumes events, renders to UI

**What Claude CLI gives you for free:**
- Conversation history management (via `--resume`)
- Tool invocation and result handling (built-in tool execution)
- Streaming output (stream-json format)
- Context window management (internal to Claude CLI)

**Mori's system prompt** (`connection.rs:427-454`): Role-specific guidance (~200 lines) with
coding standards, rules, tool usage guidance, and artifact hints. Built by a simple
`format!()` — not a 9-layer builder. Effective because it actually gets sent.

**Mori's tool whitelisting** (`connection.rs:2485-2536`): Each `AgentRole` gets specific
tools:
- Conductor: Read, Glob, Grep, WebFetch, WebSearch (read-only)
- Implementer: Read, Glob, Grep, Edit, Write, Bash (code tools)
- Scribe: Read, Glob, Grep, Write, Edit, WebFetch, WebSearch (docs)

---

## 7. What Roko Does (That's Broken)

### The Chat Path (what happens when you type `roko`)

```
cmd_unified_chat() in unified.rs
  → detect_auth() → AuthMethod enum (ClaudeCli|AnthropicApi|OpenAiCompat)
  → chat_inline::run_unified_inline(&auth)
    → Creates ChatSession { dispatch: DispatchMode::Direct { auth } }
    → Event loop (33ms tick):
        User presses Enter → dispatch_prompt(&session, &text)
          → tokio::spawn → dispatch_direct::dispatch_prompt(&auth, &text)
            → match auth:
                ClaudeCli → dispatch_claude_cli(prompt)
                AnthropicApi → dispatch_anthropic_api(key, model, prompt)
                OpenAiCompat → dispatch_openai_compat(key, url, model, prompt)
          → Returns DispatchResult { text, model, tokens, tool_outputs, session_id }
          → Sends on mpsc channel
        Event loop polls channel → renders response
```

### The Claude CLI Path (`dispatch_direct.rs:141-279`)

```rust
Command::new("claude")
    .args(["--print", "--output-format", "stream-json"])
    // MISSING: --append-system-prompt
    // MISSING: --tools
    // MISSING: --mcp-config
    // MISSING: --resume
    // MISSING: --model
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()
```

Reads all stdout lines, buffers entire response, returns it. **No streaming to UI.**

The stream-json events ARE parsed (tool outputs and session_id extracted) but:
- `session_id` is returned in `DispatchResult` but **never stored or reused**
- `tool_outputs` are returned but tools only work because Claude CLI runs them itself
- The model gets zero context about what repo it's in or what role it has

### The Anthropic API Path (`dispatch_direct.rs:285-338`)

```rust
json!({
    "model": "claude-sonnet-4-6-20250514",  // HARDCODED
    "max_tokens": 8192,                      // HARDCODED
    "messages": [{"role": "user", "content": prompt}]  // SINGLE MESSAGE, NO HISTORY
    // MISSING: "system" field
    // MISSING: "tools" field
})
```

Fresh `reqwest::Client::new()` per request. No connection reuse. No streaming (`"stream": true`
not set). Single message (no conversation history). No system prompt.

### The OpenAI-Compat Path (`dispatch_direct.rs:366-418`)

Same problems as Anthropic API: fresh client, single message, no system, no tools, no streaming.

### What Exists But Isn't Called

| Infrastructure | Location | Status |
|---|---|---|
| 9-layer SystemPromptBuilder | `roko-compose/src/system_prompt_builder.rs` (2081 LOC) | Works, used by `roko plan run`, NOT by chat |
| PromptAssemblyService | `roko-compose/src/prompt_assembly_service.rs` | Works, NOT called from chat |
| 11 role templates | `roko-compose/src/templates/` | Built, used by orchestrator only |
| AgentContract (role-based tools) | `roko-agent/src/safety/contract.rs` | Works, NOT consulted by chat |
| ToolDispatcher | `roko-agent/src/dispatcher/mod.rs` | Works, NOT called from chat |
| 19 builtin tools | `roko-std/src/tool/builtin/` | Built, NOT exposed to chat |
| MCP auto-discovery | `roko-agent/src/mcp/` | Works for plan runs, NOT for chat |
| CascadeRouter | `roko-learn/src/cascade_router.rs` | Built, NEVER called from any live path |
| EpisodeLogger | `roko-learn/src/` | Works, NOT called from chat |
| ModelCallService | `roko-agent/src/model_call_service.rs` | Exists, only used in one-shot mode |
| StreamingState | `roko-cli/src/inline/primitives/streaming.rs` | Built, never connected |
| Knowledge store | `roko-neuro/src/` | Built, NOT queried from chat |

**177K LOC of infrastructure. None of it called from the interactive chat path.**

---

## 8. The 11 Systemic Problems

From `tmp/binary-issues/MASTER-INDEX.md` — the canonical tracker:

| # | Problem | Severity | Effort | Summary |
|---|---|---|---|---|
| **S1** | Dispatch is a thin pipe | Critical | 10-15h | Chat sends bare text, no prompt/tools/history/context |
| **S2** | Throwaway HTTP clients | High | 2-3h | Fresh `reqwest::Client::new()` per API call |
| **S3** | Confirmation theater | Medium | 3-4h | 6 slash commands confirm success but do nothing |
| **S4** | Silent error swallowing | Medium | 4-6h | 18+ `.ok()` calls, errors silently discarded |
| **S5** | Security-off-by-default | **Critical** | 2-3h | Unauthenticated PTY, no CORS, public gists |
| **S6** | No streaming | High | 8-12h | Full response appears at once after 7-60s spinner |
| **S7** | Hardcoded values | Low | 2-3h | Model names, URLs, pricing scattered as literals |
| **S8** | Phantom features | Medium | 3-4h | Built but never wired (compaction, dreams, bandit weights) |
| **S9** | Subprocess management | Medium | 3-4h | No timeouts, stderr inherited, no cancellation |
| **S10** | Duplicate code / two engines | Low | 10-15h | orchestrate.rs (21K LOC) + runner v2 coexist |
| **S11** | Mutex/unwrap risks | Low | 1-2h | `std::sync::Mutex` with `.expect("poisoned")` |

**S1 is the root cause of the user-facing brokenness.** Fix S1 and the chat experience
goes from "useless text pipe" to "working agent with tools and context."

---

## 9. The Fix (Shortest Path to Mori Parity)

**Full details in `FINAL-SOLUTION.md`** (supersedes `solution-ACTUAL.md`).

Key refinement from review agent: do NOT grow `dispatch_direct.rs`. Route through existing
`ClaudeCliAgent` (which already has the correct flags) and provider adapters (which already
have tool loops). Create a small `ChatAgentSession` wrapper that owns session state.

Summary:

### Batch 1: Wire the Claude CLI Path (4-6h)

Pass the missing flags to `dispatch_claude_cli()` in `dispatch_direct.rs`:

- `--model`: from `roko.toml` config (already loaded in `unified.rs`)
- `--append-system-prompt`: from `PromptAssemblyService` (build once at session start)
- `--tools`: from `AgentContract` role-based restrictions (or config)
- `--mcp-config`: from `.roko/mcp-config.json` (auto-discovery already works)
- `--resume`: from previous turn's `DispatchResult.session_id` (already parsed, just not stored)

In `chat_inline.rs`: store `session_id` between turns, build system prompt at session
start, re-build when `/system`/`/model`/`/mode` change.

### Batch 2: Wire the API Paths (4-6h)

For Anthropic API and OpenAI-compat:
- Add `"system"` field to request body (from PromptAssemblyService)
- Add `"tools"` field (formatted as provider-specific tool schemas)
- Send full conversation history (not just last message)
- Handle `tool_use` stop reason → execute tool → send `tool_result` → re-request
- Shared `reqwest::Client` (created once, not per-request)

### Batch 3: Streaming (6-8h)

- Claude CLI: forward stream-json events to UI as they arrive (not buffer-then-display)
- Connect `StreamingState` in `chat_inline.rs` to streaming channel
- Anthropic API: `"stream": true`, parse SSE events
- OpenAI-compat: `"stream": true`, parse SSE data chunks

### Batch 4: Security + Reliability (4-6h)

- Auth enabled by default in `roko-serve`
- Terminal routes behind auth middleware
- CORS restricted to localhost
- Private gists, LogScrubber on share payloads
- Timeout on Claude CLI subprocess (120s)
- CancellationToken for Ctrl+C during dispatch

**Total: ~25-30 hours to mori parity for interactive chat.**

---

## 10. Source Directories (Where Everything Lives)

### Audit Documentation (read these for context)

| Path | What | Files |
|---|---|---|
| `tmp/binary-issues/` | User-facing bugs, MASTER-INDEX | 19 files |
| `tmp/binary-issues/MASTER-INDEX.md` | **Canonical tracker** (S1-S11) | 1 file |
| `tmp/mori-diffs/` | Mori↔Roko parity gaps | 40 files |
| `tmp/subsystem-audits/` | Per-subsystem deep dives | ~90 files in 17 dirs |
| `tmp/subsystem-audits/gateway/` | Gateway crate spec (9 docs) | 9 files |
| `tmp/subsystem-audits/inference-dispatch/` | Dispatch unification | 4 files |
| `tmp/subsystem-audits/prompt-assembly/` | Prompt builder wiring | 4 files |
| `tmp/subsystem-audits/ux/` | UX architecture | 8 files |
| `tmp/subsystem-audits/acp-protocol/` | Editor integration | 5 files |
| `tmp/unified/` | Target architecture spec (v3.0) | 31 files |
| `tmp/unified-depth/` | Deep implementation detail | 40 directories |
| `tmp/solutions/` | Solution proposals (this folder) | 9 files |

### Roko Source Code (what you'll modify)

| Path | What | Key detail |
|---|---|---|
| `crates/roko-cli/src/dispatch_direct.rs` | **THE FILE TO FIX** — bare Claude CLI / API dispatch | ~500 LOC, missing flags |
| `crates/roko-cli/src/chat_inline.rs` | Chat REPL event loop | ~4500 LOC, `dispatch_prompt()` at line ~1456 |
| `crates/roko-cli/src/unified.rs` | Entry point for `roko` command | ~200 LOC, auth detection + session start |
| `crates/roko-cli/src/run.rs` | `roko run` one-shot mode | ~2400 LOC |
| `crates/roko-compose/src/prompt_assembly_service.rs` | 9-layer prompt builder | Works, needs to be called from chat |
| `crates/roko-compose/src/system_prompt_builder.rs` | The builder itself | ~2081 LOC, works |
| `crates/roko-agent/src/safety/contract.rs` | Role-based tool restrictions | Works, needs to be consulted from chat |
| `crates/roko-agent/src/model_call_service.rs` | Unified model call service | ~1762 LOC, only used in one-shot |
| `crates/roko-serve/src/lib.rs` | HTTP server | Security fixes needed here |
| `crates/roko-serve/src/routes/mod.rs` | Route mounting | Terminal routes bypass auth (line 137) |
| `crates/roko-serve/src/terminal.rs` | PTY terminal | Unauthenticated (security fix) |

### Mori Reference Code (read-only)

| Path | What |
|---|---|
| `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs` | Agent spawn reference (lines 2444-2620) |
| `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/events.rs` | AgentEvent enum |
| `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/mod.rs` | AgentPool + AgentConnection |

---

## 11. Solution Files in This Folder (History)

**Canonical implementation plan: `FINAL-SOLUTION.md`** — this is what to implement.

Supporting docs:
- `MY-TAKE-SHORTEST-PATH.md` — review agent's refinement (use existing adapters)
- `MORI-PARITY-BATCH-PLAN.md` — detailed batch plan with acceptance criteria
- `CLARIFYING-QUESTIONS.md` — decision questions (answered in FINAL-SOLUTION.md)
- `E2E-ROKO-DOGFOOD-AUDIT.md` — fresh-repo end-to-end command matrix and revised fix order

Historical proposals (read for investigation context, do NOT implement):

| File | Proposed approach | Estimate | Verdict |
|---|---|---|---|
| `effort-estimates.md` | Time estimates per S1-S11 | N/A | Useful reference |
| `solution-A-surgical.md` | 7 independent patches | ~35-40h | Treats symptoms |
| `solution-B-architectural.md` | InferenceGateway struct | ~55h | Over-engineered |
| `solution-C-phased-migration.md` | Grow dispatch into gateway | ~50h | Too much infrastructure |
| `solution-1-service-triad.md` | 3 deployable services | ~80h | Massively over-scoped |
| `solution-2-cell-graph-engine.md` | Cell/Graph engine (unified spec) | ~130h | Platform before product |
| `solution-3-hybrid-engine-first.md` | Thin engine + cells | ~90h | Still infrastructure-first |
| `solution-ACTUAL.md` | Wire 4 CLI flags + API fields | ~25-30h | Correct direction, wrong target file |
| **`FINAL-SOLUTION.md`** | **ChatAgentSession → existing adapters** | **~25-30h** | **IMPLEMENT THIS** |
| **`E2E-ROKO-DOGFOOD-AUDIT.md`** | **Execution contract repair before M0-A/M0-B** | **~1-2d prerequisite** | **READ BEFORE IMPLEMENTING** |

The progression shows how the investigation evolved: from diagnosing individual bugs →
organizing into systemic causes → proposing grand architectures → realizing the answer
is just wiring what exists → refining WHERE to wire (not dispatch_direct.rs, but through
existing adapter code that already works).

### Demo Run Findings (2026-04-28)

A test run of `roko prd draft new system-prompt-wiring` + `roko prd plan system-prompt-wiring`
at `/tmp/roko-demo-1777396797076` proved the diagnosis:

1. **Agent generated plan for greenfield crates** — created `roko-prompt` and
   `roko-orchestrate`. The demo workspace appears to be a temp workspace rather than the actual
   Roko source tree, so the precise root cause is a context-root contract failure plus missing
   artifact validation. If the feature was meant for Roko, the command needed the Roko repo
   context. If it was meant for a blank demo project, the artifact should have been labeled as a
   new-project plan instead of confident Roko-internal architecture.
2. **Cost tracking is $0.00** — two sessions ran 247s + 273s each with Claude Opus, both
   showing 0 tokens and $0 cost. Episode logger never extracts token counts from stream. (S4)
3. **Learning system has 0 observations** — cascade router role_table is statically seeded
   but `total_observations: 0`. Feedback loop is dead. (S8)
4. **PRD quality is excellent** — the generated spec has academic citations, mermaid diagrams,
   20 requirements, and a 20-task plan. The agent is maximally capable and minimally informed.

### E2E Dogfood Findings (2026-04-28)

A clean Rust test in `/private/tmp/roko-e2e-rust-yW9yhe` and a separate `roko run` test in
`/private/tmp/roko-e2e-run-1777399243` added a prerequisite to the final solution:

1. **Model selection is inconsistent across commands.** `prd plan`, `plan regenerate`, `run`,
   positional prompts, provider tests, and route explanations did not consistently honor
   `--model`.
2. **Generated plans can be invalid.** `prd plan greet-function` wrote a plausible
   `tasks.toml` that missed required `role` fields and used unconfigured `sonnet`/`haiku`
   aliases. `plan regenerate` did not fix it.
3. **Migration is necessary but not sufficient.** `config migrate` moved top-level `run` onto
   Claude CLI, but it stayed interactive, preserved `shell:true`, and did not prevent an
   implementer/autofix loop for a no-edit prompt.
4. **Runner can do useful work after manual repair.** After adding roles and replacing aliases
   with `claude-sonnet-4-6`, `plan run` implemented the requested Rust function and tests, but
   final gates still failed on doctest/clippy and auto-fix did not recover.
5. **Gate and telemetry records are not trustworthy enough.** Stub gates pass, legacy runs can
   pass on `shell:true`, PRD/plan cost is recorded as zero, and runner costs are double-counted.
6. **State paths disagree.** `roko resume` hardcodes `./plans` while generated PRD plans live in
   `.roko/plans`; status and plan list disagree with run-state.

This changes the first implementation move: before ChatAgentSession or PRD grounding, add
M0-0 Execution Contract Repair. Make model/provider selection, plan acceptance, gate status,
usage/cost telemetry, and resume paths true across every command surface.

---

## 12. What NOT to Do

These are lessons learned from this investigation:

1. **Don't build new infrastructure.** Roko has 177K LOC. The 9-layer prompt builder, tool
   registry, safety contracts, MCP discovery, CascadeRouter — all exist. The problem is
   wiring, not building.

2. **Don't create new crates for existing Roko concepts.** No `roko-gateway`, no `roko-engine`,
   no `roko-session`, no `roko-prompt`, no `roko-orchestrate` as a first move. The fix lives in
   routing chat through existing `ClaudeCliAgent` and provider adapters, plus grounding PRD/plan
   generation in the intended repo before accepting artifacts. The demo run proved that a missing
   or wrong repo context turns into duplicate architecture unless Roko stops it.

3. **Don't redesign the architecture.** The `tmp/unified/` spec is the long-term vision.
   It's not what needs to happen now. Wire the 4 flags first. Refactor toward the spec later.

4. **Don't touch `orchestrate.rs`.** It's 21K LOC of dead code (PlanRunner is never
   instantiated from any live CLI path). The live plan runner is `runner/event_loop.rs`.
   Leave the dead code alone — removing it is a separate, high-risk task.

5. **Don't build a tool loop for the Claude CLI path.** Claude CLI already executes tools
   internally. You only need a tool loop for the API paths (Anthropic, OpenAI-compat).

6. **Don't try to fix all 90 issues at once.** First fix the M0-0 execution contract:
   model/provider selection, schema v2 first-run config, generated plan validity, truthful
   gates, truthful telemetry, and resume/state pathing. Then fix S1 (dispatch wiring) and S5
   (security).

7. **Read `CLAUDE.md` in the repo root.** It has critical rules including "WIRE, don't build"
   and "Search before writing" — this codebase has duplicate implementations from parallel
   development.

---

## 13. Key Files to Read

**Before writing any code, read these files in this order:**

1. `CLAUDE.md` (repo root) — project rules and architecture
2. `tmp/binary-issues/MASTER-INDEX.md` — the 11 systemic problems with file:line references
3. `tmp/solutions/E2E-ROKO-DOGFOOD-AUDIT.md` — the latest command-matrix evidence
4. `tmp/solutions/FINAL-SOLUTION.md` — the implementation plan (supersedes solution-ACTUAL)
5. `crates/roko-agent/src/claude_cli_agent.rs:305-341` — the CORRECT command builder to use
6. `crates/roko-cli/src/dispatch_direct.rs` — the broken dispatch (do NOT grow this)
7. `crates/roko-cli/src/chat_inline.rs` — the chat event loop (dispatch_prompt at ~line 1456)
8. `crates/roko-cli/src/unified.rs` — the entry point
9. Mori reference: `/Users/will/dev/uniswap/bardo/apps/mori/src/agent/connection.rs:2444-2620`

**For security fixes, also read:**
8. `crates/roko-serve/src/lib.rs`
9. `crates/roko-serve/src/routes/mod.rs`
10. `tmp/binary-issues/19-SERVE-AND-SECURITY.md`

---

## 14. After Mori Parity: What's Next

Once the chat works (~25-30h), these are the next priorities in order:

1. **Wire CascadeRouter** — so model selection improves over time (currently never called
   from any live path)
2. **Episode recording in chat** — so the system learns from every conversation
3. **Knowledge injection** — query neuro store in system prompt (layer 7 anti-patterns)
4. **Plan execution streaming** — real-time feedback during `roko plan run`
5. **ACP integration** — wire SystemPromptBuilder, safety, learning into editor path
6. **Gateway crate** — for teams sharing API keys, centralized caching (long-term)
7. **Cell/Graph engine** — implement the unified spec (long-term, see `tmp/unified/`)

But none of those are prerequisites for "roko works like mori." The CLI flags get you
there first.

---

## Appendix: Pre-Commit Checklist

Before committing any changes:

```bash
cargo +nightly fmt --all
cargo clippy --workspace --no-deps -- -D warnings
cargo test --workspace
```

CI uses latest stable rustc (1.91+). Run `rustup update stable` if needed.

Never push directly to main. Always use a branch. Ask before any git/GitHub action.
