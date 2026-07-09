# Binary UX Audit — Everything That's Wrong

> **See [MASTER-INDEX.md](MASTER-INDEX.md) for the consolidated tracker organized by root cause.**
> This file (00-AUDIT.md) is the original audit. The master index deduplicates and
> organizes all findings from docs 01-20 into 11 systemic problems with checklists.

## The Core Problem

roko has **6 different ways to start things**, each requiring different prerequisites, ports, and mental models. A user who just wants to "talk to an agent" has to:

1. Know that `roko serve` exists and must be running
2. OR know that `roko agent serve --agent-id X` is a different thing
3. Know that `roko agent chat --agent X` tries sidecar first, then serve
4. Know that the sidecar registers in `.roko/runtime/agents.json`
5. Know which port things are on
6. Know that the Anthropic API key needs to be set OR that `claude` CLI needs to be logged in
7. Know the difference between `anthropic_api` and `claude_cli` provider kinds

Compare to Claude Code: `claude`. One command. Everything works.

## Service Dependency Spaghetti

```
Current state (user has to understand all of this):

roko run ────────────── standalone, no deps
roko run --serve ────── starts serve in background
roko run --share ────── starts serve + uploads gist
roko serve ──────────── starts HTTP on :6677
roko serve --tui ────── starts HTTP + embedded TUI
roko agent serve ────── starts per-agent sidecar on random port
roko agent chat ─────── needs either sidecar OR serve
roko dashboard ──────── standalone, file-based
roko plan run ──────── standalone, no deps
roko up ─────────────── starts serve + all agents from config
roko acp ────────────── stdio JSON-RPC for editors
```

**11 different entry points.** The user has to choose the right combination.

## Anti-Patterns Present

### 1. Multiple process choreography
To use chat, you must start a sidecar in terminal 1, then chat in terminal 2. Or start serve in terminal 1. The user is managing processes manually.

### 2. Port guessing
Serve is on 6677. Sidecars are on random ports. The user has to know this.

### 3. Config before use
`roko chat` fails without provider config. `roko run` fails without `roko.toml`. Nothing works zero-config.

### 4. Silent failures with bad error messages
- "error send message" — what does this mean?
- "load routing config" — what routing config?
- "Missing API key: env var ANTHROPIC_API_KEY not set" — but I'm using Claude CLI auth!

### 5. Help text as default
`roko` with no args shows help text. It should drop into the most useful mode (chat or status).

### 6. Duplicate commands
- `roko serve` vs `roko agent serve` — different things, confusing names
- `roko dashboard` vs `roko serve --tui` — same TUI, different entry points
- `roko chat` (old REPL) vs `roko agent chat` (new inline) — why are there two?

### 7. Provider config maze
- `claude_cli` needs `command = "claude"` in the provider
- `anthropic_api` needs `api_key_env = "ANTHROPIC_API_KEY"`
- `openai_compat` needs `base_url`
- The agent_serve code has a hardcoded override: "ANTHROPIC_API_KEY set — overriding provider to anthropic"
- Config schema version warning on every startup

## The 6 Things That Should Be 1

| What the user wants | Current commands needed | What it should be |
|---|---|---|
| Talk to an agent | `roko agent serve --agent-id X` + `roko agent chat --agent X` | `roko` |
| Run a task | `roko run "prompt"` | `roko run "prompt"` (ok) |
| See what's happening | `roko dashboard` or `roko serve --tui` | `roko` (built into main UI) |
| Start the web UI | `roko serve` then open browser | `roko --web` or auto |
| Execute a plan | `roko plan run plans/` | `roko plan run` (ok) |
| Share a run | `roko run --share` (needs serve + gh) | `roko run --share` (ok) |

## What Claude Code Gets Right That roko Gets Wrong

| Claude Code | roko |
|---|---|
| `claude` → interactive session | `roko` → help text |
| Zero config needed | Needs `roko.toml` + provider config |
| Auth handled transparently | 3 different auth paths that conflict |
| One process | 2-3 processes for basic chat |
| Tool calls rendered inline | Tool calls not visible in chat |
| Progress always visible | Spinners only in some paths |
| Every output suggests next action | Outputs are dead ends |

## What "Fixed" Looks Like

```
roko                    → drop into chat TUI (starts serve in background)
roko "fix the bug"      → one-shot run (no serve needed)
roko --web              → start serve + open browser
roko plan run           → execute plan (starts agent as needed)
roko status             → workspace health (instant)
```

**One binary. One process. Everything auto-starts, auto-discovers, auto-connects.**

The serve HTTP server starts in-process (background tokio task). Agent dispatch happens in-process (no sidecar needed for basic chat). The TUI is the primary interface. Web is optional.

