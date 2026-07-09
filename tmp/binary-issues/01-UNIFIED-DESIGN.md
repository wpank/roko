# Unified Command Design

## Principle: The Bare Command Is the Product

```bash
roko
```

This single command:
1. Auto-detects auth (claude CLI session → API key → prompt to log in)
2. Auto-creates `.roko/` if missing (silent, no wizard)
3. Starts serve in-process as a background tokio task (no separate process)
4. Drops into the **chat TUI** — the primary interface

The chat TUI is roko's "home screen." From here you can:
- Talk to agents
- Run tasks
- View status
- Switch between agents
- See gate results inline
- Access all features via `/` commands

## Command Surface (Simplified)

### Tier 1: Daily Use (3 commands)

```bash
roko                     # Chat TUI (the home screen)
roko "fix the bug"       # One-shot run (inline output, no TUI)
roko status              # Quick health check (instant, no TUI)
```

### Tier 2: Workflows (accessed from TUI or CLI)

```bash
roko plan run [dir]      # Execute a plan
roko plan generate       # Generate plan from prompt/PRD
roko prd idea "..."      # Capture an idea
roko research "topic"    # Deep research
```

### Tier 3: Infrastructure (rarely typed)

```bash
roko serve               # Start ONLY the HTTP server (for external tools)
roko dashboard           # Start ONLY the file-based TUI (no chat)
roko acp                 # Editor integration (stdio, spawned by editors)
roko config ...          # Configuration management
roko deploy ...          # Cloud deployment
```

### Removed/Merged

| Old command | Replaced by |
|---|---|
| `roko agent serve` | Built into `roko` (in-process dispatch) |
| `roko agent chat` | `roko` (the default command) |
| `roko agent list` | `/agents` command in chat TUI |
| `roko agent create/start/stop` | `/agent create X` in chat TUI |
| `roko up` | `roko` (starts everything) |
| `roko run --serve` | `roko "prompt"` (serve always available) |

## The Chat TUI — Home Screen

```
┌─────────────────────────────────────────────────────────────────┐
│  ◆ roko                                        haiku · $0.03   │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ◆ researcher                                                   │
│  │ The Q3 earnings data shows margin compression across         │
│  │ mid-cap fintech names. Key findings:                         │
│  │                                                              │
│  │ • Interchange revenue declined 12% QoQ                       │
│  │ • Cross-border volumes remained strong (+14%)                │
│  │                                                              │
│  │ ▸ ReadFile  earnings/q3.csv (142 lines, 0.2s)               │
│  │ ▸ Search "fintech margin 2026" (3 results, 0.8s)            │
│  │                                                              │
│  │ gates  compile ✔  test ✔                                    │
│  │ actual $0.031 (-28%)  ·  9.8s  ·  haiku                     │
│  └                                                              │
│                                                                 │
│  ❯ Summarize Q3 fintech earnings                               │
│                                                                 │
│  ◆ researcher                                                   │
│  ⠋ Thinking... (2.3s)                                          │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ ❯ █                                                             │
├─────────────────────────────────────────────────────────────────┤
│ researcher · haiku · $0.06 · 4,821 tok · /help for commands     │
└─────────────────────────────────────────────────────────────────┘
```

### Chat TUI Features

**Input:**
- Multi-line (shift+enter)
- History (up/down)
- `/` commands
- Tab completion for commands and file paths

**Output:**
- Streaming markdown with syntax highlighting
- Collapsible tool calls
- Inline gate results
- Cost tracking per message

**`/` Commands:**
```
/help               Show available commands
/agent <name>       Switch to a different agent
/agents             List all available agents
/model <name>       Switch model (haiku/sonnet/opus)
/cost               Show session cost summary
/status             Show workspace status
/plan run [dir]     Execute a plan
/share              Share last response as Gist
/web                Open browser to localhost:6677
/clear              Clear conversation
/quit               Exit
```

**Status Bar (bottom):**
- Current agent name
- Current model
- Session cost
- Token count
- Hint text ("type /help")

## In-Process Architecture

```
roko (single process)
├── Chat TUI (ratatui inline viewport)
│   ├── Input handler (crossterm events)
│   ├── Streaming renderer (markdown + tool calls)
│   └── Status bar
├── Agent Dispatcher (in-process, no sidecar)
│   ├── Claude CLI backend (spawns claude subprocess)
│   ├── Anthropic API backend (direct HTTP)
│   ├── OpenAI compat backend
│   └── Ollama backend
├── Serve (background tokio task, optional)
│   ├── HTTP API on :6677
│   ├── WebSocket events
│   ├── Terminal PTY sessions
│   └── Static demo files
├── Gate Pipeline (in-process)
│   ├── Compile gate
│   ├── Test gate
│   └── Clippy gate
└── Learning (in-process)
    ├── Efficiency events
    ├── Episode logging
    └── Cascade router
```

**No sidecars. No separate processes. No port hunting.**

The agent dispatch happens in the same process as the chat. When you type a message, it's dispatched directly to the LLM backend (claude CLI subprocess or API call). No HTTP intermediary.

The serve HTTP server runs as a background task in the same process — it's there for:
- Web demo pages
- External tool integration (editors, scripts)
- `--share` URL serving
- Terminal PTY sessions

But it's **invisible to the user**. They never start it manually.

## Auth Auto-Detection

On startup, roko checks (in order) — API keys first, CLI probes last:

```rust
fn detect_auth() -> AuthMethod {
    // 1. Zhipu/GLM (ZAI_API_KEY + optional ZAI_MODEL)
    // 2. Anthropic API key (ANTHROPIC_API_KEY)
    // 3. OpenAI-compatible (OPENAI_API_KEY + OPENAI_API_BASE)
    // 4. Claude CLI fallback (claude --version probe)
    // 5. NeedsSetup
}
```

**Current state (2026-04-27):** Auth detection is pure env-based. It does NOT
read roko.toml's `[agent].default_backend` or `[providers]` section. This is
tracked as Issue 6E/19 in `03-UNIFIED-CHAT-BLOCKERS.md`.

**Runtime model switching:** `/model <name>` command in chat TUI. Only works
for OpenAI-compat providers (Zhipu, OpenAI, Moonshot, Ollama). Claude CLI
picks its own model.

If `NeedsSetup`:
```
◆ roko

  No LLM provider detected. Choose one:

  [1] Log in with Claude CLI (recommended)
      Run: claude login
  [2] Set Anthropic API key
      Run: export ANTHROPIC_API_KEY=sk-...
  [3] Use OpenAI
      Run: export OPENAI_API_KEY=sk-...
```

## Migration Path

### Phase 1: Make `roko` drop into chat
- Default command (no args) → chat TUI
- In-process agent dispatch (no sidecar needed)
- Auto-start serve as background task
- Auth auto-detection

### Phase 2: Unify agent management
- Remove `roko agent serve` (dispatch is in-process)
- Remove `roko agent chat` (it's just `roko`)
- Move agent management to `/` commands in chat

### Phase 3: Progressive config
- `roko` works with zero config (claude CLI auth)
- `roko.toml` is optional, auto-created when needed
- Providers auto-detected from env vars

### Phase 4: TUI polish
- Streaming markdown in viewport
- Collapsible tool calls with animations
- Cost waterfall after each response
- Knowledge display inline
- Gate results as a ribbon
