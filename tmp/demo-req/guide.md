# roko demo guide

## Quick start

```bash
# 1. Build the release binary (avoids compilation warnings in demos)
cargo build -p roko-cli --release

# 2. Initialize workspace (if not already done)
./target/release/roko init

# 3. Start the server
RUST_LOG=roko=warn,roko_serve=info ./target/release/roko serve

# 4. Open the demo
open http://localhost:6677
```

The root page at `http://localhost:6677/` links to everything.

---

## Prerequisites

- **Rust 1.91+**: `rustup update stable`
- **roko workspace**: a directory with `roko.toml` and `.roko/`
- **LLM provider**: set `ANTHROPIC_API_KEY` env var for agent dispatch
- **Pre-built binary**: `cargo build -p roko-cli --release`
- **Claude CLI**: `claude` must be on PATH for `roko acp` (install via `npm install -g @anthropic-ai/claude-code`)
- **gh CLI** (optional): for `--share` Gist uploads — `brew install gh && gh auth login`

### Suppressing chain/relay logs

```bash
RUST_LOG=roko=warn,roko_serve=info ./target/release/roko serve
```

---

## ACP — Editor Agent Integration

ACP (Agent Client Protocol) lets editors embed Roko as a coding agent alongside Claude, Codex, etc. Roko appears in the agent picker dropdown and streams tool calls, thinking, and text through a standard JSON-RPC 2.0 stdio protocol.

### How it works

1. The editor spawns `roko acp` as a subprocess
2. Communication happens via newline-delimited JSON-RPC over stdin/stdout
3. `roko acp` spawns `claude --print --output-format stream-json --verbose` for each prompt
4. Claude CLI stream events are translated to ACP `session/update` notifications
5. The editor renders text chunks, tool call cards, and thinking inline

### Protocol flow

```
Editor                          roko acp                     claude CLI
  │                                │                            │
  │──initialize───────────────────>│                            │
  │<─────────────capabilities──────│                            │
  │──session/new──────────────────>│                            │
  │<─────────────session_id────────│                            │
  │──session/prompt───────────────>│──spawn───────────────────>│
  │<─session/update (text chunk)───│<─assistant {text}──────────│
  │<─session/update (tool call)────│<─assistant {tool_use}──────│
  │<─session/update (tool done)────│<─tool {result}─────────────│
  │<─session/update (thinking)─────│<─assistant {thinking}──────│
  │<─────────────prompt result─────│<─result────────────────────│
  │                                │                            │
  │──session/cancel───────────────>│──kill──────────────────────│
```

### CLI flags

```
roko acp [OPTIONS]

Options:
  --workdir <PATH>      Working directory for agent operations (default: cwd)
  --profile <NAME>      Configuration profile (default: "default")
  --config <PATH>       Path to roko.toml
  --log-file <PATH>     Log file path (default: .roko/acp.log)
```

Logs go to `.roko/acp.log` by default — check this file for debugging.

### What gets streamed to the editor

| Claude CLI event | ACP session update | What the editor shows |
|---|---|---|
| `assistant` with `text` block | `agent_message_chunk` | Streamed text output |
| `assistant` with `thinking` block | `thought_message_chunk` | Thinking/reasoning (collapsible) |
| `assistant` with `tool_use` block | `tool_call` (status: in_progress) | Tool card: "Edit", "Bash", etc. |
| `tool` result | `tool_call_update` (status: completed) | Tool output content |
| `result` | Prompt result with usage | Final token counts + cost |

### Editor setup

#### Zed

Add to `~/.config/zed/settings.json`:

```json
{
  "agent_servers": {
    "Roko": {
      "type": "custom",
      "command": "/path/to/roko",
      "args": ["acp"],
      "env": {}
    }
  }
}
```

With a specific project directory:

```json
{
  "agent_servers": {
    "Roko": {
      "type": "custom",
      "command": "/path/to/roko",
      "args": ["acp", "--workdir", "/path/to/your/project"],
      "env": {}
    }
  }
}
```

After adding, restart Zed. "Roko" appears in the agent dropdown in the ACP panel.

**Using a debug build:**

```json
{
  "agent_servers": {
    "Roko (dev)": {
      "type": "custom",
      "command": "/Users/will/dev/nunchi/roko/roko/target/debug/roko",
      "args": ["acp"],
      "env": {}
    }
  }
}
```

#### Cursor

Add to `.cursor/mcp.json` or Cursor settings under agent servers:

```json
{
  "agent_servers": {
    "roko": {
      "command": "/path/to/roko",
      "args": ["acp"],
      "env": {}
    }
  }
}
```

