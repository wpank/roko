# Master Index — Binary Issues & Systemic Audit

**Last updated**: 2026-04-28
**Status**: 90+ distinct issues identified, organized by root cause

This document is the canonical tracker for everything wrong with the roko binary and its
supporting infrastructure. Issues are organized not by where they were found, but by **what
systemic problem they are symptoms of**.

---

## How To Read This Document

Each section describes a **systemic problem** — a design gap or anti-pattern that manifests
as multiple individual bugs. Under each systemic problem:

- **Symptoms**: the individual bugs, with file:line references
- **What good looks like**: the redesign that fixes all symptoms at once
- **Checklist**: trackable items

Items marked with a status:
- `[ ]` — open
- `[~]` — partially done
- `[x]` — verified fixed

---

## S1. The Dispatch Layer Is a Thin Pipe, Not an Agent Session

**Root cause**: The chat REPL was built as a proof-of-concept text pipe (send string, get
string back). The rich agent infrastructure (system prompts, tools, conversation history,
knowledge store, MCP, feedback) was wired into the orchestrator path but never into the
chat path. The result is two completely separate dispatch worlds: one that works (plan
execution) and one that's broken (interactive chat).

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S1.1 | No system prompt sent to any model | `dispatch_direct.rs:293,378` | 12, 16 |
| S1.2 | `/system` stores message but dispatch ignores it | `chat_inline.rs:2134` → `1457` | 12, 16 |
| S1.3 | No tools in any API path (zero `"tools"` in requests) | `dispatch_direct.rs` (entire file) | 12, 16 |
| S1.4 | No conversation history — each turn is stateless | `dispatch_direct.rs:293,378` | 12, 16 |
| S1.5 | No workspace context (model doesn't know the repo) | `dispatch_direct.rs` | 12, 16 |
| S1.6 | `roko run` has no codebase context (`workspace = "Single-shot..."`) | `run.rs:1124` | 17 |
| S1.7 | Knowledge store not queried in chat | — | 20 |
| S1.8 | MCP config not passed in chat path | — | 12 |
| S1.9 | `roko run` bypasses ModelCallService (TODO at run.rs:1271) | `run.rs:1271` | new |
| S1.10 | Config loaded but never used for dispatch | `unified.rs:29-42` | 03 |

### What good looks like

A **session-scoped agent** created once at startup:
- Holds the HTTP client, system prompt, tools, conversation history, model config
- System prompt built from `SystemPromptBuilder` (same as orchestrator, lighter weight)
- Tools registered via `ToolRegistry` for all providers
- Conversation history accumulated and sent with each turn (with context windowing)
- `/system`, `/model`, `/effort` mutate the session agent (and actually take effect)
- All dispatch paths (chat, run, plan) go through `ModelCallService`

### Checklist

- [ ] S1.1 — Build system prompt at session start (workspace, tools, role)
- [ ] S1.2 — Wire `session.system_message` into dispatch
- [ ] S1.3 — Add tool definitions to Anthropic API and OpenAI-compat requests
- [ ] S1.4 — Send conversation history with each turn
- [ ] S1.5 — Include workspace path, git branch, directory listing in prompt
- [ ] S1.6 — Inject codebase context in `roko run` (not static string)
- [ ] S1.7 — Query knowledge store in chat path
- [ ] S1.8 — Pass MCP config in chat dispatch
- [ ] S1.9 — Migrate `roko run` to use ModelCallService
- [ ] S1.10 — Pass loaded config to auth detection and dispatch

---

## S2. Throwaway HTTP Clients — No Connection Reuse

**Root cause**: Every API call creates `reqwest::Client::new()`, discarding the connection
pool. `reqwest::Client` is designed to be created once and reused — its internal pool is its
primary value. This was likely a quick-and-dirty implementation that was never revisited.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S2.1 | Fresh client per request in OpenAI-compat dispatch | `dispatch_direct.rs:372` | 12 |
| S2.2 | Fresh client per request in Anthropic API dispatch | `dispatch_direct.rs:290` | 12 |
| S2.3 | Fresh client in streaming path (bypasses ReqwestPoster) | `openai_compat_backend.rs:318` | 12 |
| S2.4 | Agent rebuilt per message in ModelCallService | `model_call_service.rs:1047` | 12 |
| S2.5 | Config re-read from disk per message (MCS path) | `dispatch_direct.rs:74-107` | 12 |
| S2.6 | No `connect_timeout` on direct dispatch | `dispatch_direct.rs:371-389` | 12 |
| S2.7 | Provider config `connect_timeout_ms` / `ttft_timeout_ms` exist but ignored | `dispatch_direct.rs` | 12 |

### What good looks like

One `reqwest::Client` per session, created at startup with proper timeouts, shared across
all dispatch calls via `Arc`. Streaming and non-streaming paths use the same client.
Provider config fields actually applied.

### Checklist

- [ ] S2.1-2 — Session-scoped client in `dispatch_direct.rs`
- [ ] S2.3 — Streaming path uses `self.poster` client
- [ ] S2.4 — Session-scoped agent reuse in ModelCallService
- [ ] S2.5 — Config loaded once, cached
- [ ] S2.6-7 — Apply timeout config to client builder

---

## S3. Confirmation Theater — Commands That Lie

**Root cause**: Slash commands were implemented as UI messages without backend wiring. The
pattern is: parse the command, print a confirmation, but change nothing. This creates the
most deceptive class of bug — the user explicitly configures behavior, gets confirmation,
and the system silently ignores it.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S3.1 | `/system` — sets field, never consumed | `chat_inline.rs:2134` | 12, 16 |
| S3.2 | `/effort` — prints "set to X", stores nothing | `chat_inline.rs:2245-2273` | 16 |
| S3.3 | `/gate` — prints hint, changes nothing | `chat_inline.rs:2304-2311` | 16 |
| S3.4 | `/config set` — prints confirmation, writes nothing | `chat_inline.rs:2449-2468` | 16 |
| S3.5 | `/run`, `/plan run`, `/prd idea`, `/research` — print CLI hints | `chat_inline.rs:2715-2821` | 16 |
| S3.6 | `roko learn tune gates` — `--dry-run` flag exists but command never writes | `learn.rs:84` | 20 |
| S3.7 | Demo speed control — displays speed but doesn't apply to playback | `Demo.tsx:21,252` | 09 |
| S3.8 | `useServerHealth` — reports "connected" on first failure | `useServerHealth.ts:22-29` | 19 |

### What good looks like

Commands either **do the thing** or **fail with an error**. Never confirm success without
acting. `/system` updates the agent's system prompt and it's sent. `/effort` maps to an
API parameter. `/run` executes inline with progress.

### Checklist

- [ ] S3.1 — Wire `/system` into dispatch
- [ ] S3.2 — Wire `/effort` into dispatch (map to API effort parameter)
- [ ] S3.3 — Wire `/gate` to modify runtime config
- [ ] S3.4 — Wire `/config set` to modify runtime config (and optionally persist)
- [ ] S3.5 — Execute `/run`, `/plan run`, `/prd idea`, `/research` inline
- [ ] S3.6 — Fix `tune gates` to apply changes (or remove --dry-run)
- [ ] S3.7 — Wire demo speed control to actual playback rate
- [ ] S3.8 — Remove investor demo hack (or gate behind env var)

---

## S4. Silent Error Swallowing

**Root cause**: The codebase uses `.ok()`, `let _ =`, and `eprintln!` to discard errors
throughout critical paths. The orchestrator alone has 18+ `.ok()` calls. The result is
silent data loss, corrupt state, and impossible-to-debug failures.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S4.1 | 18+ `.ok()` calls in orchestrate.rs | `orchestrate.rs` (multiple) | 18 |
| S4.2 | Episode JSONL write/flush errors discarded | `jsonl_logger.rs:76-77` | new |
| S4.3 | AffectPolicy persist failure discarded | `workflow_engine.rs:438` | new |
| S4.4 | Background serve failure logged at warn only | `unified.rs:141` | 16, 18 |
| S4.5 | Episode logger failure → eprintln, run continues | `run.rs:1063` | 18, 20 |
| S4.6 | Missing env var → empty string, delayed failure | `config.rs:2197` | 18 |
| S4.7 | PTY send_input errors silently discarded | `terminal.rs:359-362` | new |
| S4.8 | FS watcher JoinHandle discarded | `fswatcher.rs:27` | new |
| S4.9 | FeedbackSink optional — learning events dropped when absent | `model_call_service.rs:422` | new |
| S4.10 | REST events delivered twice on EventBus (known FIXME) | `lib.rs:1219` | new |

### What good looks like

Categorize every error into one of three buckets:
1. **Surface to user** — config errors, auth failures, dispatch errors
2. **Log and continue** — non-fatal subsystem failures (episode write, affect persist)
3. **Abort** — corruption risks, safety violations

Replace `.ok()` with explicit match + log. Add `#[deny(clippy::disallowed_methods)]` for
`.ok()` in hot paths. Fail-fast on config issues (empty env vars).

### Checklist

- [ ] S4.1 — Audit and replace `.ok()` in orchestrate.rs
- [ ] S4.2 — Log JSONL write failures (at minimum warn)
- [ ] S4.3 — Log affect persist failures
- [ ] S4.4 — Surface serve failure to user (not just tracing)
- [ ] S4.5 — Surface episode logger failure more prominently
- [ ] S4.6 — Fail-fast when env var interpolation produces empty values
- [ ] S4.9 — Make FeedbackSink mandatory (or loud warning when absent)
- [ ] S4.10 — Fix double event delivery

---

## S5. Security-Off-By-Default

**Root cause**: Every security control defaults to permissive. Auth, CORS, permissions,
terminal sandboxing — all disabled. The deployment path (`PORT` env var) makes things
worse by switching to public bind without enabling protections.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S5.1 | Auth disabled by default | `config/serve.rs:54-63` | 19 |
| S5.2 | Terminal PTY routes bypass auth entirely | `routes/mod.rs:137-138` | 19 |
| S5.3 | `PORT` env var → 0.0.0.0 without auth warning | `lib.rs:225-233` | 19 |
| S5.4 | CORS fully permissive by default | `routes/middleware.rs:426-437` | 19 |
| S5.5 | `--share` creates PUBLIC gists, unscrubbed | `share.rs:84` | 19 |
| S5.6 | Terminal accepts arbitrary commands from request | `terminal.rs:115-125` | 19 |
| S5.7 | No PTY session count limit — FD exhaustion | `terminal.rs:76` | new |
| S5.8 | `dangerously_skip_permissions: true` hardcoded | `commands/plan.rs:290` | 17 |
| S5.9 | Post-dispatch safety violations are warnings, not blocks | `safety/mod.rs:696` | 20 |
| S5.10 | `AgentContract::permissive()` still exists | `contract.rs:78` | 20 |
| S5.11 | Implementer can reach network via Python subprocess | implementer contract | 20 |

### What good looks like

**Secure by default, opt-in to permissive.** Auth enabled when binding to 0.0.0.0 (or
always). Terminal behind auth. CORS restricted to localhost. `--share` uses private gists.
Agent permissions start restricted and escalate with approval.

### Checklist

- [ ] S5.1 — Enable auth by default (or auto-enable on 0.0.0.0)
- [ ] S5.2 — Move terminal routes inside auth middleware
- [ ] S5.3 — Warn/block when PORT is set without auth enabled
- [ ] S5.4 — Restrict default CORS to localhost
- [ ] S5.5 — Default to private gists, run LogScrubber on payloads
- [ ] S5.6 — Terminal command allowlist
- [ ] S5.7 — Session count limit + TTL for PTY sessions
- [ ] S5.8 — Remove hardcoded `dangerously_skip_permissions`
- [ ] S5.9 — Promote secret leak / forbidden write to Block severity
- [ ] S5.10 — `#[cfg(test)]`-gate `AgentContract::permissive()`

---

## S6. No Streaming — The 2026 Spinner Problem

**Root cause**: The chat REPL and `roko run` both dispatch to a background task and wait
for the complete response. `StreamingState` infrastructure exists but is never connected.
The user sees a spinner for 7-60 seconds, then the entire response appears at once.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S6.1 | Chat response arrives all-at-once via mpsc | `chat_inline.rs:1457-1491` | 16 |
| S6.2 | `StreamingState` exists but is never fed | `inline/primitives/streaming.rs` | 16 |
| S6.3 | Claude CLI reads stream-json but buffers to end | `dispatch_direct.rs:141-279` | 16 |
| S6.4 | Anthropic API uses non-streaming request | `dispatch_direct.rs:285-338` | 16 |
| S6.5 | No real-time feedback during plan execution (without TUI) | `runner/event_loop.rs:338-345` | 17 |
| S6.6 | `roko run` v2 engine silent until completion | `run.rs:583-678` | 17 |

### What good looks like

Tokens stream to the terminal as they arrive. The `StreamingState` is connected to the
dispatch path. Claude CLI output is forwarded line-by-line. API calls use `"stream": true`.
Plan execution shows per-turn agent output.

### Checklist

- [ ] S6.1 — Connect `StreamingState` to dispatch path
- [ ] S6.3 — Forward Claude CLI stream-json to UI incrementally
- [ ] S6.4 — Use `"stream": true` for Anthropic API
- [ ] S6.5 — Print agent progress to terminal during plan execution
- [ ] S6.6 — Show real-time output for `roko run`

---

## S7. Hardcoded Values Scattered Across the Codebase

**Root cause**: Quick implementation used inline string literals for model names, URLs,
timeouts, and env var names. These were never consolidated into constants or config.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S7.1 | `"claude-sonnet-4-6-20250514"` in dispatch_direct | `dispatch_direct.rs:291` | 03 |
| S7.2 | `"gpt-4o"` fallback | `dispatch_direct.rs:374` | 03 |
| S7.3 | `"claude-opus-4-6"` at ~8 sites in orchestrate.rs | `orchestrate.rs:4982-12783` | new |
| S7.4 | `"https://api.anthropic.com/v1/messages"` | `dispatch_direct.rs:300` | 03 |
| S7.5 | `"2023-06-01"` API version | `dispatch_direct.rs:302` | 03 |
| S7.6 | `8192` max_tokens (inconsistent across paths) | `dispatch_direct.rs:295,379` | 03, 12 |
| S7.7 | `$15/$75` hardcoded Opus pricing in naive_opus_cost | `chat_inline.rs:3516` | 12 |
| S7.8 | CostTable defaults missing many models | `cost_table.rs:99-122` | 20 |
| S7.9 | Perplexity URL and model hardcoded | `web_search.rs:32,35` | new |
| S7.10 | PID file path hardcoded relative to cwd | `process/registry.rs:27` | new |
| S7.11 | Hardcoded model display string in `run.rs` | `run.rs:612` | 17 |

### What good looks like

Single source of truth: model defaults, URLs, timeouts, and pricing all come from config
with well-named constants as compile-time fallbacks. `roko config show` reveals effective
values.

### Checklist

- [ ] S7.1-3 — Consolidate model defaults into one constant or config field
- [ ] S7.4-5 — Move API URLs and versions to provider config
- [ ] S7.6 — Make max_tokens configurable (per-provider or per-role)
- [ ] S7.7 — Configurable baseline model for savings calculation
- [ ] S7.8 — Load pricing from config file, hardcoded as fallback only
- [ ] S7.9 — Make web search URL/model configurable

---

## S8. Phantom Features — Built But Never Wired

**Root cause**: Features were implemented (structs, methods, tests) but the call sites were
never added to the runtime. The code exists, CI passes, but the feature doesn't function.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S8.1 | `EpisodeLogger::compact()` — never auto-triggered | `episode_logger.rs:1086` | 20 |
| S8.2 | Dream trigger JSONL — no consumer exists | `runtime_feedback/dreams.rs:20` | 20 |
| S8.3 | LinUCB bandit weights — `save()` exists, never called | `model_router.rs:1157` | 20 |
| S8.4 | VCG auction — computed but greedy path dominates | `prompt.rs:870` | new |
| S8.5 | `MaxCostPerTurn` — governance rule exists, not enforced | `contract.rs:442` | 20 |
| S8.6 | Human approval — auto-approved by default stub | `workflow_engine.rs:474` | new |
| S8.7 | TUI `StateHub` and serve `StateHub` are separate instances | `main.rs:2080`, `util.rs:230` | new |
| S8.8 | `create_share` writes empty transcript (no run data populated) | `shared_runs.rs:89-104` | 19 |
| S8.9 | Share.tsx fetches from wrong endpoint (`/api/share/` vs `/api/shared/`) | `Share.tsx:28` | 19 |

### What good looks like

Every feature either works end-to-end or is clearly marked as `#[cfg(feature = "...")]`
or behind a config flag. Dead code paths are removed or gated. Integration tests verify
the full path from CLI invocation to observable effect.

### Checklist

- [ ] S8.1 — Auto-trigger compaction on session start or after N episodes
- [ ] S8.2 — Implement dream trigger consumer (inline at plan completion)
- [ ] S8.3 — Call `LinUCBRouter::save()` from `CascadeRouter::save()`
- [ ] S8.5 — Wire cumulative cost enforcement into tool context
- [ ] S8.7 — Share StateHub between TUI and serve when co-located
- [ ] S8.8 — Populate actual run data in `create_share`
- [ ] S8.9 — Fix Share.tsx endpoint path

---

## S9. Subprocess & Process Management Gaps

**Root cause**: `roko` spawns many subprocesses (Claude CLI, MCP servers, chain-watcher,
gates) but process lifecycle management is inconsistent. Some inherit stderr (leaking logs
into the UI), some have no timeout, some have no cleanup.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S9.1 | `detect_auth()` spawns `claude --version` with no timeout | `auth_detect.rs:99-104` | 16 |
| S9.2 | MCP server stderr inherited (`Stdio::inherit()`) | `mcp/client.rs:187` | 04 |
| S9.3 | Claude CLI dispatch has no timeout on `child.wait()` | `dispatch_direct.rs:246` | 16 |
| S9.4 | Ctrl+C during dispatch doesn't cancel background task | `chat_inline.rs:1313-1327` | 03 |
| S9.5 | No CancellationToken threaded into chat loop | `chat_inline.rs:1299-1413` | 03 |
| S9.6 | Chain-watcher task handle discarded (no kill on shutdown) | `lib.rs:300-334` | 03 |
| S9.7 | Background serve awaited before chat starts | `unified.rs:48` | 12 |
| S9.8 | ~12 bare `eprintln!` in claude_cli_agent.rs | `claude_cli_agent.rs:271-645` | 04 |
| S9.9 | `eprintln!` in main.rs without `tui_mode` guards | `main.rs:2305,2347,2382` | 04 |

### What good looks like

Every subprocess has: a timeout, captured stderr (not inherited), a stored handle for
cleanup, and a cancellation token. `eprintln!` replaced with `tracing::warn!` everywhere.

### Checklist

- [ ] S9.1 — Add 3s timeout to auth detection probe
- [ ] S9.2 — Redirect MCP stderr to log file (not inherit)
- [ ] S9.3 — Add timeout to Claude CLI dispatch
- [ ] S9.4-5 — Thread CancellationToken into chat dispatch
- [ ] S9.6 — Store chain-watcher handle, kill on shutdown
- [ ] S9.7 — Non-blocking background serve startup
- [ ] S9.8-9 — Replace bare eprintln! with tracing

---

## S10. Duplicate Code Paths & Two-Engine Problem

**Root cause**: The codebase evolved through parallel development. Multiple implementations
of the same thing coexist: two plan execution engines, two chat event loops, duplicate
dispatch paths, duplicated session summary rendering.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S10.1 | Runner v2 AND legacy PlanRunner coexist | `runner/` vs `orchestrate.rs` | 17 |
| S10.2 | Two chat event loops in chat_inline.rs | `chat_inline.rs:1044-1203` and `1299-1449` | 03 |
| S10.3 | `dispatch_direct.rs` duplicates `roko-agent` (~500 LOC) | `dispatch_direct.rs` | 03 |
| S10.4 | Session summary rendered at two locations | `chat_inline.rs:1170-1196` and `1416-1439` | 12 |
| S10.5 | `chat.rs` and `chat_inline.rs` both exist | both files | 03 |
| S10.6 | Two init paths: `roko init` vs `roko config init` | `commands/util.rs:98`, `config_cmd.rs:49` | 18 |

### What good looks like

One execution engine. One chat event loop. One dispatch layer. Deprecated paths removed,
not left alongside the replacement.

### Checklist

- [ ] S10.1 — Deprecate one engine (migrate features, remove the other)
- [ ] S10.2 — Merge the two chat event loops into one
- [ ] S10.3 — Route dispatch_direct through roko-agent's provider adapters
- [ ] S10.4 — Extract session summary into one function
- [ ] S10.5 — Remove legacy chat.rs
- [ ] S10.6 — Unify init paths

---

## S11. Mutex Poisoning & Unwrap Risks

**Root cause**: `std::sync::Mutex` with `.expect("poisoned")` and `Option::unwrap()` after
`.is_some()` checks create panic vectors in production code. Compounded by crate-level
`#[allow(dead_code, clippy::unwrap_used)]` that suppresses compiler warnings.

### Symptoms

| ID | Description | File:Line | Docs |
|---|---|---|---|
| S11.1 | Audit sink mutex — `.expect("audit signals lock")` | `dispatcher/mod.rs:779,787` | new |
| S11.2 | LRU cache mutexes — `.expect("cache mutex poisoned")` | `model_call_service.rs:689-728` | new |
| S11.3 | TOCTOU `.is_some()` / `.unwrap()` on chain_client | `orchestrate.rs:15268` | new |
| S11.4 | `.expect("just registered")` in route handler | `routes/feeds.rs:127` | new |
| S11.5 | `roko-agent` crate silences `dead_code`, `unwrap_used`, `expect_used` | `roko-agent/src/lib.rs:22` | new |

### What good looks like

Use `parking_lot::Mutex` (non-poisoning). Replace `.is_some()` + `.unwrap()` with
`if let Some(x) =`. Remove crate-level lint suppression; fix individual cases.

### Checklist

- [ ] S11.1-2 — Switch to `parking_lot::Mutex` or handle poison
- [ ] S11.3 — Replace with `if let Some(ref client) = self.chain_client`
- [ ] S11.4 — Replace `.expect()` with `.ok_or()` + error return
- [ ] S11.5 — Remove crate-level lint suppression, fix individual issues

---

## Verified Fixed (No Longer Issues)

These were documented in earlier audit docs but have been verified as resolved:

| ID | What | Where fixed |
|---|---|---|
| Chain-watcher stderr flood | Redirected to `.roko/chain-watcher.log` | `lib.rs:307-325` |
| Auth reorder (API keys first) | ZAI → Anthropic → OpenAI → CLI | `auth_detect.rs:65-107` |
| ZAI_MODEL env var | Reads `ZAI_MODEL` | `auth_detect.rs:69` |
| tui_mode includes unified chat | Detected and routed | `main.rs:1689-1694` |
| Cost tracking broken ($0.00) | Real tokens used | `chat_inline.rs:1121-1128` |
| Multi-line input (Shift+Enter) | `insert_newline()` | `chat_inline.rs:573,1595` |
| Tab completion (slash commands) | Full CompletionState | `chat_inline.rs:57-127` |
| Fuzzy matching for completions | `fuzzy_match()` | `chat_inline.rs:128` |
| Ghost text suggestions | History-based | `chat_inline.rs:543` |
| Command palette (Ctrl+K) | Full implementation | `chat_inline.rs:381` |
| Animated thinking indicator | Phase-based labels | `chat_inline.rs:895` |
| Ctrl+R history search | `HistorySearch` struct | `chat_inline.rs:299` |
| Box-drawing code blocks | `render_code_block()` | `inline/markdown.rs:47` |
| Welcome banner with system info | Version, workspace, auth | `chat_inline.rs:970` |
| Session auto-save | Every 5 turns + on exit | `chat_inline.rs:828,1136` |
| Error recovery suggestions | 6 error patterns matched | `chat_inline.rs:3816` |
| Reading time indicator | >100 words | `chat_inline.rs:3707` |
| Keyboard shortcut overlay | `render_help_modal()` | `tui/modals/help.rs:12` |
| Agent persona colors | `Theme::role_accent()` | `chat_inline.rs:3778` |
| Conversation export (md/json) | `/export` command | `chat_inline.rs:2916,2944` |

---

## Cross-Document Duplicates (Consolidated Above)

These items appeared in multiple audit docs describing the same underlying problem:

| Item | Appeared in | Consolidated to |
|---|---|---|
| Stateless per-turn dispatch | Doc 12 + Doc 16 | S1.4 |
| No system prompt to model | Doc 12 + Doc 16 | S1.1 |
| Per-request HTTP client | Doc 12 (root cause of Doc 16 streaming issues) | S2.1-3 |
| Episode logger failure silent | Doc 18 + Doc 20 | S4.5 |
| Background serve failure silent | Doc 16 + Doc 18 | S4.4 |
| `dangerously_skip_permissions` | Doc 17 + Doc 20 | S5.8 |

---

## Priority Order For Implementation

### P0 — Security (must-fix before any deployment)
- S5.1-4 (auth, terminal, CORS, PORT binding)
- S5.5 (public gists)

### P1 — Core chat usability (makes `roko` actually useful)
- S1.1-5 (system prompt, tools, history, workspace context)
- S2.1-3 (connection reuse — halves latency)
- S6.1-4 (streaming output)
- S3.1-2 (wire `/system` and `/effort`)

### P2 — Execution quality (makes `roko run` / `plan run` work well)
- S1.6, S1.9 (codebase context, ModelCallService for run)
- S5.8 (permissions not hardcoded to skip)
- S6.5-6 (progress feedback during execution)
- S7.1-6 (consolidate hardcoded values)

### P3 — Reliability & learning (makes the system robust over time)
- S4.1-6 (silent error swallowing)
- S8.1-3 (episode compaction, dream consumer, bandit persistence)
- S9.1-5 (subprocess management)
- S11.1-5 (unwrap/expect safety)

### P4 — Code health (reduces maintenance burden)
- S10.1-6 (duplicate code paths)
- S3.3-5 (remaining no-op commands)
- S7.7-10 (remaining hardcoded values)
