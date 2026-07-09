# Task 02: Plans Endpoint Parity

**Priority**: P0
**Crate**: `roko-serve`
**File**: `crates/roko-serve/src/routes/plans.rs`

## Problem

Two gaps between what roko-serve returns and what the dashboard expects for plan endpoints.

### GET /api/plans — missing `completed_task_count`

**What roko-serve returns now:**
```json
[
  {
    "id": "plan-uuid",
    "title": "Plan Title",
    "task_count": 5,
    "completed": false
  }
]
```

**What the dashboard expects** (type `Plan` in `rokoApi.ts`):
```json
[
  {
    "id": "plan-uuid",
    "title": "Plan Title",
    "task_count": 5,
    "completed": false,
    "completed_task_count": 3
  }
]
```

The dashboard uses `completed_task_count` to render progress bars and percentage displays
in the Plans tab. Without it, progress shows as 0% or undefined.

### GET /api/plans/{id} — missing `status` per task

**What roko-serve returns now per task:**
```json
{
  "id": "T1",
  "description": "Task description",
  "depends_on": ["T0"],
  "files": ["src/main.rs"],
  "completed": false
}
```

**What the dashboard expects:**
```json
{
  "id": "T1",
  "description": "Task description",
  "depends_on": ["T0"],
  "files": ["src/main.rs"],
  "completed": false,
  "status": "pending"
}
```

The `status` field is used by the dashboard's `PlanCard` component and the Plans detail view
to show task states (`"pending"`, `"running"`, `"completed"`, `"failed"`). The Atelier chat
`runMockPlanReplay` also sets `status` per task as execution progresses.

## Implementation

### Step 1: Add `completed_task_count` to plan list

In the GET `/api/plans` handler (around lines 29-61 of `plans.rs`):

1. When iterating plans from `.roko/plans/` directory, count tasks with `completed: true`
2. Add `completed_task_count: usize` to the plan summary serialization struct

This is a simple count — you're already loading the plan to get `task_count`, so counting
completed tasks is trivial.

**Struct change:**
```rust
#[derive(Serialize)]
struct PlanSummary {
    id: String,
    title: String,
    task_count: usize,
    completed: bool,
    completed_task_count: usize,  // ADD THIS
}
```

### Step 2: Add `status` to plan detail tasks

In the GET `/api/plans/{id}` handler (around lines 63-70, 376-389):

1. Derive `status` from the task's state:
   - If `completed == true` → `"completed"`
   - If task is currently being executed (check active runs in AppState) → `"running"`
   - If task has a failed gate result → `"failed"`
   - Otherwise → `"pending"`
2. Add `status: String` to the task serialization struct

**Check the executor state** — `AppState` tracks active plans and their progress. The
`PlanHandle` or similar struct likely has per-task status. Use that if available rather
than re-deriving from files.

**Struct change:**
```rust
#[derive(Serialize)]
struct TaskSummary {
    id: String,
    description: String,
    depends_on: Vec<String>,
    files: Vec<String>,
    completed: bool,
    status: String,  // ADD THIS: "pending" | "running" | "completed" | "failed"
}
```

### Step 3: Verify plan execution updates status

When `POST /api/plans/{id}/execute` runs tasks, the plan detail endpoint should reflect
live status. Verify:

1. Before execution: all tasks show `"pending"`
2. During execution: active task shows `"running"`, completed ones show `"completed"`
3. After execution: all tasks show `"completed"` or `"failed"`

This may already work if AppState tracks execution progress — check `PlanHandle` fields.

## Files to modify

| File | Change |
|------|--------|
| `crates/roko-serve/src/routes/plans.rs` | Add `completed_task_count` and `status` fields |
| `crates/roko-serve/src/state.rs` | May need to expose task-level status from PlanHandle (only if not already accessible) |

## Verification

### Automated

```bash
cargo build -p roko-serve
cargo test -p roko-serve
cargo clippy -p roko-serve --no-deps -- -D warnings
```

### Manual — completed_task_count

```bash
cargo run -p roko-cli -- serve &

# List plans
PLANS=$(curl -s http://127.0.0.1:6677/api/plans)

# Verify completed_task_count exists and is a number
echo "$PLANS" | jq '.[0] | has("completed_task_count")'
# MUST be true

echo "$PLANS" | jq '.[0].completed_task_count | type'
# MUST be "number"

# Verify it's <= task_count
echo "$PLANS" | jq '.[0] | .completed_task_count <= .task_count'
# MUST be true
```

### Manual — task status

```bash
# Get a specific plan
PLAN_ID=$(curl -s http://127.0.0.1:6677/api/plans | jq -r '.[0].id')
DETAIL=$(curl -s "http://127.0.0.1:6677/api/plans/${PLAN_ID}")

# Verify tasks have status field
echo "$DETAIL" | jq '.tasks[0] | has("status")'
# MUST be true

# Verify status is a valid string
echo "$DETAIL" | jq '.tasks[0].status'
# MUST be one of: "pending", "running", "completed", "failed"

# Verify consistency: completed == true iff status == "completed"
echo "$DETAIL" | jq '.tasks[] | select(.completed == true) | .status'
# All MUST be "completed"

echo "$DETAIL" | jq '.tasks[] | select(.completed == false) | .status'
# All MUST be "pending", "running", or "failed" — NOT "completed"
```

### Manual — live execution status

```bash
# If a plan exists with tasks, execute it and poll status
curl -s -X POST "http://127.0.0.1:6677/api/plans/${PLAN_ID}/execute"

# Poll plan detail during execution
for i in $(seq 1 10); do
  curl -s "http://127.0.0.1:6677/api/plans/${PLAN_ID}" | \
    jq '[.tasks[] | .status] | group_by(.) | map({(.[0]): length}) | add'
  sleep 2
done
# Should see status progression: mostly "pending" → some "running" → "completed"/"failed"
```

## Acceptance criteria

- [ ] `GET /api/plans` returns `completed_task_count` (number) for every plan
- [ ] `completed_task_count` equals the count of tasks where `completed == true`
- [ ] `GET /api/plans/{id}` returns `status` (string) for every task
- [ ] `status` is one of: `"pending"`, `"running"`, `"completed"`, `"failed"`
- [ ] `status` and `completed` are consistent (completed==true ↔ status=="completed")
- [ ] During plan execution, task statuses update in real-time when polled
- [ ] All existing tests still pass
- [ ] No new clippy warnings
