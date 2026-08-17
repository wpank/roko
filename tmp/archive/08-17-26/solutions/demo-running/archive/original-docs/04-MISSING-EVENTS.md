# Catalog: Unwired ServerEvent Variants

## Overview

`crates/roko-serve/src/events.rs` defines 50+ `ServerEvent` variants.
The frontend `demo/demo-app/src/transport/types.ts` consumes all of them.
Only ~20 are actually emitted at runtime.

This document catalogs which variants are never emitted and where they should be.

## Emission Matrix

### Currently Emitted (working)

| Variant | Emitted From | Trigger |
|---------|-------------|---------|
| PlanStarted | routes/plans.rs:219, orchestrate.rs:7655 | Plan execution begins |
| PlanCompleted | routes/plans.rs:235, orchestrate.rs:7858 | Plan execution ends |
| Error | routes/plans.rs:229, routes/templates.rs:187 | Various errors |
| OperationCompleted | routes/plans.rs:321,393, routes/templates.rs:169 | Generic op done |
| AgentOutput | routes/agents.rs:1274+, routes/run.rs:298+, dispatch.rs:1749 | Agent produces text |
| GateResult | dispatch.rs:1393, orchestrate.rs:8693 | Gate passes/fails |
| AgentSpawned | orchestrate.rs:8530,17112 | Agent dispatched |
| PhaseTransition | orchestrate.rs:14776 | Plan phase change |
| Execution | orchestrate.rs:5737 | Sub-events during execution |
| BenchRunStarted | routes/bench.rs:141 | Bench run begins |
| BenchTaskStarted | routes/bench.rs:239 | Bench task begins |
| BenchTaskCompleted | routes/bench.rs:324 | Bench task ends |
| BenchProgress | routes/bench.rs:366 | Bench progress update |
| BenchLearningEvent | routes/bench.rs:339 | Bench learning data |
| BenchRunCompleted | routes/bench.rs:414 | Bench run ends |
| JobCreated | routes/jobs.rs:820 | Job posted |
| JobUpdated | routes/jobs.rs:823 | Job state change |
| JobTransitioned | routes/jobs.rs:831 | Job lifecycle |
| JobPostedToCandidate | routes/jobs.rs:598 | Job offered to agent |
| DeploymentCreated | routes/templates.rs:308 | Deployment starts |

### NEVER Emitted (gaps to fix)

#### Critical Priority — Plan Execution Visibility

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **TaskStarted** | orchestrate.rs (per-task start) | Has DashboardEvent equivalent; needs ServerEvent emit |
| **TaskCompleted** | orchestrate.rs (per-task end) | Same — DashboardEvent exists, ServerEvent not emitted |
| **TaskFailed** | orchestrate.rs (task gate failure) | Should fire when gate fails |

These three are the most critical. The frontend shows task progress cards — without
them, the execution panel is blank.

#### High Priority — Inference Tracking

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **InferenceStarted** | roko-agent dispatcher (pre-LLM call) | Before HTTP call to Claude/OpenAI/etc |
| **InferenceCompleted** | roko-agent dispatcher (post-LLM call) | After response; has tokens, cost, duration |
| **InferenceFailed** | roko-agent dispatcher (LLM error) | On timeout, rate limit, API error |

The agent dispatcher (`crates/roko-agent/src/dispatcher/mod.rs`) makes LLM calls but
doesn't emit these events. The data exists (token counts, latency) in the response —
it just needs to be published.

#### High Priority — Agent Lifecycle

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **AgentStarted** | agent start command / runtime | When long-running agent begins |
| **AgentStopped** | agent stop command / runtime | When agent halts |
| **AgentTrace** | agent tool loop (per-turn) | Detailed trace with reasoning + tool calls |

`AgentTrace` is the richest event for debugging. It should fire per-turn in the
agent's tool loop, containing the full reasoning chain and tool call list.

#### Medium Priority — Somatic/Affect

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **SomaticMarkerFired** | orchestrate.rs (daimon integration) | When DaimonState fires a somatic marker |

The daimon state is loaded and consulted in orchestrate.rs. When a marker fires
(affecting dispatch decisions), this event should be emitted.

