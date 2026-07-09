# IDE Integration: Running Roko Agents Inside Editors

> **Goal**: Make roko agents available inside IDEs (VS Code, Zed, JetBrains, Neovim, Emacs)
> so developers interact with plan execution, gate results, learning loops, and dashboards
> directly from their editor — without pre-preparing PRDs/plans/tasks via CLI.
>
> **Key insight**: Roko's differentiator is NOT the IDE shell — it's the plan execution,
> gate pipeline, learning loops, and self-improvement. The IDE integration just makes these
> visible and interactive. Don't rebuild an editor; expose roko through protocols.

---

## Table of Contents

1. [The Five Approaches (effort vs depth)](#1-the-five-approaches)
2. [Agent Client Protocol (ACP) — The Recommended Path](#2-agent-client-protocol-acp)
3. [VS Code Extension Architecture](#3-vs-code-extension-architecture)
4. [VS Code APIs for AI Features](#4-vs-code-apis-for-ai-features)
5. [How Existing Agents Integrate](#5-how-existing-agents-integrate)
6. [VS Code Fork Analysis (Why NOT to Fork)](#6-vs-code-fork-analysis)
7. [Alternative Editors (Theia, Zed, JetBrains, OpenSumi)](#7-alternative-editors)
8. [MCP as Integration Layer](#8-mcp-as-integration-layer)
9. [The Standard Tool Set for IDE Agents](#9-the-standard-tool-set)
10. [Implementation Plan](#10-implementation-plan)
11. [Roko-Specific Extensions](#11-roko-specific-extensions)
12. [Competitive Comparison](#12-competitive-comparison)
13. [Research Sources](#13-research-sources)

---

## 1. The Five Approaches

| # | Approach | Effort | Time to MVP | Maintenance | IDE Support |
|---|----------|--------|-------------|-------------|-------------|
| 1 | **MCP Server** | ~1 week | Days | Minimal | VS Code Copilot, Cursor, Continue, any MCP client |
| 2 | **ACP Agent** (`roko acp`) | ~2-3 weeks | Weeks | Low | Zed, JetBrains, Neovim, Emacs, future VS Code |
| 3 | **VS Code Extension** (chat) | ~1-2 months | Month | Medium | VS Code only |
| 4 | **VS Code Extension** (full) | ~3-6 months | Months | Medium-High | VS Code only |
| 5 | **VS Code Fork** | ~6-12+ months | Half year+ | **Extreme** | Fork only |

**Recommendation**: Start with **ACP** (approach 2) for multi-IDE support with minimal code, then build a **VS Code extension** (approach 3-4) for deeper integration. Never fork VS Code.

### Why ACP First

- **Rust SDK exists**: `agent-client-protocol` crate on crates.io
- **Multi-IDE**: One implementation works in Zed, JetBrains (all IDEs), Neovim, Emacs
- **Maps to roko**: Sessions map to plan runs, tool calls map to file edits, permissions map to gate approvals
- **Growing adoption**: Cursor joined the ACP registry in March 2026
- **Low maintenance**: Protocol is stable, agent is an independent process

### Why Not Fork VS Code

- Cursor (the success story) needs 300+ engineers and is still 4 versions behind with 80-94 CVEs in their Chromium
- Void Editor (small team fork) abandoned active development — maintenance was unsustainable
- Forks cannot access the VS Code Marketplace (only OpenVSX, which has security issues)
- The fork approach only makes sense at Cursor's scale ($2B ARR)

---

## 2. Agent Client Protocol (ACP)

### What ACP Is

ACP is a JSON-RPC 2.0 protocol over stdio that standardizes bidirectional communication between code editors (clients) and AI coding agents (servers). Created by Zed Industries and JetBrains in October 2025. Fully open (MIT license).

**Spec**: [agentclientprotocol.com](https://agentclientprotocol.com)
**GitHub**: [github.com/agentclientprotocol/agent-client-protocol](https://github.com/agentclientprotocol/agent-client-protocol)
**Rust SDK**: [crates.io/crates/agent-client-protocol](https://crates.io/crates/agent-client-protocol)

### Protocol Lifecycle

```
IDE (Client)                          roko (Agent)
  |--- initialize ------------------>|   version + capabilities
  |<-- initialize response ----------|   agent capabilities
  |--- session/new ----------------->|   cwd, MCP servers
  |<-- session ID -------------------|
  |--- session/prompt --------------->|   user text + images + resources
  |<-- session/update (notification) -|   streaming: thoughts, messages, tool calls
  |<-- session/update (notification) -|   gate results, plan progress
  |<-- session/update (notification) -|   ...
  |<-- PromptResponse ----------------|   stopReason: done/cancelled
  |                                   |
  |--- session/prompt --------------->|   follow-up
  |<-- ...                            |
```

### Agent Methods (IDE → roko)

| Method | Purpose | Roko Mapping |
|---|---|---|
| `initialize` | Exchange capabilities, protocol version | Return roko version + feature flags |
| `authenticate` | Verify credentials (optional) | Not needed for local agent |
| `session/new` | Create conversation session | Set working dir, load roko.toml, init LearningRuntime |
| `session/load` | Resume previous session | `--resume` from executor snapshot |
| `session/prompt` | Send user input | Dispatch to universal loop or plan executor |
| `session/cancel` | Interrupt current operation | Set shutdown flag, drain agents |
| `session/setMode` | Switch operating mode | Switch between `run` / `plan` / `research` modes |
| `session/setModel` | Change model | Override default_model in config |

### Client Methods (roko → IDE)

| Method | Purpose | Roko Mapping |
|---|---|---|
| `fs/readTextFile` | Read file from workspace | Supplement to roko's own Read tool |
| `fs/writeTextFile` | Write file to workspace | Supplement to roko's own Write tool |
| `session/requestPermission` | Ask user approval | Gate approval, destructive edit confirmation |
| `session/update` | Stream progress (notification) | Plan progress, gate results, agent output |
| `terminal/create` | Launch shell command | Compile, test, clippy gates |
| `terminal/output` | Send stdin / get stdout | Gate output streaming |
| `terminal/waitForExit` | Block until command completes | Gate completion |
| `terminal/kill` | Terminate process | Agent abort |

### Session Update Types (streamed via `session/update`)

| Update Kind | What It Contains |
|---|---|
| `AgentMessageChunk` | Streamed response content |
| `AgentThoughtChunk` | Reasoning/thinking trace |
| `ToolCallStart` | Tool invocation beginning (name, args) |
| `ToolCallProgress` | Intermediate results |
| `ToolCallResult` | Final tool output or error |
| `Plan` | Multi-step plan with step descriptions |

### Capability Negotiation

```json
// roko advertises:
{
  "sessions": { "new": true, "load": true, "list": true,
    "modes": ["run", "plan", "research"],
    "configOptions": ["model", "effort", "budget"]
  },
  "mcpCapabilities": { "http": false, "sse": false },
  "promptCapabilities": { "audio": false, "image": false, "embeddedContext": true }
}

// IDE advertises:
{
  "fileSystem": { "readTextFile": true, "writeTextFile": true },
  "terminal": { "create": true, "output": true, "waitForExit": true, "kill": true },
  "prompts": { "audio": false, "image": true, "embeddedContext": true }
}
```

### Extension Mechanism

Both sides support custom methods prefixed with `_`. Roko would use `_roko.dev/` prefix:

```json
// Custom notification:
{ "method": "session/update", "params": {
    "kind": "_roko.dev/gate/result",
    "data": { "gate": "compile", "passed": true, "duration_ms": 3200 }
}}
```

### Transport

- **Local**: JSON-RPC 2.0 over stdio (newline-delimited JSON). IDE spawns `roko acp` as subprocess.
- **Remote** (future): HTTP or WebSocket for cloud deployments.
- **Buffer limit**: 50MB default.

### Current Adopters

| Agent/IDE | Status |
|---|---|
| **Zed** | First implementation (August 2025) |
| **JetBrains** (all IDEs) | GA via ACP Registry (2025.3+) |
| **Cursor** | Joined ACP registry (March 2026) |
| **Google Gemini CLI** | Original motivator |
| **Kimi Code** (Moonshot) | `kimi acp` command |
| **Goose** (Block) | `goose acp` command |
| **Kiro** (AWS) | ACP support |
| **Neovim** | Via plugins |
| **Emacs** | Via packages |

### IDE Configuration Examples

**Zed** (`~/.config/zed/settings.json`):
```json
{
  "agent_servers": {
    "Roko": {
      "type": "custom",
      "command": "roko",
      "args": ["acp", "--workdir", "."]
    }
  }
}
```

**JetBrains** (`~/.jetbrains/acp.json`):
```json
{
  "agent_servers": {
    "Roko": {
      "command": "/path/to/roko",
      "args": ["acp"]
    }
  }
}
```

### How ACP Compares to Other Protocols

| Aspect | ACP | LSP | MCP | DAP |
|---|---|---|---|---|
| Purpose | Agent ↔ IDE | Language server ↔ IDE | Agent ↔ Tools/Data | Debugger ↔ IDE |
| Direction | Bidirectional | Mostly client→server | Client→server | Bidirectional |
| Sessions | Multi-turn conversations | Document state | Stateless tool calls | Debug sessions |
| Streaming | Yes (notifications) | Limited | No | Events |
| Permissions | Native (`requestPermission`) | No | No | No |
| Tool calls | Native (plan/tool lifecycle) | No | Native (tools/call) | No |
| Transport | stdio, HTTP (future) | stdio, TCP | stdio, HTTP, SSE | stdio, TCP |

---

## 3. VS Code Extension Architecture

For deeper VS Code integration beyond ACP, build a dedicated extension.

### Architecture

```
VS Code
├── Extension (TypeScript, ~5-20K LOC)
│   ├── Chat Participant (@roko)
│   │   ├── Handles @roko mentions in VS Code chat panel
│   │   ├── Slash commands: /plan, /run, /status, /dashboard
│   │   └── Streams responses via ChatResponseStream
│   │
│   ├── WebView Panel (dashboard)
│   │   ├── React/Svelte app rendering roko dashboard
│   │   ├── Provider health, model comparison, experiment results
│   │   ├── Plan DAG visualization
│   │   └── Gate results timeline
│   │
│   ├── Process Manager
│   │   ├── Spawns `roko acp` as child process
│   │   ├── Manages process lifecycle (start, kill, restart)
│   │   └── Handles graceful shutdown
│   │
│   ├── ACP Client
│   │   ├── JSON-RPC message serialization
│   │   ├── Session management
│   │   └── Notification routing to UI components
│   │
│   ├── Diff Provider
│   │   ├── TextDocumentContentProvider for proposed edits
│   │   ├── Shows before/after in diff editor
│   │   └── Approval flow (accept/reject per edit)
│   │
│   ├── Terminal Integration
│   │   ├── Pseudoterminal for roko output streaming
│   │   └── Gate execution in VS Code's terminal panel
│   │
│   └── Status Bar
│       ├── Plan execution progress
│       ├── Current model + provider
│       └── Budget remaining
│
└── roko binary (Rust, existing)
    └── `roko acp` mode
```

### Communication Flow

```
User types @roko in chat panel
  → Chat Participant handler receives request
  → Forwards to ACP client
  → ACP client sends session/prompt to roko subprocess
  → roko streams session/update notifications:
      AgentThoughtChunk → rendered in chat as thinking
      ToolCallStart     → rendered as "Editing file.rs..."
      ToolCallResult    → rendered as code block
      _roko.dev/gate/result → rendered as pass/fail badge
      _roko.dev/plan/status → rendered as task checklist
  → Final PromptResponse
  → Chat Participant renders final output
```

### File Edit Flow

1. Roko generates file edits via its tool loop
2. ACP notification: `ToolCallResult` with file path + content
3. Extension creates `TextDocumentContentProvider` with proposed content
4. Opens VS Code diff editor: `vscode.commands.executeCommand('vscode.diff', originalUri, proposedUri)`
5. User reviews inline diff
6. Accept → `workspace.applyEdit()` writes to disk
7. Reject → discard proposed content, notify roko via `session/prompt`

### WebView Dashboard

```typescript
const panel = vscode.window.createWebviewPanel(
  'rokoDashboard', 'Roko Dashboard', vscode.ViewColumn.Two,
  { enableScripts: true, retainContextWhenHidden: true }
);

// Feed data from roko's ACP stream
acpClient.on('_roko.dev/dashboard/snapshot', (data) => {
  panel.webview.postMessage({ type: 'dashboard-update', data });
});
```

The dashboard WebView can reuse roko's existing serve API data — the same JSON that powers `roko dashboard` text mode and `GET /api/learn/*` endpoints can render in a React/Svelte panel inside VS Code.

---

## 4. VS Code APIs for AI Features

### Language Model API (`vscode.lm`)

Direct access to LLMs from extensions:

```typescript
const [model] = await vscode.lm.selectChatModels({ vendor: 'copilot', family: 'gpt-4o' });
const messages = [vscode.LanguageModelChatMessage.User('Explain this code')];
const response = await model.sendRequest(messages, {}, token);
for await (const fragment of response.text) { /* stream */ }
```

**Relevance**: Roko could expose its CascadeRouter as a Language Model Chat Provider, making roko's multi-model routing available to other VS Code extensions.

### Language Model Chat Provider API (v1.104+)

Register roko as a model provider in VS Code:

```json
{
  "contributes": {
    "languageModelChatProviders": [{
      "vendor": "roko",
      "displayName": "Roko Agent"
    }]
  }
}
```

The provider implements `provideLanguageModelChatResponse()` — when another extension or Copilot selects "roko" as the model, requests route through roko's CascadeRouter to whichever backend model is optimal.

### Chat Participants API

Register `@roko` in the chat panel:

```json
{
  "contributes": {
    "chatParticipants": [{
      "id": "roko.agent",
      "name": "roko",
      "fullName": "Roko Agent",
      "description": "Self-developing coding agent with plan execution, gates, and learning",
      "isSticky": true
    }],
    "chatParticipants[0].commands": [
      { "name": "plan", "description": "Create and execute a plan" },
      { "name": "run", "description": "Execute a single prompt" },
      { "name": "status", "description": "Show plan/agent status" },
      { "name": "dashboard", "description": "Open the dashboard panel" },
      { "name": "research", "description": "Deep research with citations" }
    ]
  }
}
```

Handler:

```typescript
const handler: vscode.ChatRequestHandler = async (request, context, stream, token) => {
  stream.progress('Starting roko agent...');

  // Forward to roko ACP subprocess
  const session = await acpClient.sessionNew({ cwd: workspaceFolder });
  const response = await acpClient.sessionPrompt(session.id, request.prompt);

  // Stream updates to chat
  for await (const update of response.updates) {
    if (update.kind === 'AgentMessageChunk') {
      stream.markdown(update.content);
    } else if (update.kind === 'ToolCallResult') {
      stream.markdown(`\`\`\`\n${update.result}\n\`\`\``);
    } else if (update.kind === '_roko.dev/gate/result') {
      stream.markdown(update.passed ? '**Gate passed**' : '**Gate failed**');
    }
  }

  return { metadata: { sessionId: session.id } };
};
```

### Language Model Tools API

Register roko's tools so Copilot's agent mode can invoke them:

```typescript
vscode.lm.registerTool('roko_planRun', {
  displayName: 'Run Plan',
  description: 'Execute a roko plan with gate verification',
  inputSchema: { type: 'object', properties: { planDir: { type: 'string' } } },
  invoke: async (invocation, token) => {
    const result = await acpClient.sessionPrompt(sessionId, `/plan run ${invocation.input.planDir}`);
    return new vscode.LanguageModelToolResult([
      new vscode.LanguageModelTextPart(result.text)
    ]);
  }
});
```

### Inline Completions API

For future tab-completion from roko's learning data:

```typescript
vscode.languages.registerInlineCompletionItemProvider('*', {
  provideInlineCompletionItems(document, position) {
    // Query roko's skill library for relevant completions
    // based on current file context
  }
});
```

### WebView Panels

For the dashboard:

```typescript
const panel = vscode.window.createWebviewPanel(
  'rokoDashboard', 'Roko Dashboard', vscode.ViewColumn.Two,
  { enableScripts: true, retainContextWhenHidden: true,
    localResourceRoots: [vscode.Uri.joinPath(context.extensionUri, 'media')] }
);

// Two-way communication
panel.webview.postMessage({ type: 'update', data: dashboardData });
panel.webview.onDidReceiveMessage(message => {
  if (message.type === 'runTask') { /* ... */ }
});
```

The webview auto-inherits VS Code's theme via CSS variables (`var(--vscode-editor-foreground)`).

### Sidebar Views

Register a WebView in the Explorer sidebar:

```json
{
  "contributes": {
    "views": {
      "explorer": [{
        "type": "webview",
        "id": "roko.planView",
        "name": "Roko Plans"
      }]
    }
  }
}
```

### Terminal Integration

For streaming roko output:

```typescript
// Pseudoterminal (full I/O control)
const writeEmitter = new vscode.EventEmitter<string>();
const pty: vscode.Pseudoterminal = {
  onDidWrite: writeEmitter.event,
  open: () => writeEmitter.fire('Roko agent started\r\n'),
  close: () => rokoProcess.kill(),
  handleInput: (data) => rokoProcess.stdin.write(data)
};
const terminal = vscode.window.createTerminal({ name: 'Roko', pty });
```

Or spawn roko directly:

```typescript
import { spawn } from 'child_process';
const roko = spawn('roko', ['acp'], { cwd: workspaceFolder });
roko.stdout.on('data', (data) => writeEmitter.fire(data.toString()));
```

### Extension Host

Extensions run in a separate Node.js process (Extension Host). They have full access to `child_process`, `fs`, `net` — can spawn Rust binaries freely. If roko is installed globally or bundled with the extension, `spawn('roko', ['acp'])` just works.

---

## 5. How Existing Agents Integrate

### Claude Code (MCP bridge)

- VS Code extension (`anthropic.claude-code`) spawns Claude Code CLI
- Extension runs a local MCP server (named `ide`) over SSE/HTTP on localhost
- CLI connects to the MCP server for IDE operations
- Only 2 of ~12 tools exposed to LLM: `getDiagnostics` and `executeCode`
- Rest are internal RPC for UI (diffs, file ops, status)
- Extension auto-installs when `claude` is run from VS Code's terminal

### Cline / Roo Code (gRPC-over-postMessage)

Architecture:
```
Controller (long-lived)
  └── Task (per conversation)
       ├── ApiHandler → LLM provider
       ├── ToolExecutor → file, terminal, browser, MCP tools
       └── PromptRegistry → system prompts + tool definitions
```

Agent loop: `prompt → LLM → parse tool calls → ToolExecutor → Task.ask() (approval gate) → execute → loop`

Key patterns:
- **Human-in-the-loop**: Every destructive action requires `Task.ask()` approval (suspends until user clicks)
- **Diff-first edits**: Proposed changes shown in diff view, written to disk only after approval
- **gRPC-over-postMessage**: Typed RPC between webview and extension via `postMessage` bridge
- **Multi-platform**: Same core works as VS Code extension, standalone server, or CLI

### Continue.dev (three-layer messenger)

Architecture:
```
GUI (React Webview) ←→ VsCodeMessenger ←→ Core Module
```

Key patterns:
- **IDE abstraction**: `VsCodeIde` implements abstract `IDE` interface (readFile, writeFile, showDiff, runCommand)
- **Portable core**: Same TypeScript core works in VS Code, IntelliJ, and CLI
- **XML tool calling**: Converts tools to XML in system message (provider-agnostic, doesn't rely on native tool-calling APIs)
- **VerticalDiffManager**: Custom inline diff rendering with per-block accept/reject

### Kimi Code (ACP native)

- `kimi acp` starts ACP agent server over stdio
- Used in Zed, JetBrains, VS Code (via Cline bridge)
- Supports session load/resume
- Streams AgentMessageChunk, ToolCall, ToolCallUpdate, TurnEnd
- Permission flow via `session/requestPermission`

### Cursor (proprietary fork)

Core modifications to VS Code internals:
- **Shadow Workspace**: Hidden editor instances validate changes before applying
- **Priompt**: JSX-based prompt composition engine (React-like)
- **Fast Apply model**: ~1,000 tokens/sec diff application via speculative decoding
- **Repository indexing**: AST-aware chunking + hybrid search (embeddings + BM25)
- **Predictive cursor**: Teleports to next edit location after change

Cursor 3 (April 2026) introduced the **Agents Window** — a standalone workspace for running many agents in parallel across repos. The traditional editor is now a complement, not the default.

---

## 6. VS Code Fork Analysis

### Cursor

| Aspect | Detail |
|---|---|
| Revenue | $2B ARR (March 2026), doubling every ~2 months in 2025 |
| Team | ~300 engineers (was 12 in early 2025) |
| Funding | $2.3B Series D at $29.3B valuation |
| VS Code version | 1.99.3 (4 minor versions behind current 1.103.2) |
| Chromium CVEs | 80-94 known vulnerabilities in their Chromium |
| Marketplace | Cannot access VS Code Marketplace (uses OpenVSX) |

### Windsurf (Codeium)

- Same fork approach, same security issues
- Invested in training custom models (SWE-1 family, MoE architecture, 950 tokens/sec)
- Acquired by Google (CEO + key staff) and Cognition (remaining business/IP)

### Void Editor (Cautionary Tale)

- Open-source MIT-licensed VS Code fork
- Small team (YC-backed)
- **Abandoned active development** — maintenance burden unsustainable
- Custom code limited to `src/vs/workbench/contrib/void/` directory
- No middleman backend (direct provider connections)

### The Maintenance Burden

Monthly VS Code releases require rebasing. Each release touches hundreds of files across the editor core. Fork modifications conflict with upstream changes. Security patches for Chromium/Electron lag behind. The extension marketplace is inaccessible.

**OX Security finding** (November 2025): Both Cursor and Windsurf had weaponizable Chromium vulnerabilities. Cursor patched within a day; Windsurf did not respond. The vulnerability affected 1.8M developers.

**OpenVSX namespace squatting**: Forks use OpenVSX instead of the VS Code Marketplace. A vulnerability allowed malicious actors to squat on extension namespaces and serve compromised extensions that appear as "recommended."

### Conclusion

Forking VS Code is a viable business strategy ONLY at Cursor's scale ($2B revenue, 300 engineers, venture-backed). For any smaller team, it is a trap. The maintenance burden grows faster than feature development.

---

## 7. Alternative Editors

### Zed

- **Language**: Rust + GPUI (custom GPU-accelerated UI framework). No Electron.
- **AI integration**: Native agent mode, multi-provider, multiplayer (human + AI edit simultaneously)
- **ACP**: Created ACP. First-class agent support. `roko acp` would work immediately.
- **Performance**: Genuine advantage from no Electron overhead
- **Team**: ~11 employees, $44.5M funding, 50K+ active users
- **Open source**: Zed, Zeta (AI), and ACP all open source

### JetBrains

- **Plugin API**: Deepest of any IDE. Full PSI (AST) access, inspections, refactoring, terminal, debug.
- **ACP**: Co-created with Zed. GA in 2025.3+. All agents in the ACP registry work.
- **Licensing**: Community Edition is Apache-2.0. Ultimate is proprietary. Can build plugins, can't fork Ultimate.
- **AI Assistant**: Built-in. Supports BYOK (Bring Your Own Key) since December 2025.

### Eclipse Theia

- **What it is**: NOT a VS Code fork. Independent IDE framework (TypeScript) with modular architecture.
- **Key advantage**: Build ON TOP, not by modifying source. No rebasing. Upstream updates are dependency upgrades.
- **AI support**: Theia AI reached GA March 2025 with explicit APIs for AI-native tools.
- **VS Code extension compat**: Partial but improving
- **Deployment**: Desktop (Electron), web app, or hybrid
- **Use case**: Custom IDE with full control over AI integration without fork maintenance

### OpenSumi (Alibaba)

- Frontend/backend separation, three extension mechanisms
- MCP client built in
- Primarily Chinese ecosystem, documentation mostly Chinese
- Less suitable for international deployment

---

## 8. MCP as Integration Layer

### MCP in VS Code (GA since July 2025)

MCP servers configured in `.vscode/mcp.json`:

```json
{
  "servers": {
    "roko-tools": {
      "command": "roko",
      "args": ["serve", "--mcp"]
    },
    "roko-remote": {
      "type": "http",
      "url": "http://localhost:9090/mcp"
    }
  }
}
```

Tools from MCP servers appear in Copilot's agent mode. Users toggle tools via "Configure Tools" in chat.

### How Roko Could Expose MCP

Roko already has `roko-serve` with 15 route groups. Adding an MCP endpoint would expose roko's capabilities as tools:

| MCP Tool | What It Does | Maps To |
|---|---|---|
| `roko_run` | Execute a single prompt through roko's loop | `roko run` |
| `roko_plan_run` | Execute a plan with gate verification | `roko plan run` |
| `roko_plan_list` | List available plans | `roko plan list` |
| `roko_status` | Get signal/episode counts | `roko status` |
| `roko_research` | Deep research with citations | `roko research topic` |
| `roko_prd_idea` | Capture a work item | `roko prd idea` |
| `roko_dashboard` | Get dashboard data | `roko dashboard` |
| `roko_provider_health` | Get provider health status | New |
| `roko_model_route` | Explain routing decision | New |

This requires zero VS Code extension code. Just implement MCP's `tools/list` and `tools/call` in roko-serve.

### MCP vs ACP

MCP and ACP are **complementary, not competing**:

- **MCP**: Agent ↔ Tools/Data (roko discovers and uses tools from MCP servers)
- **ACP**: Agent ↔ IDE/User (IDE communicates with roko for UI, permissions, streaming)

When an IDE creates an ACP session, it passes MCP server configs to the agent. The agent uses MCP for tool access and ACP for IDE interaction.

```
IDE
 │
 ├── ACP ──→ roko (agent)
 │              │
 │              ├── MCP ──→ code search server
 │              ├── MCP ──→ documentation server
 │              └── MCP ──→ database server
 │
 └── MCP ──→ roko (as tool provider to Copilot)
```

---

## 9. The Standard Tool Set for IDE Agents

### The Convergence Finding

Multiple independent coding agents converged on the same 6 core tools:

| Tool | Claude Code | Cline | Continue | Codex | Purpose |
|---|---|---|---|---|---|
| **Read file** | `Read` | `read_file` | `read_file` | yes | Read file contents |
| **Write file** | `Write` | `write_to_file` | `create_new_file` | yes | Create/overwrite file |
| **Edit file** | `Edit` (old_str/new_str) | `replace_in_file` | `edit_existing_file` | `apply_patch` | Modify existing file |
| **Shell** | `Bash` | `execute_command` | `run_terminal_command` | yes | Run commands |
| **Search content** | `Grep` | `search_files` | `grep_search` | yes | Find text in files |
| **Search files** | `Glob` | `list_files` | `glob_search` | yes | Find files by pattern |

### Edit Format Convergence

Five agents independently converged on **string-replacement edit semantics** (old_str/new_str). This outperforms line-number-based editing because:
- Line numbers shift as edits accumulate
- String matching is robust to whitespace and formatting changes
- Multiple fallback strategies: exact → trimmed → whitespace-insensitive → fuzzy

### VS Code Agent Mode Built-in Tools

VS Code's native agent mode (for Copilot) exposes 22 tools:

**Edit**: createDirectory, createFile, editFiles, editNotebook
**Execute**: createAndRunTask, getTerminalOutput, runInTerminal, runNotebookCell, testFailure
**Read**: getNotebookSummary, problems, readFile, readNotebookCellOutput, terminalLastCommand
**Search**: changes, codebase, fileSearch, listDirectory, textSearch, usages
**VS Code**: askQuestions, extensions, installExtension, runCommand, VSCodeAPI
**Agent**: runSubagent
**Web**: fetch

### Practical Finding: Agents Use Local FS, Not IDE APIs

Most successful coding agents (Claude Code, Codex, Goose, Kimi) read/write files **directly from the filesystem** and use the IDE only for UI features (diff viewing, diagnostics display, approval prompts). This is because direct filesystem access is faster, more reliable, and works without the IDE running.

For roko: keep the existing Read/Write/Edit/Bash/Glob/Grep tools as the primary path. Use ACP's `fs/readTextFile` only for IDE-specific scenarios (reading unsaved buffers, accessing remote files via VS Code's virtual filesystem).

---

## 10. Implementation Plan

### Phase 1: ACP Agent (2-3 weeks)

```
New file: crates/roko-cli/src/acp.rs (~1-2K Rust LOC)
New subcommand: `roko acp`
Dep: agent-client-protocol crate

What it does:
  1. Listen on stdin for JSON-RPC messages
  2. Handle `initialize` → return roko capabilities
  3. Handle `session/new` → set up workdir, load config, init LearningRuntime
  4. Handle `session/prompt` → dispatch to roko's universal loop or plan executor
  5. Stream `session/update` → plan progress, gate results, agent output
  6. Handle `session/load` → resume from executor snapshot
  7. Handle `session/cancel` → graceful shutdown with checkpoint
  8. Use `terminal/create` for gate execution in IDE terminal
  9. Use `session/requestPermission` for destructive operations
  10. Define roko extensions (_roko.dev/*) for dashboard, gates, learning metrics

Task breakdown:
  - [ ] Add `acp` subcommand to roko-cli/src/main.rs
  - [ ] Implement ACP server initialization (capabilities, version)
  - [ ] Implement session/new → config loading, LearningRuntime init
  - [ ] Implement session/prompt → dispatch to run.rs or orchestrate.rs logic
  - [ ] Implement session/update streaming for agent output
  - [ ] Implement session/update streaming for gate results
  - [ ] Implement session/update streaming for plan progress
  - [ ] Implement session/load → resume from snapshot
  - [ ] Implement session/cancel → graceful shutdown
  - [ ] Implement session/requestPermission for edit approval
  - [ ] Define _roko.dev/ extension messages
  - [ ] Write integration test with mock IDE client
  - [ ] Test in Zed
  - [ ] Test in JetBrains
```

### Phase 2: MCP Server (1 week, can parallel with Phase 1)

```
New file: crates/roko-serve/src/mcp_server.rs (~500 LOC)
New flag: `roko serve --mcp`

What it does:
  1. Expose roko CLI commands as MCP tools
  2. Implement tools/list → return tool definitions
  3. Implement tools/call → execute roko commands
  4. Works with VS Code Copilot, Cursor, Continue without extension
```

### Phase 3: VS Code Extension (1-2 months)

```
New directory: extensions/vscode-roko/ (~5-10K TypeScript)

Structure:
  src/
    extension.ts          — activate(), deactivate(), process lifecycle
    chatParticipant.ts    — @roko chat handler with /plan, /run, /status, /dashboard
    acpClient.ts          — JSON-RPC client for communication with roko subprocess
    processManager.ts     — spawn, kill, restart roko acp process
    diffProvider.ts       — TextDocumentContentProvider for edit previews
    dashboardPanel.ts     — WebviewPanel for roko dashboard
    planTreeView.ts       — TreeDataProvider for plan DAG in sidebar
    statusBar.ts          — StatusBarItem showing plan progress + model
    gateDecorations.ts    — Editor decorations for gate results
  webview/
    dashboard/            — React/Svelte app for dashboard WebView
    plan-view/            — Plan DAG visualization component

package.json contributions:
  - chatParticipant: roko.agent with 5 slash commands
  - views: roko.planView in explorer sidebar
  - commands: roko.start, roko.stop, roko.dashboard, roko.approve, roko.reject
  - configuration: roko.binary (path to roko), roko.autoStart, roko.model
```

### Phase 4: Enhanced Integration (ongoing)

```
  - Language Model Chat Provider (expose CascadeRouter to VS Code's model picker)
  - Inline completions from skill library
  - Problems panel integration (gate errors as diagnostics)
  - Source Control integration (show roko-generated changes as staged edits)
  - CodeLens for plan tasks (show task status inline in code)
  - Notebook support (interactive roko sessions in .ipynb)
```

---

## 11. Roko-Specific ACP Extensions

Standard ACP covers basic agent operations. Roko needs custom extensions for its unique features:

### Plan Execution

```jsonc
// Notification: plan progress update
{
  "method": "session/update",
  "params": {
    "kind": "_roko.dev/plan/status",
    "data": {
      "plan_id": "05-glm-integration",
      "total_tasks": 14,
      "completed": 7,
      "in_progress": 2,
      "failed": 0,
      "pending": 5,
      "tasks": [
        { "id": "2D.01", "status": "completed", "model": "glm-5.1" },
        { "id": "2D.02", "status": "in_progress", "model": "glm-5.1", "iteration": 2 },
        // ...
      ]
    }
  }
}
```

### Gate Results

```jsonc
// Notification: gate pipeline result
{
  "method": "session/update",
  "params": {
    "kind": "_roko.dev/gate/result",
    "data": {
      "task_id": "2D.02",
      "pipeline": [
        { "gate": "compile", "passed": true, "duration_ms": 3200 },
        { "gate": "test", "passed": false, "duration_ms": 15400,
          "details": "3 tests failed: test_glm_thinking, test_glm_cache, test_glm_error" },
        { "gate": "clippy", "passed": true, "duration_ms": 8100 }
      ],
      "overall_passed": false,
      "iteration": 2,
      "conductor_action": "retry_with_error_digest"
    }
  }
}
```

### Learning Metrics

```jsonc
// Notification: routing decision explanation
{
  "method": "session/update",
  "params": {
    "kind": "_roko.dev/learn/routing",
    "data": {
      "selected_model": "glm-5.1",
      "provider": "zai",
      "stage": "ucb",
      "score": 0.847,
      "candidates": [
        { "model": "glm-5.1", "score": 0.847, "pass_rate": 0.82, "cost_norm": 0.04 },
        { "model": "kimi-k2.5", "score": 0.791, "pass_rate": 0.78, "cost_norm": 0.02 },
        { "model": "claude-sonnet", "score": 0.723, "pass_rate": 0.88, "cost_norm": 0.42 }
      ],
      "cache_affinity_bonus": 0.15,
      "total_observations": 423
    }
  }
}
```

### Dashboard Snapshot

```jsonc
// Notification: dashboard data for WebView rendering
{
  "method": "session/update",
  "params": {
    "kind": "_roko.dev/dashboard/snapshot",
    "data": {
      "provider_health": [
        { "id": "zai", "state": "healthy", "p50_ms": 1200, "error_rate": 0.01 },
        { "id": "moonshot", "state": "probing", "p50_ms": null, "error_rate": 0.15 }
      ],
      "model_comparison": [
        { "model": "glm-5.1", "pass_rate": 0.82, "avg_cost": 0.19, "observations": 203 },
        { "model": "kimi-k2.5", "pass_rate": 0.78, "avg_cost": 0.08, "observations": 145 }
      ],
      "experiments": [
        { "id": "glm-vs-kimi-impl", "status": "running", "variants": 2, "trials": 27 }
      ],
      "budget": { "used": 4.20, "limit": 10.00 },
      "router_stage": "ucb",
      "total_observations": 423
    }
  }
}
```

### Episode Event

```jsonc
// Notification: agent turn recording
{
  "method": "session/update",
  "params": {
    "kind": "_roko.dev/episode/event",
    "data": {
      "episode_id": "ep_abc123",
      "task_id": "2D.02",
      "model": "glm-5.1",
      "provider": "zai",
      "input_tokens": 12500,
      "output_tokens": 3400,
      "cost_usd": 0.032,
      "wall_ms": 8400,
      "gate_passed": false,
      "iteration": 2,
      "tool_calls": ["Read", "Edit", "Bash"]
    }
  }
}
```

---

## 12. Competitive Comparison

| Feature | Claude Code | Cline | Continue | Cursor | **Roko (proposed)** |
|---|---|---|---|---|---|
| **Protocol** | MCP (custom) | gRPC-over-postMessage | Custom messenger | Proprietary | **ACP (open standard)** |
| **IDE support** | VS Code | VS Code | VS Code, JetBrains | Cursor only | **Zed, JetBrains, VS Code, Neovim, Emacs** |
| **Multi-plan execution** | No | No | No | No | **Yes (DAG executor)** |
| **Gate pipeline** | No | No | No | No | **Yes (compile/test/clippy/diff)** |
| **Learning/routing** | No | No | No | Yes (model router) | **Yes (LinUCB bandit + Thompson)** |
| **Self-improvement** | No | No | No | No | **Yes (knowledge distillation)** |
| **Dashboard** | No | No | No | No | **Yes (13 pages)** |
| **Cost tracking** | Basic | Basic | No | Credit-based | **Yes (full CostTable + budget guardrails)** |
| **Provider health** | No | No | No | No | **Yes (circuit breaker + latency tracking)** |
| **A/B experiments** | No | No | No | No | **Yes (prompt + model experiments)** |
| **Open source** | No | Yes (Apache 2.0) | Yes (Apache 2.0) | No | **Yes** |
| **Model agnostic** | Anthropic only | Yes (15+ providers) | Yes (20+ providers) | Yes (limited) | **Yes (any OpenAI-compat + Claude CLI)** |

Roko's IDE integration story isn't about competing with Cursor on editor features. It's about **exposing capabilities that no other agent has** (plan execution, gates, learning, self-improvement) through a standard protocol that works everywhere.

---

## 13. Research Sources

### ACP Protocol
- [ACP Introduction](https://agentclientprotocol.com/get-started/introduction)
- [ACP GitHub](https://github.com/agentclientprotocol/agent-client-protocol)
- [ACP Protocol Schema](https://agentclientprotocol.com/protocol/schema)
- [ACP Rust SDK](https://docs.rs/agent-client-protocol-schema/latest/agent_client_protocol_schema/)
- [ACP Explained (CodeStandUp)](https://codestandup.com/posts/2025/agent-client-protocol-acp-explained/)
- [Goose ACP Blog](https://block.github.io/goose/blog/2025/10/24/intro-to-agent-client-protocol-acp/)
- [JetBrains ACP](https://www.jetbrains.com/acp/)
- [Zed ACP](https://zed.dev/acp)
- [Cursor + ACP (JetBrains Blog)](https://blog.jetbrains.com/ai/2026/03/cursor-joined-the-acp-registry-and-is-now-live-in-your-jetbrains-ide/)
- [ACP as LSP for Agents (PromptLayer)](https://blog.promptlayer.com/agent-client-protocol-the-lsp-for-ai-coding-agents/)

### VS Code Extension Development
- [AI Extensibility Overview](https://code.visualstudio.com/api/extension-guides/ai/ai-extensibility-overview)
- [Language Model API](https://code.visualstudio.com/api/extension-guides/ai/language-model)
- [Language Model Chat Provider API](https://code.visualstudio.com/api/extension-guides/ai/language-model-chat-provider)
- [Language Model Tool API](https://code.visualstudio.com/api/extension-guides/ai/tools)
- [Chat Participant API](https://code.visualstudio.com/api/extension-guides/ai/chat)
- [Chat Tutorial](https://code.visualstudio.com/api/extension-guides/ai/chat-tutorial)
- [Webview API Guide](https://code.visualstudio.com/api/extension-guides/webview)
- [Terminal API Reference](https://code.visualstudio.com/api/references/vscode-api)
- [Extension Host Architecture](https://code.visualstudio.com/api/advanced-topics/extension-host)
- [Review AI Code Edits](https://code.visualstudio.com/docs/copilot/chat/review-code-edits)
- [Agent Tools Reference](https://code.visualstudio.com/docs/copilot/agents/agent-tools)
- [Agent Mode Blog](https://code.visualstudio.com/blogs/2025/04/07/agentMode)

### MCP in IDEs
- [MCP GA in VS Code](https://github.blog/changelog/2025-07-14-model-context-protocol-mcp-support-in-vs-code-is-generally-available/)
- [VS Code MCP Server Configuration](https://code.visualstudio.com/docs/copilot/customization/mcp-servers)
- [GitHub Copilot MCP Docs](https://docs.github.com/copilot/customizing-copilot/using-model-context-protocol/extending-copilot-chat-with-mcp)

### Agent Architectures
- [Cline Architecture (DeepWiki)](https://deepwiki.com/cline/cline/1.3-architecture-overview)
- [Cline GitHub](https://github.com/cline/cline)
- [Roo Code GitHub](https://github.com/RooCodeInc/Roo-Code)
- [Continue Agent Mode](https://docs.continue.dev/ide-extensions/agent/how-it-works)
- [Continue Architecture (DeepWiki)](https://deepwiki.com/continuedev/continue/6-vs-code-extension)
- [Claude Code VS Code Docs](https://code.claude.com/docs/en/vs-code)
- [Claude Code Architecture (Medium)](https://medium.com/@yuxiaojian/under-the-hood-of-claude-code-its-not-magic-it-s-engineering-e1336c5669d4)
- [Claude Code Tool System](https://callsphere.tech/blog/claude-code-tool-system-explained)
- [Kimi Code IDE Docs](https://www.kimi.com/code/docs/en/kimi-cli/guides/ides.html)

### VS Code Forks
- [Cursor Deep Dive (MMNTM)](https://www.mmntm.net/articles/cursor-deep-dive)
- [Forked by Cursor (DEV)](https://dev.to/pullflow/forked-by-cursor-the-hidden-cost-of-vs-code-fragmentation-4p1)
- [VS Code Fork Wars (OpenReplay)](https://blog.openreplay.com/vs-code-fork-wars-cursor-windsurf-firebase-studio/)
- [OX Security: 94 Vulnerabilities in Cursor/Windsurf](https://www.ox.security/blog/94-vulnerabilities-in-cursor-and-windsurf-put-1-8m-developers-at-risk/)
- [Cursor Revenue/Valuation](https://aifundingtracker.com/cursor-revenue-valuation/)
- [Cursor 3 Blog](https://cursor.com/blog/cursor-3)
- [Void Editor (InfoQ)](https://www.infoq.com/news/2025/06/void-ide-beta-release/)

### Alternative Editors
- [Zed AI](https://zed.dev/ai)
- [Zed IDE Guide 2026](https://agmazon.com/blog/articles/technology/202603/zed-ide-complete-guide-en.html)
- [Theia vs VS Code (EclipseSource)](https://eclipsesource.com/blogs/2024/07/12/vs-code-vs-theia-ide/)
- [Why Not Fork VS Code (Eclipse Blog)](https://blogs.eclipse.org/post/thomas-froment/why-cursor-windsurf-and-co-fork-vs-code-shouldnt)
- [Theia in Production 2026](https://newsroom.eclipse.org/eclipse-newsletter/2026/march/eclipse-theia-eclipse-foundation-tool-platform-production)
- [JetBrains AI Assistant](https://www.jetbrains.com/help/ai-assistant/about-ai-assistant.html)
- [IntelliJ Platform 2025.3](https://blog.jetbrains.com/platform/2025/11/intellij-platform-2025-3-what-plugin-developers-should-know/)
- [OpenSumi](https://opensumi.com/en/docs/integrate/overview/)

### Protocols & Standards
- [LSP-AI GitHub](https://github.com/SilasMarvin/lsp-ai)
- [LSAP GitHub](https://github.com/lsp-client/LSAP)
- [AG-UI Protocol (CopilotKit)](https://docs.ag-ui.com/introduction)
- [DAP Specification](https://microsoft.github.io/debug-adapter-protocol/specification.html)
- [Coding Agent Loop Spec](https://github.com/strongdm/attractor/blob/main/coding-agent-loop-spec.md)
- [File Edit Format Analysis](https://fabianhertwig.com/blog/coding-assistants-file-edits/)

### Licensing
- [Code-OSS vs VS Code](https://github.com/microsoft/vscode/wiki/Differences-between-the-repository-and-Visual-Studio-Code)
- [VSCodium](https://vscodium.com/)
