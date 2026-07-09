# MCP Crates — code / github / slack / scripts / stdio
> Status-quo audit · re-verified 2026-07-08 @ HEAD 5852c93c · sources: 30+ files (5 crate sources, 5 Cargo.tomls, 7 roko-agent/mcp modules, orchestrate.rs, cli main.rs, roko-acp bridge_events/types, roko-core config/agent.rs, 5 v1/v2 design docs, 2 tmp intent docs, GAPS.md, worker/cloud.rs, serve/templates.rs, init.rs)

## What changed since 2026-07-07
Two consumer-side additions landed that the prior audit missed; every crate-level and wiring-defect claim below was re-checked line-by-line and still holds at this HEAD.
- **`AgentConfig.mcp_config` now EXISTS** — `crates/roko-core/src/config/agent.rs:93` (`pub mcp_config: Option<PathBuf>`). Resolves item B of `tmp/tmp-feedback/2/31-MCP-PASSTHROUGH-GAPS.md` (that doc is now stale on this point).
- **ACP gained a full session-scoped MCP tool-loop** — `crates/roko-acp/src/bridge_events.rs` `setup_session_mcp_tools` (bridge_events.rs:2635) spawns per-session servers from `session/new`'s `mcp_servers` (types.rs:277), discovers tools, emits `McpStatusUpdate` events, and dispatches via `run_openai_compat_mcp_tool_loop` (bridge_events.rs:2272). **This is a sixth, previously-undocumented MCP consumer.** It is gated to **OpenAI-compat providers only** (`openai_compat_tool_loop_supported`, bridge_events.rs:2189) and **rejects HTTP transport** (bridge_events.rs:2652-2664). The **Claude-CLI ACP path still does NOT thread session `mcp_servers` into `--mcp-config`** → passthrough-gaps item A remains valid *for the Claude-CLI backend*.
- Everything else in the passthrough-gaps + redesign docs is unimplemented (see Drift + §Cross-cutting).

## Summary

All five MCP crates are **implemented and compile as workspace members** (`Cargo.toml:39-43`). This is ahead of both CLAUDE.md ("Partial; see `tmp/ux-followup/05-partially-wired-subsystems.md`" — **that file no longer exists**, stale ref; only `tmp/archive/done-runners/ux-followup-runner/` logs remain) and the v1 design docs (`docs/v1/18-tools/10..13` all still say "Implementation: Scaffold / Planned" while github/slack/scripts are fully coded). The client side (`crates/roko-agent/src/mcp/`) is genuinely wired: plan runner spawns servers, discovers tools, merges them into a dynamic registry, and passes config through to Claude CLI. The weak spots are on the *seams*: 4 competing config-file conventions, two different writers of `.roko/mcp-config.json` with **incompatible JSON shapes**, per-server `env` dropped in the orchestrator spawn path, a `split("__")` vs `.`-separator grouping bug, and HTTP transport that exists only as a config enum. None of the five servers is registered by any default template — `roko init` only writes token *comments* (`commands/init.rs:41,43`).

Usability ranking end-to-end today:
1. **roko-mcp-code** ✅ — richest server (14 tools), tested, works with no credentials (index from cwd/`ROKO_WORKSPACE_ROOT`).
2. **roko-mcp-github** ✅ — 19 real GitHub tools, rate-limit retry, mock-HTTP tests; needs `GITHUB_TOKEN`; default `github_mcp_command` for the cloud worker (`worker/cloud.rs:166`).
3. **roko-mcp-scripts** ✅ — 2 tools but hardened (traversal guard, env allowlist, timeout); works out of the box on `.roko/scripts`.
4. **roko-mcp-slack** 🟡 — 9 tools, needs `SLACK_BOT_TOKEN`, only 2 trivial tests, inconsistent tool naming (`slack.post_message` vs `slack_reply`).
5. **roko-mcp-stdio** ✅(as lib) — not a server; the shared line-delimited JSON-RPC transport all four servers use.

`.roko/GAPS.md` contains **zero MCP entries** — the gaps below are untracked.

## Per-crate census (tools exposed, transport, launchable?, tests)

### roko-mcp-stdio (shared transport library)
- **Role**: line-delimited JSON-RPC 2.0 over stdin/stdout; `serve_stdio(reader, writer, handler)` loop + `JsonRpcRequest`/`JsonRpcError` types (`crates/roko-mcp-stdio/src/lib.rs:110-196`). Handles parse errors, notifications (no `id` → response discarded, lib.rs:171-173), spec error codes (lib.rs:40-46).
- **Not** the "scaffold with registry/handler traits" the design describes (`docs/v1/18-tools/13-mcp-stdio.md:27-50` promises `protocol.rs/server.rs/registry.rs/handler.rs`) — actual crate is a single 252-line file; each server hand-rolls its own `tools/list` JSON. Simpler than designed, works.
- **Launchable**: N/A (no binary; `Cargo.toml` has no `[[bin]]`, lib only). **Tests**: 2 (lib.rs:212, 237).

