# Mori Reference Appendix

Use this appendix when a batch or prompt mentions `apps/mori/*`, "Mori
parity", or a stale path in the old monorepo. The goal is to anchor the
current Roko tree with exact file paths instead of letting an agent invent
its own mapping.

Reality rule: if a path, file, or symbol below is missing from the current
worktree, verify with `rg --files` / `rg -n` and stop. Do not guess a
substitute path.

| Mori anchor | Current Roko anchor(s) | Notes |
|---|---|---|
| `apps/mori/src/tui/views/dashboard.rs` | `crates/roko-cli/src/tui/views/dashboard_view.rs` | Master-detail dashboard render. |
| `apps/mori/src/tui/views/agents.rs` | `crates/roko-cli/src/tui/views/agents_view.rs` | Agents tab roster + live output. |
| `apps/mori/src/tui/views/plans.rs` | `crates/roko-cli/src/tui/views/plans_view.rs` | Plans / wave browser. |
| `apps/mori/src/tui/views/git_view.rs` | `crates/roko-cli/src/tui/views/git_view.rs` | Git panel is already file-aligned. |
| `apps/mori/src/tui/views/logs.rs` | `crates/roko-cli/src/tui/views/logs_view.rs` | Unified logs / tail view. |
| `apps/mori/src/tui/views/context.rs` | `crates/roko-cli/src/tui/views/context_view.rs` | Inspect / context panel. |
| `apps/mori/src/tui/input.rs` | `crates/roko-cli/src/tui/input.rs` | Focus, keybind, filter, and input handling. |
| `apps/mori/src/tui/tabs.rs` | `crates/roko-cli/src/tui/tabs.rs` | Top-level tab model. |
| `apps/mori/src/tui/theme.rs` | `crates/roko-cli/src/tui/theme.rs` | Theme / style constants. |
| `apps/mori/src/tui/modals/*` | `crates/roko-cli/src/tui/modals/*` | Modal tree stayed directory-based. |
| `apps/mori/src/agent/connection.rs` | `crates/roko-agent/src/dispatcher/mod.rs`, `crates/roko-agent/src/claude_cli_agent.rs` | Dispatch seam + Claude CLI adapter. |
| `apps/mori/src/agent/protocol.rs` | `crates/roko-agent/src/chat_types.rs`, `crates/roko-core/src/chat_types.rs` | Protocol / message types split across agent + core. |
| `apps/mori/src/agent/roles.rs` | `crates/roko-cli/src/agent_config.rs`, `crates/roko-core/src/config/schema.rs`, `crates/roko-agent/src/safety/mod.rs`, `crates/roko-core/src/tool/role_allowlist.rs` | Role config and enforcement now spans several crates. |
| `apps/mori/src/orchestrator/prompts.rs` | `crates/roko-compose/src/prompt.rs`, `crates/roko-compose/src/role_prompts.rs`, `crates/roko-compose/src/templates/*` | Prompt assembly moved into `roko-compose`. |
| `apps/mori/src/support_enrich/prompts.rs` | `crates/roko-compose/src/enrichment/prompts.rs` | Enrichment prompt builders. |
| `apps/mori/src/orchestrator/dag.rs` | `crates/roko-orchestrator/src/dag.rs` | DAG logic. |
| `apps/mori/src/orchestrator/unified_dag.rs` | `crates/roko-orchestrator/src/executor/`, `crates/roko-orchestrator/src/plan_discovery.rs` | Split across executor + discovery now. |
| `apps/mori/src/orchestrator/plan.rs` | `crates/roko-orchestrator/src/plan_discovery.rs`, `crates/roko-cli/src/orchestrate.rs` | Plan discovery and boot wiring. |
| `apps/mori/src/orchestrator/queue.rs` | `crates/roko-orchestrator/src/merge_queue.rs` | Queue / merge coordination. |
| `apps/mori/src/orchestrator/executor.rs` | `crates/roko-orchestrator/src/executor/mod.rs` | Executor surface. |
| `apps/mori/src/server/router.rs` | `crates/roko-serve/src/routes/*`, `crates/roko-agent-server/src/lib.rs` | HTTP routing is split across the serve and sidecar crates. |
| `apps/mori/src/server/handlers.rs` | `crates/roko-serve/src/routes/*`, `crates/roko-agent-server/src/features/*` | Route handlers and feature modules. |
| `apps/mori/src/state/config.rs` | `crates/roko-cli/src/agent_config.rs`, `crates/roko-core/src/config/schema.rs` | Config schema and CLI loading. |
| `apps/mori/src/git/worktree.rs` | `crates/roko-orchestrator/src/worktree.rs`, `crates/roko-cli/src/workspace_paths.rs` | Worktree lifecycle and path helpers. |

Use this appendix as a lookup table, not as a license to invent new paths.
If a batch needs an exact current anchor that is not listed here, add a
follow-up note and verify the path in the current tree before editing.
