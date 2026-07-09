# 04 — Orchestrator → Agent Pipeline

> **Priority**: 🔴 P0 — Orchestrator emits actions but nothing dispatches them
> **Parity sections**: §14 (Plan execution), I.2 (Orchestrator wiring)
> **Checklist ref**: `MORI-PARITY-CHECKLIST.md` §14, I.2

## Problem statement

Roko's orchestrator (`/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/`) is a pure state
machine that emits `ExecutorAction` variants (DispatchAgent, RunGate, MergeBranch, etc.) but the
**runtime harness that dispatches these actions to actual agents doesn't exist**.

The executor at `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/executor/` has:
- `ExecutorAction::DispatchAgent { plan_id, task_id, role }` — but nothing spawns an agent
- `ExecutorAction::RunGate { plan_id, task_id }` — but nothing runs a gate
- `ExecutorAction::MergeBranch { plan_id }` — but nothing does git merge
- DAG traversal, dependency resolution, parallel scheduling — all implemented
- Progress tracking, event log — all implemented

Meanwhile `roko-cli/src/run.rs` bypasses the orchestrator entirely and just does:
```rust
let agent = ExecAgent::new(&config.agent.command, config.agent.args.clone());
agent.run(&prompt_signal).await
```

## Checklist

### Phase A: Runtime harness

- [ ] **4.1** Create `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/harness.rs` — the runtime loop that:
  1. Calls `executor.tick()` to get `Vec<ExecutorAction>`
  2. Dispatches each action to the appropriate subsystem
  3. Feeds results back as events
- [ ] **4.2** `DispatchAgent` → spawn ClaudeCliAgent (or appropriate backend) with role-specific config
- [ ] **4.3** `RunGate` → invoke gate runner (roko-gate crate)
- [ ] **4.4** `MergeBranch` → git merge worktree into plan branch
- [ ] **4.5** `StartPlan` / `PausePlan` / `ResumePlan` → plan lifecycle management
- [ ] **4.6** `ReorderQueue` → priority queue management

### Phase B: Agent pool integration

- [ ] **4.7** Wire `AgentPool` (`/Users/will/dev/nunchi/roko/roko/crates/roko-agent/src/pool.rs`) into harness
- [ ] **4.8** `MultiPool` (`multi_pool.rs`) for multi-backend agent pool
- [ ] **4.9** Pool respects `config.conductor.max_agents` limit
- [ ] **4.10** Pool respects `config.conductor.max_parallel_plans` limit

### Phase C: Gate pipeline integration

- [ ] **4.11** Wire gate runner to execute 6-rung gate system
- [ ] **4.12** Gate results feed back as `GateResult` events to executor
- [ ] **4.13** Failed gates trigger conductor intervention logic

### Phase D: Git/worktree integration

- [ ] **4.14** Worktree creation per task (mori-ref: `apps/mori/src/git/worktree.rs`)
- [ ] **4.15** Branch merge on task completion
- [ ] **4.16** Conflict resolution / conductor escalation

### Phase E: CLI integration

- [ ] **4.17** New `roko orchestrate` subcommand that uses the full harness (vs `roko run` which is single-shot)
- [ ] **4.18** Or: upgrade `roko run` to optionally use orchestrator when plans/ directory exists

> Maps to checklist: I.2.1 through I.2.9
