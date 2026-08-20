# MCP & Tool System Comparison: Mori vs Roko

## 1. Architecture Overview

### Mori: Single MCP Server + CLI Passthrough

Mori runs a single custom MCP server binary (`mori-mcp`) that exposes code
intelligence tools over stdio JSON-RPC. Every agent subprocess gets the
server injected via CLI flags. The server is the sole bridge between
agents and the AST index.

```
Agent (claude/codex/cursor subprocess)
  |
  |-- --mcp-config / -c mcp_servers.mori.* flags
  |
  v
mori-mcp  (stdio child process of the agent CLI)
  |
  v
mori-index  (SQLite + in-memory graph + HDC + optional embeddings)
```

### Roko: Multi-Layer Tool Architecture

Roko separates concerns into three layers:

1. **Built-in tool registry** (`roko-std`) -- 16 local tools + 19 GitHub MCP stubs
2. **MCP code server** (`roko-mcp-code`) -- code intelligence over stdio
3. **MCP runtime bridge** (`roko-agent/mcp`) -- discovers, deduplicates, and
   retains MCP server connections for HTTP-provider tool loops

```
Agent dispatch  (runner-v2 / roko-agent provider)
  |
  +-- Static tool registry (roko-std): 16 builtins + 19 GitHub MCP
  |
  +-- MCP runtime bridge (roko-agent/mcp)
  |     |-- .mcp.json walk-up discovery
  |     |-- McpClient<StdioTransport> per server
  |     |-- DynamicToolRegistry = static + MCP merged
  |     |-- McpHandlerResolver routes calls to correct client
  |     |
  |     +-- roko-mcp-code server (one possible MCP server)
  |     +-- roko-mcp-github / slack / scripts (other MCP servers)
  |
  +-- Role/domain/override tool policy (roko-std/roles.rs)
  +-- AgentContract tool allowlists (runner event_loop.rs)
  +-- Safety layer: tool immune graph, capability checks
```

---

## 2. MCP Server Implementation

### Mori: `mori-mcp` (crate: `crates/mori-mcp/`)

**Files:**
- `main.rs` -- CLI with `context-server` and `index` subcommands
- `server.rs` -- stdio JSON-RPC loop, McpServer state, result caching
- `protocol.rs` -- hand-rolled JSON-RPC 2.0 + MCP types
- `tools.rs` -- 12 tools via MoriContextService
- `pricing.rs` -- x402 metering per tool call
- `learn.rs` -- namespace-scoped memory notes
- `init.rs` -- index initialization

**Protocol:** MCP 2024-11-05. Supports both Content-Length framed and
newline-delimited JSON-RPC transports (auto-detected per message).

**Advertised local tools (8 in tools/list, 4 hidden but callable):**

| Tool | Description | Cacheable |
|------|-------------|-----------|
| `search_code` | Symbol name search, kind/visibility filters | Yes |
| `get_symbol_context` | Symbol + related signatures (graph required) | Yes |
| `get_file_ast` | All indexed symbols from a file | Yes |
| `find_similar_patterns` | HDC fingerprint similarity search (graph) | Yes |
| `find_references` | Callers, importers, type users | Yes |
| `workspace_map` | Crate dependency graph + symbol counts | Yes |
| `get_callers` | Transitive call/dependency graph N hops | Yes |
| `get_plan_context` | On-demand plan context by type (11 types) | Yes |
| `get_index_stats` | File/symbol/ref counts (hidden) | Yes |
| `get_context` | Full context assembly (hidden) | Yes |
| `find_implementations` | Trait implementors (hidden) | Yes |
| `remember_context` / `recall_context` | Namespace-scoped notes | No / Yes |
| `get_mcp_savings` | Token savings metrics (special) | No |

**Remote tools (2, exposed when `MORI_HTTP_URL` is set):**

| Tool | Description |
|------|-------------|
| `mori_remote_queue_state` | Fetch running Mori session state |
| `mori_remote_steering_command` | Send force_advance / clean_retry |

**Key features:**
- **Result caching:** 5-minute TTL, hash-keyed by tool name + arguments,
  eviction at 500 entries every 50 calls
- **Token savings tracking:** Each tool has an estimated token equivalent
  (500-5000); cumulative savings reported via `get_mcp_savings`
- **x402 payment metering:** Per-tool USDC microcent pricing ($0.0001-$0.005),
  session-based billing, payment credential in `_payment` argument field
- **Namespace overlays:** Per-worktree and per-agent context isolation
  without mutating the base SQLite index
- **Privacy/redaction:** Per-request privacy controls with redaction policies
- **Background index refresh:** On `notifications/initialized`, index
  update spawns on a blocking task to avoid handshake timeout

