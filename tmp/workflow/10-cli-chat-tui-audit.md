# CLI, Chat & TUI Subsystem Audit

Every path that renders LLM output to the user — chat modes, terminal rendering, TUI dashboard, input handling, session management, and where the duplication lives.

## The Problem

There are **5 separate user-facing modes**, 2 independent terminal rendering systems (inline vs fullscreen TUI), 2 nearly-identical chat event loops (2000+ duplicated LOC), and no shared response rendering pipeline. Tool output, cost display, streaming state, and session management are all implemented differently per mode.

---

## 1. User-Facing Modes (Every Way Output Reaches The User)

### 1A. `roko` (no args) — Unified Inline Chat

**File:** `chat_inline.rs` (4,100 lines) → `run_unified_inline(auth)`
**Entry:** `unified.rs` → auto-detect auth → launch inline chat

**What it does:**
- Multi-line input editor with tab completion, history search (Ctrl+R), command palette (Ctrl+K)
- Dispatches to Claude CLI / Anthropic API / OpenAI-compat based on detected auth
- Renders response as markdown with vertical bar prefix
- Tracks cost in session-local CostMeter
- Persists conversation to `~/.cache/roko/chat-session.json`

**Response rendering:**
```
dispatch_direct::dispatch_prompt()
→ DispatchResult { text, tool_outputs, model, tokens, session_id }
→ push_tool_outputs()  — ⚙ toolname  preview (+N lines)
→ push_agent_response() — ◆ agent  markdown with │ bar
→ CostMeter update → status bar
```

### 1B. `roko "prompt"` — One-Shot

**File:** `unified.rs:84-113` → `cmd_oneshot_inline(prompt, quiet)`

**What it does:**
- Single dispatch, print result, exit
- Tool outputs to stderr, response text to stdout

**Response rendering:**
```
dispatch_direct::dispatch_prompt()
→ eprintln!("[toolname] first_line") for each tool output
→ println!(text)
→ eprintln!("[model | tokens]") if not quiet
```

**What's different from 1A:** No markdown rendering, no styling, no session, no cost tracking display. Completely separate code path.

### 1C. `roko chat` — REPL Mode

**File:** `chat.rs` (659 lines) → `run_chat_repl(agent_id, serve_url)`

**What it does:**
- Line-oriented stdin reading (BufRead, no rustyline)
- Routes to agent sidecar (`/message`) or roko-serve proxy
- Cyan `you>` prompt, yellow `{agent_id}>` response
- Calls `extract_clean_text()` to parse 13+ response formats

**Response rendering:**
```
HTTP POST /message or /api/agents/{id}/message
→ extract_clean_text(raw_body)  — 246-line parser
→ println!("{agent_id}> {text}")
```

**What's different:** No ratatui, no markdown, no tool output display, no cost tracking, no session persistence. Plain `println!()` only.

### 1D. `roko run <prompt>` — Universal Loop

**Files:** `run.rs` (1,555 lines) + `run_inline.rs` (160 lines)

**What it does:**
- Composes prompt from config (role + task + files)
- Dispatches to agent via configured backend
- Runs gate pipeline (compile, test, clippy, shell)
- Emits Episode signal with verdicts

**Response rendering:**
```
run_once() → RunReport { output_text, gate_verdicts, episode_id }
→ run_once_inline() wrapper:
  → Header block (role, gate count)
  → Gate results block (✔/✖ per gate)
  → render_markdown_with_bar(output_text)
  → Success/failure summary with elapsed time
```

**What's different:** Only mode that runs gates. Uses InlineTerminal but not the chat event loop. No multi-turn conversation.

### 1E. `roko dashboard` — Fullscreen TUI

**Files:** `tui/` directory (~24K LOC total)

**What it does:**
- Full ratatui terminal application (10 tabs, F1-F10)
- Monitors orchestration state, agents, plans, git, logs, knowledge, learning
- Two data paths: StateHub push (in-process) or filesystem polling (standalone)
- WebSocket streaming for agent output on Agents tab (F3)

