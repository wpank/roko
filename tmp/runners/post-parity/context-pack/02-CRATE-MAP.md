# Post-Parity Crate Map

## Key Crates

| Crate | Path | Role in Post-Parity |
|---|---|---|
| roko-agent | `crates/roko-agent/` | HttpPoster trait, ReqwestPoster, ClaudeCliAgent, ClaudeApiAgent, safety |
| roko-cli | `crates/roko-cli/` | Chat dispatch, slash commands, TUI, orchestrate.rs freeze |
| roko-compose | `crates/roko-compose/` | SystemPromptBuilder (already works, just needs to be called) |
| roko-serve | `crates/roko-serve/` | HTTP control plane routes (needs shared client) |
| roko-agent-server | `crates/roko-agent-server/` | Per-agent sidecar (needs shared client) |
| roko-core | `crates/roko-core/` | Config types, tool specs |

## Key File Paths

| File | Owner | Role |
|---|---|---|
| `crates/roko-agent/src/http.rs` | PA | HttpPoster trait + ReqwestPoster impl |
| `crates/roko-agent/src/claude_cli_agent.rs` | PB | Claude CLI agent dispatch |
| `crates/roko-agent/src/claude_agent.rs` | PB | Claude API agent dispatch |
| `crates/roko-agent/src/safety/mod.rs` | PE | Safety layer, contract_for_role |
| `crates/roko-agent/src/safety/contract.rs` | PE | AgentContract struct |
| `crates/roko-agent/src/provider/claude_cli.rs` | PE | dangerously_skip_permissions wiring |
| `crates/roko-agent/src/provider/mod.rs` | PE | Provider options struct |
| `crates/roko-cli/src/chat_inline.rs` | PB/PD | Chat REPL + slash commands |
| `crates/roko-cli/src/chat_session.rs` | PB | ChatAgentSession struct |
| `crates/roko-cli/src/dispatch_direct.rs` | PB | Legacy dispatch (to be replaced) |
| `crates/roko-cli/src/unified.rs` | PB | Entry point for roko "prompt" |
| `crates/roko-cli/src/inline/primitives/streaming.rs` | PC | StreamingState struct |
| `crates/roko-cli/src/orchestrate.rs` | PF/PG | Legacy orchestrator (freeze target) |
| `crates/roko-cli/src/tui/views/agents_view.rs` | PG | TUI agent view (model "-" bug) |
| `crates/roko-cli/Cargo.toml` | PF | legacy-orchestrate feature flag |

## Shared File Hotspots

| File | Runners | Allowed Edits | Forbidden |
|---|---|---|---|
| `chat_inline.rs` | PB, PD | PB: dispatch wiring; PD: slash command bodies | Structural changes |
| `chat_session.rs` | PB | Session field usage | New session state owners |
| `http.rs` | PA | Shared client factory | New HttpPoster impls |
| `orchestrate.rs` | PF, PG | PF: deprecation attr; PG: drain efficiency_events | ANY other changes |
| `Cargo.toml` (roko-cli) | PF | Feature default change | New features |
