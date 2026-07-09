# Layer 4: Plan execution UI

**Goal**: Users create plans visually, execute them with agents, and monitor progress through the dashboard.

**Depends on**: Plan 01 (dashboard resilience)

**Effort**: M (2-3 days for basic flow, +2 days for chat editor and cost estimation)

---

## Current state

### What exists in roko-serve

Eleven plan routes in `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs`:

| Route | What it does | Status |
|-------|--------------|--------|
| `GET /api/plans` | Lists plans from `.roko/plans/` (TOML and JSON) | ✅ |
| `GET /api/plans/{id}` | Loads a single plan | ✅ |
| `POST /api/plans` | Creates a plan from JSON body (title, description, tasks) | ✅ |
| `POST /api/plans/{id}/execute` | Spawns background plan execution via `CliRuntime::run_once` | ✅ |
| `GET /api/plans/{id}/status` | Checks whether a plan is actively executing | ✅ |
| `POST /api/plans/generate` | Generates a plan from a PRD slug | ✅ |
| `POST /api/plans/{id}/pause` | Cancels running execution, saves paused snapshot | ✅ NEW |
| `POST /api/plans/{id}/resume` | Resumes paused plan from snapshot | ✅ NEW |
| `GET /api/plans/{id}/gates` | Query gate results for a specific plan | ✅ NEW |
| `POST /api/plans/{id}/chat` | LLM-powered plan editing via natural language | ✅ NEW |
| `POST /api/plans/{id}/estimate` | Cost/time estimation from historical efficiency data | ✅ NEW |

The plan data model (`/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/plan_types.rs`) is flat: `Plan { id, title, description, tasks }` where each `PlanTask` has `id, description, depends_on, files, completed`. No status enum, no parallel groups, no checkpoints, no error policy.

Execution events (`/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/events.rs`) exist: `PlanStarted`, `TaskStarted`, `TaskPhaseChanged`, `GateResult`, `TaskCompleted`, `PlanCompleted`, `ReplanTriggered`, `WatcherAlert`. These stream over SSE and WebSocket.

### What exists in orchestrate.rs

`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (800K+ LOC) runs the full plan-execute-gate-persist loop. It reads `tasks.toml`, builds a DAG via `ParallelExecutor`, dispatches agents, runs gate pipelines per task, persists results, and supports `--resume` via `ExecutorSnapshot`. Pause is not exposed as an API -- only crash recovery.

### What needs to be built

| Feature | Where | Status |
|---------|-------|--------|
| Plan list page | Dashboard (`src/pages/forge/Plans.tsx`) | Frontend pending |
| Plan detail with task cards | Dashboard (`src/pages/forge/PlanDetail.tsx`) | Frontend pending |
| Plan creation via card UI | Dashboard (`src/pages/forge/CreatePlan.tsx`) | Frontend pending |
| Plan execution monitor | Dashboard (`src/pages/forge/PlanExecution.tsx`) | Frontend pending |
| `POST /api/plans/{id}/pause` | roko-serve `routes/plans.rs` | ✅ DONE |
| `POST /api/plans/{id}/resume` | roko-serve `routes/plans.rs` | ✅ DONE |
| `GET /api/plans/{id}/gates` | roko-serve `routes/plans.rs` | ✅ DONE |
| `POST /api/plans/{id}/chat` | roko-serve `routes/plans.rs` | ✅ DONE |
| `POST /api/plans/{id}/estimate` | roko-serve `routes/plans.rs` | ✅ DONE |

---

## Tasks

### 4.1 Plan list page

Display all plans from roko-serve with status badges.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` (lines 30-63, `list_plans` handler)
- Dashboard route structure and data fetching patterns in the existing codebase

**Target**: `src/pages/forge/Plans.tsx` (in nunchi-dashboard)

**API contract**:
```
GET /api/plans

Response 200:
[
  {
    "id": "plan-abc",
    "title": "Add JWT auth",
    "task_count": 4,
    "completed": false,
    "completed_task_count": 1
  }
]
```

This endpoint already exists. No backend changes needed.

**Implementation**:
- [ ] Create `Plans.tsx` with a table/card layout
- [ ] Columns: title, task count, completion fraction, created date
- [ ] Each row links to `/forge/plans/{id}` (plan detail page)
- [ ] "New plan" button links to the creation page
- [ ] Use TanStack Query for data fetching, poll every 5s (WS subscription comes later)

**Acceptance criteria**:
- Plans from `.roko/plans/` appear in the dashboard table
- Empty state shows "No plans yet" with a create button
- Clicking a row navigates to the detail page

---

