+++
id = "spec-a0403b"
kind = "spec"
title = "Graph Engine Watchdog Integration"
status = "open"
triage = "verified"
severity = "p0"
size = "L"
goal = "core"
subsystem = ["roko-cli"]
created = 2026-09-21
updated = 2026-09-29
last_verified = 2026-09-30
last_verified_rev = "a17d9d766"
source = "tmp/backlog/archive/401-graph-engine-watchdog-integration.md#401 — Graph Engine Watchdog Integration"
discovered_from = "audit:tmp/backlog/archive/401-graph-engine-watchdog-integration.md#401 — Graph Engine Watchdog Integration"
anchors = ["crates/roko-cli/src/graph_task_dispatch.rs::GraphTaskDispatcher", "crates/roko-cli/src/graph_task_dispatch/streaming.rs::dispatch_streaming", "crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan", "crates/roko-cli/src/runner/conductor_adapter.rs::ConductorRing", "crates/roko-cli/src/runner/types.rs::RunConfig::from_roko_config", "crates/roko-cli/src/graph_execution/control_adapter.rs::GraphExecutionControlAdapter", "crates/roko-graph/src/cells/task_executor.rs::GraphTaskEvent", "crates/roko-core/src/config/schema.rs::ConductorConfig"]
links = { depends_on = [], blocks = [], related = ["gap-ebd656", "gap-fab31c"], supersedes = [], duplicate_of = "" }

[[verify]]
command = "grep -rqw 'fn graph_watchdog_intervenes_on_stalled_task' crates/roko-cli/ && cargo test -p roko-cli graph_watchdog_intervenes_on_stalled_task"
+++

## Problem

During `roko plan run` (Graph engine, the only executor) nothing supervises an agent while it runs. The only hang protection is the hard deadline: `effective_timeout_secs` = `spec.timeout_secs`, or `config.timeouts.agent_dispatch_secs` when that is 0 (`graph_task_dispatch.rs:3631` and `:4277`). So:

- A provider that hangs at the TCP level, or an agent that goes silent, holds its task slot until the full deadline (often tens of minutes), then is retried.
- An agent that keeps producing output but makes no progress (empty turns, the same compile error each iteration, runaway cost within budget) is never detected. The `roko-conductor` watchers for these cases (GhostTurn, IterationLoop, CompileFailRepeat, TestFailureBudget, CostOverrun, TimeOverrun, StuckPattern, and others) receive no signals from Graph runs.
- `[conductor] silence_timeout_secs` (default 180) and `task_stall_secs` (default 300) exist in config (`crates/roko-core/src/config/schema.rs:1714-1731`), but only the TUI conductor panel reads them.

Expected:

- A task with no progress for `task_stall_secs` is cancelled and retried under its normal `max_retries`, with a visible diagnosis.
- Conductor watchers are evaluated on a ticker during the run. A `Restart` decision cancels and retries the in-flight task; a `Fail` decision aborts the plan with an error naming the watcher and the reason.

## Why it matters

- Goal `core` (plan runs work reliably), severity p0. A plan run has no safety net beyond raw timeouts, so one stuck provider stalls a whole wave, and cost overrun is caught only by the hard budget stop.
- It is a prerequisite for unattended self-hosting runs.
- Related:
  - `gap-ebd656`: graduated escalation (Nudge, ForceAdvance) on top of this loop. It depends on this item; its verify greps for `Conductor::from_config`/`evaluate_full` in the Graph files;
  - parked `gap-fab31c`: the old Runner-v2 supervisor tick only logged. That code is gone.

## Where

- `crates/roko-cli/src/graph_task_dispatch.rs`:
  - `GraphTaskDispatcher` (:1112) has no conductor fields today;
  - `TaskDispatcher::dispatch` (:3176). Its `tokio::select!` loop (:3736-3758) races `run_bridge_with_failover` against a 5 s `AGENT_HEARTBEAT_INTERVAL` tick that only calls `TuiBridge::agent_heartbeat` for the TUI elapsed-time counter (added in `5c62bf0d4`). Live agent events reach it only when a TUI bridge and live output are set (:3703-3725);
  - `StreamingTaskDispatcher::dispatch_streaming` (:4128) sends `GraphTaskEvent`s on `event_tx`: `AttemptStarted` (:4178), `Text`/`ToolCall`/`ToolOutput`/`Usage` (:4355-4382, :4405) and `AttemptTerminal` (:4525, :4542).
