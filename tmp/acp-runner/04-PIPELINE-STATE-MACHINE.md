# Pipeline State Machine

## How Bardo's Pipeline Worked

The pipeline is a **pure state machine**. It receives events and emits actions. No side effects in the state machine itself — all effects are performed by the executor.

```rust
// Simplified from bardo's pipeline.rs
enum PipelineEvent {
    AgentCompleted { instance_id, output },
    AgentFailed { instance_id, error },
    GatePassed { gate_type },
    GateFailed { gate_type, output },
    ReviewVerdict { verdict, findings },
    Timeout,
    UserInput { response },
}

enum PipelineAction {
    SpawnAgent { role, prompt, effort, model, working_dir },
    KillAgent { instance_id },
    RunGate { gate_type, working_dir },
    Commit { message },
    Complete,
    Halt { reason },
    WaitForUser { prompt, choices },
}
```

## State Machine for ACP Runner

### States (PipelinePhase)

```
┌─────────────┐
│   Pending   │  (workflow configured but not started)
└──────┬──────┘
       │ user prompt
       ▼
┌─────────────┐
│ Strategizing│  (strategist analyzing, producing brief)
└──────┬──────┘
       │ brief complete (or skipped)
       ▼
┌──────────────┐
│Implementing  │  (implementer writing code)
└──────┬───────┘
       │ implementation complete
       ▼
┌─────────────┐
│   Gating    │  (compile → test → clippy)
└──────┬──────┘
       │ gates pass
       ▼
┌─────────────┐
│  Reviewing  │  (reviewer agents analyzing changes)
└──────┬──────┘
       │ verdict: approve
       ▼
┌─────────────┐
│  Committing │  (creating commit, persisting state)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  Complete   │
└─────────────┘
```

### Failure Transitions

```
Gating ──[fail, attempt < max]──→ AutoFixing ──→ Gating
Gating ──[fail, attempt >= max]──→ Implementing (with error context)
Reviewing ──[revise, quick-fixable]──→ QuickFixing ──→ Gating
Reviewing ──[revise, complex]──→ Implementing (with review feedback)
Reviewing ──[revise, docs-only]──→ DocRevision ──→ Committing
Any ──[timeout]──→ Halted (state persisted for resume)
Any ──[budget exceeded]──→ Halted (state persisted)
Any ──[user cancel]──→ Cancelled
```

### State Data

```rust
struct PipelineState {
    phase: PipelinePhase,
    iteration: u32,          // how many times we've looped
    max_iterations: u32,     // cap (usually 2)

    // Inputs
    original_prompt: String,
    workflow_template: WorkflowTemplate,

    // Accumulated context
    strategist_brief: Option<String>,
    review_feedback: Vec<ReviewFinding>,
    gate_failures: Vec<GateFailure>,

    // Tracking
    started_at: DateTime<Utc>,
    agents_spawned: Vec<AgentInstance>,
    total_cost_usd: f64,
    total_tokens: u64,
}
```

## Multi-Task Execution (Plan Runner)

When a workflow involves multiple tasks (from a plan), the executor manages a **task DAG**:

```
Plan tasks.toml:
  T1 (no deps) ─┐
  T2 (no deps) ─┼─► T4 (depends on T1, T2)
  T3 (no deps) ─┘         │
                           ▼
                     T5 (depends on T4)
```

**Parallel scheduling rules** (from bardo):
1. A task is ready when all its `depends_on` are complete
2. Up to `max_parallel` tasks can run simultaneously
3. Each task runs through its own pipeline (complexity-appropriate)
4. Exclusive files: tasks touching the same files cannot run in parallel
5. Cross-plan dependencies: task in plan B can depend on task in plan A

### ExecutorState (For Persistence)

```rust
struct ExecutorState {
    // Task tracking
    completed_tasks: Vec<GlobalTaskId>,
    in_flight_tasks: HashMap<GlobalTaskId, AgentInstanceId>,
    failed_tasks: HashMap<GlobalTaskId, u32>,  // failure count
    skipped_tasks: Vec<GlobalTaskId>,

    // Plan tracking
    plan_phases: HashMap<String, PipelinePhase>,
    plan_iterations: HashMap<String, u32>,

    // Review state
    review_feedback: HashMap<GlobalTaskId, Vec<String>>,

    // Merge queue (dependency-ordered)
    merge_queue: Vec<String>,

    // Convergence detection
    verify_error_signatures: HashMap<String, Vec<u64>>,
}
```

## ACP Session Updates During Workflow

As the pipeline progresses, the ACP session streams updates to the editor:

### Plan Phase Updates
```json
{
  "sessionUpdate": "plan",
  "entries": [
    { "content": "Strategist analyzing approach...", "priority": "high", "status": "in_progress" },
    { "content": "Implement code changes", "priority": "high", "status": "pending" },
    { "content": "Run compile + test gates", "priority": "medium", "status": "pending" },
    { "content": "Review changes", "priority": "medium", "status": "pending" },
    { "content": "Commit result", "priority": "low", "status": "pending" }
  ]
}
```

### Agent Activity Updates
```json
{
  "sessionUpdate": "agent_thought_chunk",
  "content": { "type": "text", "text": "[Strategist] Analyzing workspace structure..." }
}
```

### Gate Result Updates
```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "gate-compile-1",
  "title": "Compile Gate",
  "kind": "terminal",
  "status": "in_progress"
}
```

### Review Verdict Updates
```json
{
  "sessionUpdate": "tool_call_update",
  "toolCallId": "review-1",
  "status": "completed",
  "content": [{ "type": "text", "text": "✓ Approved: No blocking issues found." }]
}
```

## Convergence Detection

Bardo detected when a pipeline was stuck in a loop:

1. Hash the error output on each gate failure
2. If the same error hash appears 2+ times → implementation is not converging
3. Action: escalate model tier, change approach, or halt with feedback

For ACP: if the same gate failure recurs, inform the user:
```
"Pipeline stalled: same compile error after 2 attempts.
Options: [Try different approach] [Escalate to opus] [Cancel]"
```

## Merge Queue

When multiple tasks complete, they must merge in dependency order:

```
T1 completes → merge immediately (no deps)
T3 completes → cannot merge yet (depends on T2)
T2 completes → merge T2, then T3 (unblocked)
```

Only one merge happens at a time. After merge, re-run gates on merged state to catch conflicts.
