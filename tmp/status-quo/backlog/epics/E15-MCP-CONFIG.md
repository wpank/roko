# E15 — MCP Config & Passthrough

> Executable backlog epic · derived from status-quo doc `48-MCP-CRATES.md` (re-verified 2026-07-08 @ HEAD 5852c93c05)
> Native task schema: `crates/roko-cli/src/task_parser.rs::TaskDef`. Run with `roko plan run plans/E15-mcp-config/`.

## Goal

Make roko's MCP passthrough **actually deliver tools to the agent**. The client side (spawn →
`initialize` → `tools/list` → dynamic registry) is genuinely wired, but the *seams* are broken:
the file handed to `claude --mcp-config` is written in the wrong JSON shape (0 servers registered),
per-server `env` is dropped before spawn, the registry groups tools on the wrong separator, the
Claude-CLI ACP path threads no session MCP at all, and no server emits annotations so every
read-only code tool is mis-classified as a Write tool.

**Single highest-value fix:** `E15-T1` — the `McpConfig → {"mcpServers":{}}` normalizer in
`resolve_mcp_config_path` (orchestrate.rs:4265). Without it, the entire Claude-CLI MCP passthrough
path is a silent no-op regardless of how servers are configured.

## The six config conventions (writers & readers)

Source: `48-MCP-CRATES.md` §"Deep trace — the six config conventions". Four *file-format* shapes,
six distinct consumers/producers; **C4 and C5 write the identical path in opposing shapes**.

| # | Convention (path) | Shape | Written by | Read by | Prod? |
|---|---|---|---|---|---|
| C1 | `roko.toml [agent] mcp_config = <path>` | points at `{"servers":[…]}` | user / `roko config` | `config.agent.mcp_config` → `resolve_mcp_config_path` fallback (orchestrate.rs:4261) | ✅ Claude-CLI passthrough |
| C2 | walk-up `.mcp.json` (then `$HOME/.mcp.json`) | roko `{"servers":[{name,transport,command,args,env,endpoint,auth_token,tier}]}` | user | `find_mcp_config` (mcp/config.rs:84); `PlanRunner::setup_mcp` + Claude-CLI fallback (claude_cli_agent.rs:271) | ✅ plan runner + CLI |
| C3 | `roko config mcp` → `.roko/mcp.json` | roko `{"servers":[…]}` | `ConfigMcpCmd::Add` (main.rs:3301) | `List/Test` → `.roko/mcp.json` → `~/.claude/mcp-config.json` → walk-up `.mcp.json` (main.rs:3324) | 🟡 CLI-only; `~/.claude` fallback is Claude-shape, unparseable |
| C4 | legacy `McpLaunch`: `$ROKO_MCP_CONFIG` → `.roko/mcp-config.local.json` → `.roko/mcp-config.json` → `.codex/config.toml` | Claude `{"mcpServers":{"roko":{…}}}` | `write_mcp_config` (process/mcp.rs:185) | `find_mcp_launch` (process/mcp.rs:148) → `cargo run -p roko-mcp` (**crate absent**) | ❌ dead |
| C5 | orchestrate runtime writer → `.roko/mcp-config.json` | roko `{"servers":[…]}` (serialized `McpConfig`) | `resolve_mcp_config_path` (orchestrate.rs:4254-4302) | `claude --mcp-config <path> --strict-mcp-config` (claude_cli_agent.rs:340) — **expects `{"mcpServers":{}}`** | 🟥 **collision** |
| C6 | ACP `session/new` `mcp_servers: Vec<McpServerConfig>` | roko in-memory (types.rs:277) | Zed/ACP client over JSON-RPC | `setup_session_mcp_tools` (bridge_events.rs:2635); **openai-compat only**; HTTP rejected; **Claude-CLI ACP path drops it** | 🔌 openai-compat ACP only |

**C4 and C5 write the identical `<workdir>/.roko/mcp-config.json` in opposing shapes** — last
writer wins and their readers disagree on schema. E15-T1 normalizes C5's output to the Claude shape;
E15-T6 neutralizes the dead C4 writer so it can no longer clobber the same path.

## Findings (from 48-MCP-CRATES §"Chain defects" + §"Config-collision failure trace")