### 4.2 Plan detail page -- task card stack

Render a plan's tasks as a vertical card stack. This is Level 1 of the visual builder from the architecture spec.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/19-visual-composition.md` (lines 500-565, `PlanSpec` data model and Level 1 card stack)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` (lines 66-72, `get_plan` handler)

**Target**: `src/pages/forge/PlanDetail.tsx` (in nunchi-dashboard)

**API contract**:
```
GET /api/plans/{id}

Response 200:
{
  "id": "plan-abc",
  "title": "Add JWT auth",
  "description": "Implement JWT middleware for the API",
  "tasks": [
    {
      "id": "t1",
      "description": "Research auth patterns",
      "depends_on": [],
      "files": ["src/auth.rs"],
      "completed": false,
      "status": "pending"
    }
  ]
}
```

This endpoint already exists. The `status` field is derived (completed -> "completed", else -> "pending").

**Implementation**:
- [ ] Create `PlanDetail.tsx` with plan header (title, description) and task card stack
- [ ] Each task card shows: title/description, status badge, dependency list, file list
- [ ] Cards are ordered top-to-bottom in execution order
- [ ] Dependency lines: show "depends on: T1, T2" text on each card (lines/arrows come in Level 2)
- [ ] "Run" button at the top triggers execution (task 4.4)
- [ ] "Edit" button links to the creation page pre-filled with this plan's data

**Acceptance criteria**:
- Clicking a plan from the list page shows its tasks as cards
- Each card displays all task metadata
- Tasks with `completed: true` show a green checkmark

---

### 4.3 Plan creation -- card stack editor

Create new plans by adding task cards, reordering them, and setting dependencies.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` (lines 114-153, `create_plan` handler)
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/19-visual-composition.md` (lines 105-167, `TaskSpec` and `TaskPatch` types)

**Target**: `src/pages/forge/CreatePlan.tsx` (in nunchi-dashboard)

**API contract**:
```
POST /api/plans
Content-Type: application/json

{
  "title": "Add JWT auth",
  "description": "Implement JWT middleware for the API",
  "tasks": [
    {
      "id": "t1",
      "description": "Research auth patterns",
      "depends_on": [],
      "files": ["src/auth.rs"]
    }
  ]
}

Response 201:
{ "id": "plan-abc" }
```

This endpoint already exists. No backend changes needed.

**Implementation**:
- [ ] Create `CreatePlan.tsx` with plan metadata form (title, description) at the top
- [ ] "Add task" button appends a blank card to the stack
- [ ] Each card is an editable form: task ID, description, files (comma-separated), depends_on (multi-select from other task IDs)
- [ ] Drag-to-reorder cards (react-beautiful-dnd or dnd-kit)
- [ ] Delete button on each card (with confirmation if the task has dependents)
- [ ] "Save" button calls `POST /api/plans`
- [ ] Client-side validation: no blank IDs, no duplicate IDs, no dependency cycles

**Acceptance criteria**:
- Create a 3-task plan with dependencies via the UI
- Saved plan appears in the plan list
- Reordering updates the visual order
- Dependency cycles are rejected with an inline error

---

### 4.4 Plan execution monitor

Run a plan and watch tasks progress through states in real time.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` (lines 155-217, `execute_plan` handler)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/events.rs` (lines 1-78, `ExecutionEvent` enum)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/ws.rs` (WebSocket endpoint)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/sse.rs` (SSE endpoint)

**Target**: `src/pages/forge/PlanExecution.tsx` (in nunchi-dashboard)

**API contract**:

Execute:
```
POST /api/plans/{id}/execute

Response 202:
{ "id": "run-xyz" }
```

Status:
```
GET /api/plans/{id}/status

Response 200:
{
  "id": "run-xyz",
  "plan_dir": "/path/to/.roko/plans",
  "status": "Running",
  "finished": false
}
```

Events (via WebSocket or SSE):
```json
{ "type": "plan_started", "plan_id": "plan-abc" }
{ "type": "task_started", "task_id": "t1", "phase": "dispatch" }
{ "type": "gate_result", "task_id": "t1", "gate": "test", "passed": true, "message": "47 tests passed" }
{ "type": "task_completed", "task_id": "t1", "outcome": "success" }
{ "type": "plan_completed", "plan_id": "plan-abc", "success": true }
```

All endpoints exist. No backend changes needed for the basic flow.