**Response rendering:**
```
StateHub watch::Receiver → drain_snapshot_channel()
  → apply_dashboard_snapshot() → TuiState
  → draw() → tab-specific view renderers
  → terminal.draw(frame)

OR (standalone):
  FileWatcher → tick_snapshot() → reload DashboardData from .roko/ files
```

**What's different:** Completely independent rendering system. No shared code with chat_inline. Different terminal mode (fullscreen vs inline). Different state model (DashboardData vs ChatSession).

---

## 2. The Two Terminal Rendering Systems

### 2A. Inline Terminal (chat_inline, run_inline)

**File:** `inline/terminal.rs` (200 lines)

**How it works:**
- ratatui `Viewport::Inline` — renders at bottom of terminal, pushes completed output into scrollback
- `push_lines()` inserts styled Lines above the viewport
- `draw()` renders the active viewport (input area + status bar)
- Completed blocks stay in terminal history (scrollable)

**Key primitives** (`inline/primitives/`):

| File | LOC | What | Used By |
|---|---|---|---|
| `streaming.rs` | 268 | Live streaming state (spinner + buffer) | chat_inline |
| `run_block.rs` | 279 | Completed run summary | run_inline |
| `tool_call.rs` | 241 | Tool invocation (collapsed/expanded) | Built but NOT wired to chat |
| `cost_meter.rs` | 148 | Cost & token tracking | chat_inline |
| `cost_waterfall.rs` | 180 | Cost breakdown by model | Built, unused |
| `gate_block.rs` | 281 | Gate verdicts + per-rung details | run_inline |
| `error_block.rs` | 194 | Error display with suggestions | chat_inline |
| `diff_block.rs` | 177 | Diff rendering | Built, unused |
| `progress_tree.rs` | 352 | Task DAG progress | plan runner |
| `replan_block.rs` | 181 | Gate failure replan display | Built, partial |
| `session_summary.rs` | 172 | End-of-session metrics | Built, partial |

**Supporting modules:**

| File | LOC | What |
|---|---|---|
| `markdown.rs` | 572 | pulldown-cmark → styled ratatui Lines |
| `styled.rs` | 257 | Clack-style line builders (◆│└ / ✔✖ / spinners) |
| `symbols.rs` | 112 | Unicode glyphs (START, BAR, END, PASS, FAIL, TOOL, etc.) |
| `plaintext.rs` | 71 | Non-TTY fallback (strip styles → plain text) |
| `agent_events.rs` | 230 | WebSocket event parsing for ACP | Partial |

### 2B. Fullscreen TUI (dashboard)

**File:** `tui/app.rs` (4,101 lines) — main event loop + rendering dispatch

**How it works:**
- ratatui fullscreen terminal — takes over entire screen
- 60fps adaptive (drops to 20fps when idle)
- Tab-based views with sub-views (1-9 keys within tabs)
- Modal overlays (approval dialogs, task detail, help)

**Core files:**

| File | LOC | What |
|---|---|---|
| `app.rs` | 4,101 | Main event loop, rendering dispatch, background tasks |
| `state.rs` | 4,968 | TuiState — all mutable state |
| `dashboard.rs` | 6,382 | DashboardData — scaffold loaded from disk/snapshot |
| `input.rs` | 1,605 | Key/mouse handling, TuiAction dispatch |
| `theme.rs` | 489 | ROSEDUST palette + high-contrast + no-color variants |
| `fs_watch.rs` | 255 | Debounced filesystem watcher (notify) |
| `git_watch.rs` | 409 | Git metadata refresh |
| `ws_client.rs` | 489 | WebSocket consumers for agent output |
| `postfx.rs` | 920 | Post-processing effects |
| `tabs.rs` | 212 | Tab enum + F-key mapping |
| `event.rs` | 80 | Crossterm event polling |
| `views/` | ~10K | Tab-specific renderers (10 tabs) |
| `widgets/` | ~7K | Reusable components (braille plots, phase timelines, badges) |
| `modals/` | ~2K | Dialog overlays |