### roko-mcp-code (code intelligence)
- **Tools (10 advertised via `tools/list`)**: `search_code`, `get_symbol_context`, `get_file_ast`, `find_similar_patterns`, `get_index_stats`, `find_references`, `find_implementations`, `get_callers`, `workspace_map`, `get_context` (`crates/roko-mcp-code/src/lib.rs:224-376`). **+4 hidden legacy tools** dispatchable but not listed: `symbol_lookup`, `call_graph`, `imports`, `semantic_search` (lib.rs:400-403).
- **Backend**: `roko_index::WorkspaceIndex` + its own line-based parsers for rs/ts/js/go (lib.rs:1138-1378). Workspace root from `ROKO_WORKSPACE_ROOT` env or cwd (lib.rs:196-199). Path-escape guard: canonicalized paths must stay under root (lib.rs:1094-1110).
- **Transport**: stdio via roko-mcp-stdio; `initialize` reports `protocolVersion "2024-11-05"` (lib.rs:210-221).
- **Launchable**: yes — binary target `src/main.rs:3-5` → `cargo run -p roko-mcp-code`. **Tests**: 13 in-crate (lib.rs:1610-1929). README claims integration-test fixtures (README.md:95) — none exist (no `tests/` dir).
- **README drift 🕰️**: README lists tools `find_symbol/walk_dependencies/find_similar/list_files_touching/get_module_tree` (README.md:16-24) — none exist; claims a `--workspace` flag (README.md:33) — main.rs accepts no args; shows `.roko/mcp-servers.json` in Claude `mcpServers` shape (README.md:48-59) that roko's own `McpConfig` parser cannot read.

### roko-mcp-github
- **Tools (19)**: `github.list_prs/get_pr/create_pr/comment_pr/review_pr/merge_pr/list_issues/create_issue/comment_issue/close_issue/add_labels/create_label/get_file/search_code/list_commits/create_branch/get_branch/compare_branches/get_actions_status` (`crates/roko-mcp-github/src/main.rs:828-852`). Design said 17 (`docs/v2-depth/13-builtin-catalog/02-mcp-as-connect-protocol.md:192`).
- **Backend**: blocking `reqwest` against GitHub REST; `GITHUB_TOKEN` env required (main.rs:1243-1256); 429 retry with `Retry-After`/exponential backoff, plus proactive throttle when `x-ratelimit-remaining < 10` (main.rs:1263-1354); base64 file decode; issues list filters out PRs (main.rs:1458-1461).
- **Limitations**: ~13 handlers hardcode `https://api.github.com` (e.g. main.rs:1361-1364, 1470) — only the 6 with an `api_base_url` param are mock-testable/GHE-ready; single binary, no lib.
- **Launchable**: yes (default bin from `src/main.rs`; `Cargo.toml` no explicit `[[bin]]` needed). Referenced at runtime as the cloud worker's default `github_mcp_command` (`crates/roko-cli/src/worker/cloud.rs:166,522`). **Tests**: 22 (main.rs:2378-3196) incl. local-HTTP-server roundtrips for create_issue/create_pr/review/merge/search_code/list_issues (main.rs:2725-3196); `github_token()` stubbed under `cfg(test)` (main.rs:1258-1261).

### roko-mcp-slack
- **Tools (9)**: `slack.post_message`, `slack_reply`, `slack_get_thread`, `slack_react`, `slack.list_channels`, `slack_lookup_user`, `slack_dm`, `slack.get_channel_history`, `slack.update_message` (`crates/roko-mcp-slack/src/main.rs:383-398`). Design said 8. **Naming is inconsistent** — 4 tools use `slack.` prefix, 5 use `slack_` 🕰️ (violates the `server.tool` namespacing convention in `02-mcp-as-connect-protocol.md:124-128`).
- **Backend**: blocking `reqwest` on Slack Web API (`chat.postMessage`, `conversations.{open,list,history,replies}`, `reactions.add`, `users.{lookupByEmail,list}`, `chat.update`); `SLACK_BOT_TOKEN` required (main.rs:644-652); cursor pagination + ts-dedup for threads (main.rs:1001-1053).
- **Launchable**: yes — `[[bin]] name = "roko-mcp-slack"` (`Cargo.toml:13-15`). **Tests**: 2 only (tools/list shape, unknown-tool rejection; main.rs:1060-1113). No HTTP mocking (URLs hardcoded). Weakest test coverage of the five.

