# Progress Tracking and State Management

## What Needs To Be Tracked

### Per-Workflow-Run State

```
Run ID: run_2026-04-27_feat-auth
Template: standard
Session: sess_abc123

Phase Timeline:
  [00:00] Started
  [00:02] Strategist: analyzing (skipped — standard mode)
  [00:02] Implementer: spawned (sonnet, bypassPermissions)
  [00:15] Implementer: completed (3 files changed)
  [00:15] Gate: compile — running
  [00:18] Gate: compile — PASSED
  [00:18] Gate: test — running
  [00:22] Gate: test — PASSED
  [00:22] Reviewer: spawned (sonnet, plan mode)
  [00:28] Reviewer: verdict=approve
  [00:28] Committing
  [00:29] Complete (total: 29s, $0.08)
```

### Per-Task State (Multi-Task Plans)

```
Plan: feature-auth (4 tasks)

  T1 [✓ done]    Add User model          (sonnet, 12s, $0.02)
  T2 [● active]  Add auth middleware      (sonnet, implementing...)
  T3 [○ pending] Add login endpoint       (blocked by T2)
  T4 [○ pending] Add integration tests    (blocked by T2, T3)

Progress: 1/4 tasks complete
Estimated remaining: ~8 minutes
```

### Agent Instance State

```
Agent: impl-T2-iter1
  Role: Implementer
  Model: sonnet
  Started: 00:15
  Duration: 3m 22s (active)
  Tokens: 12,400 in / 3,200 out
  Cost: $0.03 so far
  Files touched: src/middleware/auth.rs, src/routes/mod.rs
  Status: writing code
```

## ACP Session Updates for Progress

### Phase Transition → Plan Update

Every time the workflow advances a phase, emit a plan update:

```json
{
  "sessionUpdate": "plan",
  "entries": [
    { "content": "✓ Implementation complete (3 files)", "priority": "high", "status": "completed" },
    { "content": "● Running gates: compile ✓, test ✓, clippy...", "priority": "high", "status": "in_progress" },
    { "content": "○ Code review (after gates pass)", "priority": "medium", "status": "pending" },
    { "content": "○ Commit changes", "priority": "low", "status": "pending" }
  ]
}
```

### Multi-Task Plan → Plan Update with Task Detail

```json
{
  "sessionUpdate": "plan",
  "entries": [
    { "content": "✓ T1: Add User model", "priority": "high", "status": "completed" },
    { "content": "● T2: Add auth middleware (implementing...)", "priority": "high", "status": "in_progress" },
    { "content": "○ T3: Add login endpoint (blocked)", "priority": "medium", "status": "pending" },
    { "content": "○ T4: Integration tests (blocked)", "priority": "medium", "status": "pending" }
  ]
}
```

### Cost/Usage Update (Cumulative)

```json
{
  "sessionUpdate": "usage_update",
  "used": 45000,
  "size": 200000,
  "cost": { "amount": 0.08, "currency": "USD" }
}
```

### Session Info Update (Name Reflects Workflow)

```json
{
  "sessionUpdate": "session_info_update",
  "sessionId": "sess_abc123",
  "sessionName": "feature-auth [Standard → Reviewing]"
}
```

## Persistence: Where State Lives

### In-Memory (AcpSession)

```rust
struct AcpSession {
    // ... existing fields ...
    conversation_history: Vec<ConversationTurn>,

    // NEW: Active workflow run
    active_run: Option<WorkflowRun>,
}

struct WorkflowRun {
    run_id: String,
    template: String,
    phase: PipelinePhase,
    iteration: u32,
    started_at: DateTime<Utc>,
    tasks: Vec<TaskState>,
    current_task: Option<String>,
    strategist_brief: Option<String>,
    review_findings: Vec<String>,
    gate_results: Vec<GateResult>,
    total_cost_usd: f64,
    total_tokens: u64,
}

struct TaskState {
    id: String,
    title: String,
    status: TaskStatus,  // pending, active, done, failed, skipped
    depends_on: Vec<String>,
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
    model_used: Option<String>,
    cost_usd: f64,
}
```

### On Disk (Session Persistence — Already Implemented)

Session JSON at `.roko/sessions/{session_id}.json` includes `active_run` and `conversation_history`. Enables:
- Resume after editor restart
- Resume after server crash
- List in-progress workflows from another client