| # | Finding | Site | Severity | Task |
|---|---|---|---|---|
| a | **Format collision** — orchestrate writes `{"servers":[…]}` but `claude --mcp-config --strict-mcp-config` expects `{"mcpServers":{}}` → key `"servers"` unknown ⇒ **0 servers registered**; `--strict` also suppresses user-scope `~/.claude` servers. No normalizer anywhere. | orchestrate.rs:4265, claude_cli_agent.rs:340 | **P0** | E15-T1 |
| b | **Per-server env dropped** — `setup_mcp` uses `StdioTransport::spawn` not `spawn_with_env`; `env` from `McpServerConfig` (config.rs:28) never reaches the child. Credential injection only works for HTTP backends / inherited parent env. | orchestrate.rs:4172 vs client.rs:180 | **P0** | E15-T2 |
| c | **Separator bug** — registry grouping splits deduped names on `"__"` but tools are namespaced `{server}.{tool}` (separator `"."`); group key becomes the full tool name, mis-attributing server names in `add_mcp_tools`. | orchestrate.rs:4227-4238 vs handler.rs:19 | P1 | E15-T3 |
| d | **Claude-CLI ACP drops session MCP** — session MCP tool-loop is gated to openai-compat providers (`openai_compat_tool_loop_supported`, bridge_events.rs:2189); Claude-CLI-backed ACP sessions in Zed get **no** MCP tools. | bridge_events.rs:2188-2202, 2635 | P1 | E15-T4 |
| e | **No tool annotations** — servers hand-roll `tools/list`; `tool_spec` returns only `{name,description,inputSchema}`, no `readOnlyHint`/`openWorldHint`. `to_tool_def.rs:38` readOnly→Read / openWorld→network mapping **never fires** → all 10 read-only code-intel tools classified `Mcp` with **Write** permission. | roko-mcp-code lib.rs:1545 → to_tool_def.rs:37-49 | P1 | E15-T5 |
| f | **Dead C4 writer clobbers C5 path** — `process/mcp.rs::write_mcp_config` writes Claude-shape to the same `.roko/mcp-config.json`; its `find_mcp_launch` normalizes to nonexistent `roko-mcp` crate. Format-conflicts with the orchestrate writer. | process/mcp.rs:103-117,185-202 | P2 | E15-T6 |

## Reconciliation with existing plan P25-mcp-acp-passthrough

`P25` (`plans/P25-mcp-acp-passthrough/tasks.toml`, 4 tasks, status `ready`) is scored **CURRENT
(on-target)** in `02-PLANS-RECONCILIATION.md` — but it **predates the config-shape-collision
finding** and is scoped entirely to the *ACP openai-compat* seam. Overlap/gaps:

| P25 task | What it does | Verdict vs E15 |
|---|---|---|
| P25-T1 | Add `mcp_config: Option<PathBuf>` to roko-core `AgentConfig` | **Now redundant** — field already landed at `agent.rs:93` (doc §"What changed"). Executing P25-T1 is a no-op / may conflict. |
| P25-T2 | Wire MCP config into ACP `run_with_workflow_engine` `ServiceConfig` (drop `mcp_config: None`) | Complementary; still open. Not in E15 (agent-config seam, not the passthrough-shape seam). |
| P25-T3 | Add `mcp_config_path` to `AcpSession` + auto-discovery on session create | Complementary; still open. Not in E15. |
| P25-T4 | Wire session MCP into bridge_events tool-loop — **explicitly scopes to "openai_compat/generic paths only; do NOT touch the Anthropic path"** | **Directly conflicts with the real gap.** The audit's finding (d) is that the *Claude-CLI/Anthropic* ACP path is the one dropping session MCP. E15-T4 targets exactly what P25-T4 excludes. |

**What P25 misses** (the whole reason E15 exists): the `{"servers"}`→`{"mcpServers"}` normalizer
(finding a — the highest-value fix), per-server `env` passing (b), the `split("__")` separator bug
(c), the **Claude-CLI** ACP session-MCP path (d — P25-T4 deliberately excludes it), and tool
annotations (e). P25 fixes the ACP *agent-config plumbing*; E15 fixes the *config-shape/dispatch
seams* that make passthrough a no-op even when plumbing is correct.

