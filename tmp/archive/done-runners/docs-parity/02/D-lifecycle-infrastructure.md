# D — Lifecycle And Infrastructure

Refresh target: `docs/02-agents/05-agent-pools.md`, `06-mcp-integration.md`, `13-creation-sites.md`

Verdict: `rewrite`

---

## Current Parity Summary

| Topic | Current state | Notes |
|---|---|---|
| `AgentPool` / `MultiAgentPool` | Shipping | real primitives in `roko-agent` |
| MCP config + discovery | Shipping | explicit config plus discovery fallback are both live |
| Per-agent sidecar | Shipping | `roko-agent-server` is a real HTTP sidecar, not a placeholder |
| `PlanRunner` lifecycle ownership | Shipping | `PlanRunner` owns `ProcessSupervisor` |
| Pool-led runtime ownership | Partial | pools are not the main plan-execution story today |

---

## What Is Definitely Live

### `PlanRunner` owns lifecycle supervision

- `PlanRunner` is defined in `crates/roko-cli/src/orchestrate.rs:2567`.
- It owns `Arc<ProcessSupervisor>` in the same struct.
- `ProcessSupervisor` itself is live in `crates/roko-runtime/src/process.rs:374`.

This is enough to say lifecycle supervision is wired through the CLI runtime.

Important narrowing:

- the docs should not imply `ProcessSupervisor` is the primary agent-construction seam
- actual agent creation still flows through scoped spawn helpers and `create_agent_for_model()`
- the supervisor is the lifecycle/shutdown owner, not the main provider-dispatch abstraction

### MCP passthrough is already a live runtime path

There are two concrete MCP paths worth documenting:

1. `PlanRunner::setup_mcp` builds dynamic tool state from stdio MCP servers.
2. `PlanRunner::resolve_mcp_config_path` supports runtime passthrough for provider and CLI paths.

The concrete handoff chain is visible:

- CLI config exposes `agent.mcp_config`
- spawn specs carry `mcp_config`
- provider-backed creation paths consume it
- Claude CLI forwards `--mcp-config`
- non-Claude hosted paths can merge discovered MCP tools into a registry

Smoke coverage exists in `crates/roko-cli/tests/smoke.rs:204`.

### `roko-agent-server` is a real sidecar

The parity pack should stop treating the sidecar as merely planned.

Live sidecar surfaces:

- `AgentServer` at `crates/roko-agent-server/src/lib.rs:52`
- `AgentState` at `crates/roko-agent-server/src/state.rs:447`
- messaging route tests at `crates/roko-agent-server/src/features/messaging.rs:390` and `:480`
- relay registration tests at `crates/roko-agent-server/tests/relay_registration.rs:158` and `:225`

This is strong enough evidence to describe the per-agent HTTP sidecar as wired.

---

## What Needs Narrow Wording

### Pools exist, but they are not the main runtime story

`AgentPool` and `MultiAgentPool` are real and should stay in the docs.

What the parity copy should avoid:

- implying `orchestrate.rs` is pool-driven today
- framing pool adoption as the main lifecycle gap for `02-agents`

That is a follow-on orchestration concern, not the core parity story here.

### Creation-site cleanup should be described as “mostly centralized”

The docs can say:

- scoped spawn helpers are the main CLI path
- some direct or specialty entrypoints still exist

They should not say:

- creation-site consolidation is fully complete
- or that batch `02` needs a large cleanup program to make the stack viable

---

## Recommended Refresh Language

- Keep: pools, MCP discovery, handler resolution, and creation-site consolidation as real topics.
- Rewrite: lifecycle ownership around `PlanRunner` + `ProcessSupervisor`.
- Rewrite: sidecar sections so they reflect the current HTTP server and test coverage.
- Narrow: pool adoption and creation-site cleanup to the remaining concrete seams.

---

## Verification Anchors

```bash
rg -n "pub struct AgentPool|pub struct MultiAgentPool" crates/roko-agent/src/pool.rs crates/roko-agent/src/multi_pool.rs
rg -n "pub fn find_mcp_config" crates/roko-agent/src/mcp/config.rs
rg -n "mcp_config|setup_mcp|resolve_mcp_config_path" crates/roko-cli/src/config.rs crates/roko-cli/src/orchestrate.rs crates/roko-cli/tests/smoke.rs
rg -n "pub struct AgentServer|pub struct AgentState" crates/roko-agent-server/src/lib.rs crates/roko-agent-server/src/state.rs
rg -n "message_with_mock_dispatcher_returns_real_content|stream_with_mock_dispatcher_streams_chunks" crates/roko-agent-server/src/features/messaging.rs
rg -n "wallet_free_relay_registration_hosts_card_and_keeps_direct_routes_working|wallet_backed_relay_registration_submits_target_abi_with_relay_card_uri" crates/roko-agent-server/tests/relay_registration.rs
```