Cursor uses the same ACP protocol as Zed for custom agent integrations.

#### JetBrains IDEs (IntelliJ, WebStorm, PyCharm, etc.)

JetBrains IDEs support ACP agents via the **AI Assistant** plugin (2025.2+). Add to your IDE settings:

1. Open **Settings → Tools → AI Assistant → Agent Servers**
2. Click **+** to add a custom agent
3. Configure:
   - **Name**: `Roko`
   - **Command**: `/path/to/roko`
   - **Arguments**: `acp`
   - **Working directory**: your project root (or leave blank for cwd)

Or add to your project's `.idea/ai-agents.json`:

```json
{
  "agents": [
    {
      "name": "Roko",
      "type": "custom",
      "command": "/path/to/roko",
      "args": ["acp"]
    }
  ]
}
```

#### Neovim (with an ACP client plugin)

If using an ACP-compatible Neovim plugin:

```lua
-- lua/plugins/acp.lua
require("acp").setup({
  servers = {
    roko = {
      command = "/path/to/roko",
      args = { "acp" },
    },
  },
})
```

#### VS Code

VS Code supports ACP agents through extensions. If you have an ACP client extension:

Add to `.vscode/settings.json`:

```json
{
  "acp.servers": {
    "roko": {
      "command": "/path/to/roko",
      "args": ["acp"],
      "env": {}
    }
  }
}
```

### Testing the ACP server manually

You can test the protocol directly from the command line:

```bash
# Send initialize + session/new + session/prompt as separate lines
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{}}}
{"jsonrpc":"2.0","id":2,"method":"session/new","params":{"mcp_servers":[]}}' | roko acp 2>/dev/null
```

Expected output (two JSON lines):

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"agentCapabilities":{"loadSession":true,"promptCapabilities":{"image":false,"audio":false,"embeddedContext":true},"mcpCapabilities":{"http":true,"sse":true}},"agentInfo":{"name":"roko","title":"Roko","version":"0.1.0"},"authMethods":[]}}
{"jsonrpc":"2.0","id":2,"result":{"sessionId":"sess_...","configOptions":[],"modes":{"currentModeId":"code","availableModes":[...]}}}
```

### Conventions

- **One ACP server per editor window.** The editor spawns `roko acp` once; all sessions share the process.
- **Sessions are isolated.** Each `session/new` creates independent state. Multiple chat threads = multiple sessions.
- **Workdir matters.** The `--workdir` flag (or cwd) determines where `claude` operates. It should point to your project root.
- **Cancellation is cooperative.** `session/cancel` signals the cancel token; the Claude CLI child process is killed.
- **Modes are advisory.** The editor can set modes (code, plan, research) but the current implementation passes all prompts to Claude CLI uniformly. Mode-specific behavior is planned.
- **Logs for debugging.** All ACP traffic is logged to `.roko/acp.log`. Set `RUST_LOG=roko_acp=trace` for verbose logging.

### Capabilities advertised

| Capability | Value | Meaning |
|---|---|---|
| `loadSession` | `true` | Sessions can be restored |
| `promptCapabilities.image` | `false` | No image input (yet) |
| `promptCapabilities.embeddedContext` | `true` | Context blocks are supported |
| `mcpCapabilities.http` | `true` | HTTP MCP servers accepted |
| `mcpCapabilities.sse` | `true` | SSE MCP servers accepted |

### Troubleshooting ACP

| Problem | Fix |
|---|---|
| Agent doesn't appear in editor | Check the binary path exists and is executable. Restart the editor. |
| "failed to spawn claude CLI" | Install Claude CLI: `npm i -g @anthropic-ai/claude-code`. Verify `claude --version` works. |
| No output after prompt | Check `.roko/acp.log` for errors. Verify `ANTHROPIC_API_KEY` is set. |
| Editor shows "disconnected" | The ACP process crashed. Check `roko acp` runs standalone first. |
| Tool calls don't render | Your editor may not support ACP tool call cards. Text output still streams. |
| Slow first response | Claude CLI has a cold start. Subsequent prompts in the same session reuse the connection. |

---

## CLI commands reference

### Core

| Command | What it does |
|---|---|
| `roko run "<prompt>"` | Single prompt → universal loop (compose → agent → gate → persist) |
| `roko run "<prompt>" --share` | Same + uploads transcript to GitHub Gist |
| `roko run "<prompt>" --serve` | Same + starts HTTP control plane for live observability |
| `roko acp` | Start ACP agent server (editor integration via stdio JSON-RPC) |
| `roko agent chat --agent <name>` | Interactive Claude Code-like chat REPL |
| `roko status` | Workspace health: signals, episodes, cost |
| `roko status --cfactor` | Full C-Factor composite quality score |
| `roko resume` | Resume last plan from checkpoint |
| `roko resume <run_id>` | Resume a specific plan run |
| `roko replay <hash>` | Walk signal DAG from a hash |
| `roko replay <hash> --as-of "step 5"` | Filter to events from step 5 forward |
| `roko replay <hash> --format json` | Output as JSON lines |

### Self-hosting workflow

| Command | What |
|---|---|
| `roko init` | Create `.roko/` and `roko.toml` |
| `roko prd idea "<text>"` | Capture a work item |
| `roko prd draft new <slug>` | Draft a PRD |
| `roko research enhance-prd <slug>` | Enrich PRD with research |
| `roko prd plan <slug>` | Generate implementation plan from PRD |
| `roko plan run plans/` | Execute the plan |
| `roko plan run plans/ --resume-plan` | Resume from checkpoint |

### Benchmarks

| Command | What |
|---|---|
| `roko bench demo` | Naive vs optimized comparison (simulated) |
| `roko bench demo --real` | Same with real LLM dispatch |
| `roko bench swe --batch-size N` | SWE-bench proxy harness |

### Demo management

| Command | What |
|---|---|
| `roko demo setup` | Build release binary, verify workspace |
| `ROKO_DEMO_CACHE=1 roko demo warm` | Pre-warm LLM response cache |

### Knowledge + learning

| Command | What |
|---|---|
| `roko knowledge stats` | Knowledge store statistics |
| `roko knowledge query "<topic>"` | Search durable knowledge |
| `roko learn all` | Full learning state overview |
| `roko learn efficiency` | Per-model cost/token efficiency |
| `roko learn tune gates` | Tune adaptive gate thresholds |
| `roko agent list` | List agents with identity + status |

---

## Web pages

Start with `./target/release/roko serve`, then open in browser.

### Root index — `http://localhost:6677/`