### roko-mcp-scripts
- **Tools (2)**: `run_script`, `list_scripts` (`crates/roko-mcp-scripts/src/main.rs:110-141`).
- **Behavior**: scans script roots for `*.sh|*.py|*.js` at startup, reads `# description:` header comments (main.rs:387-497); executes via `bash/python3/node` with timeout (default 60s, exit 124 on timeout), `env_clear()` + allowlist (PATH always kept) (main.rs:266-285), parent-dir traversal rejection + canonical-root containment (main.rs:327-368). Config: `ROKO_SCRIPTS_DIR`/`ROKO_MCP_SCRIPTS_DIR` (path-list), `--scripts-dir/--working-dir/--timeout-secs/--env-allowlist` flags, default `<cwd>/.roko/scripts` (main.rs:519-615).
- **Design divergence 🕰️**: design specifies `scripts.toml` declarative tool entries each becoming a named MCP tool with param schemas (`docs/v1/18-tools/12-mcp-scripts.md:41-49`, `02-mcp-as-connect-protocol.md:218-241`); implementation is directory-scan + generic `run_script(name,args)` — no per-script schemas. The `toml` and `glob` deps in `Cargo.toml:21,25` are **unused** in main.rs.
- **Launchable**: yes — `[[bin]]` (`Cargo.toml:13-15`). **Tests**: 8 incl. a tokio timeout test (main.rs:617-765).

## Wiring chain (config → dispatch → agent)

1. **Config sources (4 competing conventions 🕰️)**
   a. `roko.toml` `[agent] mcp_config = <path>` → `config.agent.mcp_config` (used at `orchestrate.rs:4139`).
   b. Walk-up `.mcp.json` (then `$HOME/.mcp.json`) in roko's own format `{"servers":[{name, transport, command, args, env, endpoint, auth_token, tier}]}` — `roko-agent/src/mcp/config.rs:84-104`; `tier: PluginTier` defaults to Sandboxed (config.rs:35-41).
   c. `roko config mcp list/test/add` manages `.roko/mcp.json`, resolving `.roko/mcp.json` → `~/.claude/mcp-config.json` → walk-up `.mcp.json` (`roko-cli/src/main.rs:3227-3341`).
   d. Legacy `McpLaunch` chain: `$ROKO_MCP_CONFIG` → `.roko/mcp-config.local.json` → `.roko/mcp-config.json` → `.codex/config.toml`, expecting Claude-style `{"mcpServers":{"roko":{...}}}` (`roko-agent/src/process/mcp.rs:139-179`); its fallback runs `cargo run -p roko-mcp` — **a crate that doesn't exist in this workspace** (process/mcp.rs:103-117) — dead path.
2. **Plan-runner spawn** — `PlanRunner::setup_mcp` (`orchestrate.rs:4125-4247`): explicit path or walk-up discovery → for each server `StdioTransport::spawn(&server.command, &server.args)` (orchestrate.rs:4172) → `initialize()` w/ 5s timeout → `list_tools()` → `mcp_to_tool_def(t, name)` → `dedup_tools` → `DynamicToolRegistry::with_preference(&base, config.tools.prefer_mcp)` (orchestrate.rs:4225; `prefer_mcp` config key at `roko-cli/src/config.rs:188`). Per-task server selection via `tasks.toml` `mcp_servers = [...]` (`task_parser.rs:77,1285`; filtered at orchestrate.rs:4168-4171, requested set built at orchestrate.rs:4459).
3. **Tool naming/routing** — tools become `{server}.{tool}` `ToolDef`s, `ToolCategory::Mcp`, write permission unless `readOnly` annotation, network iff `openWorld` (`to_tool_def.rs:22-65`). Dispatch resolves the prefix back to the live client via `McpHandlerResolver` (static resolver first, then `.`-split; `handler.rs:19-79`).
4. **Claude CLI passthrough** — `resolve_mcp_config_path` writes running-server list to `.roko/mcp-config.json` (orchestrate.rs:4249-4302), flows into invocation options (`mcp_config:` at orchestrate.rs:1777/1838/1938, used at 10370/16551) → `provider/claude_cli.rs:73-74` → `claude_cli_agent.rs:340-342`: `--mcp-config <path> --strict-mcp-config`; capability declared as `McpMode::ConfigFile("--mcp-config")` (`harness/capability.rs:562`). If no explicit/runtime config, `claude_cli_agent.rs:271-276` falls back to walk-up `.mcp.json` discovery.
5. **HTTP-backend bridge** — non-CLI backends can't take `--mcp-config`, so `openai_compat.rs:324-335` loads the config and calls `discover_mcp_tools` (`mcp/bridge.rs:38-96`), which uses `spawn_with_env` (env **honored** here, unlike step 2) and rejects `transport != stdio` with `UnsupportedTransport` (bridge.rs:42-47).
6. **ACP session-scoped MCP (new, sixth consumer)** — `session/new` accepts `mcp_servers: Vec<McpServerConfig>` (`roko-acp/src/types.rs:277`; transport enum `McpTransport::{Stdio,Http}` types.rs:359). On prompt dispatch, if the resolved provider is OpenAI-compat and `!mcp_servers.is_empty()`, `run_openai_compat_mcp_tool_loop` (bridge_events.rs:2272) calls `setup_session_mcp_tools` (bridge_events.rs:2635): per server `McpStdioTransport::spawn` → `initialize` (timeout) → `tools/list` → `mcp_to_tool_def` → `AcpMcpToolHandler` + `AcpMcpHandlerResolver` (bridge_events.rs:2829-2847). Per-server results surface as `McpServerStatus`/`McpInitStatus::{Ready,SpawnFailed,InitializeTimeout,ToolsListFailed,TransportUnsupported}` (types.rs:296-354) via `SessionUpdate::McpStatusUpdate` (bridge_events.rs:4240). HTTP transport is rejected with `TransportUnsupported` (bridge_events.rs:2652-2664) — same stdio-only limitation as the plan-runner/bridge paths. This path **does not** reuse `orchestrate::setup_mcp` (parallel implementation) and is **not** reached for Claude-CLI-backed ACP sessions.
7. **Other consumers** — roko-serve validates template MCP server names against discovered config (`roko-serve/src/routes/templates.rs:228-243`); dreams runner and orchestrator service factory carry `mcp_config: None`/`Option<PathBuf>` (`roko-dreams/src/runner.rs:160`, `roko-orchestrator/src/service_factory.rs:38,194-195`); roko-acp `runner.rs:463` still passes `mcp_config: None` at the AgentConfig level (session MCP is the live path, not agent-config MCP).