- `crates/roko-graph/src/cells/task_executor.rs::GraphTaskEvent` (:263): variants are `Text`, `ToolCall`, `ToolOutput`, `Usage { input_tokens, output_tokens, cost_usd }`, `Progress`, `AttemptStarted`, and `AttemptTerminal { outcome: Succeeded | Failed | Cancelled | TimedOut }`. The original spec assumed `TaskStart`/`GateResult`/`CostUpdate` variants; they do not exist.
- `crates/roko-cli/src/runner/conductor_adapter.rs`:
  - `ConductorRing` (:265): bounded ring with `push`, drop-oldest and `snapshot`;
  - `runner_event_to_signal` (:51), `agent_event_to_signal` (:188) and `feedback_event_to_signal` (:357): the existing mappers to copy;
  - `ConductorRingSink` (:456) and `compute_conductor_load` (:510).
  - No `GraphTaskEvent` mapper exists.
- `crates/roko-cli/src/runner/types.rs::RunConfig::from_roko_config` (:2598): builds `Conductor::from_config(&roko_config.conductor)` and a `ConductorRing` into `RunConfig.conductor`/`conductor_ring` (:2480). `plan_runner.rs:856` calls it for Graph runs (`graph_run_config`), but neither value is passed on. They are built and dropped.
- `crates/roko-cli/src/graph_execution/plan_runner.rs::run_graph_plan`: the Graph host that builds `SharedAgentFactory` (:868) and the dispatcher. This is where to spawn a ticker.
- `crates/roko-cli/src/graph_execution/control_adapter.rs::GraphExecutionControlAdapter` (:86): owns the graph `CancellationToken`, pause/cancel flags and process-supervisor shutdown. It is the existing way to abort a whole run, as `roko plan cancel` does.
- `crates/roko-cli/src/graph_execution/feedback.rs::ConductorSink` (:602): the post-settlement feedback sink for intervention-policy learning. This is the only conductor hook on the Graph path today, and it runs after a task finishes.
- `crates/roko-conductor/src/conductor.rs`: `Conductor::from_config` (:240), `evaluate_full(&[Signal], &Context) -> ConductorEvaluation` (:371) and `record_intervention_outcome` (:341).
- `crates/roko-core/src/dashboard_snapshot.rs`: `DashboardEvent::Diagnosis { summary: DiagnosisSummary }` (:192), with `severity`, `subject`, `detail`, `suggested_action` and `intervention_taken`. This is the existing way to make an intervention visible in the TUI, serve SSE and the `--log-file` JSONL (as `dashboard.diagnosis`).

## Current state

- Still open at HEAD. No conductor ring feed, supervision ticker or stall watchdog exists anywhere in `graph_task_dispatch.rs` or `graph_execution/`. The only conductor references are `conductor.express_mode` (:688-696), `max_parallel_plans` and `max_agents`.
- Since `725f21e05` the Graph path enforces per-tier turn caps and per-task USD caps (`budget.max_task_usd`, `max_task_retry_usd`) alongside `timeout_secs`. Those are hard limits, not stall detection.
- The Runner-v2 `conductor_supervision_tick` that the original spec wanted to port was deleted with Runner-v2 on 2026-09-06 (`runner/graph_conductor.rs` and `event_loop.rs` no longer exist).
- Unknown: which of `dispatch` or `dispatch_streaming` the task-executor cell uses by default in production. Cover both.

## Plan

Two parts. Part A is the P0 safety net and satisfies the verify command. Part B adds the semantic watchers. Do A first, and land B in the same item if time allows; otherwise split B out.