**Recommendation:** keep P25-T2/T3 (ACP agent-config wiring), **drop P25-T1** (already done),
**supersede P25-T4 with E15-T4** (broaden to the Claude-CLI branch). Run E15-T1 first regardless —
it is independent of all P25 work and unblocks the primary plan-runner passthrough path.

## Tasks

| ID | Title | Tier | Files | Depends | Finding |
|---|---|---|---|---|---|
| E15-T1 | Normalize `McpConfig` → Claude `{"mcpServers":{}}` in `resolve_mcp_config_path` | focused | orchestrate.rs | — | a |
| E15-T2 | Pass per-server `env` via `spawn_with_env` in `setup_mcp` | focused | orchestrate.rs | — | b |
| E15-T3 | Fix registry grouping to split on `.` not `__` | mechanical | orchestrate.rs | — | c |
| E15-T4 | Thread session `mcp_servers` into the Claude-CLI ACP dispatch (`--mcp-config`) | integrative | bridge_events.rs | E15-T1 | d |
| E15-T5 | Emit `readOnlyHint`/`openWorldHint` annotations from `tool_spec` | focused | roko-mcp-code/lib.rs | — | e |
| E15-T6 | Neutralize the dead C4 writer (`process/mcp.rs`) so it can't clobber C5's path | mechanical | process/mcp.rs | E15-T1 | f |

### First three tasks (valid native TOML)

```toml
[meta]
plan = "E15-mcp-config"
total = 6
done = 0
status = "ready"
max_parallel = 2

# ── E15-T1: Normalize McpConfig → Claude {"mcpServers":{}} shape ──────────────
#
# resolve_mcp_config_path serializes `McpConfig { servers: Vec<..> }` as
# {"servers":[...]} and hands it to `claude --mcp-config <path> --strict-mcp-config`.
# Claude expects {"mcpServers":{"<name>":{command,args,env}}}. The "servers" key is
# unknown → 0 servers registered → agent sees NO roko MCP tools. Convert to the
# Claude map shape before writing the file. This is the single highest-value MCP fix.

[[task]]
id = "E15-T1"
title = "Normalize McpConfig to Claude mcpServers shape in resolve_mcp_config_path"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-20250514"
max_loc = 55
files = ["crates/roko-cli/src/orchestrate.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-cli/src/orchestrate.rs", lines = "4254-4302", why = "resolve_mcp_config_path — builds McpConfig { servers } and serde_json::to_string_pretty at 4277; replace with Claude-shape serialization" },
    { path = "crates/roko-agent/src/mcp/config.rs", lines = "14-41", why = "McpServerConfig fields: name, command, args, env (HashMap), transport, endpoint, auth_token, tier — source for the mcpServers map entries" },
    { path = "crates/roko-agent/src/provider/claude_cli_agent.rs", lines = "336-344", why = "consumer: --mcp-config <path> --strict-mcp-config — the file MUST be Claude shape" },
]
symbols = [
    "PlanRunner::resolve_mcp_config_path — async fn at orchestrate.rs:4254, returns Option<PathBuf>",
    "McpServerConfig { name, command, args, env } — config.rs:14",
]
anti_patterns = [
    "Do NOT change the roko `{\"servers\":[…]}` reader format for .mcp.json (C2) — only the OUTPUT written for the Claude CLI must be the mcpServers map.",
    "Do NOT emit the roko-only fields (tier, transport, endpoint, auth_token) into the Claude map — Claude accepts command/args/env only.",
    "Do NOT drop per-server env from the emitted map — include env when non-empty (pairs with E15-T2).",
    "Do NOT hand-format JSON with string concatenation — build a serde_json::json!/Map value and to_string_pretty it.",
]

# Build `{"mcpServers": { <name>: {"command":.., "args":[..], "env":{..}} , ... }}`.
# Serialize that Value (not the McpConfig) and write it to .roko/mcp-config.json.

[[task.verify]]
phase = "structural"
command = "grep -q 'mcpServers' crates/roko-cli/src/orchestrate.rs"
fail_msg = "resolve_mcp_config_path must emit a top-level mcpServers key"