## Config Progressive Disclosure

**Layer 0: Zero config**
```bash
roko  # works immediately if claude CLI is logged in
```

**Layer 1: Minimal config**
```toml
# roko.toml (optional, auto-created)
[agent]
model = "claude-sonnet-4-6"
```

**Layer 2: Full config**
```toml
# Only added when the user needs it
[providers.anthropic]
kind = "claude_cli"
command = "claude"

[providers.openai]
kind = "openai_compat"
api_key_env = "OPENAI_API_KEY"
```

## Auth Resolution (Simplified)

```
1. If claude CLI is on PATH and logged in → use claude_cli
2. If ANTHROPIC_API_KEY is set → use anthropic_api
3. If OPENAI_API_KEY is set → use openai_compat
4. If none → prompt: "Run `claude` to log in, or set ANTHROPIC_API_KEY"
```

No config file needed for step 1. Claude Code users (the primary audience) just type `roko` and it works.

## Document Index

### Pre-existing docs (01-11)
| # | Title | Severity | Status |
|---|---|---|---|
| [01](01-UNIFIED-DESIGN.md) | Unified Design | — | — |
| [02](02-EDGE-CASES.md) | Edge Cases | — | — |
| [03](03-UNIFIED-CHAT-BLOCKERS.md) | Unified Chat Blockers | — | — |
| [04](04-SUBPROCESS-LOG-LEAK-CATALOG.md) | Subprocess Log Leak | — | — |
| [05](05-COMPLETION-AND-SUGGESTIONS.md) | Completion & Suggestions | — | — |
| [06](06-CHAT-UX-OVERHAUL.md) | Chat UX Overhaul | — | — |
| [07](07-TUI-VISUAL-POLISH.md) | TUI Visual Polish | — | — |
| [08](08-INTERACTIVE-FEATURES.md) | Interactive Features | — | — |
| [09](09-DEMO-ENHANCEMENTS.md) | Demo Enhancements | — | — |
| [10](10-QUALITY-OF-LIFE.md) | Quality of Life | — | — |
| [11](11-SLASH-COMMANDS.md) | Slash Commands | — | — |

### Full-system audit (12-20)
| # | Title | Severity | Status |
|---|---|---|---|
| [12](12-PROVIDER-DISPATCH-AUDIT.md) | Provider Dispatch (latency, context, cost) | critical | open |
| [16](16-CHAT-SESSION-LIFECYCLE.md) | Chat Session Lifecycle (startup → exit) | critical | open |
| [17](17-PLAN-EXECUTION-WORKFLOW.md) | Plan & Execution Workflow | critical | open |
| [18](18-CONFIG-AND-ERRORS.md) | Configuration & Error Handling | medium | open |
| [19](19-SERVE-AND-SECURITY.md) | HTTP Server & Security | **critical (security)** | open |
| [20](20-SAFETY-LEARNING-KNOWLEDGE.md) | Safety, Learning, Knowledge | medium | open |

### What each audit doc covers

**Doc 12** — Provider dispatch layer:
- Fresh `reqwest::Client` per request (7.2s+ overhead)
- No system prompt, tools, history, or workspace context in chat
- Misleading "17x savings" metric based on hardcoded Opus pricing
- 10 differences between roko and mori dispatch

**Doc 16** — Chat session from startup to exit:
- Auth detection blocks startup, no validation, wrong priority order
- 6 slash commands that confirm success but do nothing (`/system`, `/effort`, `/gate`, etc.)
- Every turn is single-shot — no conversation history sent to model
- No streaming output — entire response appears at once

**Doc 17** — Plan generation and execution:
- One-shot mode gives agents zero codebase context
- `dangerously_skip_permissions: true` hardcoded for all plan runs
- Parallel execution hardcoded to 1 despite DAG support
- Two separate execution engines, neither deprecated

**Doc 18** — Configuration and error handling:
- 18+ `.ok()` calls silently swallowing errors in orchestrate.rs
- Missing env vars silently use empty strings, causing delayed failures
- Two separate init paths never connected
- No proxy support anywhere in codebase

**Doc 19** — HTTP server and security:
- Terminal PTY routes have NO authentication (full shell access)
- Auth disabled by default; PORT env var switches to 0.0.0.0 without auth
- CORS fully permissive by default
- `--share` creates PUBLIC gists with unscrubbed output

**Doc 20** — Safety, learning, and knowledge:
- LinUCB bandit weights not persisted (learning lost on restart)
- Episode compaction never automatically triggered
- Dream trigger consumer doesn't exist
- Cost tracking returns $0.00 for unknown models