#### Medium Priority — Run Lifecycle

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **RunStarted** | `roko run` command handler | Single-prompt run begins |
| **RunCompleted** | `roko run` command handler | Single-prompt run ends |
| **OperationStarted** | Various route handlers | Generic operation begins |

#### Medium Priority — Config/Strategy

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **ConfigReloaded** | config file watcher / `roko config set` | When config changes at runtime |
| **StrategyReloaded** | strategy file watcher | When goals/tactics change |

#### Medium Priority — Vision Loop

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **VisionLoopIteration** | vision loop runner | Each iteration score |
| **VisionLoopCompleted** | vision loop runner | Loop exits |

#### Lower Priority — Deployment Lifecycle

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **DeploymentReady** | deploy command (after health check) | URL available |
| **DeploymentFailed** | deploy command (on error) | Deploy failed |
| **DeploymentTornDown** | teardown handler | Deployment removed |

#### Lower Priority — Job Execution

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **JobExecutionStarted** | job executor | Job agent begins work |
| **JobProgress** | job executor | Progress update |
| **JobAgentOutput** | job executor (streaming) | Agent output for job |
| **JobSubmitted** | job executor | Work submitted |
| **JobEvaluated** | job evaluator | Acceptance/rejection |
| **JobStateChanged** | job state machine | Generic state transition |

#### Lower Priority — Worker

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **WorkerTaskStarted** | worker runtime | Task begins on worker |
| **WorkerTaskCompleted** | worker runtime | Task ends on worker |

#### Lower Priority — Other

| Variant | Where to Emit | Notes |
|---------|--------------|-------|
| **HeartbeatReceived** | heartbeat handler | From remote agents |
| **Heartbeat** | heartbeat emitter | Local agent heartbeat |
| **ChainTriageResult** | chain triage runner | Anomaly detection results |
| **WebhookReceived** | webhook handler | Incoming signal |
| **ServerShutdown** | graceful shutdown handler | Server stopping |
| **MatrixRunStarted/LaneCompleted/RunCompleted** | matrix bench runner | Matrix sweep events |
| **SweRunStarted/InstanceCompleted/RunCompleted** | SWE-bench runner | SWE-bench events |
| **BenchGateVerdict** | bench gate evaluator | Individual gate in bench |
| **BenchTokenVelocity** | bench metrics | Tokens/sec tracking |
| **BenchAgentOutput** | bench agent streaming | Agent output during bench |

## Implementation Priority

### Phase 1 — Unblocks demo (do first)
1. TaskStarted / TaskCompleted / TaskFailed
2. Wire plan execution to serve hub (see `01-PLAN-EXECUTION.md`)

### Phase 2 — Rich monitoring
3. InferenceStarted / InferenceCompleted / InferenceFailed
4. AgentTrace (per-turn detail)
5. AgentStarted / AgentStopped
6. SomaticMarkerFired

### Phase 3 — Full coverage
7. Run lifecycle (RunStarted, RunCompleted)
8. Config/Strategy reload events
9. Vision loop events
10. Remaining job/deployment/worker events

## Where to Add Emissions

The pattern is consistent across the codebase:

```rust
// In route handlers (roko-serve):
state.event_bus.publish(ServerEvent::TaskStarted { ... });

// In orchestrate.rs (legacy path):
self.emit_server_event(ServerEvent::InferenceStarted { ... });

// In the runner event loop (new path):
if let Some(sink) = &http_sink {
    sink.emit(ServerEvent::AgentTrace { ... });
}
```

For events generated deep in library crates (like `roko-agent`), use a callback
trait or channel rather than depending on `roko-serve` types directly:

```rust
// In roko-agent:
pub trait InferenceObserver: Send + Sync {
    fn on_inference_start(&self, model: &str, agent_id: &str);
    fn on_inference_complete(&self, model: &str, tokens: TokenUsage, duration: Duration);
    fn on_inference_error(&self, model: &str, error: &str);
}
```

The concrete implementation in `roko-cli` or `roko-serve` converts to `ServerEvent`.
