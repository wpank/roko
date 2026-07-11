# Event Loop Issues

Investigation of `crates/roko-cli/src/runner/event_loop.rs` — the core execution engine.

## Critical

### Blocking sync I/O inside the async event loop
- `signals.jsonl`, `run-ledger.jsonl`, and `persist_run_ledger` use `std::fs::OpenOptions` (blocking) directly from the async event loop without `spawn_blocking`. Under I/O contention this stalls the entire `tokio::select!` iteration. Lines 1418-1436, 6418-6428, 6453-6463.
- `append_agent_event` (line 2907) has the same problem on every agent event.
- `git_diff_entries_since_task_start` runs two synchronous `std::process::Command` git invocations inline (lines 6479-6535).
- `commit_task_changes` runs `git status`, `git add`, `git commit` as blocking `Command::status()` calls (lines 7032-7071).

### Fabricated metrics in reports and feedback
- `build_report` (lines 7101-7138): `tasks_completed` always set to total count; `tasks_failed` is 0 or 1 regardless of actuals; `task_costs` unconditionally empty.
- `FeedbackEvent::PlanCompleted` (lines 3648-3651): hard-codes `tasks_completed: 0`, `tasks_failed: 0`, `total_cost_usd: 0.0`. Bandit models trained on garbage.
- `tool_call_count` hardcoded to 0 in learning events (line 2949).
- `RuntimeEvent::AgentCompleted` always emits `tokens_used: 0` (line 3214).
- `TaskStarted` runtime event hard-codes `role: String::new()` (line 3274).

### Logic errors in state machine
- `ExecutorAction::RunGate` ignores the `rung` parameter and substitutes `ctx.config.max_gate_rung` (lines 5248-5274). Multi-rung pipelines always run the final rung.
- `split_into` task decomposition is populated but never executed (lines 6916-6922). No code reads `split_into` to create real tasks.
- `FailPlan` action double-counts a failed task (lines 5544-5564).

## High

### Concurrency issues
- `emit_runner_event_with_facades` fires `tokio::spawn` without tracking JoinSet (lines 3136-3147). Tasks abandoned on exit.
- Bridge spawn-failure path never clears DAG entry (lines 5175-5244). Task stays permanently stuck as "running".
- Double timeout check at lines 2213-2228 — dead code that could cause double-shutdown.

### Unbounded growth
- `section_diagnostics` and `task_playbook_ids` never pruned on retry failure (lines 849-859).

## Medium — Hardcoded values

- `CARGO_BUILD_JOBS: "2"` in Bridge dispatch (line 5193)
- Worktree idle TTL 30 min (line 198) — not configurable
- Agent stop grace period 3s (lines 2202, 6039)
- Gate output truncation: 3000/2000 bytes (lines 6676-6685) — not configurable
- Dream consolidation uses hardcoded `"claude"` command (lines 6122-6127)

## Low

- `chrono::Utc::now()` called twice for same timestamp (lines 2896-2897)
- `complete_plan_after_successful_verify` can leave plan in limbo (lines 2593-2641)