**A. Per-attempt stall watchdog, in the dispatcher**

1. Track progress with an `Arc<AtomicU64>` "last progress" timestamp per attempt:
   - in `dispatch_streaming`, put a small forwarder between the provider events and `event_tx` that updates the timestamp on every `Text`/`ToolCall`/`ToolOutput`/`Usage`/`Progress` event;
   - in `dispatch`, always create the live-output channel, even without a TUI, and update the timestamp in its receiver. Keep forwarding to the TUI when it is set.
2. In both select loops, reuse the heartbeat tick as the check:
   - after `silence_timeout_secs` with no progress, publish a `DashboardEvent::Diagnosis` (severity warning, subject `plan/task`) through the `TuiBridge`/StateHub;
   - after `task_stall_secs`, drop the dispatch future (which cancels the provider call), release the worktree lease as the error path does (:3760-3775), and return a failed attempt whose error/outcome says "stalled" (map it to `TaskDispatchOutcomeKind::TimedOut`), so the engine's normal `max_retries` handling retries it;
   - publish a Diagnosis with `intervention_taken = "cancelled stalled attempt"`.
3. Read both thresholds from `self.config.conductor`. `0` disables the watchdog. Keep the hard `timeout_secs` as the outer bound.
4. Add a test `graph_watchdog_intervenes_on_stalled_task`. Use a fake provider or bridge that emits one event and then sleeps, with `task_stall_secs = 1`, and assert that the attempt ends as stalled or timed out well before `timeout_secs` and that a Diagnosis was published. Use a paused tokio clock (`tokio::time::pause`) if the dispatcher's timers allow it.

**B. Conductor ring and supervision ticker, at the host**

5. Add `graph_task_event_to_signal(plan_id, task_id, &GraphTaskEvent) -> Option<Signal>` in `conductor_adapter.rs`, modelled on `runner_event_to_signal`:
   - `Usage { cost_usd }` becomes a cost metric;
   - empty or repeated `Text` becomes a ghost-turn candidate;
   - `AttemptStarted` becomes a dispatch plan-phase;
   - `AttemptTerminal` becomes a terminal outcome.
   Gate verdicts are not `GraphTaskEvent`s. Feed them from the gate settlement path, where `TaskGateVerdict` is produced, using the same signal kinds as `feedback_event_to_signal`.
6. Add `with_conductor(Arc<Conductor>, ConductorRing)` to `GraphTaskDispatcher` and push mapped signals from the forwarder in step 1. `ConductorRing` is `Clone` over an `Arc<Mutex<VecDeque<Signal>>>` (`conductor_adapter.rs:264-268`), so clones share one buffer. Never hold its lock across `.await`.
7. In `run_graph_plan`, take `graph_run_config.conductor`/`conductor_ring` and pass them to the dispatcher. Spawn a ticker task that every N seconds calls `conductor.evaluate_full(&ring.snapshot(), &Context::now())`. There is no `supervision_interval_secs` in `ConductorConfig`, so add one (default 5) or reuse the 5 s heartbeat constant. Stop the ticker with the run's cancellation token. On a decision:
   - `Continue`: nothing;
   - `Restart`: cancel the in-flight attempt(s) named by the evaluation. Reuse the part A cancel path, for example through a per-task `CancellationToken` map in the dispatcher, and let `max_retries` retry;
   - `Fail`: abort the run through `GraphExecutionControlAdapter` (the same path as `roko plan cancel`) and make `run_graph_plan` return an error naming the watcher and reason.
   - Publish each non-`Continue` decision as `DashboardEvent::Diagnosis`, and call `record_intervention_outcome` when the task's outcome is known.
8. Tests: a ring and ticker test with a fake `Conductor` evaluation that returns `Fail`, asserting the run aborts with the watcher named. A mapping unit test for `graph_task_event_to_signal`.

## Done when