**Implementation**:
- [ ] Create `PlanExecution.tsx` showing the task card stack from 4.2 plus live status
- [ ] Each card updates status via WS events: `waiting` -> `running` (spinner) -> `done` (green) or `failed` (red)
- [ ] Subscribe to SSE or WS on mount, unsubscribe on unmount
- [ ] Show a running timer for the overall plan and per-task
- [ ] "Run" button on plan detail triggers `POST /api/plans/{id}/execute`, then navigates to execution view
- [ ] When `plan_completed` arrives, show a summary banner (pass/fail, total time, cost if available)

**Acceptance criteria**:
- Execute a plan from the dashboard
- Task cards update status in real time as events arrive
- Completed plan shows a summary with pass/fail

---

### 4.5 Gate results display

Show gate verdicts per task during and after execution.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/events.rs` (lines 36-45, `GateResult` variant)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/gate_pipeline.rs` (gate pipeline types)

**Target**: Extend `PlanExecution.tsx` task cards (from task 4.4)

**API contract**: No new endpoints. Gate results arrive via the existing `ExecutionEvent::GateResult` event:

```json
{
  "type": "gate_result",
  "task_id": "t1",
  "gate": "test",
  "passed": true,
  "message": "47 tests passed, 0 failed"
}
```

```json
{
  "type": "gate_result",
  "task_id": "t2",
  "gate": "clippy",
  "passed": false,
  "message": "3 warnings, 1 error: unused import in src/lib.rs:12"
}
```

**Implementation**:
- [ ] Add a "Gates" section below each task card in the execution view
- [ ] Each gate result renders as a pill: green check + gate name for pass, red X + gate name for fail
- [ ] Clicking a gate pill expands to show the full message text
- [ ] Failed gates highlight the task card border in red
- [ ] Accumulate gate results per task as events stream in

**Acceptance criteria**:
- Execute a plan with test and clippy gates
- Gate results appear on the corresponding task card
- Failed gate shows the error message on expand

---

### 4.6 Plan pause and resume

Pause a running plan, optionally edit remaining tasks, then resume.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/19-visual-composition.md` (lines 388-493, plan states and lifecycle, pause/resume contracts)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` (lines 155-217, current execute_plan)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs` (PlanHandle, OperationStatus types)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/orchestrate.rs` (ExecutorSnapshot for resume)

**Target (backend -- NEW ROUTES)**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` -- add `pause_plan` and `resume_plan` handlers
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/plan_types.rs` -- add `PlanStatus` enum
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/state.rs` -- extend `PlanHandle` with pause state

**Target (frontend)**:
- Extend `PlanExecution.tsx` with pause/resume controls

**API contract**:

Pause:
```
POST /api/plans/{id}/pause

Response 200:
{
  "execution_id": "run-xyz",
  "status": "paused",
  "completed_tasks": ["t1"],
  "paused_tasks": ["t2a"],
  "remaining_tasks": ["t2b", "t3"],
  "snapshot_id": "snap-002"
}
```

Resume:
```
POST /api/plans/{id}/resume

Response 200:
{
  "execution_id": "run-xyz",
  "status": "executing",
  "resuming_from": "snap-002",
  "remaining_tasks": ["t2a", "t2b", "t3"]
}
```

**Implementation (backend)**:
- [ ] Add `PlanStatus` enum to `plan_types.rs`: `Draft`, `Executing { execution_id }`, `Paused { snapshot_id }`, `Completed { execution_id }`, `Failed { execution_id, reason }`
- [ ] Extend `PlanHandle` in `state.rs` with status field and snapshot path
- [ ] Implement `pause_plan` handler: look up active plan, cancel the tokio task via `CancelToken`, write snapshot to `.roko/state/plan-{id}-snap-{n}.json`, transition status to Paused
- [ ] Implement `resume_plan` handler: load snapshot, re-spawn execution task with `--resume` flag, transition status back to Executing
- [ ] Register routes: `.route("/plans/{id}/pause", post(pause_plan))` and `.route("/plans/{id}/resume", post(resume_plan))`
- [ ] Emit `ServerEvent::PlanPaused` and `ServerEvent::PlanResumed` events

**Implementation (frontend)**:
- [ ] Add "Pause" button visible during execution (replaces "Run")
- [ ] Paused state shows a frost/dim overlay on remaining task cards
- [ ] Completed task cards remain green, paused tasks show amber
- [ ] "Resume" button appears when paused
- [ ] While paused, remaining task cards become editable (description, files, depends_on)

**Acceptance criteria**:
- Start a multi-task plan, pause mid-execution
- Completed tasks stay completed, remaining tasks show paused
- Edit a remaining task description while paused
- Resume -- execution continues from the snapshot
- Events stream correctly through pause/resume transitions

---

### 4.7 Conversation-as-plan-editor (basic)

Chat input that sends a message to an agent, receives structured mutations, and applies them to the plan canvas.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/19-visual-composition.md` (lines 47-384, mutation protocol, chat endpoint contract, mutation types)
- `/Users/will/dev/nunchi/nunchi-dashboard/docs/plan/dashboard-roko-integration.md` (conversation-as-plan-editor flow)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` (existing plan routes to extend)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/runtime.rs` (CliRuntime trait for LLM dispatch)

