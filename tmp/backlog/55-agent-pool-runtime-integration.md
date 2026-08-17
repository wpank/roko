# AgentPool Runtime Integration

**Priority**: P2 — built with 56 tests but zero runtime callers
**Size**: M (2-3d)
**Crate**: `crates/roko-agent/`, `crates/roko-cli/`

---

## Problem

`AgentPool` (676 lines, 24 tests) and `MultiAgentPool` (1130 lines, 32 tests) provide
per-role sequential execution with automatic fallback retry, parallel multi-role dispatch
with configurable concurrency limits, warm pool pre-spawning, checked reuse policies with
context fingerprinting, and full lifecycle tracking (Warm/Pending/Active/Done/Failed/Cancelled).

Neither is instantiated anywhere in the runtime. Runner-v2 creates a fresh provider instance
per task via `factory.spawn_shared_agent_bridge()`. The existing `WarmPool` in
`dispatch/warm_pool.rs` is a simpler structure that stores `(role, agent)` tuples without
lifecycle tracking, concurrency limits, or fallback retry.

This means:
- Every task pays provider setup overhead (subprocess launch for CLI providers)
- No concurrency limits per role (relies on external semaphores)
- No automatic fallback retry at the pool level
- No warm agent reuse with context fingerprint validation

---

## What already exists

| Component | File | Status |
|---|---|---|
| `AgentPool` | `roko-agent/src/pool.rs` | Built, 24 tests, 0 callers |
| `MultiAgentPool` | `roko-agent/src/multi_pool.rs` | Built, 32 tests, 0 callers |
| `WarmPool` (simpler) | `roko-cli/src/dispatch/warm_pool.rs` | Wired in runner |
| TUI agent pool modal | `roko-cli/src/tui/modals/agent_pool_modal.rs` | Display-only |
| Per-task dispatch | `runner/event_loop.rs` | `spawn_shared_agent_bridge()` |
| Provider semaphores | `roko-agent/src/provider/mod.rs` | Reused across tasks |
| MCP runtime | `roko-agent/src/mcp/` | Reused across tasks |

### What the runner currently reuses (no pool needed):
- `ProviderSemaphores` (concurrency per provider)
- MCP runtime (tool definitions, clients)
- `Dispatcher` (model router, prompt assembler)
- `ProviderRateLimiter` (shared RPM/TPM budget)
- `ProviderHealthRegistry` (provider health state)

### What the runner rebuilds per task (pool would help):
- The actual provider instance (ClaudeCliAgent, OpenAiAgent, etc.)
- Subprocess launching (for CLI providers)
- Provider options + configuration

---

## What to do

**Step 1.** Replace the simple `WarmPool` with `MultiAgentPool` in
`crates/roko-cli/src/dispatch/factory.rs`. Initialize it with the configured roles and
concurrency limits from `roko.toml`.

**Step 2.** In the runner's task dispatch path (`event_loop.rs` around
`spawn_shared_agent_bridge`), use `MultiAgentPool::run_task_with_auto_activation()` instead
of creating a fresh provider per task. This gives:
- Warm agent promotion (no cold start)
- Automatic fallback retry on primary failure
- Per-role concurrency enforcement

**Step 3.** At phase boundaries (e.g., implementation → review), use
`MultiAgentPool::pre_spawn_warm()` to prepare agents for the next role while the current
task's gate is running.

**Step 4.** Wire `MultiAgentPool` status into the TUI agent pool modal so it shows live
pool state rather than just agent metadata.

**Step 5.** On plan completion or cancellation, call `MultiAgentPool::kill_plan_agents()`
for cleanup.

---

## Acceptance criteria

- [ ] `MultiAgentPool` instantiated in the runner factory
- [ ] Task dispatch uses pool instead of per-task provider creation
- [ ] Warm pre-spawning at phase boundaries reduces cold-start latency
- [ ] Per-role concurrency limits enforced by the pool
- [ ] Automatic fallback retry on primary agent failure
- [ ] TUI modal shows live pool state (warm/active/done counts)
- [ ] `kill_plan_agents()` called on plan completion/cancellation
- [ ] All existing tests pass (`cargo test -p roko-agent -p roko-cli`)

---

**Origin**: GAPS.md "Built-but-Unwired: AgentPool / MultiAgentPool" (2026-08-17)