Links to all demo surfaces and API endpoints.

### Builder — `http://localhost:6677/demo/builder.html`

**The Stripe moment.** Type a request, watch roko build it live in a temporary repo.

**How it works:**
1. Type "Build a CLI calculator in Rust" (or click a task card)
2. Terminal creates a temp directory
3. Agent scaffolds the project, writes code, runs tests
4. Gate pipeline shows compile ✔ / test ✔ / clippy ✔ in real time
5. File tree sidebar updates as files appear

**Layout:** File tree (left) | Terminal (center) | Gates bar (bottom) | Chat input (bottom)

**Pre-set tasks:** calculator, REST API, md→html converter, file dedup, commit message generator

### Terminal — `http://localhost:6677/demo/terminal.html`

Multiple real terminals in the browser, each connected to a real PTY via WebSocket.

**Buttons:**
- **+ Terminal** — add a pane
- **Run All Demos** — 4 terminals run demo commands in parallel
- **1 / 2 / 4** — switch grid layout
- **Clear All** — destroy all sessions

**"Run All Demos" runs:**
1. Self-hosting: `roko init → prd idea → status`
2. Benchmark: `roko bench demo`
3. Agents: `roko agent list → status --cfactor`
4. Learning: `roko learn efficiency → learn all`

### Scripted demo — `http://localhost:6677/demo/index.html`

Pre-baked JavaScript animation. **No server needed.** Works offline. Click "Run Demo" for the full self-hosting sequence with typing animation, spinners, cost waterfall, and session summary.

### Shareable runs — `http://localhost:6677/runs/{id}`

After `roko run --share`, the transcript is viewable as a styled HTML page at this URL. Also available as JSON at `/api/runs/{id}`.

---

## REST API (selected endpoints)

| Method | Path | Description |
|---|---|---|
| `GET` | `/health` | Liveness probe → `{"status":"ok"}` |
| `GET` | `/api/health` | Detailed health |
| `GET` | `/api/status` | Workspace status |
| `GET` | `/api/episodes` | Episode list |
| `GET` | `/api/metrics` | Prometheus metrics |
| `POST` | `/api/run` | Start a background run |
| `GET` | `/api/run/{id}/status` | Poll run status |
| `GET` | `/api/plans` | List plans |
| `POST` | `/api/agents/{id}/message` | Send message to agent |
| `GET` | `/api/events` | SSE event stream |
| `GET` | `/ws` | WebSocket event stream |
| `POST` | `/api/terminal/sessions` | Create PTY session |
| `GET` | `/api/terminal/sessions` | List PTY sessions |
| `GET` | `/ws/terminal/{id}` | WebSocket PTY bridge |
| `GET` | `/api/runs/{id}` | Run transcript (JSON) |
| `GET` | `/runs/{id}` | Run transcript (HTML) |