**Target (backend -- NEW ROUTE)**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` -- add `plan_chat` handler
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/plan_types.rs` -- add `PlanMutation`, `TaskSpec`, `TaskPatch` types

**Target (frontend)**:
- `src/pages/forge/PlanChat.tsx` or chat drawer component in `PlanDetail.tsx`

**API contract**:

```
POST /api/plans/{id}/chat
Content-Type: application/json

{
  "message": "Add a test task after the implementation",
  "context": {
    "selected_tasks": ["t2"],
    "viewport": "card_stack"
  }
}

Response 200:
{
  "reply": "Added a test task that depends on the implementation task.",
  "mutations": [
    {
      "op": "add_task",
      "task": {
        "id": "t3",
        "title": "Write integration tests",
        "description": "Test the JWT middleware",
        "agent_profile": "coding",
        "depends_on": ["t2"],
        "est_minutes": 8
      },
      "after": "t2"
    }
  ],
  "rejected": [],
  "plan_state": {
    "task_count": 3,
    "dependency_count": 2,
    "est_total_minutes": 28
  }
}
```

**Implementation (backend)**:
- [ ] Add mutation types to `plan_types.rs`: `PlanMutation` enum with variants `AddTask`, `RemoveTask`, `UpdateTask`, `AddDependency`, `RemoveDependency`, `Reorder`, `SetParallel`, `AddCheckpoint`, `UpdatePlanMeta`
- [ ] Add `TaskSpec` struct with fields: `id, title, description, agent_profile, model, repo, depends_on, files, est_minutes, budget_usd, gate_pipeline`
- [ ] Add `TaskPatch` struct with all-optional versions of `TaskSpec` fields
- [ ] Implement `plan_chat` handler:
  1. Load the current plan
  2. Build a system prompt that includes: current plan state as JSON, the `PlanMutation` schema, instructions to return both natural language and structured mutations
  3. Call `CliRuntime::run_once` with the assembled prompt (or better: call the agent backend directly for structured output)
  4. Parse the LLM response to extract `reply` and `mutations`
  5. Validate mutations (reject duplicate task IDs, reject cycles, reject refs to nonexistent tasks)
  6. Apply valid mutations to the plan file on disk
  7. Return response with `reply`, `mutations`, `rejected`, `plan_state`
- [ ] Register route: `.route("/plans/{id}/chat", post(plan_chat))`
- [ ] Emit `ServerEvent::PlanMutationApplied` event

**Implementation (frontend)**:
- [ ] Add a chat drawer component (right side, collapsible) to the plan detail/creation page
- [ ] Text input at the bottom, message history above
- [ ] On send: POST to `/api/plans/{id}/chat`, show the reply in chat
- [ ] Apply returned mutations to the local plan state -- add/remove/update cards
- [ ] Animate card additions (slide in) and removals (fade out)
- [ ] Show rejected mutations as inline warnings in the chat

**Acceptance criteria**:
- Open a plan, open the chat drawer
- Type "add a test task after t2"
- A new task card appears in the plan view
- Chat shows the agent's explanation
- The plan file on disk is updated

---

### 4.8 Cost estimation

Show estimated cost and time before running a plan.

**Read**:
- `/Users/will/dev/nunchi/roko/roko/tmp/architecture/19-visual-composition.md` (lines 765-845, cost projection algorithm and estimate endpoint)
- `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/model_router.rs` (CascadeRouter for model pricing)
- `/Users/will/dev/nunchi/roko/roko/.roko/learn/efficiency.jsonl` (historical efficiency data)

**Target (backend -- NEW ROUTE)**:
- `/Users/will/dev/nunchi/roko/roko/crates/roko-serve/src/routes/plans.rs` -- add `estimate_plan` handler

**Target (frontend)**:
- Extend `PlanDetail.tsx` with cost/time display before the "Run" button

**API contract**:

```
POST /api/plans/{id}/estimate

Response 200:
{
  "total_usd": 1.80,
  "per_task": [
    {
      "task_id": "t1",
      "model": "claude-sonnet-4-6",
      "estimated_input_tokens": 4000,
      "estimated_output_tokens": 2000,
      "estimated_usd": 0.25,
      "estimated_minutes": 5
    }
  ],
  "time_estimate_mins": 28,
  "critical_path_mins": 20,
  "confidence": 0.5
}
```

