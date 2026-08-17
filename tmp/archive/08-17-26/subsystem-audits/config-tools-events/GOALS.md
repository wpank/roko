# Config, Tools & Events: Goals

## End State

Role definitions, workflow templates, gate configurations, and tool profiles are all config-driven. New roles and workflows created by dropping TOML files. Community marketplace for sharing configs.

## Key Properties

- **Roles as config**: `roles/*.toml` files define identity, tools, budget, model hint. No code changes to add a role.
- **Workflows as config**: `workflows/*.toml` files define step chains. Composable, shareable.
- **Gates as config**: `gates/*.toml` files define verification steps with configurable thresholds.
- **Tool profiles from role config**: Tool allow/deny lists read from role TOML, not hardcoded match arms.
- **Hot-reload**: Config changes take effect without restart.
- **5-tier marketplace**: Prompts → Config profiles → Declarative tools → WASM → Native Rust.
- **Plugin isolation**: Plugins run sandboxed (no crash propagation).
- **Signal decay learning**: Decay functions tuned per-task, not hardcoded per Kind.

## What Exists Today

- **28 roles** as `AgentRole` Rust enum in `crates/roko-core/src/agent.rs` (hardcoded; `ALL_AGENTS = [27]` + `Conductor`)
- **3 workflows** as `WorkflowTemplate` enum (`Express`, `Standard`, `Full`) in `crates/roko-acp/src/pipeline.rs`
- **Tool profiles** as `RoleToolProfile` + `DomainToolProfile` constants in `crates/roko-std/src/roles.rs` (not match statements — already structured constants with `allowed_tools`/`denied_tools` arrays)
- **Config wizard** (`roko init`) works well; covers agent/gates/budget sections
- **Signal system** with 32 kinds (`Kind` enum in `crates/roko-core/src/kind.rs`; well-designed)
- **Plugin system** in `crates/roko-plugin/` (basic, no isolation)
- **Hot-reload** is implemented in `crates/roko-core/src/config/hot_reload.rs` for 7 sections, but not wired to a file-watcher trigger
- **30 builtin tools** (16 std + 14 chain-domain via `TOOL_COUNT = 30` in `crates/roko-std/src/tool/builtin/mod.rs`)

## From v2 UX Showcase (9 Scenarios)

- **Slash command palette** (all): Modal overlay — 47 builtin across 10 groups, 4 user-defined from roko.toml (/ship-it, /post-mortem, /budget-review, /onboard shown in brand color with dashed border), 12 from workflows. Search/filter with keyboard nav (↵ run, ⇥ complete, ↑↓ navigate).
- **User-defined slash commands**: Commands from roko.toml highlighted differently — brand-colored, dashed border. Distinct from builtin commands.
- **Workflow-installed commands**: Footer shows "47 builtin · 4 user · 12 from workflows" — workflows contribute their own slash commands.
- **MCP servers panel** (right rail): Per-server display with name, tool count (8t/12t/6t/4t/9t), active/idle status dot, call count during session. Servers: datadog, github, linear, postgres, filesystem.
- **MCP tool calls with server badge** (incident, debug): ToolCall cards show mcpServer pill (e.g. "datadog", "linear") identifying which MCP server handled the call.

### Data Feeds Required
- `SlashCommandRegistry` — builtin commands, user_commands (from TOML), workflow_commands (from installed workflows)
- `MCPServerState` — per-server: name, tool_count, status (active/idle), call_count
- `MCPToolCall` — tool_name, server_name, result, duration

## Gap

- Role TOML schema + loader + registry (roles are `AgentRole` enum variants; no TOML-driven role addition)
- Workflow TOML schema + loader (`WorkflowTemplate` has 3 hardcoded variants; no TOML-driven extension)
- Config hot-reload **trigger**: `apply_hot_reload` exists but nothing calls `config_diff` on file change
- Plugin sandboxing (plugins panic → process crash)
- Dynamic slash command registration from TOML
- Composable command chaining
- Session → workflow promotion
- Marketplace infrastructure (very large)

---

## Sources

| Claim | Source |
|---|---|
| 28 AgentRole variants | `crates/roko-core/src/agent.rs` — `pub enum AgentRole` + `ALL_AGENTS: [Self; 27]` |
| 3 WorkflowTemplate variants | `crates/roko-acp/src/pipeline.rs` — `pub enum WorkflowTemplate` |
| Tool profiles as constants | `crates/roko-std/src/roles.rs` — `RoleToolProfile`, `DomainToolProfile` |
| Hot-reload implemented | `crates/roko-core/src/config/hot_reload.rs` — `apply_hot_reload`, `config_diff` |
| 30 builtin tools | `crates/roko-std/src/tool/builtin/mod.rs` — `TOOL_COUNT = 30` |
| Plugin SDK | `crates/roko-plugin/src/lib.rs` |