### 2C. What They Share: Nothing

| Aspect | Inline | Fullscreen TUI |
|---|---|---|
| Terminal mode | `Viewport::Inline` | Fullscreen |
| Theme | `Theme::from_env()` | `Theme::from_env()` (same struct, loaded separately) |
| Markdown | `inline/markdown.rs` | Not used (TUI renders raw text in views) |
| Symbols | `inline/symbols.rs` | `tui/symbols.rs` (possibly duplicated) |
| Agent output | `push_agent_response()` | `AgentStreamClient` WebSocket |
| Tool output | `push_tool_outputs()` | Not rendered in TUI |
| Cost display | `CostMeter` primitive | `DashboardData` cost aggregation |
| Input | Custom key handler in chat_inline | `TuiAction` dispatch in `input.rs` |
| State | `ChatSession` (in chat_inline) | `TuiState` + `DashboardData` |

---

## 3. Duplication: The Two Chat Loops

`chat_inline.rs` contains **two nearly-identical event loops** (each ~800 lines):

| Function | Entry Point | Dispatch | Lines |
|---|---|---|---|
| `run_chat_inline(agent_id, serve_url)` | `roko chat --inline` | HTTP to sidecar/serve | ~700 |
| `run_unified_inline(auth)` | `roko` (no args) | Direct via dispatch_direct | ~700 |

**What's identical:**
- Input handling (key events, multi-line, history, completion, palette)
- Viewport rendering (streaming state, agent response, cost bar)
- Session management (conversation history, save/load)
- Error handling (retry prompt, suggestions)
- Phase state machine (Input → Thinking → Streaming → Error → Input)

**What's different:**
- Dispatch mechanism (HTTP POST vs `dispatch_prompt()`)
- Response type (`SendMessageResponse` vs `DispatchResult`)
- Conversion between them (lines 3170-3180)

**Impact:** Every bug fix, every new feature (like tool output rendering) must be applied in both loops. The tool output fix correctly added `push_tool_outputs()` to both (lines 1089 and 1336), but this will happen again for every future change.

**Fix:** Extract shared `ChatSession`, `render_viewport()`, `handle_input_key()` into a module. The dispatch mechanism becomes a trait:
```rust
trait ChatBackend {
    async fn send(&self, prompt: &str) -> Result<DispatchResult>;
}
```

---

## 4. Streaming State

### Current: Spinner Only

`StreamingState` (`inline/primitives/streaming.rs`, 268 lines) has infrastructure for token streaming but currently only shows a spinner:
- `append(text)` exists but never called with live token deltas
- `update_usage(tokens, cost)` exists but not called during streaming
- `scroll_up/down()` exist for manual viewport control

### What's Missing: Live Token Streaming

No path currently streams tokens to the user in real-time. The flow is:
1. Dispatch async task
2. Show spinner
3. Wait for complete response
4. Render full response at once

Mori streamed tokens via WebSocket. The infrastructure is partially here:
- `AgentStreamClient` in `ws_client.rs` receives `StreamChunk::Text(delta)`
- But only used in TUI Agents tab, not in chat_inline
- `dispatch_claude_cli()` reads lines from subprocess but buffers them all

---

## 5. Session Management

### 5A. Chat Session Persistence

| What | Where | Format |
|---|---|---|
| Readline history | `~/.cache/roko/chat-history.txt` | Line-per-entry |
| Conversation snapshot | `~/.cache/roko/chat-session.json` | JSON array of `{role, text, timestamp}` |
| Last session summary | Loaded on startup | Shows last 5 turns |

**Missing:**
- No conversation resume (starts fresh each session)
- No session_id tracking (Claude CLI returns session_id in Result event, now captured but unused)
- No multi-session management
- No export/import

### 5B. TUI State Persistence

- Fully in-memory (`TuiState`)
- Loaded from `.roko/` files via `DashboardData` on startup
- Refreshed via StateHub push or filesystem polling
- No TUI session state saved between runs