- A Graph task whose agent goes silent for `task_stall_secs` is cancelled and retried before `timeout_secs`, and a `dashboard.diagnosis` line appears in the `--log-file` JSONL and the TUI.
- (Part B) During a Graph run, the conductor is evaluated on an interval. `Restart` retries the task and `Fail` aborts the plan with the watcher and reason in the error.
- Nothing changes when both thresholds are 0.
- Verify: `grep -rqw 'fn graph_watchdog_intervenes_on_stalled_task' crates/roko-cli/ && cargo test -p roko-cli graph_watchdog_intervenes_on_stalled_task`

## Notes

- Hot path: `graph_task_dispatch.rs` is large (7,000+ lines) and many items touch it (provider failover, budget, live output, reflexes). Do not run this in parallel with other items anchored there.
- Cancelling a dispatch must release worktree leases and settle budget reservations exactly as the existing error path does. Do not leak a lease or double-count spend on a stalled attempt.
- Nudge and ForceAdvance, the TUI conductor panel and an HTTP `/conductor` route are out of scope: `gap-ebd656` and later items.
- Do not reintroduce `RunnerEvent` for this. Graph runs surface state through StateHub `DashboardEvent`s.
- Size L overall. Part A alone is about M.
- Implemented on `work/spec-a0403b` at `4e4572802` (part A `21f83c618`, part B `4e4572802`); cargo verification deferred to the batch check. Locally, `graph_watchdog_intervenes_on_stalled_task` and the `graph_task_dispatch::`, `graph_execution::` and `runner::conductor_adapter` lib suites pass, and clippy `-D warnings` is clean. Silence counts only after an attempt's first live event (the Codex CLI reports nothing until it finishes) and pauses while a tool call runs, so a long `cargo test` is not killed. With both thresholds 0 there is no watchdog, conductor or ticker. Part B feeds only live messages and tool calls, so in practice only the ghost-turn watcher can fire; cost and gate signals arrive after the provider call and are not evaluated. Follow-ups: bug-739dcc (Claude CLI has no cancel path), bug-aa2044 (stalled attempts record no cost).

## Original notes

the Graph engine is the sole production executor since PR #260 and has zero behavioral safety net beyond raw `timeout_secs`. PR #260 made the Graph engine the default and sole production executor for plan execution (`roko plan run`). PR #276 retired `WorkflowEngine`. The Runner-v2 event loop is…

Imported without verification from:
- `tmp/backlog/archive/401-graph-engine-watchdog-integration.md#401 — Graph Engine Watchdog Integration`
- `tmp/archive/plan-audit-2026-09-23/03-ACTIONABLE-TASKS.md#T0-05: Implement progress-heartbeat watchdog`
- `tmp/archive/plan-audit-2026-09-23/09-safeguards-watchdog.md`

Some cited files are gone: `crates/roko-cli/src/runner/graph_conductor.rs`, `runner/graph_conductor.rs`.

How to verify: Check: `GraphTaskDispatcher::dispatch()` and `dispatch_streaming()` push mapped signals to the conductor ring for each `GraphTaskEvent`.; The supervision ticker runs alongside graph plan execution at the configured… [evidence: no status line; no index/roll-up evidence] / grep graph_task_dispatch/graph_execution for conductor/watchdog/heartbeat.

Merged 2 mined candidates: m1-124, m3-113.

Verified 2026-09-28: still true - graph_task_dispatch.rs and graph_execution/ contain no conductor ring feed, supervision ticker or progress watchdog (only conductor.express_mode at graph_task_dispatch.rs:669 and the post-settlement ConductorSink at graph_execution/feedback.rs:602); ConductorAdapter is referenced only from runner/mod.rs, runner/types.rs, commands/do_cmd.rs (runner::run stub path) and runtime_feedback/routing.rs.

Re-verified 2026-09-29: still open. The Graph dispatcher's 5 s agent_heartbeat (graph_task_dispatch.rs:3733-3758, added in 5c62bf0d4) only updates the TUI elapsed-time counter. It does not detect stalls, feed the conductor ring or run a supervision ticker. Since 725f21e05 the Graph path does enforce per-tier turn caps and per-task USD caps alongside timeout_secs.