**Chain defects found**: (1) `setup_mcp` uses `spawn` not `spawn_with_env` — per-server `env` from config is **dropped** in the plan-runner path (orchestrate.rs:4172 vs `client.rs:172-188`; bridge does it right), so credential injection per design (`02-mcp-as-connect-protocol.md:360-386`) only works for HTTP backends or inherited parent env. (2) Registry grouping splits deduped names on `"__"` but the separator is `"."` — group key becomes the full tool name, mis-attributing server names in `add_mcp_tools` (orchestrate.rs:4227-4238 vs `handler.rs:19`). (3) Format collision on `.roko/mcp-config.json`: orchestrate writes `{"servers":[...]}` (orchestrate.rs:4265-4277) while `process/mcp.rs:185-202` writes `{"mcpServers":{"roko":...}}` to the same path; the orchestrate-generated file is then handed to `claude --mcp-config`, which expects the `mcpServers` shape — likely means Claude sees **no** roko-defined MCP servers (and `--strict-mcp-config` suppresses user-scope ones). Needs live verification. (4) Client targets protocol `2025-11-25` (`client.rs:21`); all four servers pin `2024-11-05` in `initialize` — no negotiation.

## Deep trace — the six config conventions (writers & readers)
> Second-pass addition · re-verified 2026-07-08 @ HEAD 5852c93c05. The "4 conventions" summary above is the *file-format* count; there are **six distinct consumers/producers**, two of which write the **same path** in **incompatible shapes**.