### Roko: `roko-mcp-code` (crate: `crates/roko-mcp-code/`)

**Files:**
- `main.rs` -- binary entrypoint
- `lib.rs` -- everything: tools, dispatch, workspace loading (~1700 lines)

**Protocol:** MCP 2024-11-05. Uses `roko-mcp-stdio` for JSON-RPC framing
(shared crate, not inlined like Mori).

**Advertised tools (12):**

| Tool | Description |
|------|-------------|
| `search_code` | Multi-strategy search (keyword/structural/hdc/embedding/hybrid) |
| `get_symbol_context` | Symbol + dependencies + callers with expansion depth |
| `get_file_ast` | File symbols with optional bodies |
| `find_similar_patterns` | HDC fingerprint similarity |
| `find_references` | References with kind classification |
| `find_implementations` | Trait implementors with methods |
| `get_callers` | Transitive call graph with optional max_depth |
| `workspace_map` | Three depth levels: crate/module/symbol |
| `get_context` | Task-oriented context assembly with token budget |
| `get_index_stats` | Statistics |
| `symbol_lookup` | Quick name-based lookup |
| `call_graph` | Function call graph with depth |

**Key differences from Mori's MCP server:**
- No result caching
- No x402 payment metering
- No namespace overlays or privacy controls
- No remote HTTP tool proxying
- No memory/note system
- Multi-language support (Rust + TypeScript + Go providers)
- Token budget-aware context assembly (`get_context` with `token_budget`)
- Five search strategies vs Mori's two (keyword + HDC)
- Richer workspace map (three depth levels vs two)
- Schema uses `additionalProperties: false` (stricter)

---

## 3. AST Index System

### Mori: `mori-index` (crate: `crates/mori-index/`)

**Storage:** SQLite at `.mori/index.db` with optional rkyv mmap snapshot.

**Modules:**
- `db.rs` -- SQLite schema, migrations, CRUD
- `parser.rs` -- `RustParser` (tree-sitter based, Rust only)
- `graph.rs` -- `SymbolGraph`: forward/reverse edges, PageRank with file biasing
- `search.rs` -- keyword, structural, HDC similarity, RRF hybrid fusion
- `fingerprint.rs` -- HDC fingerprints for structural similarity
- `embedding.rs` -- optional dense embeddings (fastembed, batch of 64)
- `snapshot.rs` -- rkyv zero-copy mmap for fast reads
- `context_overlay.rs` -- namespace-scoped transient overlays
- `privacy.rs` -- redaction policies per retrieval surface
- `memo.rs` -- salsa-based incremental computation (feature-gated)
- `update.rs` -- incremental file scanning
- `merkle.rs` -- content-addressed tree

**Scale (production stats):** 6.1k files, 153.6k symbols, 92% routing coverage.

**Search strategies (RRF fusion):**
1. Keyword (SQL LIKE)
2. HDC similarity (1280-byte vectors, cosine)
3. Dense embedding (fastembed, feature-gated)
4. Hybrid = RRF merge of all active strategies (k=60)

**Graph features:**
- Forward edges (dependencies) + reverse edges (dependents)
- PageRank with file-biased personalization
- Lazy rebuild: only rebuilt when graph-dependent tools are first called

### Roko: `roko-index` (crate: `crates/roko-index/`)

**Storage:** In-memory (no SQLite). Parsed fresh from filesystem.

