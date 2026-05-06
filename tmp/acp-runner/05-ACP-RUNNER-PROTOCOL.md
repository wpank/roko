# ACP Protocol Extensions for Workflow Execution

## Current ACP Capabilities

Today ACP supports:
- `session/prompt` → single agent response
- `session/update` → streaming chunks, tool calls, plan entries
- `session/cancel` → interrupt active prompt
- Config options → model, effort, temperament, routing, gates
- Modes → code, plan, research

## New: Workflow-Aware Session Updates

### Plan Updates (Already in Protocol)

ACP already has `plan` session updates. We use these to show workflow progress:

```json
{
  "sessionId": "sess_xxx",
  "update": {
    "sessionUpdate": "plan",
    "entries": [
      { "content": "✓ Strategy brief generated", "priority": "high", "status": "completed" },
      { "content": "Implementing: edit src/main.rs", "priority": "high", "status": "in_progress" },
      { "content": "Run gates (compile + test + clippy)", "priority": "medium", "status": "pending" },
      { "content": "Code review", "priority": "medium", "status": "pending" }
    ]
  }
}
```

### Tool Calls for Agent Phases

Each agent phase appears as a tool call card in the editor:

```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "phase-strategist-1",
  "title": "Strategist: Analyzing approach",
  "kind": "other",
  "status": "in_progress",
  "content": []
}
```

When the strategist completes:
```json
{
  "sessionUpdate": "tool_call_update",
  "toolCallId": "phase-strategist-1",
  "status": "completed",
  "content": [{ "type": "text", "text": "Brief: Use trait-based dispatch pattern..." }]
}
```

### Gate Results as Tool Calls

```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "gate-compile",
  "title": "Gate: cargo build",
  "kind": "terminal",
  "status": "in_progress",
  "content": []
}
```

Pass:
```json
{
  "sessionUpdate": "tool_call_update",
  "toolCallId": "gate-compile",
  "status": "completed",
  "content": [{ "type": "text", "text": "✓ Compiled successfully" }]
}
```

Fail:
```json
{
  "sessionUpdate": "tool_call_update",
  "toolCallId": "gate-compile",
  "status": "failed",
  "content": [{ "type": "text", "text": "error[E0308]: mismatched types..." }]
}
```

### Review Verdicts

```json
{
  "sessionUpdate": "tool_call",
  "toolCallId": "review-auditor",
  "title": "Auditor: Security review",
  "kind": "other",
  "status": "completed",
  "content": [{
    "type": "text",
    "text": "**Verdict: approve**\n\nNo blocking issues.\n\nMinor: Consider adding input validation on line 42."
  }]
}
```

## New Config Options

### Workflow/Pipeline Selector
```json
{
  "id": "workflow",
  "name": "Workflow",
  "type": "select",
  "category": "execution",
  "currentValue": "standard",
  "options": [
    { "value": "express", "name": "Express", "description": "Implement → gate → commit (fastest)" },
    { "value": "standard", "name": "Standard", "description": "Implement → gate → review → commit" },
    { "value": "full", "name": "Full", "description": "Strategy → implement → gate → multi-review → commit" },
    { "value": "research", "name": "Research", "description": "Research → synthesize (no code changes)" },
    { "value": "auto", "name": "Auto", "description": "Select based on complexity" }
  ]
}
```

### Review Strictness
```json
{
  "id": "review_strictness",
  "name": "Review",
  "type": "select",
  "category": "execution",
  "currentValue": "standard",
  "options": [
    { "value": "none", "name": "None", "description": "Skip all reviews" },
    { "value": "quick", "name": "Quick", "description": "Single-pass QuickReviewer" },
    { "value": "standard", "name": "Standard", "description": "Architecture + correctness" },
    { "value": "thorough", "name": "Thorough", "description": "Architecture + audit + docs" }
  ]
}
```

### Max Iterations
```json
{
  "id": "max_iterations",
  "name": "Max Retries",
  "type": "select",
  "category": "execution",
  "currentValue": "2",
  "options": [
    { "value": "1", "name": "1", "description": "No retries" },
    { "value": "2", "name": "2", "description": "Standard" },
    { "value": "3", "name": "3", "description": "Persistent" }
  ]
}
```

## New Slash Commands for Workflows

| Command | What it does |
|---------|-------------|
| `/workflow list` | List available workflow templates |
| `/workflow run <name>` | Execute a named workflow |
| `/workflow status` | Show current workflow progress |
| `/workflow cancel` | Cancel running workflow |
| `/workflow resume` | Resume halted workflow |
| `/pipeline <name>` | Shorthand for workflow run |
| `/express <prompt>` | Run express pipeline on prompt |
| `/full <prompt>` | Run full pipeline on prompt |
| `/review-this` | Run review pipeline on current changes |

## Session State for Workflows

### WorkflowRun (persisted in session)

```rust
struct WorkflowRun {
    run_id: String,
    template: String,          // "express", "standard", "full", etc.
    phase: PipelinePhase,
    iteration: u32,
    started_at: DateTime<Utc>,

    // Task-level (for multi-task plans)
    tasks: Vec<TaskStatus>,
    current_task: Option<String>,

    // Accumulated context
    strategist_brief: Option<String>,
    review_findings: Vec<ReviewFinding>,
    gate_results: Vec<GateResult>,

    // Metrics
    total_cost_usd: f64,
    total_tokens: u64,
    agents_spawned: u32,
}
```

This gets persisted alongside the session in `.roko/sessions/{id}.json`, enabling resume after disconnect.

## Interaction Patterns

### Pattern 1: Transparent Workflow
User types a prompt. Based on configured workflow, the system automatically runs the pipeline. User sees live progress via plan updates and tool call cards. Result: committed code.

### Pattern 2: Interactive Workflow
At key decision points (review verdict, gate failure), the system pauses and asks the user:
```
Agent message: "Review found 2 issues. Fix and retry, or commit as-is?"
```
User responds, workflow continues.

### Pattern 3: Background Workflow
User starts a workflow (e.g., `/plan-run plans/`) and continues chatting. Workflow runs in background. Status shown in session info. User can check with `/workflow status`.

### Pattern 4: Trigger-Based Workflow
User configures a trigger (e.g., "review PRs automatically"). No active session needed. Results persisted to `.roko/runs/`. User reviews later via `/workflow status` or dashboard.
