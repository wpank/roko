# 16 — Chat Session Lifecycle Audit

**Status**: open (critical)
**Scope**: `crates/roko-cli/src/chat_inline.rs`, `crates/roko-cli/src/unified.rs`, `crates/roko-cli/src/auth_detect.rs`, `crates/roko-cli/src/dispatch_direct.rs`

## What This Document Covers

When a user types `roko` and starts chatting, every step from startup to exit has issues.
This doc catalogs them all: auth detection, input handling, slash commands, session state,
error handling, and output rendering.

---

## 1. Startup Sequence

### How startup works

1. `detect_auth()` — probes env vars and spawns `claude --version` subprocess
2. `ensure_workspace()` — creates `.roko/` if missing
3. `load_config_or_defaults()` — reads `roko.toml` or uses defaults
4. `spawn_background_serve()` — starts HTTP server on :6677 (awaited)
5. Print auth banner
6. Enter `run_unified_inline()` chat loop

### Issues

**S1. `detect_auth()` blocks startup synchronously** (`auth_detect.rs:99-104`)

```rust
Command::new("claude").arg("--version").output()
```

Synchronous subprocess spawn with no timeout. If `claude` is installed via a slow npm
shim, or if it hangs (e.g., waiting on a login prompt), this blocks indefinitely. The user
sees nothing during this wait — no spinner, no progress indication.

**S2. No startup progress indication** (`unified.rs:23-64`)

Zero output between typing `roko` and the chat prompt appearing. Every startup step is
silent. Claude Code shows a startup spinner; roko shows a blank terminal.

**S3. Background serve failure is completely silent** (`unified.rs:133-145`)

```rust
Err(e) => {
    tracing::warn!("background serve failed to start: {e:#}");
    None
}
```

If port 6677 is in use, the warning goes to a log file. The user sees the banner without
`, serve :6677` but no explanation of why serve is absent.

**S4. `.roko/` subdirectories not created at startup** (`unified.rs:152-160`)

Only `create_dir_all(".roko")` runs. Subdirs like `.roko/sessions/`, `.roko/learn/`,
`.roko/exports/` are created lazily. Commands like `/status` or `/learn` may report
"not found" on a fresh workspace.

---

## 2. Auth Detection

### Priority order (`auth_detect.rs:65-107`)

1. `ZAI_API_KEY` (Zhipu/GLM) — checked first
2. `ANTHROPIC_API_KEY` — checked second
3. `OPENAI_API_KEY` — checked third
4. `claude --version` CLI probe — last resort

### Issues

**A1. ZAI_API_KEY silently takes priority over ANTHROPIC_API_KEY** (`auth_detect.rs:67`)

A user with both env vars set gets routed to a Chinese LLM vendor instead of Claude. This
is counterintuitive for a tool built around the Claude ecosystem. No user expects a
`ZAI_API_KEY` from some other tool to hijack their roko session.

**A2. No API key validation at startup** (`auth_detect.rs:79-83`)

The code checks if the env var is non-empty, not whether the key is valid. An expired or
malformed key passes detection, and the user discovers the problem only when they send
their first message. Claude Code validates the key on startup.

**A3. Claude CLI detected but login state not checked** (`auth_detect.rs:99-104`)

`claude --version` succeeds even when the CLI is not logged in. The user enters the chat
thinking auth is fine, then hits an error on first dispatch. The code comment at line 63
acknowledges this: "CLI probes can succeed yet fail at dispatch time."

**A4. No way to switch auth during a session** (`chat_inline.rs:2174`)

The `/auth` command is read-only — it shows current auth but cannot switch it. To use a
different provider, the user must exit, change env vars, and restart.

---

## 3. Input Handling

### Architecture

Custom crossterm/ratatui input with a manual `InputState` struct. Not rustyline.