**Modules:**
- `parser.rs` -- language-agnostic via `LanguageProvider` trait
- `graph.rs` -- `SymbolGraph`: edges with `EdgeKind`, PageRank variants
- `hdc.rs` -- 10,240-bit HDC fingerprints (vs Mori's 1,280-byte)
- `symbol.rs` -- `SymbolId`, `SymbolRef`
- `workspace.rs` -- `WorkspaceIndex`, `CodeIndex`, context assembly
- `sqlite.rs` -- optional SQLite persistence (feature-gated)

**Language support:** Rust, TypeScript, Go (via `roko-lang-*` crates).

**Search strategies:**
1. Keyword (in-memory substring)
2. Structural (kind + visibility + file pattern + callers + PageRank)
3. HDC similarity
4. Embedding (type defined, implementation pending)
5. Hybrid (multi-strategy fusion)

**Graph features:**
- Typed edges (`EdgeKind`: calls, imports, implements, references, etc.)
- Three PageRank variants: standard, personalized, weighted
- `CallGraph` type with depth-limited BFS in both directions

**Key differences from Mori:**
- No persistent SQLite index by default (parses on startup)
- Multi-language (3 providers vs Mori's Rust-only tree-sitter)
- Richer edge typing (`EdgeKind` enum vs plain `(i64, i64)` pairs)
- Larger HDC fingerprints (10,240-bit vs 10,240-bit / 1,280-byte)
- No rkyv snapshot layer
- No namespace overlays (those live in Mori's `mori-index`)
- No Merkle tree
- Token-budget-aware context assembly in the workspace module

---

## 4. MCP Configuration & Server Lifecycle

### Mori: Per-Backend Config Generation

Mori writes MCP config files for **three** backends into every worktree
and the repo root:

**Config files generated per worktree:**
1. `.cursor/mcp.json` -- Cursor IDE format
2. `.mori/mcp-config.local.json` -- Claude CLI format (overrides repo-level)
3. `.codex/config.toml` -- Codex CLI format

**Config files generated at repo root:**
1. `.cursor/mcp.json`
2. `.mori/mcp-config.json`
3. `.codex/config.toml`

**Binary resolution** (`worktree.rs`):**
1. Check `target/release/mori-mcp` and `target/debug/mori-mcp`
2. Fall back to `cargo run -p mori-mcp --`
3. IDE configs prefer relative paths for portability

**Server injection per backend:**

| Backend | Mechanism |
|---------|-----------|
| Claude CLI | `--mcp-config .mori/mcp-config{.local}.json` + `--strict-mcp-config` |
| Codex CLI | `-c mcp_servers.mori.command=...` + `required=true` + timeouts |
| Cursor ACP | `.cursor/mcp.json` + `cursor-cli mcp enable mori` approval |

**Lifecycle:**
- `ensure_repo_mcp_configs()` -- writes all repo-root configs on startup
- `ensure_all_mcp_configs()` -- also refreshes all existing worktree configs
- Each worktree creation (plan / task / utility / detached) writes configs
- AutoFixer and Conductor roles skip MCP (latency + resource contention)
- `MORI_MCP_CONFIG` env var overrides walk-up discovery

**Timeouts:**
- Startup: 60-90 seconds
- Tool call: 60-120 seconds

### Roko: `.mcp.json` Walk-Up Discovery

Roko uses a simpler model:

**Discovery** (`roko-agent/src/mcp/config.rs`):
- Walk up from `start_dir` looking for `.mcp.json`
- Fall back to `$HOME/.mcp.json`
- `MORI_MCP_CONFIG` override not used; config path is structural

**Config format:**
```json
{
  "servers": [{
    "name": "filesystem",
    "transport": "stdio",
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem"],
    "env": {"HOME": "/tmp"},
    "tier": 2
  }]
}
```

**Security checks:**
- Command allowlist (26 known-safe commands)
- Sensitive env key detection (9 patterns)
- Unset env-var reference detection
- Hardcoded secret detection
- Trust tier per server (1-5, maps to `PluginTier`)

**Server injection per backend:**

| Backend | Mechanism |
|---------|-----------|
| Claude CLI | `--mcp-config` passthrough (subprocess owns lifecycle) |
| HTTP providers | MCP bridge discovers tools, retains clients, resolves calls |
| ACP (Cursor) | Context injection (not canonical runner receipt protocol) |

**Bridge runtime** (`roko-agent/src/mcp/bridge.rs`):
- `discover_mcp_runtime()` initializes all configured servers
- `McpRuntime` retains `Arc<McpClient<McpRuntimeTransport>>` per server
- `McpRuntimeResolver` implements `HandlerResolver` for tool dispatch
- Discovery timeout: configurable via `DEFAULT_MCP_DISCOVERY_TIMEOUT_SECS`

---

## 5. Tool System & Policy

### Mori: Role-Based CLI Flag Restrictions

Mori applies tool restrictions directly via Claude CLI `--tools` flags
when spawning agent subprocesses:

| Role | Tools |
|------|-------|
| Conductor | `Read,Glob,Grep,WebFetch,WebSearch` (read-only, plan mode) |
| Scribe | `Read,Glob,Grep,Write,Edit,WebFetch,WebSearch` (no bash) |
| QuickReviewer / Auditor / Critic | `Read,Glob,Grep,Bash,WebFetch,WebSearch` + JSON schema |
| Architect | `Read,Glob,Grep,Bash,WebFetch,WebSearch` + JSON schema |
| Researcher | `Read,Glob,Grep,Bash,WebFetch,WebSearch` |
| Implementer/AutoFixer (low/medium) | `Read,Glob,Grep,Edit,Write,Bash` (no web) |
| Implementer/AutoFixer (high/max) | `Read,Glob,Grep,Edit,Write,Bash,WebFetch,WebSearch` |

System prompt explicitly names MCP tools:
```
"Use MCP tools (search_code, get_symbol_context, find_references) for
symbol lookup. Use rg only for text grep."
```

### Roko: Three-Layer Tool Policy

Roko composes tool access from three independent layers:

**Layer 1: Role profiles** (`roko-std/roles.rs`):
5 role archetypes (Implementer, Researcher, Reviewer, Strategist, Scribe)
with allowed/denied tool lists.

**Layer 2: Domain profiles** (`roko-std/roles.rs`):
4 domain profiles (Coding, Chain, Research, General) with extra/excluded
tool lists.

**Layer 3: Config overrides** (`ToolOverrides`):
Per-task allow/deny from `tasks.toml` or CLI flags.

**Composition rule:**
```
effective = (role_allowed UNION domain_extra)
          \ (role_denied UNION domain_excluded UNION override_deny)
```
If `overrides.allow` is set, further restrict to only those tools.

**AgentContract enforcement** (runner `event_loop.rs`):
Role/task allowlists intersect, denials win, unknown roles deny all,
unsupported policy-bearing dispatches are rejected.

**Safety layer** (roko-agent/safety):
- Tool immune graph screens all host-visible tool results
- Five-head corrigibility ordering
- Provider isolation and tool cooldown/isolation
- Capability-based wrappers per Cell/Graph/Space
- Sandbox policy (5 levels)
- Missing or unknown safety contracts fail closed

**16 built-in tools:**
`read_file`, `write_file`, `edit_file`, `multi_edit`, `glob`, `grep`,
`bash`, `ls`, `web_fetch`, `web_search`, `notebook_edit`, `todo_write`,
`task` (agent), `exit_plan_mode`, `apply_patch`, `run_tests`

Plus 19 GitHub MCP tool catalog entries (typed placeholders).
Plus 17 chain-domain tools (feature-gated).

---

## 6. Tool Call Tracking & TUI

### Mori: F7 Inspect Tab with Live Tool Stats

**State tracked** (`state/mod.rs: McpRuntimeState`):
- Per-backend tool call counts (codex/claude/cursor)
- Per-backend MCP call counts
- Per-tool-name call counts (`HashMap<String, u64>`)
- Recent 8 tool calls with backend:role prefix
- Index stats (files, symbols, refs)
- Config presence per backend (repo + worktree level)
- Binary presence, root kind, refresh timestamp
- Last error

**TUI F7 Inspect tab** (`tui/views/context.rs`) renders three panels:

1. **MCP Runtime summary** -- status, backends enabled, selected plan,
   task route, live efficiency metrics, learning summary, fixture status
2. **Server panel** -- config paths, binary location, root kind
3. **Tool panel** -- per-tool call counts, recent calls
4. **Index panel** -- file/symbol/ref statistics
5. **Token burn sparklines** -- per-agent token consumption history

Also has a compact mode for dashboard embedding showing status + stats
in 3 lines.

### Roko: Telemetry Lens + Named Surfaces

Roko does not have a dedicated F7-equivalent MCP inspect tab. Instead:

- **E33 Telemetry Lens** tracks 39 production event variants including
  tool-related events, with bounded queued delivery, breaker controls,
  and restart-durable history
- **E37 Named Surfaces** provide typed projections (Workbench/Inbox/
  Canvas/Minimap/Autonomy) via StateHub-backed routes
- **Efficiency events** (`.roko/learn/efficiency.jsonl`) record per-turn
  metrics from runner dispatch
- **Episode logger** (`.roko/episodes.jsonl`) records agent turns + gate
  results

Tool call tracking happens at the provider level within `roko-agent`
rather than in a centralized TUI state struct.

---

## 7. Per-Worktree MCP Integration

### Mori: Automatic Config Propagation

Every worktree gets its own MCP configuration:

```
worktree/
  .cursor/mcp.json           -- Cursor picks this up
  .mori/mcp-config.local.json -- Claude CLI --mcp-config
  .codex/config.toml          -- Codex -c mcp_servers.mori.*
```

The MCP server's `--root` flag points to the worktree path, so the
index is scoped to the worktree's file tree. When the worktree diverges
from main, agents see only the worktree's version of the code.

Config refresh happens:
- On boot (`ensure_all_mcp_configs`)
- On each worktree creation (plan/task/utility/detached)
- Binary path resolution tries release then debug then cargo-run fallback

### Roko: No Automatic Worktree MCP Config

Roko does not auto-generate MCP configs in worktrees. The `.mcp.json`
walk-up discovery will find the repo-root config from any worktree
subdirectory, but the MCP server root is not automatically scoped to
the worktree.

For Claude CLI dispatch, `--mcp-config` is a passthrough flag -- the
CLI subprocess manages the MCP server lifecycle directly.

For HTTP providers, the MCP bridge discovers tools once during provider
construction and retains the client connections for the session.

---

## 8. Context Compression Before Prompt Spending

### Mori: Index as Prompt Compression

The MCP system is fundamentally a prompt compression mechanism. The
system prompt tells agents:

> "Use MCP tools (search_code, get_symbol_context, find_references)
> for symbol lookup. Use rg only for text grep."

Instead of reading entire files into the prompt, agents call structured
MCP tools that return targeted, scored results. Token savings per tool:

| Tool | Est. tokens saved |
|------|-------------------|
| `search_code` | 500 |
| `get_symbol_context` | 2,000 |
| `find_references` | 3,000 |
| `get_callers` | 5,000 |
| `workspace_map` | 5,000 |
| `get_context` | 4,000 |
| `get_plan_context` | 3,000 |

The `get_plan_context` tool is particularly significant: it replaces
injecting entire PRD documents, decomposition files, verify-tasks, and
review-tasks into the prompt. Instead, agents fetch specific context
types on demand.

### Roko: Token-Budget-Aware Context Assembly

Roko's `get_context` tool accepts a `token_budget` parameter and
assembles context within that budget:

```json
{
  "task": "implement the dispatch router",
  "token_budget": 8000,
  "include_tests": false
}
```

This is more explicit than Mori's approach -- the agent negotiates
how much context it can afford rather than the server deciding.

The runner-v2 enrichment system (`dispatch_agent_with` in event_loop.rs)
also injects targeted context: HDC fingerprints, playbook matches,
knowledge store lookups, and cascade router advice directly into the
system prompt rather than relying on MCP tool calls.

---

## 9. Key Architectural Differences Summary

| Dimension | Mori | Roko |
|-----------|------|------|
| MCP server binary | Single `mori-mcp` | `roko-mcp-code` + N plugin MCP servers |
| Index storage | SQLite + rkyv snapshot | In-memory (optional SQLite) |
| Languages indexed | Rust only | Rust + TypeScript + Go |
| JSON-RPC transport | Hand-rolled in server.rs | Shared `roko-mcp-stdio` crate |
| Result caching | 5-min TTL, in-server | None |
| Payment metering | x402 USDC microcents | None in MCP server |
| Namespace isolation | Worktree + agent overlays | Not in MCP server |
| Privacy/redaction | Per-request policies | Not in MCP server |
| Memory notes | In-server per-namespace | Separate neuro store |
| Config format | 3 backend-specific files | Single `.mcp.json` |
| Worktree config | Auto-generated per worktree | Walk-up from worktree |
| Tool policy | CLI --tools flags | 3-layer compose + safety graph |
| Built-in tools | 0 (all via host CLI) | 16 local + 19 GitHub MCP stubs |
| Tool tracking | McpRuntimeState in TUI | E33 telemetry + efficiency JSONL |
| TUI integration | F7 Inspect tab | Named surfaces (not specialized) |
| Remote tools | Queue state + steering | Via relay / serve routes |
| Search strategies | 2-3 (keyword + HDC + embed) | 5 (keyword + structural + HDC + embed + hybrid) |
| Graph edge types | Untyped (i64, i64) | Typed `EdgeKind` enum |
| Plan context | `get_plan_context` MCP tool | Runner enrichment injection |
| Token tracking | Cumulative MCP savings | Per-turn efficiency events |

---

## 10. Gaps and Product Residuals

### What Mori has that Roko lacks in MCP/tool:
1. MCP result caching (5-min TTL, hash-keyed)
2. Token savings tracking per tool call
3. x402 payment metering
4. Namespace-scoped overlays in the index server
5. Privacy/redaction policies on tool responses
6. In-server memory notes (remember/recall)
7. Dedicated TUI tab for MCP runtime introspection
8. Automatic worktree MCP config generation for 3 backends
9. Remote Mori HTTP proxy tools
10. `get_plan_context` for on-demand plan document retrieval

### What Roko has that Mori lacks:
1. Multi-language indexing (TypeScript, Go)
2. Three-layer composable tool policy (role x domain x override)
3. Safety layer with immune graph, capability checks, quarantine
4. DynamicToolRegistry merging static + MCP tools
5. Typed edge kinds in the symbol graph
6. Token-budget-aware context assembly
7. MCP server trust tiers (PluginTier 1-5)
8. MCP command allowlists and secret detection
9. HTTP transport support for remote MCP servers
10. AgentContract enforcement with fail-closed unknown roles
