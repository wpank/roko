# IMPL-08: Product surfaces and UX

**Implements:** PRD-08 (Deployment and user experience)
**Status:** Draft
**Date:** 2026-04-21
**Estimated effort:** 10-14 weeks across 7 phases

---

## Context

Roko is a Rust workspace at `/Users/will/dev/nunchi/roko/roko/` with 18 crates. Korai is the companion blockchain. This document specifies every task required to build the user-facing surfaces: CLI extensions for agent lifecycle management, CLI developer experience improvements, persistent chat, TUI enhancements, Agent Studio web interface, OpenClaw end-user product, and MCP distribution.

The guiding principle: the system's internal power is worthless if people cannot access it. These surfaces expose the capabilities built in IMPL-01 through IMPL-07 to three user populations: developers (CLI + TUI), operators (Agent Studio), and end users (OpenClaw).

### Workspace layout

| Crate | Path | Role in surfaces |
|-------|------|-----------------|
| `roko-cli` | `crates/roko-cli/` | CLI binary, TUI dashboard, all subcommands |
| `roko-serve` | `crates/roko-serve/` | HTTP control plane (~85 routes), WebSocket, SSE |
| `roko-agent-server` | `crates/roko-agent-server/` | Per-agent HTTP sidecar (13+ routes) |
| `roko-mcp-code` | `crates/roko-mcp-code/` | Code intelligence MCP server |
| `roko-agent` | `crates/roko-agent/` | Agent dispatch, LLM backends, tool loop |
| `roko-core` | `crates/roko-core/` | Config schema, agent types, shared types |

### What already exists

Substantial CLI and server infrastructure is wired. Read these files before writing anything:

**CLI entry point and commands:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` -- `Cli` struct with clap, `Subcommand` enum, 35+ subcommands, exit codes, effort levels, log format
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` contains: `init`, `run`, `plan` (list/show/create/run), `prd` (idea/list/status/draft/plan/consolidate), `research` (topic/enhance-prd/enhance-plan/enhance-tasks/analyze), `config` (init/show/path/edit/set), `status`, `replay`, `dashboard`, `serve`, `chat`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_serve.rs` -- `AgentCmd` subcommand (start/stop/list/status for agent lifecycle)

**TUI infrastructure:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/mod.rs` -- main TUI module
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/app.rs` -- `App` struct, main event loop
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/tabs.rs` -- F1-F7 tab navigation
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/theme.rs` -- configurable color themes
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/ansi.rs` -- ANSI rendering in ratatui
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/fs_watch.rs` -- `notify::RecommendedWatcher` for live file updates
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/ws_client.rs` -- WebSocket client for streaming from `roko serve`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/modals/` -- overlay system for confirmations and input
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/` -- reusable ratatui components
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/pages/` -- per-tab page implementations
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/views/` -- view components

**HTTP control plane:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` -- route registry
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` -- agent management routes
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` -- plan routes
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/status.rs` -- status routes
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs` -- WebSocket routes
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/sse.rs` -- Server-Sent Events