---

## Chat

```bash
# In terminal 1:
./target/release/roko serve

# In terminal 2:
./target/release/roko agent chat --agent researcher
```

**Features:**
- Inline ratatui viewport with spinner during thinking
- Markdown-rendered agent responses
- `/help`, `/cost`, `/quit` commands
- Ctrl-C to interrupt, Ctrl-D to exit
- Up/down for command history
- Session cost tracking in status bar

**If you get errors:**
- `"POST ... is roko serve running?"` — start `roko serve` first
- `"502: ..."` — check `ANTHROPIC_API_KEY` is set
- `"404: ..."` — the agent isn't registered; serve's fallback `run_once` should handle it

---

## Benchmark comparison

```bash
./target/release/roko bench demo
```

Runs 5 tasks in two modes:

| Mode | Config |
|---|---|
| **Naive** | Single model (opus), no caching, no routing, no knowledge |
| **Optimized** | CascadeRouter + caching + knowledge + gate early-exit |

**Output includes:**
- Per-task: pass/fail, cost, tokens, latency, model, cache hit rate
- Comparison table: naive vs optimized, improvement percentages
- Cost waterfall: decomposed savings (caching 5x, routing 3.1x, knowledge 1.4x, gates 1.2x)
- Session summary: totals, gate pass count, replans, primary model

---

## Response cache for determinism

```bash
# Enable file-backed cache
export ROKO_DEMO_CACHE=1

# First run hits real API, caches response
./target/release/roko run "Build a calculator"

# Second run returns instantly from cache
./target/release/roko run "Build a calculator"

# Cache lives at .roko/demo-cache/{blake3_hash}.json
ls .roko/demo-cache/

# Pre-warm multiple prompts
./target/release/roko demo warm

# Clear cache
rm -rf .roko/demo-cache/
```

---

## Visual primitives (for developers)

```bash
# Interactive demo of all 11 rendering primitives
cargo run -p roko-cli --example inline_demo

# Non-interactive version (pipe-safe)
cargo run -p roko-cli --example inline_autoplay

# Benchmark comparison output
cargo run -p roko-cli --example bench_autoplay
```

The primitives are reusable building blocks:

| Primitive | What |
|---|---|
| RunBlock | Completed run summary (agent/predict/gates/tools/cost/chain) |
| StreamingBlock | Live viewport with auto-scroll and cursor |
| ToolCallBlock | Collapsed/expanded tool call display |
| GateBlock | Gate pipeline with per-rung status |
| CostMeter | Cumulative session cost/tokens/cache |
| ErrorBlock | Structured error with severity + retry info |
| ReplanBlock | Gate failure → auto-replan visualization |
| SessionSummary | End-of-session roll-up |
| CostWaterfall | Decomposed savings breakdown |
| DiffBlock | File changes with +/- counts |
| ProgressTree | Hierarchical plan progress with waves |

---

## Troubleshooting

| Problem | Fix |
|---|---|
| "error send message" in chat | Start `roko serve` first. Check `ANTHROPIC_API_KEY`. |
| Compilation warnings in terminal | Use `./target/release/roko` not `cargo run`. |
| Chain/relay log noise | `RUST_LOG=roko=warn,roko_serve=info` |
| xterm.js text wrapping | Reload page (resize sync triggers on connect). |
| Demo page "server not reachable" | Start `roko serve` first. |
| `--share` says "gh CLI not found" | Install: `brew install gh && gh auth login` |
| Demo cache not working | Set `ROKO_DEMO_CACHE=1` env var before running. |
| ACP agent not in editor dropdown | Verify binary path, restart editor. Check `.roko/acp.log`. |
| ACP "failed to spawn claude CLI" | Install: `npm i -g @anthropic-ai/claude-code`. |
| Chat shows `400 invalid_request_error` | Provider not configured. Add `[providers.anthropic]` to `roko.toml` (see below). |

### Configuring providers for chat

The `roko agent chat` fallback path uses `run_once` which needs a configured provider. Add to `roko.toml`:

```toml
[providers.anthropic]
kind = "anthropic_api"
api_key_env = "ANTHROPIC_API_KEY"
default_model = "claude-sonnet-4-6"
```

Or use the Claude CLI backend (simpler — just needs `claude` on PATH):

```toml
[agent]
command = "claude"
```

Verify your API key is set: `echo $ANTHROPIC_API_KEY`