| # | Convention (path) | Shape | Written by (file:line) | Read by (file:line) | Reachable in prod? |
|---|---|---|---|---|---|
| C1 | `roko.toml [agent] mcp_config = <path>` | points at a `{"servers":[…]}` file | user / `roko config` (not auto) | `config.agent.mcp_config` → `resolve_mcp_config_path` fallback (orchestrate.rs:4261,4272,4290,4298); loaded at orchestrate.rs:4139 | ✅ Claude-CLI passthrough |
| C2 | walk-up `.mcp.json` (then `$HOME/.mcp.json`) | roko `{"servers":[{name,transport,command,args,env,endpoint,auth_token,tier}]}` | user | `find_mcp_config` (mcp/config.rs:84-104); `load_config` at config.rs:107-117; consumed by `PlanRunner::setup_mcp` walk-up (orchestrate.rs:4125-4171) and Claude-CLI fallback (claude_cli_agent.rs:271-276) | ✅ plan runner + CLI backend |
| C3 | `roko config mcp` → `.roko/mcp.json` | roko `{"servers":[…]}` | `ConfigMcpCmd::Add` (main.rs:3301-3312) | `ConfigMcpCmd::List/Test` via `resolve_mcp_config_path(None, wd)` (main.rs:3234,3256,3328) → `.roko/mcp.json` **→ `~/.claude/mcp-config.json` → walk-up `.mcp.json`** (main.rs:3324-3341) | 🟡 CLI-only; the `~/.claude` fallback is **Claude-shape, unparseable by `McpConfig::load`** (open Q3) |
| C4 | legacy `McpLaunch` chain: `$ROKO_MCP_CONFIG` → `.roko/mcp-config.local.json` → `.roko/mcp-config.json` → `.codex/config.toml` | Claude `{"mcpServers":{"roko":{command,args}}}` | `write_mcp_config` (process/mcp.rs:185-202) | `find_mcp_launch` (process/mcp.rs:148-179); normalizes to `cargo run -p roko-mcp` — **crate absent from workspace** (process/mcp.rs:104-117) | ❌ dead (roko-mcp crate missing) |
| C5 | orchestrate runtime writer → `.roko/mcp-config.json` | roko `{"servers":[…]}` (serialized `McpConfig`, orchestrate.rs:4265,4277) | `resolve_mcp_config_path` (orchestrate.rs:4254-4302) | handed to `claude --mcp-config <path> --strict-mcp-config` (claude_cli_agent.rs:340-342) — **Claude expects `{"mcpServers":{}}`, not `{"servers":[]}`** | 🟥 **collision** (see trace) |
| C6 | ACP `session/new` `mcp_servers: Vec<McpServerConfig>` | roko in-memory (types.rs:277, transport enum types.rs:359) | Zed/ACP client over JSON-RPC | `setup_session_mcp_tools` (bridge_events.rs:2635) → per-session spawn; **openai-compat providers only** (bridge_events.rs:2188-2189); HTTP rejected (2652-2664); Claude-CLI ACP path drops it | 🔌 openai-compat ACP only |

**C4 and C5 write the identical path `<workdir>/.roko/mcp-config.json` in opposing shapes** — last writer wins, and their readers disagree on schema.

### Config-collision failure trace (`.roko/mcp-config.json`)
```
plan run
  └─ PlanRunner::setup_mcp (orchestrate.rs:4125)         spawns servers, fills mcp_state.server_configs
       └─ resolve_mcp_config_path (orchestrate.rs:4254)
            └─ McpConfig { servers }.serialize  ───────►  writes {"servers":[{name,command,…}]}   ← ROKO shape
                                                          to <workdir>/.roko/mcp-config.json (4275)
  └─ Claude CLI invocation (claude_cli_agent.rs:340)
       └─ --mcp-config <that file> --strict-mcp-config
            └─ claude parses, expects  {"mcpServers":{"<name>":{command,args,env}}}   ← CLAUDE shape
                 └─ key "servers" unknown ⇒ 0 servers registered
                     └─ --strict-mcp-config also suppresses user-scope ~/.claude servers
                         ⇒ agent sees NO roko MCP tools  (open Q1 — needs live claude probe)
  ── meanwhile ──
  process/mcp.rs::write_mcp_config (185)  ─────────────►  writes {"mcpServers":{"roko":…}}         ← CLAUDE shape
                                                          to the SAME path (dead C4 path, roko-mcp missing)
```
The single highest-value fix is a `McpConfig → {"mcpServers":{}}` normalizer inside `resolve_mcp_config_path` (orchestrate.rs:4265) — the redesign paper's "normalizer ❌" item.

### Tool-schema catalog (advertised via `tools/list`)
Servers hand-roll `tools/list`; **none emit MCP annotations** — `roko-mcp-code::tool_spec` (lib.rs:1545-1551) returns only `{name,description,inputSchema}`, no `readOnlyHint`/`openWorldHint`. Consequence: the client's `to_tool_def.rs:38-53` readOnly→Read / openWorld→network mapping **never fires**, so all 10 read-only code-intel tools are classified `ToolCategory::Mcp` **with Write permission** (conservative default). Same for github/slack/scripts.

| Server | Tool | Required params | Optional params (defaults) |
|---|---|---|---|
| code | `search_code` | `query` | `strategy∈{keyword,structural,hdc,embedding,hybrid}`(hybrid), `max_results`(10), `file_pattern`, `kind_filter` (lib.rs:227-249) |
| code | `get_symbol_context` | `symbol_name` | `file_path`, `include_dependencies`(true), `include_callers`(true), `expansion_depth`(1) (250-265) |
| code | `get_file_ast` | `file_path` | `include_bodies`(false) (266-278) |
| code | `find_similar_patterns` | `reference` | `min_similarity`(0.6), `max_results`(10) (279-292) |
| code | `get_index_stats` | — | — (293-301) |
| code | `find_references` | `symbol_name` | `file_path`, `include_definitions`(false) (302-315) |
| code | `find_implementations` | `trait_name` | `include_methods`(true) (316-328) |
| code | `get_callers` | `function_name` | `file_path`, `transitive`(false), `max_depth`(2) (329-343) |
| code | `workspace_map` | — | `depth∈{crate,module,symbol}`(module), `focus_path` (344-359) |
| code | `get_context` | `task` | `token_budget`(40000), `include_tests`(false) (360-373) |
| code | *hidden* `symbol_lookup`/`call_graph`/`imports`/`semantic_search` | — | dispatchable (lib.rs:400-403) but **absent from `tools/list`** — invisible to any client |
| github | 19 tools `github.list_prs`…`get_actions_status` | per-tool | 6 accept `api_base_url` (mockable); 13 hardcode `api.github.com` (main.rs:828-852) |
| slack | 9 tools, mixed `slack.` / `slack_` prefixes | per-tool | server pre-embeds prefix ⇒ post-client double-prefix (main.rs:383-398) |
| scripts | `run_script`(name,args), `list_scripts` | `name` | generic — no per-script schema (main.rs:110-141) |