### What works well
- Ctrl-A/E (home/end), Ctrl-U (clear line), Ctrl-L (clear screen)
- Ctrl-C (cancel/exit), Ctrl-D (exit)
- Ctrl-R (reverse history search)
- Ctrl-K (command palette)
- Up/Down (history navigation)
- Multi-line via Shift+Enter
- Tab completion for slash commands
- Ghost text suggestions from history
- History persistence to `.roko/chat_history` (capped at 500)

### Issues

**I1. No Ctrl-W (delete word backward)** (`chat_inline.rs:1700+`)

Standard readline binding. Users of bash, zsh, Claude Code, and every terminal expect it.
Only Ctrl-U (clear entire line) exists.

**I2. No Alt-Left/Alt-Right or Ctrl-Left/Ctrl-Right (word movement)** (`chat_inline.rs`)

Only character-level cursor movement. Word-level movement is a standard binding in every
modern terminal and editor.

**I3. No visual indicator that tab completion exists** (`chat_inline.rs`)

The only way to discover tab completion is typing `/` and pressing Tab, or reading `/help`.
No hint in the prompt or welcome banner.

---

## 4. Slash Commands — No-Ops and Broken Commands

### The core problem

Many slash commands confirm success to the user but have **zero effect on behavior**.
This is the most deceptive class of bug in the REPL — the user believes they've changed
something, but nothing changed.

**C1. `/system` stores the message but never sends it to the LLM** (`chat_inline.rs:2120-2137`)

```rust
session.system_message = Some(msg.to_string());
```

The field is set but `dispatch_prompt()` at line 1457 never reads `session.system_message`.
The Anthropic API path hardcodes `"messages": [{"role":"user","content":prompt}]` with no
`"system"` field. The user sees "system prompt set" but the model behavior is unchanged.

**This is the single most deceptive bug.** The user explicitly configures behavior, gets
confirmation, and the system silently ignores it.

**C2. `/effort` is a complete no-op** (`chat_inline.rs:2245-2273`)

Displays "set to {arg}" but stores nothing. No field in ChatSession, not consumed by
dispatch. Pure theater.

**C3. `/gate` toggle is a no-op** (`chat_inline.rs:2304-2311`)

Says "gate toggle: X" and tells the user to "update roko.toml" — but the user expects
the command to do that for them.

**C4. `/config set` is a no-op** (`chat_inline.rs:2449-2468`)

Displays "set {key} = {value}" and says "edit roko.toml to persist" but changes nothing
in memory or on disk.

**C5. `/run`, `/plan run`, `/plan generate`, `/prd idea`, `/research` just print CLI hints**
(`chat_inline.rs:2715-2821`)

These commands print "run this in a terminal" messages instead of executing. The user is
inside a chat session and expects these to work inline.

```rust
"/run" => styled::continuation(theme, "command", &format!("roko run \"{prompt}\""), None)
"/plan run" => styled::continuation(theme, "plan", "roko plan run {dir}", Some("run in terminal"))
```

### Summary of slash command status

| Command | Status | What happens |
|---|---|---|
| `/system` | **BROKEN** | Sets field, never consumed by dispatch |
| `/effort` | **NO-OP** | Prints confirmation, stores nothing |
| `/gate` | **NO-OP** | Prints hint, changes nothing |
| `/config set` | **NO-OP** | Prints confirmation, changes nothing |
| `/run` | **REDIRECT** | Prints CLI command hint |
| `/plan run` | **REDIRECT** | Prints CLI command hint |
| `/plan generate` | **REDIRECT** | Prints CLI command hint |
| `/prd idea` | **REDIRECT** | Prints CLI command hint |
| `/research` | **REDIRECT** | Prints CLI command hint |
| `/model` | Works | Switches model for next turn |
| `/help` | Works | Shows help text |
| `/history` | Works | Shows conversation history |
| `/export` | Works | Exports conversation |
| `/clear` | Works | Clears conversation |
| `/copy` | Works | Copies last response to clipboard |
| `/stats` | Works | Shows session statistics |