[[task.verify]]
phase = "structural"
command = "! grep -n '\"servers\"' crates/roko-cli/src/orchestrate.rs | grep -q 'to_string_pretty'"
fail_msg = "The file written for the Claude CLI must not serialize the roko servers-array shape"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile after the normalizer change"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli -- resolve_mcp_config 2>&1"
fail_msg = "Add/keep a unit test asserting the generated JSON has an mcpServers map keyed by server name"

# ── E15-T2: Pass per-server env via spawn_with_env in setup_mcp ───────────────
#
# setup_mcp spawns each server with StdioTransport::spawn(&command, &args), which
# ignores the per-server `env` map from McpServerConfig. Credentials declared in
# .mcp.json (e.g. GITHUB_TOKEN) never reach the child. The bridge path already uses
# spawn_with_env; the plan-runner path must too.

[[task]]
id = "E15-T2"
title = "Pass per-server env via spawn_with_env in PlanRunner::setup_mcp"
status = "ready"
tier = "focused"
model_hint = "claude-sonnet-4-20250514"
max_loc = 20
files = ["crates/roko-cli/src/orchestrate.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-cli/src/orchestrate.rs", lines = "4168-4215", why = "setup_mcp spawn loop — StdioTransport::spawn at 4172 drops env; swap to spawn_with_env(&server.command, &server.args, &server.env)" },
    { path = "crates/roko-agent/src/mcp/client.rs", lines = "172-200", why = "StdioTransport::spawn (172) vs spawn_with_env (180) — signature to call" },
    { path = "crates/roko-agent/src/mcp/config.rs", lines = "14-41", why = "McpServerConfig.env: HashMap<String,String> at config.rs:28" },
    { path = "crates/roko-agent/src/mcp/bridge.rs", lines = "38-96", why = "reference: bridge path already calls spawn_with_env correctly" },
]
symbols = [
    "StdioTransport::spawn_with_env(command: &str, args: &[String], env: &HashMap<String,String>) -> Result<Self, McpError> — client.rs:180",
    "McpServerConfig.env — HashMap<String,String> at config.rs:28",
]
anti_patterns = [
    "Do NOT env_clear the child — spawn_with_env should layer server.env over the inherited environment, matching the bridge path.",
    "Do NOT expand ${VAR} yourself unless the bridge path does — keep behavior identical to bridge.rs for consistency.",
]

[[task.verify]]
phase = "structural"
command = "grep -q 'spawn_with_env' crates/roko-cli/src/orchestrate.rs"
fail_msg = "setup_mcp must call spawn_with_env so per-server env reaches the child"

[[task.verify]]
phase = "structural"
command = "! grep -q 'StdioTransport::spawn(' crates/roko-cli/src/orchestrate.rs"
fail_msg = "The env-dropping StdioTransport::spawn call must be replaced"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile after switching to spawn_with_env"

# ── E15-T3: Fix registry grouping separator __ → . ───────────────────────────
#
# After dedup, tools are named `{server}.{tool}` (separator "."). setup_mcp groups
# them by tool.name.split("__").next(), so the group key becomes the entire tool
# name and add_mcp_tools mis-attributes the server. Split on "." (or reuse
# handler.rs::split_prefixed_tool_name) so `github.get_pr` groups under `github`.

[[task]]
id = "E15-T3"
title = "Group MCP tools by the '.' server prefix, not '__'"
status = "ready"
tier = "mechanical"
model_hint = "claude-haiku-4-5-20250514"
max_loc = 15
files = ["crates/roko-cli/src/orchestrate.rs"]
role = "implementer"
depends_on = []

[task.context]
read_files = [
    { path = "crates/roko-cli/src/orchestrate.rs", lines = "4221-4239", why = "grouping loop: tool.name.split(\"__\").next() at 4231 — change delimiter to '.'" },
    { path = "crates/roko-agent/src/mcp/handler.rs", lines = "19-79", why = "split_prefixed_tool_name splits on '.' — the canonical namespacing; reuse or mirror it" },
    { path = "crates/roko-agent/src/mcp/dedup.rs", lines = "21-43", why = "confirms deduped names use `{server}.{tool}` (server.tool namespacing)" },
]
symbols = [
    "by_server grouping loop — orchestrate.rs:4227-4238",
    "roko_agent::mcp::handler::split_prefixed_tool_name — handler.rs:19",
]
anti_patterns = [
    "Do NOT split_once from the RIGHT — server prefix is everything before the FIRST '.'; use splitn/split_once on the first '.'.",
    "Do NOT change the dispatch resolver in handler.rs — it already splits on '.' correctly; only the grouping loop is wrong.",
]