## Current state table

| Component | Design source | Code | Status | Evidence |
|---|---|---|---|---|
| Shared stdio transport (server side) | v1 `18-tools/13-mcp-stdio.md` | `roko-mcp-stdio/src/lib.rs` | ✅ (simpler than design; no registry/handler traits) | lib.rs:110-196 vs 13-mcp-stdio.md:27-50 |
| Code-intelligence server | v1 `15-code-intelligence/07`, v2 `22-code-intelligence/05` | `roko-mcp-code/src/lib.rs` | ✅ 14 tools, 13 tests; README stale 🕰️ | lib.rs:224-376,400-403; README.md:16-33 |
| GitHub server | v1 `18-tools/10-mcp-github.md` ("Scaffold") | `roko-mcp-github/src/main.rs` | ✅ 19 tools, rate-limit handling, 22 tests; hardcoded api.github.com 🟡 | main.rs:828-852,1263-1354; 10-mcp-github.md:7 |
| Slack server | v1 `18-tools/11-mcp-slack.md` ("Scaffold") | `roko-mcp-slack/src/main.rs` | 🟡 9 tools work but 2 tests, mixed `slack.`/`slack_` naming 🕰️ | main.rs:383-398,1060-1113 |
| Scripts server | v1 `18-tools/12-mcp-scripts.md` (scripts.toml) | `roko-mcp-scripts/src/main.rs` | 🟡 works, but dir-scan model ≠ designed declarative scripts.toml 🕰️; unused toml/glob deps | main.rs:110-141,387-497; Cargo.toml:21,25 |
| Client: spawn/init/list/call | v2 `02-mcp-as-connect-protocol.md` §2 | `roko-agent/src/mcp/client.rs` | ✅ | client.rs:21,172-188 |
| Tool→ToolDef conversion + annotations | v2 §3 | `mcp/to_tool_def.rs` | ✅ (readOnly/openWorld/idempotent mapped) | to_tool_def.rs:22-65 |
| Multi-server dedup + namespacing | v2 §3-4 | `mcp/dedup.rs` | ✅ (`server.tool`, last-writer-wins) | dedup.rs:21-43 |
| Dynamic registry merge | v2 §4 (3-layer) | `mcp/dynamic_registry.rs` + orchestrate | 🔌 2-layer (static+MCP, `prefer_mcp` flag); `__` grouping bug | orchestrate.rs:4221-4238 |
| Dispatch routing to live clients | v2 §2 | `mcp/handler.rs` | ✅ incl. error accumulator | handler.rs:19-79 |
| roko.toml → `--mcp-config` passthrough | v1 `18-tools/09` | orchestrate + claude_cli_agent | ✅ wired / 🟡 generated-file format suspect | orchestrate.rs:4254-4302; claude_cli_agent.rs:340-342 |
| Per-agent `AgentConfig.mcp_config` field | passthrough-gaps Fix 2 | `roko-core/config/agent.rs` | ✅ landed (was ❌ in intent doc) | agent.rs:93,147 |
| ACP session-scoped MCP (`session/new` mcpServers) | passthrough-gaps Fix 1/3, redesign AC | `roko-acp/bridge_events.rs` | 🔌 openai-compat only; **Claude-CLI ACP still drops it**; HTTP rejected | bridge_events.rs:2188-2202,2635-2791; types.rs:277 |
| Per-server env credential injection | v2 §9 | `client.rs::spawn_with_env` | 🔌 bridge path only; dropped in setup_mcp | bridge.rs:49; orchestrate.rs:4172 |
| Per-task `mcp_servers` selection | — (implementation-led) | task_parser + orchestrate | ✅ | task_parser.rs:77; orchestrate.rs:4168-4171 |
| `roko config mcp list/test/add` | — | `roko-cli/src/main.rs` | ✅ (writes `.roko/mcp.json`) | main.rs:3227-3341 |
| HTTP (Streamable) transport | v2 §7 | enum only | ❌ config parses, bridge rejects, setup_mcp ignores `transport` | config.rs:47-53; bridge.rs:42-47 |
| `tools/list_changed` hot reload | v2 §6 | — | ❌ | no handler anywhere in `mcp/` |
| Sampling (reverse connect) | v2 §8 | — | ❌ | — |
| Trust tier enforcement (`tier: PluginTier`) | v2 §4 trust table | parsed, surfaced in `config mcp list` | 🔌 parsed; enforcement in dispatch unverified | config.rs:35-41; main.rs:3246 |
| Default registration of roko-mcp-* servers | v2 §7 config example | — | ❌ `roko init` writes token comments only | commands/init.rs:41,43 |
| Legacy `McpLaunch`/`roko-mcp` fallback | — (mori heritage) | `process/mcp.rs` | 🕰️ references nonexistent `roko-mcp` crate; format-conflicts with orchestrate writer | process/mcp.rs:103-117,185-202 |