---

## 5. Session State

### What persists
- Input history: `.roko/chat_history` (one line per entry, capped at 500)
- Last session: `.roko/sessions/last.json` (auto-saved every 5 turns and on exit)
- Snapshot includes: turn count, cost, tokens, model, all messages, system_message

### Issues

**SS1. No conversation resume** (`chat_inline.rs:866`)

Sessions are saved to `.roko/sessions/last.json` and the previous session summary is shown
on startup. But there is no `/resume` command and no `--resume` flag. The conversation
messages are saved but never reloaded. The user cannot continue where they left off.

**SS2. Every turn is single-shot — no conversation context sent** (`dispatch_direct.rs:109-115`)

Each API call sends exactly one user message:
```rust
"messages": [{"role": "user", "content": prompt}]
```

The `session.conversation` vector records messages for display/export but is **never sent
to the API**. Each turn is stateless. The model has no memory of previous exchanges.

This is the fundamental difference from Claude Code, which maintains a running conversation.

**SS3. Cost tracking does not persist across restarts** (`chat_inline.rs:1284`)

`CostMeter` is initialized fresh every session. No way to see cumulative costs across
sessions.

---

## 6. Error Handling

### Error flow
1. Dispatch error arrives via mpsc channel
2. `push_error_with_suggestions()` renders with contextual hints
3. User can press `[r]` to retry or `[q]`/`Esc` to return

### Error suggestions (`chat_inline.rs:3816-3877`)
- "connection refused" → suggest `roko serve`, check port
- "unauthorized" / "401" → suggest checking API key
- "429" / "rate limit" → suggest wait & retry, switch model
- "not found" / "404" → suggest listing models
- "context length" → suggest `/clear`

### Issues

**E1. No automatic retry for rate limits** (`chat_inline.rs:3831`)

429 errors show "try again in 30 seconds" but don't auto-retry. Claude Code implements
automatic exponential backoff. The user must manually press `[r]`.

**E2. 120s HTTP timeout with no indication** (`chat_inline.rs:3587`)

The spinner shows "Thinking..." for up to 2 minutes before showing a timeout error. No
indication of network issues during the wait.

**E3. Claude CLI dispatch has no timeout at all** (`dispatch_direct.rs:246`)

`child.wait()` blocks indefinitely. A hung Claude CLI subprocess freezes the session.

**E4. Error phase `[s]witch` option planned but never implemented** (`chat_inline.rs:1079-1095`)

The Phase enum mentions `[r]etry / [s]witch / [q]uit` but only `[r]` and `[q]` are
rendered and handled. No way to switch models after an error.

**E5. Channel disconnect produces generic error** (`chat_inline.rs:1153-1162`)

If the background task panics, the user sees "response channel closed" with no context.

---

## 7. Output Rendering

### How output works
1. Response arrives **all at once** (not streamed) via mpsc channel
2. Full text rendered through `render_markdown_with_bar()` using `pulldown-cmark`
3. Supports: headers, bold, italic, code blocks, tables, lists, links, blockquotes

### Issues

**O1. No streaming output — entire response appears at once** (`chat_inline.rs:1457-1491`)

The dispatch spawns a background task and receives the complete result via mpsc. The
`StreamingState` struct exists with `append()` and `scroll_offset` fields but is never
used in the chat path.

The user sees:
1. "Thinking..." spinner for the entire request duration (could be 30-60 seconds)
2. Then the full response appears at once

Claude Code streams tokens as they arrive. This is the single biggest perceptual
difference. For long responses, the user has no idea if anything is happening.

The Claude CLI path reads `stream-json` line by line but pipes everything into a buffer
and returns it at the end, rather than streaming to the UI.

**O2. Audible bell on responses >10s** (`chat_inline.rs:1132-1134`)

```rust
if latency > 10.0 { print!("\x07"); }
```

Surprising and not configurable.