### Run Artifacts (Separate from Session)

Long-lived artifacts go to `.roko/runs/{run_id}/`:
```
.roko/runs/run_2026-04-27_feat-auth/
├── state.json          # ExecutorState snapshot
├── brief.md            # Strategist output
├── review-verdict.json # Review findings
├── gate-results.json   # Gate pass/fail log
├── episodes.jsonl      # Per-agent episode records
└── tasks-status.toml   # Updated task statuses
```

## Live Progress Events (What The User Sees)

### Scenario: Standard Pipeline

1. User sends prompt: "Add authentication middleware"
2. Editor shows plan:
   ```
   ● Implementing authentication middleware...
   ○ Compile + test gates
   ○ Code review
   ○ Commit
   ```
3. Agent streams thought chunks (if thinking visible):
   ```
   [Implementer] Reading existing middleware patterns...
   [Implementer] Creating src/middleware/auth.rs...
   ```
4. Tool call cards appear for file edits:
   ```
   📝 Edit: src/middleware/auth.rs [completed]
   📝 Create: src/middleware/mod.rs [completed]
   ```
5. Phase advances, plan updates:
   ```
   ✓ Implementation complete (2 files)
   ● Running gates: compile...
   ○ Code review
   ○ Commit
   ```
6. Gate tool call:
   ```
   🖥️ Gate: cargo build --workspace [completed ✓]
   🖥️ Gate: cargo test --workspace [completed ✓]
   ```
7. Phase advances:
   ```
   ✓ Implementation complete (2 files)
   ✓ Gates passed (compile, test)
   ● Reviewing changes...
   ○ Commit
   ```
8. Review completes:
   ```
   ✓ Review: approved (no blocking issues)
   ```
9. Final message:
   ```
   ✓ Workflow complete. Committed: "feat: add auth middleware"
   Cost: $0.06 | Tokens: 28K | Duration: 45s
   ```

### Scenario: Gate Failure + Retry

```
✓ Implementation complete (2 files)
✗ Gate: cargo test — 2 tests failed
  ● test auth::test_invalid_token — assertion failed
● AutoFix: patching test failures...
```

Then after fix:
```
✓ AutoFix complete (1 file changed)
● Re-running gates: compile...
✓ Gates passed (attempt 2)
● Reviewing changes...
```

### Scenario: Review Rejection + Retry

```
✓ Gates passed
✗ Review: revise (2 findings)
  - [major] Missing error handling in auth_check()
  - [minor] Consider using From trait instead of manual conversion
● Re-implementing with feedback...
```

## Halted State (For Resume)

When a workflow is halted (timeout, budget, user cancel), state is preserved:

```json
{
  "run_id": "run_xxx",
  "phase": "implementing",
  "iteration": 1,
  "halted_reason": "timeout",
  "halted_at": "2026-04-27T10:45:00Z",
  "resumable": true,
  "completed_tasks": ["T1"],
  "current_task": "T2",
  "context": {
    "strategist_brief": "...",
    "last_gate_output": "..."
  }
}
```

Resume via: `/workflow resume` or session reload.

## Status Queries

### `/workflow status` Response

```
Active Workflow: feature-auth [Standard]
Phase: Reviewing (iteration 1/2)
Progress: 3/4 tasks complete
Current: T4 — Integration tests (reviewing)
Duration: 4m 12s
Cost: $0.14
Agents spawned: 6 (4 implementers, 1 reviewer, 1 autofixer)

Task Status:
  T1 [✓] Add User model           12s  $0.02
  T2 [✓] Add auth middleware       28s  $0.04
  T3 [✓] Add login endpoint        35s  $0.05
  T4 [●] Add integration tests     (reviewing...)
```

## Learning From Workflow Runs

Every workflow run feeds back into roko's learning systems:

1. **Episode log**: Per-agent records (model, tokens, cost, success/fail)
2. **Cascade router**: Which models succeed at which roles
3. **Gate thresholds**: Adaptive pass/fail criteria
4. **Prompt experiments**: Which system prompts produce better results
5. **Workflow metrics**: Which templates are most effective for which task types
6. **Convergence data**: How many iterations typically needed per complexity band