## V2-aligned
- Connect-protocol lifecycle is implemented in substance: spawn→`initialize`→`tools/list`→`tools/call`→drop matches connect/query/execute/disconnect (`client.rs`, orchestrate.rs:4172-4215 vs `02-mcp-as-connect-protocol.md:18-23`).
- `server.tool` namespacing + registry merge + conservative Write-trust default with `readOnly` downgrade exactly as specced (to_tool_def.rs:38-53 vs doc §3-4).
- Dedup with last-writer-wins (dedup.rs:21-43); per-server crash isolation with warn-and-continue (orchestrate.rs:4177-4214).
- HTTP-backend bridge is a v2-consistent adaptation the docs don't cover (bridge.rs:1-6).

## Cross-cutting: tmp intent docs vs code
- **`tmp/tmp-feedback/2/31-MCP-PASSTHROUGH-GAPS.md` (P2)** — Fix 2 (AgentConfig.mcp_config) **DONE** (agent.rs:93). Fix 1 (ACP Claude-CLI drops session MCP) **still open for Claude-CLI backend** — session MCP only wired for openai-compat (bridge_events.rs:2188-2189). Fix 3 refs a `crates/roko-cli/src/mcp_discovery.rs` that **does not exist** (auto-discovery is `roko_agent::mcp::find_mcp_config`); ACP now runs its own per-session discovery in `setup_session_mcp_tools`, so Fix 3's intent is partially met on the openai-compat path only.
- **`tmp/relay-bus/demo-ide-issue-4-mcp-redesign.md` (2026-05-08, position paper)** — argues per-agent MCP config over a hosted gateway. Acceptance criteria status: per-agent config field ✅ (agent.rs:93); ACP `session/new` mcpServers ✅ (types.rs:277); **Claude-style `mcpServers`→Roko `servers` normalizer ❌** (this is the same P0 format-collision below — no normalizer anywhere; two writers disagree); **Railway `ROKO_AGENT_MCP_CONFIG_JSON`/`_B64` materialization ❌** (grep: absent from all crates); **`mcp.*` relay-bus audit events ❌** (`mcp.config.materialized`, `mcp.tool.call.*` — none emitted); allowlist-enforced-by-runtime = the same unverified `PluginTier` question (open Q2). Net: the redesign's *direction* matches code, but its concrete new artifacts are unbuilt.

## Old paradigm & tech debt 🕰️
- **Config sprawl**: 4 file conventions (`.mcp.json`, `.roko/mcp.json`, `.roko/mcp-config(.local).json`, `agent.mcp_config`) with 2 incompatible JSON shapes; two writers of the *same* `.roko/mcp-config.json` path (orchestrate.rs:4275 vs process/mcp.rs:188).
- **Dead `roko-mcp` singleton path**: `process/mcp.rs` normalizes to `cargo run -p roko-mcp` — crate absent from workspace members (Cargo.toml:39-43).
- **Slack tool naming** mixes `slack.x` and `slack_x`; server-side names also pre-embed the prefix, so after client prefixing tools become `github.github.list_prs`-style — double prefix vs design's `{server}.{tool}`.
- **Design docs stale in both directions**: 10/11/12/13 say Scaffold/Planned though implemented; v2 doc's github=17/slack=8 counts outdated; roko-mcp-code README describes a different tool set and a nonexistent `--workspace` flag.
- **scripts crate** diverges from the declarative scripts.toml design; carries unused `toml`/`glob` deps.
- 4 hidden legacy tools in roko-mcp-code dispatch not exposed in `tools/list` (lib.rs:400-403).

## Not implemented ❌
- HTTP/Streamable transport (enum + endpoint/auth_token fields parsed, no client); `tools/list_changed` notifications / hot reload; MCP sampling; resources/prompts surfaces; protocol-version negotiation (client 2025-11-25 vs servers 2024-11-05); default registration/packaging of the four servers (no template, no install step); GHE support in roko-mcp-github (hardcoded api.github.com); MCP entries in `.roko/GAPS.md`.