**Agent sidecar:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/lib.rs` -- `AgentServer` builder pattern, feature flags (messaging, predictions, research, tasks), bearer auth, 13 routes
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/` -- feature modules: health, messaging, predictions, research, tasks, logs
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/state.rs` -- `AgentState` with metrics, predictions, research, tasks

**MCP server:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs` -- code intelligence MCP server
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/main.rs` -- MCP binary entry point

**Critical:** The CLI already has 35+ subcommands. The TUI has F1-F7 tabs. The HTTP server has ~85 routes. The agent sidecar has 13 routes. Extend what exists. Do not rebuild.

---

## Phase 1: CLI extensions

Goal: add agent lifecycle management commands (`agent start/stop/list/status`), benchmarking commands, and environment diagnostics.

### Task 1.1: Implement `roko agent start --profile <name>`

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_serve.rs` (existing `AgentCmd`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (CLI structure, how commands dispatch)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/lib.rs` (`AgentServer::builder()`)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_serve.rs`

**What to implement:**

Start a persistent agent with a domain profile. The command:
1. Loads the domain profile (coding, blockchain, research, security)
2. Provisions extensions based on the profile
3. Starts the heartbeat loop
4. Starts the HTTP sidecar
5. Registers the agent in the local agent registry

```
roko agent start --profile blockchain --config chain.toml --name blockchain-1 --serve 0.0.0.0:8080
```

**Checklist:**
- [ ] Add `--profile <name>` flag to `AgentCmd::Start` (required: coding, blockchain, research, security, custom)
- [ ] Add `--config <path>` flag for agent-specific configuration file
- [ ] Add `--name <name>` flag (auto-generated UUID suffix if omitted)
- [ ] Add `--serve <addr>` flag for sidecar address (default: 127.0.0.1:0 for auto-assigned port)
- [ ] Add `--observe-only` flag to start in observation mode (no execution authority)
- [ ] Add `--mcp-config <path>` flag for MCP server configuration
- [ ] Load profile from `roko.toml` or built-in defaults
- [ ] Start the `AgentServer` from `roko-agent-server` with the configured features
- [ ] Register the running agent in `.roko/agents.json` (name, PID, sidecar address, profile, start time)
- [ ] Print startup summary: name, profile, extensions loaded, sidecar URL
- [ ] Test: `cargo run -p roko-cli -- agent start --profile coding` starts and prints sidecar URL
- [ ] Test: agent appears in `.roko/agents.json` after start

**Test:** `cargo test -p roko-cli -- agent_start`

---

### Task 1.2: Implement `roko agent list`

**Read first:**
- Task 1.1 (agent registry in `.roko/agents.json`)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_serve.rs`

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_serve.rs`

**What to implement:**

List all agents (running, stopped, errored) from the local agent registry.

```
$ roko agent list
NAME           STATUS   PROFILE      SIDECAR                STARTED
blockchain-1   running  blockchain   http://127.0.0.1:8901  2m ago
research-1     stopped  research     -                      1h ago
coding-a7f3    error    coding       -                      3h ago (exit code 1)
```

**Checklist:**
- [ ] Add `AgentCmd::List` variant with `--status <filter>` (running, stopped, error, all)
- [ ] Add `--profile <name>` filter
- [ ] Add `--format <fmt>` flag (table, json, csv)
- [ ] Read `.roko/agents.json` for registered agents
- [ ] Check PID liveness: if PID exists and process is alive -> running, else -> stopped/error
- [ ] Format output as aligned table (default), JSON, or CSV
- [ ] Handle empty registry: print "No agents registered" instead of empty table
- [ ] Test: register 2 agents, list, verify both appear
- [ ] Test: stop 1 agent, list with `--status running`, verify only 1 appears

**Test:** `cargo test -p roko-cli -- agent_list`

---

### Task 1.3: Implement `roko agent stop <id>`

**Read first:** Tasks 1.1, 1.2

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_serve.rs`

**What to implement:**

Gracefully stop a running agent.

```
$ roko agent stop blockchain-1
Stopping agent 'blockchain-1'... done (ran for 2h 14m)
```

**Checklist:**
- [ ] Add `AgentCmd::Stop` variant with `<id>` positional argument
- [ ] Add `--force` flag (send SIGKILL instead of SIGTERM)
- [ ] Add `--timeout <secs>` flag (grace period before force-kill, default 30s)
- [ ] Look up agent by name in `.roko/agents.json`
- [ ] Send SIGTERM (or SIGKILL with --force) to the agent's PID
- [ ] Wait for process to exit (up to timeout)
- [ ] Update `.roko/agents.json` status to "stopped"
- [ ] Print runtime duration
- [ ] Handle "agent not found" and "agent already stopped" errors with clear messages
- [ ] Test: start agent, stop it, verify status changes to stopped

**Test:** `cargo test -p roko-cli -- agent_stop`

---

### Task 1.4: Implement `roko agent status <id>`

**Read first:** Tasks 1.1, 1.2

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/agent_serve.rs`

**What to implement:**

Detailed status for a single agent, querying its sidecar for live data.

```
$ roko agent status blockchain-1
Name:        blockchain-1
Status:      running
Profile:     blockchain
Sidecar:     http://127.0.0.1:8901
Started:     2026-04-21 14:32:01 (2h 14m ago)
PID:         42891

Extensions:  ChainSubscriber [OK], HedgeManager [OK], CostTracker [OK]
Tick:        4,523 (gamma, 5s interval)
Regime:      Normal
Vitality:    0.72
T0/T1/T2:    94.2% / 3.8% / 2.0%
Cost (24h):  $28.43
```

**Checklist:**
- [ ] Add `AgentCmd::Status` variant with `<id>` positional argument
- [ ] Add `--json` flag for machine-readable output
- [ ] Look up agent in `.roko/agents.json`
- [ ] If agent is running, HTTP GET the sidecar's `/stats` endpoint for live metrics
- [ ] Parse and display: tick count, regime, vitality, tier distribution, cost
- [ ] If sidecar is unreachable, show "sidecar unreachable" but still display static info from registry
- [ ] Test: start agent, query status, verify fields populated

**Test:** `cargo test -p roko-cli -- agent_status`

---

### Task 1.5: Implement `roko chat --agent <id>`

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (existing chat subcommand)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/` (messaging feature)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/ws_client.rs` (WebSocket client)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (chat dispatch)

**What to implement:**

Persistent chat with a running agent via the sidecar's `/stream` WebSocket endpoint.

```
$ roko chat --agent blockchain-1
[blockchain-1] Connected. Phase: Active | Vitality: 0.72 | Tick: 4,523
you> How are positions performing?
[blockchain-1] Checking position state...
[blockchain-1] ETH/USDC LP: +2.3% (7d), IL: -0.4%, net: +1.9%
you>
```

**Checklist:**
- [ ] Resolve agent name to sidecar address from `.roko/agents.json`
- [ ] Open WebSocket connection to `ws://<addr>/stream`
- [ ] Display initial status frame from agent
- [ ] Implement readline loop: user types message, send as JSON to WebSocket, display response
- [ ] Handle connection loss: reconnect with exponential backoff, notify user
- [ ] Support Ctrl+C to exit chat (not kill the agent)
- [ ] Store chat history in `.roko/chat/<agent-id>/` as JSONL
- [ ] Add `--session <id>` flag to load a previous session
- [ ] Implement command parsing: messages starting with `/` are directives (e.g., `/status`, `/observe-only`)
- [ ] Test: connect to agent sidecar, send message, receive response

**Test:** `cargo test -p roko-cli -- chat_connect` (requires running agent -- feature-gated integration test)

---

### Task 1.6: Implement `roko bench arena --name <arena>`

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (CLI structure)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/bench.rs`

**What to implement:**

Run arena evaluation against a named arena. Arenas are domain-specific evaluation suites.

```
$ roko bench arena --name swe-bench --batches 5
Running SWE-bench arena (batch 1/5)...
  Problem: django__django-16527  PASS  (4.2s, $0.08)
  Problem: astropy__astropy-14995 FAIL  (8.1s, $0.15)
  ...
Batch 1 complete: 12/20 passed (60.0%)
```

**Checklist:**
- [ ] Add `Bench` subcommand to CLI with `arena`, `swe`, `report` sub-subcommands
- [ ] Add `--name <arena>` to `arena` subcommand (swe-bench, oracle-resolution, risk-detection)
- [ ] Add `--batches <n>` flag (number of evaluation batches, default 1)
- [ ] Add `--agent <id>` flag (evaluate a specific running agent, default: spawn fresh)
- [ ] Add `--compare <ids>` flag (compare multiple agents head-to-head)
- [ ] Implement arena discovery: look for arena definitions in `.roko/arenas/` and built-in defaults
- [ ] Implement batch execution: run problems sequentially within a batch, report pass/fail/cost per problem
- [ ] Implement `report` subcommand: generate evaluation report (markdown, json, html)
- [ ] Test: run a mock arena with 3 problems, verify pass/fail counts

**Test:** `cargo test -p roko-cli -- bench_arena`

---

### Task 1.7: Implement `roko doctor`

**Read first:**
- PRD-08 section 4 (P2-05: roko doctor)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs`

**What to implement:**

Environment validation command that checks all prerequisites.

```
$ roko doctor
  Rust toolchain:    1.91.0      OK
  .roko/ directory:  present     OK
  roko.toml:         valid       OK
  Claude API key:    set         OK
  Git:               2.44.0      OK
  MCP config:        .mcp.json   OK (3 servers configured)
  Disk space:        42GB free   OK

All checks passed.
```

**Checklist:**
- [ ] Add `Doctor` subcommand to CLI (no arguments)
- [ ] Check Rust toolchain version: `rustc --version`, verify >= 1.91
- [ ] Check `.roko/` directory exists
- [ ] Check `roko.toml` exists and parses without error
- [ ] Check `ANTHROPIC_API_KEY` environment variable is set (mask the value, show "set" or "not set")
- [ ] Check git version: `git --version`
- [ ] Check MCP config: look for `.mcp.json` or `mcp_config` in `roko.toml`, count servers
- [ ] Check disk space: verify at least 1GB free in the working directory
- [ ] Check write permissions: verify `.roko/` is writable
- [ ] Print colored OK/FAIL per check (respect NO_COLOR)
- [ ] Print summary: "All checks passed" or "N issues found:" with remediation hints
- [ ] If API key missing, print exact commands to set it
- [ ] Test: run doctor in a valid environment, verify all checks pass

**Test:** `cargo test -p roko-cli -- doctor`

---

### Task 1.8: End-to-end CLI test

**Read first:** Tasks 1.1-1.7

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/cli_e2e.rs`

**What to implement:**

Integration test that exercises the full agent lifecycle through the CLI.

**Checklist:**
- [ ] `roko doctor` -> all checks pass (or expected subset in CI)
- [ ] `roko agent start --profile coding --name test-agent` -> agent starts, appears in registry
- [ ] `roko agent list` -> test-agent appears with status "running"
- [ ] `roko agent status test-agent` -> displays live metrics
- [ ] `roko agent stop test-agent` -> agent stops
- [ ] `roko agent list --status stopped` -> test-agent appears as stopped
- [ ] Each step asserts exit code 0 (or expected code for error cases)

**Test:** `cargo test -p roko-cli --test cli_e2e`

---

## Phase 2: CLI DX (developer experience quick wins)

Goal: polish the CLI with shell init, NO_COLOR compliance, command timing, enhanced version output, and shell completions.

### Task 2.1: Shell init

**Read first:** PRD-08 section 4 (P1-01)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/shell_init.rs`

**What to implement:**

`eval "$(roko shell-init zsh)"` outputs shell functions and completions.

**Checklist:**
- [ ] Add `ShellInit` subcommand with `<shell>` argument (zsh, bash, fish)
- [ ] Generate shell completion script using `clap_complete`
- [ ] Generate convenience aliases (if configured in `roko.toml`)
- [ ] Generate PATH addition for `~/.roko/bin/` if not already present
- [ ] Output to stdout for piping to `eval`
- [ ] Test: `roko shell-init zsh` outputs valid zsh code (parse check)
- [ ] Test: `roko shell-init bash` outputs valid bash code

**Test:** `cargo test -p roko-cli -- shell_init`

---

### Task 2.2: NO_COLOR / CLICOLOR compliance

**Read first:** PRD-08 section 4 (P1-02)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` (startup)

**What to implement:**

Respect the `NO_COLOR` environment variable (no-color.org) and `CLICOLOR` / `CLICOLOR_FORCE` for broader compatibility.

**Checklist:**
- [ ] Define `ColorMode` enum: `Always`, `Auto`, `Never`
- [ ] Check at startup: `NO_COLOR` set -> `Never`, `CLICOLOR_FORCE=1` -> `Always`, `CLICOLOR=0` -> `Never`, else `Auto`
- [ ] In `Auto` mode, check `stdout.is_terminal()` -- disable color if not a TTY
- [ ] Propagate `ColorMode` through a global or thread-local
- [ ] Wire into all output formatting (tables, status indicators, error messages)
- [ ] Wire into TUI theme (TUI always uses color since it requires a terminal)
- [ ] Test: set `NO_COLOR=1`, run `roko status`, verify no ANSI escape codes in output
- [ ] Test: unset `NO_COLOR`, pipe to file, verify no ANSI codes (non-TTY detection)

**Test:** `cargo test -p roko-cli -- color_mode`

---

### Task 2.3: Command timing display

**Read first:** PRD-08 section 4 (P1-03)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**What to implement:**

Print elapsed wall time and token cost for every command that dispatches to an agent.

```
Completed in 4.2s | 12,340 tokens | $0.03
```

**Checklist:**
- [ ] Add `--timing` global flag to `Cli`
- [ ] Check `ROKO_TIMING=1` environment variable as alternative
- [ ] Wrap command execution with `Instant::now()` / `elapsed()`
- [ ] Collect token count and cost from agent dispatch (read from orchestrate.rs efficiency events)
- [ ] Print timing line to stderr (not stdout, to avoid interfering with piped output)
- [ ] Only print timing for commands that actually dispatch to agents (not `config show`, `agent list`, etc.)
- [ ] Test: run a command with `--timing`, verify timing line appears on stderr

**Test:** `cargo test -p roko-cli -- command_timing`

---

### Task 2.4: Enhanced --version

**Read first:** PRD-08 section 4 (P1-04)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**What to implement:**

```
roko 0.4.2 (rustc 1.91.0, target aarch64-apple-darwin, git 5dd7f46)
```

**Checklist:**
- [ ] Add `built` or `shadow-rs` dependency for compile-time metadata injection
- [ ] Override clap's `--version` output to include: rustc version, target triple, git short hash
- [ ] Include build timestamp (ISO 8601)
- [ ] Handle missing git info gracefully (e.g., building from tarball without .git)
- [ ] Test: `roko --version` includes rustc version and git hash

**Test:** `cargo test -p roko-cli -- version_info`

---

### Task 2.5: Shell completions for all shells

**Read first:** PRD-08 section 4 (P1-05)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**What to implement:**

`roko completions <shell>` generates completion scripts for bash, zsh, fish, PowerShell, nushell.

**Checklist:**
- [ ] Add `Completions` subcommand with `<shell>` argument (bash, zsh, fish, powershell, nushell)
- [ ] Use `clap_complete` to generate completions from the CLI definition
- [ ] Output to stdout for redirection: `roko completions zsh > ~/.zfunc/_roko`
- [ ] Generate dynamic completions where possible (agent names from `roko agent list`, plan names from `roko plan list`)
- [ ] Test: `roko completions zsh` produces non-empty output
- [ ] Test: `roko completions bash` produces non-empty output
- [ ] Test: generated zsh completion is syntactically valid

**Test:** `cargo test -p roko-cli -- completions`

---

## Phase 3: Persistent chat

Goal: WebSocket-based operator chat with a running agent, supporting status queries, directives, and session persistence.

### Task 3.1: WebSocket server in agent sidecar

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/lib.rs` (server builder, feature flags)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/` (existing features)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/chat.rs`

**What to implement:**

Add a `/chat` WebSocket endpoint to the agent sidecar for persistent bidirectional communication.

**Checklist:**
- [ ] Create `chat.rs` in the features directory
- [ ] Implement WebSocket upgrade handler at `/chat`
- [ ] Define `ChatMessage` types: `UserMessage { text: String, timestamp: u64 }`, `AgentMessage { text: String, source: String, timestamp: u64 }`
- [ ] Implement connection lifecycle: on connect, send initial status frame (phase, vitality, tick, regime)
- [ ] Implement message routing: incoming messages route through the agent's extension chain
- [ ] Implement heartbeat (ping/pong every 30s) to detect dead connections
- [ ] Support multiple concurrent chat connections
- [ ] Add `chat` feature flag to `FeatureFlags`
- [ ] Wire into `AgentServerBuilder::chat()` method
- [ ] Unit test: WebSocket connects, receives status frame, sends message, receives response

**Test:** `cargo test -p roko-agent-server -- chat_ws`

---

### Task 3.2: Chat message routing through extensions

**Read first:**
- Task 3.1
- `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/state.rs` (`AgentState`)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/chat.rs`

**What to implement:**

User messages route through the agent's extension `on_message()` hooks. Each loaded extension gets an opportunity to contribute to the response.

**Checklist:**
- [ ] Define `MessageHandler` trait: `fn on_message(&self, msg: &str) -> Option<String>`
- [ ] Route incoming chat messages through all registered handlers
- [ ] Collect responses from each handler, format as a unified response
- [ ] Handle no-response case: if no handler responds, use the LLM backend as fallback
- [ ] Implement response streaming: send partial responses as they arrive (not all at once)
- [ ] Tag each response segment with the source extension name
- [ ] Unit test: register 2 handlers, send message, verify both contribute to response

**Test:** `cargo test -p roko-agent-server -- chat_routing`

---

### Task 3.3: Status display and command parsing

**Read first:** Tasks 3.1, 3.2

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/src/features/chat.rs`

**What to implement:**

Status display shows agent state. Command parsing distinguishes directives from free text.

Commands:
- `/status` -- current agent state (phase, vitality, tick, regime)
- `/ticks <n>` -- show last N ticks with tier decisions and costs
- `/observe-only` -- switch to observation mode
- `/resume` -- switch back to active mode
- `/extensions` -- list loaded extensions and their status

Free text (no `/` prefix) routes to extension handlers and LLM.

**Checklist:**
- [ ] Implement command parser: messages starting with `/` are commands, everything else is free text
- [ ] Implement `/status` handler: read from `AgentState`, format as status block
- [ ] Implement `/ticks <n>` handler: return last N tick summaries with tier/cost
- [ ] Implement `/observe-only` and `/resume` handlers: update runtime mode
- [ ] Implement `/extensions` handler: list loaded extensions with OK/ERROR status
- [ ] Free text: pass to extension chain (Task 3.2) then LLM fallback
- [ ] Unknown commands: respond with "Unknown command. Available: /status, /ticks, /observe-only, /resume, /extensions"
- [ ] Unit test: send `/status`, verify structured status response
- [ ] Unit test: send free text, verify routed to extension chain

**Test:** `cargo test -p roko-agent-server -- chat_commands`

---

### Task 3.4: Chat integration test

**Read first:** Tasks 3.1-3.3, Task 1.5

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-agent-server/tests/chat_e2e.rs`

**What to implement:**

End-to-end: start agent server with chat feature, connect via WebSocket, exchange messages.

**Checklist:**
- [ ] Start an `AgentServer` with `.chat()` enabled
- [ ] Connect WebSocket client to `/chat`
- [ ] Verify initial status frame received
- [ ] Send `/status` command, verify structured response
- [ ] Send free text message, verify response received
- [ ] Send `/extensions`, verify extension list returned
- [ ] Disconnect, verify server handles cleanup
- [ ] Reconnect, verify new status frame

**Test:** `cargo test -p roko-agent-server --test chat_e2e`

---

## Phase 4: TUI enhancements

Goal: add cognitive frequency visualization, CorticalState heatmap, extension activity timeline, and cost tracking with tier breakdown.

### Task 4.1: Cognitive frequency visualization

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/pages/` (existing page implementations)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/` (existing widgets)
- PRD-08 section 6 (cognitive frequency bars: gamma/theta/delta)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/frequency_bars.rs`

**What to implement:**

Three horizontal bars per agent showing gamma, theta, and delta cycle progress:

```
Gamma  |:::::::::..........|  tick #4,523  5s interval  T0: 94%
Theta  |:::::..............|  consolidation #312  in 45s
Delta  |:...................|  next dream cycle in 2h 14m
```

**Checklist:**
- [ ] Create `FrequencyBars` widget implementing `ratatui::Widget`
- [ ] Accept agent state data: current tick, tick interval, tier distribution, theta/delta timing
- [ ] Render 3 horizontal bars with proportional fill based on position within current cycle
- [ ] Color code by tier: green for T0-dominated, yellow for T1, red for T2
- [ ] Display cycle metadata: tick number, interval, tier percentage, time to next consolidation/dream
- [ ] Update every gamma tick (driven by WebSocket events from sidecar)
- [ ] Add to the F2 (Agents) tab page
- [ ] Test: render widget with mock data, verify bar widths are proportional to cycle progress

**Test:** `cargo test -p roko-cli -- frequency_bars`

---

### Task 4.2: CorticalState heatmap

**Read first:**
- PRD-08 section 6 (CorticalState heatmap)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/`

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/cortical_heatmap.rs`

**What to implement:**

A grid visualization of the agent's internal cognitive state:

```
             Low ---- Med ---- High
Affect       [##........]  0.23
Vitality     [#######...]  0.72
Pred. Error  [##........]  0.18
Arousal      [###.......]  0.31
Confidence   [########..]  0.81
Sleep Press  [#...........]  0.09
```

**Checklist:**
- [ ] Create `CorticalHeatmap` widget implementing `ratatui::Widget`
- [ ] Accept CorticalState values: affect, vitality, prediction_error, arousal, confidence, sleep_pressure
- [ ] Render as labeled horizontal bars with value labels
- [ ] Color code: green for healthy ranges, yellow for attention, red for intervention
- [ ] Define healthy ranges per dimension (e.g., vitality > 0.5 = green, 0.3-0.5 = yellow, <0.3 = red)
- [ ] Compact mode: single-line summary when the agent is in cruise mode
- [ ] Add to the F2 (Agents) tab page alongside frequency bars
- [ ] Test: render with mock CorticalState, verify correct colors and bar widths

**Test:** `cargo test -p roko-cli -- cortical_heatmap`

---

### Task 4.3: Extension activity timeline

**Read first:**
- PRD-08 section 6 (extension timeline)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/`

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/activity_timeline.rs`

**What to implement:**

A scrollable timeline showing extension-level events:

```
14:32:01  ChainSubscriber  Block 19,847,231  3 relevant txns  T0
14:32:06  ChainSubscriber  Block 19,847,232  0 relevant txns  T0
14:32:11  HedgeManager     Rate check         no action       T0
14:32:16  ChainSubscriber  Block 19,847,233  1 relevant txn   T1
14:32:17  HedgeManager     Rate deviation     adjust hedge    T2 ($0.05)
```

**Checklist:**
- [ ] Create `ActivityTimeline` widget implementing `ratatui::StatefulWidget`
- [ ] Define `TimelineEvent` struct: `{ timestamp, extension_name, description, action, tier, cost }`
- [ ] Accept a `Vec<TimelineEvent>` as data source
- [ ] Render as a scrollable list with timestamp, extension, description, tier, and cost columns
- [ ] Color each row by tier (T0=green, T1=yellow, T2=red)
- [ ] Support scrolling: up/down arrow keys navigate through events
- [ ] Support filtering: press `f` to filter by extension name
- [ ] Receive events via WebSocket from the agent sidecar's SSE or event stream
- [ ] Add to the F2 (Agents) tab page
- [ ] Test: render with 20 mock events, verify scroll behavior

**Test:** `cargo test -p roko-cli -- activity_timeline`

---

### Task 4.4: Cost tracking with tier breakdown

**Read first:**
- PRD-08 section 6 (cost tracking, stacked area chart)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/pages/`

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/widgets/cost_chart.rs`

**What to implement:**

Stacked area chart showing T0/T1/T2 cost distribution over time, displayed on the F5 (Cost) tab.

```
Cost: Last 24h                      Total: $28.43

$3 |    .
   |   / \     .
$2 |  /   \   / \         T2 (red)
   | /     \_/   \
$1 |/              \___   T1 (yellow)
   |                      T0 (green)
$0 +--+--+--+--+--+--+--
   0h 4h 8h 12 16 20 24

Breakdown:
  T0: 11,234 ticks   $0.00  (82.1%)
  T1:  1,891 ticks   $1.89  (13.8%)
  T2:    558 ticks  $26.54   (4.1%)
```

**Checklist:**
- [ ] Create `CostChart` widget implementing `ratatui::StatefulWidget`
- [ ] Accept time-series data: `Vec<(timestamp, t0_cost, t1_cost, t2_cost)>`
- [ ] Render stacked area chart using ratatui sparkline or custom drawing
- [ ] Show T0/T1/T2 breakdown below the chart with tick counts, costs, and percentages
- [ ] Support time range selection: hourly, daily, weekly (press `h`, `d`, `w` to switch)
- [ ] Support per-agent filtering when multiple agents are running
- [ ] Display total cost prominently in the header
- [ ] Read cost data from efficiency events in `.roko/learn/efficiency.jsonl`
- [ ] Add to the F5 (Cost) tab page
- [ ] Test: render with 24 hours of mock cost data, verify chart renders and breakdown sums correctly

**Test:** `cargo test -p roko-cli -- cost_chart`

---

### Task 4.5: TUI integration test

**Read first:** Tasks 4.1-4.4

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/tests/tui_widgets.rs`

**What to implement:**

Test that all new TUI widgets render without panicking with realistic mock data.

**Checklist:**
- [ ] Create a ratatui `TestBackend` with 120x40 terminal size
- [ ] Render `FrequencyBars` with mock agent state
- [ ] Render `CorticalHeatmap` with mock CorticalState values
- [ ] Render `ActivityTimeline` with 50 mock events, test scrolling
- [ ] Render `CostChart` with 24 hours of mock data
- [ ] Verify no panics or rendering errors
- [ ] Verify output buffer contains expected text fragments (extension names, cost values, etc.)

**Test:** `cargo test -p roko-cli --test tui_widgets`

---

## Phase 5: Agent Studio (web)

Goal: implement the web-based Agent Studio dashboard served by `roko serve` for deployment, monitoring, and cost analytics.

### Task 5.1: Agent management routes

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs` (existing agent routes)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/mod.rs` (route registry)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs`

**What to implement:**

REST routes for agent lifecycle management, consumed by the Agent Studio web frontend.

**Checklist:**
- [ ] `GET /api/agents` -- list all agents with status, profile, sidecar address (check existing, extend if needed)
- [ ] `POST /api/agents` -- start a new agent (body: profile, config, name)
- [ ] `DELETE /api/agents/:id` -- stop an agent
- [ ] `GET /api/agents/:id` -- detailed agent status (proxy to sidecar `/stats`)
- [ ] `GET /api/agents/:id/logs` -- stream agent logs (proxy to sidecar `/logs`)
- [ ] `GET /api/agents/:id/ticks` -- recent tick history (proxy to sidecar or read from episodes)
- [ ] `GET /api/agents/:id/cost` -- cost breakdown (T0/T1/T2, total, per-hour)
- [ ] All routes return JSON
- [ ] Authentication: require bearer token (from `roko.toml` or environment)
- [ ] Test: start agent via POST, list via GET, verify agent appears, stop via DELETE

**Test:** `cargo test -p roko-serve -- agent_routes`

---

### Task 5.2: Cost analytics routes

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/`
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/costs_log.rs`

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/analytics.rs`

**What to implement:**

Routes that power the cost analytics dashboard.

**Checklist:**
- [ ] `GET /api/analytics/cost` -- aggregate cost data (query params: agent_id, from, to, granularity)
- [ ] `GET /api/analytics/cost/breakdown` -- T0/T1/T2 breakdown per agent
- [ ] `GET /api/analytics/cost/projection` -- projected monthly cost based on trailing 7-day average
- [ ] `GET /api/analytics/model-usage` -- token usage per model (sonnet, haiku, opus)
- [ ] Read from `.roko/learn/efficiency.jsonl` and `CostsLog`
- [ ] Support time range queries with `from` and `to` query parameters (ISO 8601)
- [ ] Support granularity: `hourly`, `daily`, `weekly`
- [ ] Register routes in `mod.rs`
- [ ] Test: write 100 efficiency events, query aggregate, verify totals match

**Test:** `cargo test -p roko-serve -- analytics_routes`

---

### Task 5.3: Cognitive frequency routes

**Read first:** Task 5.1

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/agents.rs`

**What to implement:**

Routes for cognitive frequency monitoring and CorticalState data.

**Checklist:**
- [ ] `GET /api/agents/:id/frequency` -- current gamma/theta/delta tick state
- [ ] `GET /api/agents/:id/cortical` -- current CorticalState (affect, vitality, PE, arousal, confidence, sleep_pressure)
- [ ] `GET /api/agents/:id/cortical/history` -- CorticalState time series (query params: from, to, granularity)
- [ ] SSE endpoint: `GET /api/agents/:id/events` -- real-time event stream for agent state changes
- [ ] Proxy to agent sidecar where possible, augment with local episode data
- [ ] Test: query frequency for a mock agent, verify response structure

**Test:** `cargo test -p roko-serve -- frequency_routes`

---

### Task 5.4: Audit trail routes

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/`

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/audit.rs`

**What to implement:**

Retrieval-to-action audit trail -- every agent decision produces a searchable trace.

**Checklist:**
- [ ] `GET /api/audit/:agent_id` -- list audit entries (query params: from, to, action_type, outcome, limit)
- [ ] `GET /api/audit/:agent_id/:trace_id` -- detailed trace for a single decision
- [ ] Define `AuditEntry` JSON schema: `{ trace_id, timestamp, observation, tier_decision, context_assembly, reasoning_summary, action, gate_result, outcome, cost }`
- [ ] Read from `.roko/episodes.jsonl` for completed traces
- [ ] Support text search across reasoning_summary and action fields
- [ ] Support filtering by outcome (success, failure, skipped)
- [ ] Register routes in `mod.rs`
- [ ] Test: write 5 episode entries, query audit, verify all 5 returned with correct structure

**Test:** `cargo test -p roko-serve -- audit_routes`

---

## Phase 6: OpenClaw (end user)

Goal: implement the end-user hedging product -- wallet connection, position scanning, clearing profile recommendation, and one-click approval.

### Task 6.1: Wallet connection

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/wallet.rs` (ChainWallet trait)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/x402.rs` (payment authorization)

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/openclaw.rs`

**What to implement:**

OpenClaw API routes for wallet connection via WalletConnect or Privy.

**Checklist:**
- [ ] `POST /api/openclaw/connect` -- initiate wallet connection (body: connection_type: "walletconnect" | "privy")
- [ ] Return connection URI for WalletConnect QR code
- [ ] `POST /api/openclaw/verify` -- verify wallet ownership (body: signature, message)
- [ ] Store connected wallet state in session
- [ ] `GET /api/openclaw/session` -- return current session state (connected wallet, permissions)
- [ ] `DELETE /api/openclaw/session` -- disconnect wallet
- [ ] No KYC for observation mode -- wallet connection gives read-only access to on-chain positions
- [ ] Register routes in `mod.rs`
- [ ] Test: mock wallet connection, verify session created

**Test:** `cargo test -p roko-serve -- openclaw_connect`

---

### Task 6.2: Position scanning

**Read first:**
- PRD-08 section 2.3 (OpenClaw flow: Step 2, protocol reading table)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-chain/src/client.rs` (ChainClient for on-chain reads)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/openclaw.rs`

**What to implement:**

Scan wallet positions across supported protocols.

**Checklist:**
- [ ] `POST /api/openclaw/scan` -- scan connected wallet's positions (body: wallet_address, protocols: ["aave-v3", "compound-v3", "morpho"])
- [ ] Implement `AaveV3Scanner`: call `getUserAccountData(address)` and `getReserveData(address)` for each reserve
- [ ] Implement `CompoundV3Scanner`: call `getAccountSnapshot(address)` for each market
- [ ] Implement `MorphoScanner`: call `position(id, address)` for known markets
- [ ] Return `PositionMap`: `{ protocol, asset, type (supply/borrow), amount, current_rate, health_factor }`
- [ ] Compute aggregate exposure: total notional by protocol, effective rate, 30d rate volatility
- [ ] Test: mock chain client returns known positions, verify scan output matches

**Test:** `cargo test -p roko-serve -- openclaw_scan`

---

### Task 6.3: Clearing profile recommendation

**Read first:**
- IMPL-06 Phase 5 (clearing profiles)
- PRD-08 section 2.3 (Steps 3-4)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/openclaw.rs`

**What to implement:**

Based on scanned positions, recommend a hedge via a clearing profile.

**Checklist:**
- [ ] `POST /api/openclaw/recommend` -- generate hedge recommendation (body: position_map)
- [ ] Compute net variable exposure: sum of all variable-rate supply minus variable-rate borrow
- [ ] Compute recommended direction: SHORT if net supply > net borrow (hedge against rate drops)
- [ ] Compute recommended trigger: ISFR minus a buffer (e.g., current rate - 100bps)
- [ ] Compute recommended notional: match net variable exposure
- [ ] Return `HedgeRecommendation`: `{ direction, trigger_bps, max_notional, estimated_cost, reasoning_trace }`
- [ ] Include reasoning trace: why this direction, why this trigger, risk analysis
- [ ] Test: $50K Aave supply at 3.2% + $20K Compound borrow at 4.7% -> short recommendation on net $70K exposure

**Test:** `cargo test -p roko-serve -- openclaw_recommend`

---

### Task 6.4: One-click approval flow

**Read first:**
- IMPL-06 Task 5.3 (quick_hedge convenience constructor)
- IMPL-07 Task 3.3 (INTENT precompile)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/openclaw.rs`

**What to implement:**

Single transaction to create a clearing profile with delegation caveats.

**Checklist:**
- [ ] `POST /api/openclaw/approve` -- create clearing profile and set delegation caveats (body: recommendation_id, caveats)
- [ ] Accept user-configured caveats: max_position_size, approved_protocols, stop_loss, rebalance_frequency
- [ ] Build `ClearingProfile` using `quick_hedge()` from IMPL-06
- [ ] Build `AgentIntent` with caveats from IMPL-07
- [ ] Return unsigned transaction for the user to sign in their wallet
- [ ] `POST /api/openclaw/submit` -- submit the signed transaction to the chain
- [ ] `GET /api/openclaw/profile/:id` -- check clearing profile status (dormant, active, filled)
- [ ] Test: recommend -> approve -> verify transaction payload includes correct profile and caveats

**Test:** `cargo test -p roko-serve -- openclaw_approve`

---

### Task 6.5: OpenClaw end-to-end test

**Read first:** Tasks 6.1-6.4

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/tests/openclaw_e2e.rs`

**What to implement:**

Full flow: connect wallet -> scan positions -> recommend hedge -> approve -> submit.

**Checklist:**
- [ ] Mock wallet connection
- [ ] Mock chain client returning Aave and Compound positions
- [ ] Call scan, verify positions returned
- [ ] Call recommend, verify hedge recommendation
- [ ] Call approve with caveats, verify transaction payload
- [ ] Call submit with mock-signed transaction, verify profile created
- [ ] Call profile status, verify "dormant" (ISFR has not crossed trigger)
- [ ] Simulate ISFR crossing trigger, verify profile status changes to "active"

**Test:** `cargo test -p roko-serve --test openclaw_e2e`

---

## Phase 7: MCP distribution

Goal: expose agent capabilities as MCP servers for consumption by external tools (Cursor, Claude Code, VS Code).

### Task 7.1: MCP server from agent capabilities

**Read first:**
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs` (existing MCP server)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/main.rs` (MCP binary)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/lib.rs`

**What to implement:**

Expose a running agent's capabilities as MCP tools. When an external tool (Cursor, Claude Code) connects to the MCP server, it can invoke agent capabilities as tool calls.

**Checklist:**
- [ ] Define MCP tool manifest from agent capabilities: each capability becomes an MCP tool
- [ ] Implement tool listing: `tools/list` returns available tools based on the connected agent's loaded extensions
- [ ] Implement tool calling: `tools/call` routes to the agent's extension chain
- [ ] Map common capabilities to standard MCP tool names:
  - `code_search` -> code intelligence queries
  - `knowledge_query` -> InsightStore/NeuroStore search
  - `research` -> agent research capability
  - `predict` -> ISFR prediction submission
- [ ] Implement resource listing: expose agent state as MCP resources (tick history, cost data, cortical state)
- [ ] Handle authentication: MCP clients authenticate with the same bearer token as the sidecar
- [ ] Test: list tools, verify capabilities appear as MCP tools
- [ ] Test: call a tool, verify it routes to the agent and returns a result

**Test:** `cargo test -p roko-mcp-code -- mcp_agent_tools`

---

### Task 7.2: Auto-discovery for MCP-compatible tools

**Read first:** Task 7.1

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/src/discovery.rs`

**What to implement:**

Auto-discovery so that MCP-compatible tools (Cursor, Claude Code) can find running Roko agents.

**Checklist:**
- [ ] Implement `.mcp.json` generation: when an agent starts with `--mcp-serve`, generate an MCP configuration file
- [ ] Write to `.mcp.json` in the project root (or a configured path)
- [ ] Include server URL, available tools, authentication token
- [ ] Implement `roko mcp advertise` CLI command: print the MCP server configuration in the format expected by Cursor/Claude Code
- [ ] Implement cleanup: remove `.mcp.json` entry when agent stops
- [ ] Support multiple agents: each agent advertises as a separate MCP server with a unique name
- [ ] Test: start agent with MCP, verify `.mcp.json` contains the server entry
- [ ] Test: stop agent, verify `.mcp.json` entry removed

**Test:** `cargo test -p roko-mcp-code -- mcp_discovery`

---

### Task 7.3: MCP integration test

**Read first:** Tasks 7.1, 7.2

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-mcp-code/tests/mcp_e2e.rs`

**What to implement:**

End-to-end: start MCP server, list tools, call a tool, verify result.

**Checklist:**
- [ ] Start MCP server backed by a mock agent
- [ ] Call `tools/list`, verify at least 3 tools returned
- [ ] Call `tools/call` on `code_search` with a query, verify response
- [ ] Call `tools/call` on `knowledge_query` with a query, verify response
- [ ] Verify `.mcp.json` was generated
- [ ] Stop server, verify `.mcp.json` cleaned up

**Test:** `cargo test -p roko-mcp-code --test mcp_e2e`

---

## Phase 8: CLI DX improvements (25 tasks)

**Goal**: Polish the developer experience across shell integration, error reporting, progress indication, and output formatting. These are split into three sub-phases by effort.

### Phase 8A: Quick wins

#### Task 8A.1: Implement shell-init eval

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/shell_init.rs` (new file)

**Read first:**
- `crates/roko-cli/src/main.rs` -- existing `Subcommand` enum
- How `rustup`, `mise`, and `starship` implement `eval "$(tool init zsh)"`

**What to do:**

1. Add `ShellInit` subcommand:
   ```rust
   /// Print shell initialization code
   ShellInit {
       /// Shell type (zsh, bash, fish)
       shell: String,
   },
   ```
2. Generate shell code that:
   - Sets up completions for the detected shell
   - Adds `roko` to PATH if installed via cargo
   - Sets up any shell hooks (e.g., directory change triggers `roko status`)
3. Output valid shell code for `eval "$(roko shell-init zsh)"`.

**Test:** `cargo run -p roko-cli -- shell-init zsh` outputs valid zsh code (no syntax errors).

- [ ] `roko shell-init zsh` outputs valid zsh code
- [ ] `roko shell-init bash` outputs valid bash code
- [ ] Shell completions set up correctly

---

#### Task 8A.2: Implement NO_COLOR compliance

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**Read first:**
- https://no-color.org/ specification
- Current color usage in CLI output

**What to do:**

1. Check for `NO_COLOR` environment variable at startup.
2. When set (any value), disable all ANSI color codes in output.
3. Wire through all output paths: tracing subscriber, table formatting, progress bars.

**Test:** `NO_COLOR=1 cargo run -p roko-cli -- status` produces output with zero ANSI escape sequences.

- [ ] `NO_COLOR` environment variable respected
- [ ] All output paths disable colors when set

---

#### Task 8A.3: Implement command timing

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**Read first:**
- Existing command dispatch in `main.rs`

**What to do:**

1. Wrap every subcommand execution with a timer.
2. On completion, print elapsed time: `Done in 1.23s`.
3. For agent dispatches, also print estimated cost: `Done in 45.2s ($0.12)`.
4. Controllable via `--timing` flag or `ROKO_TIMING=1` env var.

**Test:** `cargo run -p roko-cli -- status --timing` prints elapsed time at the end.

- [ ] Elapsed time printed after every command
- [ ] Cost estimate printed for agent dispatches
- [ ] Controllable via flag or env var

---

#### Task 8A.4: Implement enhanced --version

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**Read first:**
- Existing `--version` output
- `build.rs` patterns for embedding build info

**What to do:**

1. Create `crates/roko-cli/build.rs` (or extend existing) to capture:
   - Git commit hash (short)
   - Git branch
   - rustc version
   - Build target triple
   - Build timestamp
2. Output format:
   ```
   roko 0.1.0 (abc1234 2026-04-21)
   rustc 1.91.0 (stable-aarch64-apple-darwin)
   target: aarch64-apple-darwin
   ```

**Test:** `cargo run -p roko-cli -- --version` includes git hash and rustc version.

- [ ] Git hash, branch, rustc version, target in --version output
- [ ] Build timestamp included

---

#### Task 8A.5: Implement PowerShell + Nushell completions

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` or `build.rs`

**Read first:**
- clap's `generate` feature for shell completions
- Existing zsh/bash completion generation (if any)

**What to do:**

1. Add a `Completions` subcommand:
   ```rust
   /// Generate shell completions
   Completions {
       /// Shell type (zsh, bash, fish, powershell, nushell, elvish)
       shell: clap_complete::Shell,
   },
   ```
2. Use `clap_complete` to generate completions for all 6 shell types.
3. For PowerShell and Nushell, verify the generated completions load without errors.

**Test:** `cargo run -p roko-cli -- completions powershell` outputs valid PowerShell completion script.

- [ ] Completions for 6 shells: zsh, bash, fish, powershell, nushell, elvish
- [ ] Generated completions are syntactically valid

---

### Phase 8B: Medium effort

#### Task 8B.1: Implement interactive fuzzy fallbacks (dialoguer)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**Read first:**
- `dialoguer` crate documentation
- Current error handling for missing/ambiguous arguments

**What to do:**

1. Add `dialoguer` dependency to `roko-cli`.
2. When a required argument is missing and stdin is a TTY, prompt interactively:
   - `roko plan run` (no path) -> fuzzy-select from discovered plan directories
   - `roko prd plan` (no slug) -> fuzzy-select from existing PRD slugs
   - `roko agent stop` (no id) -> fuzzy-select from running agents
3. When stdin is not a TTY (piped), fall back to the existing error message.

**Test:** Verify fuzzy selection compiles. Non-TTY mode still produces error messages (no hang).

- [ ] Interactive fuzzy selection for missing arguments (TTY only)
- [ ] Non-TTY falls back to error messages
- [ ] `dialoguer` dependency added

---

#### Task 8B.2: Implement progress indicators (indicatif)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/` (multiple files)

**Read first:**
- `indicatif` crate documentation
- Current long-running operations in CLI

**What to do:**

1. Add `indicatif` dependency.
2. Wrap long-running operations with progress bars:
   - `roko plan run`: progress bar showing task N/total, current task name
   - `roko prd plan`: spinner with "Generating plan..."
   - `roko research topic`: spinner with "Researching..."
   - `roko install`: progress bar for download
3. Progress bars respect `NO_COLOR` and non-TTY environments.

- [ ] Progress bars for plan execution, PRD generation, research, install
- [ ] Respects NO_COLOR and non-TTY

---

#### Task 8B.3: Implement grouped help

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**Read first:**
- clap's `help_heading` attribute
- Current `--help` output

**What to do:**

1. Group subcommands by category in `--help` output:
   ```
   Agent commands:
     run          Single prompt execution
     plan         Plan management (list, show, create, run)
     prd          PRD lifecycle (idea, list, draft, plan, status)
     research     Research operations (topic, enhance-prd, enhance-plan)

   Monitoring:
     status       Show system status
     dashboard    Interactive TUI
     serve        HTTP control plane

   Configuration:
     init         Initialize .roko/ directory
     config       Configuration management

   Package management:
     install      Install a package
     remove       Remove a package
     search       Search the registry
   ```
2. Use clap's `help_heading` derive attribute on each subcommand variant.

- [ ] Subcommands grouped by category in --help output
- [ ] Categories: Agent, Monitoring, Configuration, Package management

---

#### Task 8B.4: Implement contextual error suggestions

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs`

**Read first:**
- Current error output format
- `Did you mean?` patterns in other CLIs

**What to do:**

1. When a subcommand is not recognized, suggest the closest match:
   ```
   error: unrecognized subcommand 'stauts'
   tip: did you mean 'status'?
   ```
2. When a required file is missing, suggest how to create it:
   ```
   error: .roko/ directory not found
   tip: run 'roko init' to create it
   ```
3. When a dependency is missing, suggest how to install it:
   ```
   error: rustc version 1.85 is below minimum 1.91
   tip: run 'rustup update stable' to update
   ```

- [ ] Fuzzy-matched command suggestions
- [ ] Missing file suggestions
- [ ] Dependency version suggestions

---

#### Task 8B.5: Implement `roko doctor`

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/doctor.rs` (new file)

**Read first:**
- `brew doctor`, `flutter doctor` patterns
- Current system requirements

**What to do:**

1. Check and report:
   - rustc version (>= 1.91)
   - cargo installed
   - `.roko/` directory exists
   - `roko.toml` is valid TOML
   - Required API keys configured (ANTHROPIC_API_KEY, etc.)
   - Disk space for `.roko/` directory
   - Network connectivity to API endpoints
   - Git version
2. Print pass/fail for each check with actionable fix instructions.
3. Exit code: 0 if all pass, 1 if any fail.

- [ ] 8+ environment checks
- [ ] Pass/fail with actionable fix instructions
- [ ] Exit code reflects overall status

---

#### Task 8B.6: Implement dry-run mode

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/main.rs` and dispatch paths

**Read first:**
- Existing plan execution path
- `--dry-run` patterns in other tools

**What to do:**

1. Add `--dry-run` global flag.
2. When set: print what would happen without executing:
   ```
   [dry-run] Would dispatch task "Wire somatic markers" to agent claude-opus-4-6
   [dry-run] Would run gate pipeline: compile, test, clippy, diff
   [dry-run] Would persist episode to .roko/episodes.jsonl
   ```
3. Wire through plan execution, agent dispatch, and gate pipeline.

- [ ] `--dry-run` flag available on all commands
- [ ] Prints planned actions without executing
- [ ] Covers plan execution, dispatch, and gates

---

### Phase 8C: Polish

#### Task 8C.1: Implement rich error diagnostics (miette)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/Cargo.toml` and error paths

**Read first:**
- `miette` crate documentation
- Current error display format

**What to do:**

1. Add `miette` dependency.
2. Wrap key error types with miette annotations:
   - Source spans for TOML parse errors
   - Related errors for multi-step failures
   - Help text for common errors
3. Example output:
   ```
   Error: invalid roko.toml
     --> roko.toml:15:3
      |
   15 |   model = "nonexistent-model"
      |   ^^^^^ unknown model identifier
      |
   help: valid models: claude-opus-4-6, claude-sonnet-4, claude-haiku-4-5
   ```

- [ ] `miette` for rich error diagnostics
- [ ] Source spans for config parse errors
- [ ] Help text for common mistakes

---

#### Task 8C.2: Implement man pages (clap_mangen)

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/build.rs`

**Read first:**
- `clap_mangen` crate documentation

**What to do:**

1. Add `clap_mangen` to build dependencies.
2. Generate man pages during build for all subcommands.
3. Install to `target/man/` directory.
4. Add `roko man` subcommand that opens the man page for a given command.

- [ ] Man pages generated for all subcommands
- [ ] `roko man <command>` opens the relevant page

---

#### Task 8C.3: Implement OSC 8 hyperlinks

**File to modify:** CLI output paths

**Read first:**
- OSC 8 terminal hyperlink specification
- Terminal support detection

**What to do:**

1. When the terminal supports OSC 8 (check `TERM_PROGRAM` for known terminals):
   - File paths become clickable links to the file
   - URLs become clickable links
   - Error locations link to the file:line
2. Fall back to plain text when unsupported.

- [ ] Clickable file paths in supported terminals
- [ ] URL detection and linking
- [ ] Graceful fallback for unsupported terminals

---

#### Task 8C.4: Implement custom aliases

**File to modify:** `/Users/will/dev/nunchi/roko/roko/crates/roko-core/src/config/schema.rs` and CLI dispatch

**Read first:**
- `roko.toml` config schema
- Git alias pattern

**What to do:**

1. Add `[aliases]` section to `roko.toml`:
   ```toml
   [aliases]
   r = "run"
   pr = "plan run"
   d = "dashboard"
   ```
2. Before clap parsing, expand aliases from config.
3. Support shell command aliases: `deploy = "!cargo build --release && roko plan run"`.

- [ ] `[aliases]` section in roko.toml
- [ ] Alias expansion before clap parsing
- [ ] Shell command aliases with `!` prefix

---

#### Task 8C.5: Implement JSON output parity (--json flag)

**File to modify:** All subcommand output paths

**Read first:**
- `jq`-friendly JSON output patterns
- Current human-readable output

**What to do:**

1. Add `--json` global flag.
2. When set, every command outputs structured JSON instead of human-readable text.
3. JSON schema: `{ "command": "status", "success": true, "data": { ... }, "timing_ms": 123 }`.
4. Cover at minimum: `status`, `plan list`, `plan show`, `prd list`, `prd status`, `agent list`, `config show`, `ls`.

- [ ] `--json` flag on all commands
- [ ] Structured JSON output with consistent schema
- [ ] Covers 8+ subcommands

---

## Phase 9: Package system CLI tasks

### Task 9.1: Implement `roko install` command (4 source types)

**Read first:**
- IMPL-09 Phase 2 Tasks 2.1-2.3

This task is defined in IMPL-09 Phase 2 (Tasks 2.1-2.6). Wire the CLI entry points here.

- [ ] `roko install crate:<name>` downloads and installs Rust extension
- [ ] `roko install npm:<name>` downloads Pi-compatible JS extension
- [ ] `roko install git:<url>` clones and auto-detects type
- [ ] `roko install <path>` symlinks local extension

---

### Task 9.2: Implement `roko remove` command

- [ ] `roko remove <name>` removes extension and updates lockfile

---

### Task 9.3: Implement `roko search` command

- [ ] `roko search <query>` queries registry and prints results table

---

### Task 9.4: Implement `roko publish` command

- [ ] `roko publish .` packages and publishes to registry

---

### Task 9.5: Implement `roko market` TUI browser

**File to create:** `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/market.rs` (new file)

**Read first:**
- `crates/roko-cli/src/tui/` -- existing TUI infrastructure
- `crates/roko-ext-registry/src/registry.rs` -- `RegistryClient`

**What to do:**

1. Create a TUI view that browses the package registry:
   - Search bar at the top
   - Results list with name, version, description, downloads
   - Detail panel showing full manifest on selection
   - Install action with Enter key
2. Wire into the TUI as a new tab (F8 or similar).

- [ ] TUI package browser with search, list, detail, install
- [ ] Integrated as a TUI tab

---

## Acceptance criteria

- [ ] `roko agent start --profile coding` starts an agent and prints sidecar URL (Task 1.1)
- [ ] `roko agent list` shows running/stopped agents with correct status (Task 1.2)
- [ ] `roko agent stop <id>` gracefully stops an agent (Task 1.3)
- [ ] `roko agent status <id>` shows live metrics from sidecar (Task 1.4)
- [ ] `roko chat --agent <id>` connects via WebSocket, sends/receives messages (Task 1.5)
- [ ] `roko bench arena --name <arena>` runs evaluation batches (Task 1.6)
- [ ] `roko doctor` validates environment and prints clear pass/fail (Task 1.7)
- [ ] Shell init produces valid shell code for zsh and bash (Task 2.1)
- [ ] NO_COLOR is respected across all output (Task 2.2)
- [ ] Command timing shows elapsed time and cost for agent dispatches (Task 2.3)
- [ ] --version includes rustc, target, and git hash (Task 2.4)
- [ ] Persistent chat connects, sends commands, receives responses (Task 3.4)
- [ ] TUI frequency bars render gamma/theta/delta correctly (Task 4.1)
- [ ] TUI cortical heatmap shows colored bars per dimension (Task 4.2)
- [ ] TUI cost chart shows T0/T1/T2 breakdown (Task 4.4)
- [ ] Agent Studio API exposes agent lifecycle, cost, and audit routes (Tasks 5.1-5.4)
- [ ] OpenClaw flow works from wallet connect to clearing profile creation (Task 6.5)
- [ ] MCP server exposes agent capabilities to external tools (Task 7.3)
- [ ] MCP auto-discovery generates and cleans up `.mcp.json` (Task 7.2)
- [ ] All tests pass: `cargo test -p roko-cli && cargo test -p roko-serve && cargo test -p roko-agent-server && cargo test -p roko-mcp-code`
- [ ] Clippy clean: `cargo clippy --workspace --no-deps -- -D warnings`

---

## Dependencies

| This phase | Depends on | Reason |
|-----------|------------|--------|
| Phase 1 | None | CLI commands extend existing infrastructure |
| Phase 2 | None | DX improvements are independent |
| Phase 3 | Phase 1 (Task 1.1) | Chat connects to running agents started by Phase 1 |
| Phase 4 | Phase 3 | TUI widgets display data from chat/sidecar WebSocket |
| Phase 5 | Phase 1 | Agent Studio API wraps CLI agent management |
| Phase 6 | IMPL-06 Phase 5, IMPL-07 Phase 3 | OpenClaw creates clearing profiles and INTENT caveats |
| Phase 7 | Phase 1 (Task 1.1) | MCP wraps running agent capabilities |

Phases 1 and 2 can be developed in parallel. Phase 5 and Phase 7 can be developed in parallel once Phase 1 is complete.

---

## Build and test commands

```bash
# Build all surface crates
cargo build -p roko-cli -p roko-serve -p roko-agent-server -p roko-mcp-code

# CLI tests
cargo test -p roko-cli -- agent
cargo test -p roko-cli -- doctor
cargo test -p roko-cli -- bench
cargo test -p roko-cli -- shell_init
cargo test -p roko-cli -- color_mode
cargo test -p roko-cli -- completions

# Agent sidecar tests
cargo test -p roko-agent-server -- chat

# HTTP server tests
cargo test -p roko-serve -- agent_routes
cargo test -p roko-serve -- analytics_routes
cargo test -p roko-serve -- openclaw

# MCP tests
cargo test -p roko-mcp-code -- mcp

# TUI widget tests
cargo test -p roko-cli --test tui_widgets

# Integration tests
cargo test -p roko-cli --test cli_e2e
cargo test -p roko-agent-server --test chat_e2e
cargo test -p roko-serve --test openclaw_e2e
cargo test -p roko-mcp-code --test mcp_e2e

# Lint
cargo clippy --workspace --no-deps -- -D warnings

# Format
cargo +nightly fmt --all
```
