# 55 — AgentPool Runtime Integration

**Priority**: P2 — `AgentPool` and `MultiAgentPool` have 56 tests but zero runtime callers; every task pays cold-start overhead and has no pool-level fallback retry
**Size**: M (2-3 days)
**Crates**: `crates/roko-agent/` (`roko-agent`), `crates/roko-cli/` (`roko-cli`)
**Depends on**: None

---

## Background

Roko's plan runner dispatches one agent per task. Creating an agent can be expensive: for subprocess-based providers like `ClaudeCliAgent`, this involves forking a new process, establishing MCP connections, and waiting for the provider to initialize. For API-based providers, it involves constructing a new request context and waiting for the first token.

The codebase has two fully-built pool types that address this: `AgentPool` (in `crates/roko-agent/src/pool.rs`) handles a single role with sequential execution, fallback retry, and lifecycle tracking. `MultiAgentPool` (in `crates/roko-agent/src/multi_pool.rs`) coordinates multiple `AgentPool` instances for concurrent execution with warm pre-spawning, per-role concurrency limits, and bulk cleanup.

Neither pool is called anywhere in the runner. The runner creates a fresh provider instance per task via `SharedAgentFactory::spawn_shared_agent_bridge()` in `crates/roko-cli/src/dispatch/factory.rs`. There is a simpler warm-pool type (`WarmPool` in `crates/roko-cli/src/dispatch/warm_pool.rs`) that stores `(model, agent_handle)` tuples for fast role transitions, but it has no lifecycle tracking, no fallback retry, and no per-role concurrency enforcement.

The TUI has an "Agent Pool" modal (`ModalState::AgentPool` in `crates/roko-cli/src/tui/modals/mod.rs`, line 86) that displays agent rows, but the `agents: Vec<AgentPoolRow>` field is populated with empty metadata because there is no live pool state to read.

---

## Current State

1. `AgentPool` is at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/pool.rs` (675 lines, 24 tests). It manages a queue of `AgentTask` values for a single role, with a primary agent and optional fallback. The `InstanceStatus` enum (line 75) has variants `Warm`, `Pending`, `Active`, `Done`, `Failed`, and `Cancelled`.

2. `MultiAgentPool` is at `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/multi_pool.rs` (1129 lines, 32 tests). Key methods:
   - `pre_spawn_warm(role, count, agent_fn)` at line 110 — fills warm slots for a role.
   - `run_task_with_auto_activation(task, agent_fn)` at line 456 — promotes a warm agent or creates one, then runs the task.
   - `kill_plan_agents(plan_id)` at line 583 — kills all agents whose instance ID contains `plan_id`.
   - `kill_all(deadline)` at line 541 — terminates everything.

3. `SharedAgentFactory` is at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/factory.rs`. The `spawn_shared_agent_bridge()` method at line 317 is the current per-task provider creation path. It spawns a Tokio task that creates a fresh `AgentDispatcherV2`, calls `run_agent_result_bridge_with_tools_and_cli_mcp()`, and returns a `JoinHandle`.

4. The existing `WarmPool` is at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/warm_pool.rs` (250 lines). It is a bounded LRU container of `WarmAgent` structs (identified by `id` and `model` strings). It is wired into `SharedAgentFactory::dispatcher()` and used in the runner's event loop (e.g., at line 4128 of `event_loop.rs`) for phase-boundary warm promotion. It does not interface with `MultiAgentPool`.

5. The TUI agent pool modal is defined in `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/modals/agent_pool_modal.rs`. `AgentPoolRow` (line 12) has fields `role`, `model`, `task`, `tokens`, `cost_usd`, `state`, and `context_pct`. The `ModalState::AgentPool { agents: Vec<AgentPoolRow>, scroll_offset: u16 }` variant is at line 86 of `modals/mod.rs`. Currently the `agents` field is populated with statically constructed rows (e.g., in tests at line 1176 of `input.rs`) rather than live pool data.

6. `WarmReusePolicy` and `WarmReuseRequest` are in `/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/session.rs`. `WarmReusePolicy::allows()` at line 133 checks TTL and optional `context_fingerprint` matching.

---

## Implementation Plan

### Step 1: Add `MultiAgentPool` to `SharedAgentFactory`

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/factory.rs`:

1. Add a field `agent_pool: MultiAgentPool` to `SharedAgentFactory` (line 43).
2. Initialize it in `SharedAgentFactory::new()` with default concurrency limits (e.g., 4 concurrent implementers, 2 reviewers). Read per-role limits from `RokoConfig` if present.
3. Add a `pub fn agent_pool_mut(&mut self) -> &mut MultiAgentPool` accessor for the runner to use.
4. Add a `pub fn agent_pool_snapshot(&self) -> Vec<AgentPoolRow>` method that iterates the pool's active and warm entries and maps them to `AgentPoolRow` structs for the TUI.

`MultiAgentPool` requires a mutable reference for `run_task_with_auto_activation` and `pre_spawn_warm`, so `SharedAgentFactory` must be held behind a `Mutex` or the methods must take `&mut self`. In the runner, `SharedAgentFactory` is already behind an `Arc<Mutex<...>>` pattern — verify this in `event_loop.rs` before proceeding.

### Step 2: Replace per-task provider creation with pool dispatch

In `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`, find the `spawn_shared_agent_bridge()` call site (around line 10544). Replace it with a call to `factory.agent_pool_mut().run_task_with_auto_activation(task, &agent_fn)` where:

- `task` is an `AgentTask` constructed from the current runner task's role, plan ID, and task ID.
- `agent_fn` is a closure that calls the existing `spawn_shared_agent_bridge()` path when no warm agent is available.

This change preserves the existing dispatch path as the cold-start fallback while enabling warm agent reuse when one is available.

### Step 3: Pre-spawn at phase boundaries

In the runner's event loop, at the point where a gate has just passed and the next phase's role is known (around line 10801 of `event_loop.rs`, where the existing `WarmPool` pre-spawn is triggered), add a call to `factory.agent_pool_mut().pre_spawn_warm(next_role, 1, &agent_fn)`. This fills a warm slot for the next phase role while the current gate runs.

This replaces the manual `WarmPool::insert()` + `warm_pool.take()` pattern with the lifecycle-aware `MultiAgentPool` version.

### Step 4: Wire cleanup on plan completion and cancellation

In the runner's plan completion and cancellation paths, call `factory.agent_pool_mut().kill_plan_agents(plan_id)` to release all agents associated with the plan. In the existing code, this is the point where `ProcessSupervisor` teardown happens — add the pool cleanup in the same block.

### Step 5: Feed live pool state into the TUI

In the runner's state publication loop (the code that pushes `RunnerEvent` to the TUI via the state hub), after each task dispatch or completion, call `factory.agent_pool_snapshot()` and publish the result as a `TuiStateUpdate::AgentPool(Vec<AgentPoolRow>)` event. In `crates/roko-cli/src/tui/app.rs`, handle this event by updating `tui_state.active_modal` when `ModalState::AgentPool` is open.

---

## Acceptance Criteria

1. `SharedAgentFactory` holds a `MultiAgentPool` instance initialized at plan run start.
2. The runner's task dispatch path calls `run_task_with_auto_activation()` instead of directly calling `spawn_shared_agent_bridge()` for each task. The cold-start path is preserved as the fallback when no warm agent exists.
3. After each gate pass, `pre_spawn_warm()` is called for the next phase's role.
4. On plan completion or cancellation, `kill_plan_agents()` is called for the finished plan.
5. The TUI's Agent Pool modal (`'p'` key or equivalent shortcut) shows live state: warm slot count, active agent count, and per-agent task/model/status.
6. `cargo test -p roko-agent -p roko-cli` passes with zero failures.
7. `cargo clippy --workspace --no-deps -- -D warnings` is clean.

---

## Verification Checklist

- [ ] Run a 3-task plan. In the TUI, open the Agent Pool modal and confirm it shows agents with non-empty `state` fields during execution.
- [ ] Check runner logs for `"warm_pool: promoted pre-spawned agent"` messages — confirm they appear after the first gate pass.
- [ ] After plan completion, confirm no orphaned subprocess entries remain in the pool.
- [ ] Run `cargo test -p roko-agent` — all 56 pool tests pass.
- [ ] Run `cargo test -p roko-cli` — all tests pass.
- [ ] Run `cargo clippy --workspace --no-deps -- -D warnings` — clean.

---

## Files to Modify

| File | Change |
|---|---|
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/factory.rs` | Add `MultiAgentPool` field to `SharedAgentFactory`; initialize in `new()`; add `agent_pool_mut()` and `agent_pool_snapshot()` accessors |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` | Replace `spawn_shared_agent_bridge()` with `run_task_with_auto_activation()`; add `pre_spawn_warm()` at gate-pass boundaries; add `kill_plan_agents()` at plan completion/cancellation |
| `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/tui/app.rs` | Handle pool snapshot events to update `ModalState::AgentPool` |