---

## 6. Tool Output Rendering — Current State

### 6A. What Each Mode Shows

| Mode | Tool Outputs? | Format |
|---|---|---|
| `roko` (inline chat) | Yes (new) | `⚙ toolname  preview (+N lines)` |
| `roko "prompt"` | Yes (new) | `[toolname] first_line` to stderr |
| `roko chat` | No | Dropped by `extract_clean_text()` unless embedded in text |
| `roko run` | No | Gate verdicts shown, but agent tool calls invisible |
| `roko dashboard` | No | Agent output shown raw in Agents tab |

### 6B. Built But Not Wired

`inline/primitives/tool_call.rs` (241 lines) has a proper `ToolCallBlock` with:
- Collapsed view (name + summary + duration)
- Expanded view (full input/output with syntax highlighting, up to 10 input lines + 5 output lines)
- Toggle support

This is **not used by `push_tool_outputs()`**. The new function builds its own rendering from scratch instead of using the existing primitive.

---

## 7. Cost & Token Display

### 7A. CostMeter (Inline Chat Only)

`CostMeter` (`inline/primitives/cost_meter.rs`, 148 lines) tracks:
- `total_cost: f64`
- `input_tokens, output_tokens: u64`
- `cache_hits, cache_misses: u64`
- `model_tokens: HashMap<String, u64>` (per-model)
- `run_count: u32`
- `naive_baseline: f64` (what Opus would cost)

Displayed in status bar: `1.2s · claude-opus-4 (3.5K in / 1.2K out) · $0.043`

Session summary (if savings > 1.5x): `◆ session  5 turns · $0.215 total · 2.3x savings`

### 7B. Cost in Other Modes