**Implementation (backend)**:
- [ ] Implement `estimate_plan` handler:
  1. Load the plan
  2. For each task, estimate token usage based on description length, file count, and agent profile
  3. Look up model pricing from `RokoConfig` (provider section) or hardcoded fallback table
  4. Query `.roko/learn/efficiency.jsonl` for historical data on similar tasks (match by agent profile and description length band)
  5. When historical data exists, use p50 actual tokens; otherwise fall back to heuristic (description_tokens * 3 for input, description_tokens * 2 for output)
  6. Compute `critical_path_mins` by finding the longest chain through the dependency DAG
  7. Set `confidence` based on the fraction of tasks with historical matches (0.5 = half heuristic)
  8. Return the estimate
- [ ] Register route: `.route("/plans/{id}/estimate", post(estimate_plan))`

**Implementation (frontend)**:
- [ ] Show "Estimated: ~$X.XX, ~Y min" above the "Run" button on the plan detail page
- [ ] Call `POST /api/plans/{id}/estimate` when the plan loads and whenever tasks change
- [ ] Expandable breakdown: per-task model, tokens, cost, time
- [ ] Confidence indicator: low (<0.5) shows "rough estimate", high (>0.8) shows "based on historical data"

**Acceptance criteria**:
- Open a plan with 3+ tasks
- Cost estimate appears before clicking Run
- Estimate updates when tasks are added/removed via the chat editor
- Per-task breakdown is visible on expand

---

## New roko-serve routes summary

These routes do not exist today and must be created:

| Route | Handler | File |
|-------|---------|------|
| `POST /api/plans/{id}/pause` | `pause_plan` | `crates/roko-serve/src/routes/plans.rs` |
| `POST /api/plans/{id}/resume` | `resume_plan` | `crates/roko-serve/src/routes/plans.rs` |
| `POST /api/plans/{id}/chat` | `plan_chat` | `crates/roko-serve/src/routes/plans.rs` |
| `POST /api/plans/{id}/estimate` | `estimate_plan` | `crates/roko-serve/src/routes/plans.rs` |

## New types summary

These types must be added to `crates/roko-serve/src/plan_types.rs`:

| Type | Fields |
|------|--------|
| `PlanStatus` | `Draft`, `Executing { execution_id }`, `Paused { snapshot_id }`, `Completed { execution_id, duration_secs }`, `Failed { execution_id, reason }` |
| `PlanMutation` | Tagged enum: `AddTask`, `RemoveTask`, `UpdateTask`, `AddDependency`, `RemoveDependency`, `Reorder`, `SetParallel`, `AddCheckpoint`, `UpdatePlanMeta` |
| `TaskSpec` | `id, title, description, agent_profile, model, repo, depends_on, files, est_minutes, budget_usd, gate_pipeline` |
| `TaskPatch` | All-optional version of `TaskSpec` fields |
| `PlanMetaPatch` | `name, description, error_handling` (all optional) |
| `ErrorPolicy` | `StopOnFailure`, `SkipAndContinue`, `Retry { max_attempts }`, `PauseOnFailure` |
| `CostEstimate` | `total_usd, per_task, time_estimate_mins, critical_path_mins, confidence` |

## New events summary

Add to `crates/roko-serve/src/events.rs`:

| Event | Fields |
|-------|--------|
| `PlanPaused` | `plan_id, completed_tasks, remaining_tasks, snapshot_id` |
| `PlanResumed` | `plan_id, resuming_from, remaining_tasks` |
| `PlanMutationApplied` | `plan_id, mutation_count, rejected_count, new_task_count` |

---

## Dependency graph

```
4.1 (plan list) ─────────────────────────────────────┐
4.2 (plan detail) ───────────────────────────────────├── independent of each other
4.3 (plan creation) ─────────────────────────────────┘
                              │
                              ▼
4.4 (execution monitor) ──── requires 4.2 for the card stack
                              │
4.5 (gate results) ────────── extends 4.4
                              │
4.6 (pause/resume) ────────── extends 4.4, requires NEW backend routes
                              │
4.7 (chat editor) ──────────── requires 4.2 or 4.3, requires NEW backend route
                              │
4.8 (cost estimation) ──────── requires 4.2, requires NEW backend route
```

Tasks 4.1, 4.2, and 4.3 can run in parallel. Tasks 4.4 and 4.7 can run in parallel once 4.2 is done. Tasks 4.5, 4.6, and 4.8 depend on 4.4 or the backend routes they introduce.