**O3. `NO_COLOR` disables the entire inline UI** (`inline/terminal.rs:186-188`)

`NO_COLOR` should disable colors, not disable the ratatui-based UX entirely. Users wanting
a color-free but interactive experience are forced into the legacy line-oriented REPL.

---

## Anti-Patterns Across the Session Lifecycle

1. **Confirmation theater**: Commands print "done" or "set to X" but change nothing.
   `/system`, `/effort`, `/gate`, `/config set` all exhibit this. The UI lies to the user.

2. **Write-only fields**: `session.system_message` is set by `/system` and saved to
   snapshots but never consumed by dispatch. Code has a field, writes to it, but never
   reads it in the hot path.

3. **Redirect instead of execute**: `/run`, `/plan run`, etc. print "run this in a
   terminal" instead of executing inline. The chat REPL should be the integration point,
   not a signpost to other commands.

4. **Stateless multi-turn**: Each API call sends one message. Conversation history exists
   in memory for display but is never sent to the model. The session appears multi-turn
   to the user but is single-shot to the model.

5. **Silent startup failures**: Auth detection can succeed for broken auth, serve can fail
   silently, subdirectories aren't created — all without user-visible feedback.

6. **No streaming in 2026**: The chat REPL shows a spinner for the full response duration,
   then dumps everything at once. Streaming infrastructure exists but is never wired in.

---

## Root Cause Fix

The chat REPL needs to be rebuilt around a **session-scoped agent** rather than a
per-turn dispatch shim:

1. **Session agent created once at startup** — holds the HTTP client, system prompt,
   tool definitions, conversation history, and model config.

2. **Slash commands mutate the session agent** — `/system` updates the agent's system
   prompt (and it's actually sent). `/model` swaps the underlying model. `/effort` maps
   to API parameters.

3. **Conversation history sent with each turn** — the agent maintains a message list,
   sends it with each call, implements context windowing.

4. **Streaming wired through** — the existing `StreamingState` struct is connected to
   the dispatch path. Tokens render as they arrive.

5. **Inline execution for workflow commands** — `/run`, `/plan run` etc. execute in the
   background with progress shown in the chat UI, not redirected to another terminal.

---

## Checklist

### Startup
- [ ] Add timeout to `detect_auth()` subprocess probe (3s max)
- [ ] Show startup spinner/progress
- [ ] Surface background serve failures to the user
- [ ] Create `.roko/` subdirectories eagerly

### Auth
- [ ] Reorder: ANTHROPIC_API_KEY before ZAI_API_KEY
- [ ] Validate API key with a lightweight probe at startup
- [ ] Check Claude CLI login state, not just `--version`
- [ ] Add `/auth switch` command for in-session switching

### Input
- [ ] Implement Ctrl-W (delete word backward)
- [ ] Implement Alt-Left/Alt-Right (word movement)
- [ ] Show tab completion hint in welcome banner

### Slash Commands
- [ ] Wire `/system` into dispatch (pass to API)
- [ ] Wire `/effort` into dispatch (store and pass)
- [ ] Wire `/gate` to modify runtime config
- [ ] Wire `/config set` to modify runtime config (and optionally persist)
- [ ] Execute `/run`, `/plan run`, `/prd idea`, `/research` inline

### Session
- [ ] Add `/resume` command to reload previous conversation
- [ ] Send conversation history with each API call
- [ ] Implement context windowing for long conversations
- [ ] Persist cumulative cost tracking across sessions

### Error Handling
- [ ] Auto-retry with exponential backoff for 429 errors
- [ ] Add timeout to Claude CLI subprocess dispatch
- [ ] Implement `[s]witch` model on error
- [ ] Better diagnostic for channel disconnect errors

### Output
- [ ] Wire streaming output (connect `StreamingState` to dispatch)
- [ ] Make audible bell configurable (or remove)
- [ ] Make `NO_COLOR` disable colors only, not the entire UI