| Mode | Cost Display | Accurate? |
|---|---|---|
| `roko` (inline) | CostMeter in status bar + session summary | Partial (no cache, no routing cost) |
| `roko "prompt"` | `eprintln!` with model + token counts | No cost in USD |
| `roko chat` | None | No |
| `roko run` | None in output | Episode records cost but not displayed |
| `roko dashboard` | DashboardData cost aggregation | From episode files (which aren't written by live paths) |

### 7C. The Problem

CostMeter only tracks what `dispatch_direct` returns. It doesn't know about:
- Cache read/write tokens (Anthropic API returns these, dispatch_direct drops them)
- Routing cost (CascadeRouter overhead)
- Multi-turn cost (tool loop iterations)
- Gate LLM judge cost

---

## 8. Theme & Styling

### ROSEDUST Palette

```
TEXT       = Rgb(165, 142, 158)   — warm rose-gray
TEXT_DIM   = Rgb(130, 108, 125)   — muted
TEXT_GHOST = Rgb(110, 85, 105)    — very muted
BG         = Rgb(0, 0, 0)        — pure black
ROSE       = Rgb(185, 120, 148)   — accent (agent headers, highlights)
SAGE       = Rgb(125, 158, 140)   — success (gates passed, completion)
EMBER      = Rgb(195, 110, 85)    — danger (errors, failures)
WARNING    = Rgb(195, 155, 95)    — warnings
DREAM      = Rgb(120, 115, 165)   — info (tool names, links)
BONE       = Rgb(200, 185, 175)   — selection foreground
VOID       = Rgb(20, 15, 25)     — deep background
```

**Variants:**
- `Theme::dark()` — default ROSEDUST
- `Theme::no_color()` — all `Color::Reset` (for `NO_COLOR` env)
- `Theme::high_contrast()` — WCAG 2.1 AA compliant

**Resolution:** `Theme::from_env()` reads `ROKO_HIGH_CONTRAST` and `NO_COLOR`

**Shared between inline and TUI:** Same `Theme` struct, but loaded independently in each system.

---

## 9. Architectural Issues

### 9A. chat_inline.rs Is a God File (4,100 Lines)

Owns:
1. Two chat event loops (unified + sidecar)
2. Input handling (key events, multi-line editor, history, completion, palette)
3. Session management (conversation, persistence, resume)
4. Response rendering (tool outputs, agent response, error display)
5. Viewport rendering (streaming state, status bar)
6. Phase state machine (Input → Thinking → Streaming → Error)
7. HttpResponse → DispatchResult conversion
8. Slash command registration (58 commands)

Should be: 6+ modules with clear boundaries.

### 9B. `roko chat` vs `roko` Are Parallel Implementations

| Feature | `roko chat` (chat.rs) | `roko` (chat_inline.rs) |
|---|---|---|
| Input | BufRead line-by-line | Custom key handler, multi-line |
| Styling | None (println) | Full ratatui + markdown |
| Tool output | Dropped | Rendered |
| Cost | Not shown | CostMeter in status bar |
| Session | Not saved | Saved to ~/.cache/roko/ |
| Streaming | Not supported | Spinner (infrastructure for tokens) |
| Completions | None | Tab completion + fuzzy |
| History | None | Ctrl+R reverse search |

These should converge. `roko chat` should either be removed or become a thin wrapper that launches `run_unified_inline()` with the sidecar backend.

### 9C. TUI Doesn't Render Tool Outputs

The Agents tab (F3) streams raw agent output via WebSocket but doesn't parse or render tool calls, thinking blocks, or structured output. It shows the raw text buffer.

### 9D. No Shared Response Rendering

Every mode renders responses differently:
- Inline: `push_agent_response()` → markdown with bar
- One-shot: `println!()` → raw text
- REPL: `println!()` → raw text with agent prefix
- Run: `render_markdown_with_bar()` → markdown with bar (different code path)
- TUI: Raw text in view panel

A shared `ResponseRenderer` could normalize this:
```rust
trait ResponseRenderer {
    fn render_tool_output(&mut self, output: &ToolOutput);
    fn render_text(&mut self, text: &str);
    fn render_cost(&mut self, cost: &CostSummary);
    fn render_gate(&mut self, verdict: &GateVerdict);
}
```

With implementations: `InlineRenderer`, `PlainRenderer`, `TuiRenderer`.

---

## 10. TUI Dashboard Details

### 10A. Tabs (F1-F10)

| F-Key | Tab | What It Shows |
|---|---|---|
| F1 | Dashboard | Health overview, mesh status, cost summary |
| F2 | Plans | Plan list, DAG view, task detail, wave progress |
| F3 | Agents | Agent list, output stream (WebSocket), gate results, token burn |
| F4 | Git | Branch tree, commit graph, worktrees |
| F5 | Logs | Filtered log viewer, signal stream |
| F6 | Config | Config view, provider health, model comparison |
| F7 | Inspect | Engram DAG, episode replay, knowledge browser |
| F8 | Marketplace | Job list, job detail, create job |
| F9 | Atelier | PRD workshop, plan explorer |
| F10 | Learning | Route decisions, history, efficiency metrics |

### 10B. Data Flow

```
                    ┌─── StateHub (in-process) ──→ watch::Receiver
                    │                                    │
roko plan run ──────┤                              drain_snapshot()
                    │                                    │
                    └─── .roko/ files ──→ FileWatcher ──→ tick_snapshot()
                                                         │
                                                    ┌────▼────┐
                                                    │ TuiState │
                                                    └────┬────┘
                                                         │
                                              ┌──────────┼──────────┐
                                              │          │          │
                                         views/     widgets/    modals/
                                              │          │          │
                                              └──────────┼──────────┘
                                                         │
                                                  terminal.draw()
```

### 10C. Agent Streaming (F3 Only)

**File:** `ws_client.rs` (489 lines)

- `AgentStreamClient::connect(agent_id, serve_url)` opens WebSocket
- Receives `StreamChunk` variants: `Text`, `Reasoning`, `ToolCall`, `Usage`, `Done`
- Only spawned when Agents tab is active
- Lazy connection (created in `sync_agent_stream_clients()`)

---

## 11. File Inventory

### All Files That Render Output to Users

| File | LOC | Mode | Status |
|---|---|---|---|
| **Chat modes** | | | |
| `chat_inline.rs` | 4,100 | `roko` + `roko chat --inline` | God file, 2 duplicate loops |
| `chat.rs` | 659 | `roko chat` (REPL) | Separate, minimal |
| `unified.rs` | 204 | `roko` entry + one-shot | Thin wrapper |
| `run.rs` | 1,555 | `roko run` | Separate rendering |
| `run_inline.rs` | 160 | `roko run` inline wrapper | Uses InlineTerminal |
| **Inline rendering** | | | |
| `inline/terminal.rs` | 200 | InlineTerminal core | Shared by chat + run |
| `inline/markdown.rs` | 572 | Markdown → styled Lines | Used by chat + run |
| `inline/styled.rs` | 257 | Line builders (◆│└) | Used by chat + run |
| `inline/symbols.rs` | 112 | Unicode glyphs | Shared |
| `inline/plaintext.rs` | 71 | Non-TTY fallback | Used by run |
| `inline/agent_events.rs` | 230 | ACP WebSocket events | Partial |
| **Inline primitives** | | | |
| `primitives/streaming.rs` | 268 | Live streaming state | Spinner only |
| `primitives/run_block.rs` | 279 | Run summary | Used by run_inline |
| `primitives/tool_call.rs` | 241 | Tool call display | **Built, NOT wired** |
| `primitives/cost_meter.rs` | 148 | Cost tracking | Used by chat_inline |
| `primitives/cost_waterfall.rs` | 180 | Cost breakdown | **Built, unused** |
| `primitives/gate_block.rs` | 281 | Gate verdicts | Used by run_inline |
| `primitives/error_block.rs` | 194 | Error display | Used by chat_inline |
| `primitives/diff_block.rs` | 177 | Diff rendering | **Built, unused** |
| `primitives/progress_tree.rs` | 352 | DAG progress | Used by plan runner |
| `primitives/replan_block.rs` | 181 | Replan display | **Built, partial** |
| `primitives/session_summary.rs` | 172 | Session metrics | **Built, partial** |
| **TUI dashboard** | | | |
| `tui/app.rs` | 4,101 | Main loop + rendering | Stable |
| `tui/state.rs` | 4,968 | TuiState | Stable |
| `tui/dashboard.rs` | 6,382 | DashboardData | Stable |
| `tui/input.rs` | 1,605 | Key/mouse handling | Stable |
| `tui/theme.rs` | 489 | Color palette | Shared struct |
| `tui/fs_watch.rs` | 255 | Filesystem watcher | Stable |
| `tui/ws_client.rs` | 489 | WebSocket agent streams | Used by F3 only |
| `tui/views/` | ~10K | Tab renderers | Stable |
| `tui/widgets/` | ~7K | Reusable components | Stable |

**Total rendering LOC:** ~45K+ across inline + TUI + chat modes.

---

## 12. Grep Gates (Acceptance Criteria)

After consolidation:

```bash
# No more duplicate chat loops (should be 1 run_inline function)
rg 'fn run_chat_inline|fn run_unified_inline' crates/roko-cli/src/ --type rust
# Should return 1 result (the unified function)

# No more raw println for agent responses
rg 'println!.*agent.*>' crates/roko-cli/src/chat.rs --type rust
# Should return 0 (chat.rs should use shared renderer)

# extract_clean_text should be gone (replaced by typed parsing)
rg 'extract_clean_text' crates/ --type rust | grep -v test
# Should return 0

# ToolCallBlock should be used by push_tool_outputs
rg 'ToolCallBlock' crates/roko-cli/src/chat_inline.rs --type rust
# Should return 1+ (wired, not bypassed)

# CostMeter should be used by all modes
rg 'CostMeter' crates/roko-cli/src/ --type rust
# Should appear in shared module, not just chat_inline
```