[[task.verify]]
phase = "structural"
command = "! grep -q 'split(\"__\")' crates/roko-cli/src/orchestrate.rs"
fail_msg = "The '__' separator split must be removed from the MCP grouping loop"

[[task.verify]]
phase = "compile"
command = "cargo check -p roko-cli 2>&1"
fail_msg = "roko-cli must compile after the separator fix"

[[task.verify]]
phase = "test"
command = "cargo test -p roko-cli -- mcp_group 2>&1"
fail_msg = "Add a unit test asserting a tool named 'github.get_pr' groups under server key 'github'"
```

### Remaining tasks (summary — author full TOML when scheduling)

- **E15-T4** (integrative, `crates/roko-acp/src/bridge_events.rs`, depends `E15-T1`): remove the
  openai-compat gate on session MCP so Claude-CLI-backed ACP sessions also get their
  `session/new` `mcp_servers`. Write the normalized (E15-T1) Claude-shape config to a temp file and
  pass `--mcp-config` on the Claude-CLI branch. **Supersedes P25-T4**, which deliberately excludes
  the Anthropic path. *Verify:* `rg -n 'mcp-config' crates/roko-acp/src/bridge_events.rs` shows the
  flag on the Claude-CLI branch, not only `run_openai_compat_mcp_tool_loop`.
- **E15-T5** (focused, `crates/roko-mcp-code/src/lib.rs`): extend `tool_spec` (lib.rs:1545) to accept
  and emit an `annotations` object with `readOnlyHint`/`openWorldHint`; mark the 10 read-only
  code-intel tools `readOnlyHint: true`. This lets `to_tool_def.rs:38-49` fire the readOnly→Read
  downgrade so they stop being classified as Write. *Verify:*
  `echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | cargo run -p roko-mcp-code` shows
  `readOnlyHint` on `search_code`; unit test in `to_tool_def.rs` asserting `write == false`.
- **E15-T6** (mechanical, `crates/roko-agent/src/process/mcp.rs`, depends `E15-T1`): stop
  `write_mcp_config` from writing the same `.roko/mcp-config.json` path (the dead C4 writer), and
  remove the `cargo run -p roko-mcp` fallback that references a crate absent from the workspace.
  Neutralizes the other half of the collision so E15-T1's output can't be clobbered. *Verify:*
  `rg -n 'roko-mcp"' crates/roko-agent/src/process/mcp.rs` returns nothing.

## Done-when

- `cargo run -p roko-cli -- plan run plans/` with a `.mcp.json` defining `roko-code` produces a
  `.roko/mcp-config.json` whose top-level key is `mcpServers` (a map), and the agent turn log lists
  `roko-code.*` tools. (E15-T1)
- A server config with `env = {GITHUB_TOKEN = "…"}` reaches the child process. (E15-T2)
- Registry grouping attributes `github.get_pr` to server `github`. (E15-T3)
- A Claude-CLI-backed ACP session in Zed receives its `session/new` MCP tools. (E15-T4)
- `search_code` and the other 9 code-intel tools resolve to `Read` permission, not `Write`. (E15-T5)
- No writer other than `resolve_mcp_config_path` targets `.roko/mcp-config.json`. (E15-T6)

## CTRL-08 ownership reconciliation

E15-T7 is the sole GitHub MCP discovery/configuration owner; E01-T13 is its
acceptance roll-up. E14 owns catalog permissions, E46 owns remote GitHub operations,
and E18-T15 documents the integrated behavior. T1/T2/T3/T7 are serialized in manifest
order because they share `orchestrate.rs`; no parallel writer may rewrite the MCP
config path. See
[`17-OPERATIONAL-OWNERSHIP.md`](../17-OPERATIONAL-OWNERSHIP.md).