## Migration checklist
- [ ] **[P0]** Fix `.roko/mcp-config.json` format handed to Claude CLI — decide one shape (`mcpServers` map) and convert `McpConfig`→Claude format in `resolve_mcp_config_path` — verify: `cargo run -p roko-cli -- plan run plans/ ` with a `.mcp.json` defining `roko-code`, then check agent log lists `roko-code.*` tools (and `claude --mcp-config .roko/mcp-config.json --strict-mcp-config -p 'list tools'` accepts the file)
- [ ] **[P0]** Pass `server.env` in the plan-runner spawn path (use `spawn_with_env` at `orchestrate.rs:4172`) — verify: server config with `env={"GITHUB_TOKEN":"${GITHUB_TOKEN}"}` reaches child: `grep -n 'spawn_with_env' crates/roko-cli/src/orchestrate.rs`
- [ ] **[P1]** Thread session `mcp_servers` into the **Claude-CLI ACP dispatch** path (currently only openai-compat gets session MCP; Claude-CLI ACP sessions in Zed get no MCP tools) — verify: `rg -n 'mcp' crates/roko-acp/src/bridge_events.rs` shows `--mcp-config` on the Claude-CLI branch, not only `run_openai_compat_mcp_tool_loop`
- [ ] **[P1]** Fix `split("__")` grouping to split on `.` (or reuse `handler.rs::split_prefixed_tool_name`) at `orchestrate.rs:4227-4235` — verify: unit test asserting `add_mcp_tools` server key == `github` for tool `github.get_pr`
- [ ] **[P1]** Unify config conventions: deprecate `process/mcp.rs` `McpLaunch` chain + nonexistent `roko-mcp` fallback; make `roko config mcp` and walk-up discovery agree on one file — verify: `rg -n 'roko-mcp"' crates/roko-agent/src/process/mcp.rs` returns nothing; `cargo run -p roko-cli -- config mcp list` finds the same file setup_mcp loads
- [ ] **[P1]** Register roko-mcp-code by default (e.g. `roko init` writes `.roko/mcp.json` entry with `command = "roko-mcp-code"`) — verify: `cargo run -p roko-cli -- init && cargo run -p roko-cli -- config mcp test roko-code`
- [ ] **[P2]** Normalize slack tool names to `slack.*`; drop server-side prefixes or client-side re-prefixing to kill `github.github.*` double prefix — verify: `echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run -p roko-mcp-slack` shows uniform names
- [ ] **[P2]** Update stale docs: v1 18-tools 10-13 status lines, v2 tool counts, roko-mcp-code README (tool table, remove `--workspace`), CLAUDE.md crate-table ref to deleted `tmp/ux-followup/05-…` — verify: `rg -n 'Scaffold|find_symbol|--workspace' docs/v1/18-tools crates/roko-mcp-code/README.md` clean
- [ ] **[P2]** Expose or delete the 4 hidden roko-mcp-code tools; add integration tests promised by README — verify: `tools/list` length matches dispatch arms in `lib.rs:389-404`
- [ ] **[P3]** Implement HTTP transport or remove `McpTransportConfig::Http`/`endpoint`/`auth_token` until real — verify: `.mcp.json` with `transport="http"` either connects or errors at config-load, not at dispatch
- [ ] **[P3]** GHE support: thread `api_base_url` (env `GITHUB_API_URL`) through the 13 hardcoded handlers — verify: `rg -c 'https://api.github.com' crates/roko-mcp-github/src/main.rs` → 0 outside defaults
- [ ] **[P3]** Add MCP section to `.roko/GAPS.md` covering the above — verify: `rg -i mcp .roko/GAPS.md`

## Open questions
1. Does Claude CLI actually reject roko's `{"servers":[...]}` shape, or silently ignore it? (Determines whether MCP-through-Claude ever worked in plan runs; needs a live `claude --mcp-config` probe.)
2. Where (if anywhere) is `McpServerConfig.tier: PluginTier` enforced at dispatch time? Parsed and printed (`main.rs:3246`) but no enforcement site was found in the tool pipeline.
3. Is the `~/.claude/mcp-config.json` fallback in `resolve_mcp_config_path` (main.rs:3332-3336) intentional given that file is Claude-format and unparseable by `McpConfig::load`?
4. Should `roko-mcp-scripts` converge on the designed `scripts.toml` declarative model (per-script schemas) or should the design doc be rewritten to the simpler dir-scan reality?
5. protocol-version pinning: servers at `2024-11-05`, client targeting `2025-11-25` — adopt shared const from one crate?
