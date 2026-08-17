# Batch ACP13 — Plan phase bridge

## Goal

Map Roko plan phase transitions to ACP plan notifications that render as checklists in the editor.

## Target files

- `crates/roko-acp/src/bridge_plan.rs` — Plan phase bridge

## Implementation details

### Phase mapping

Roko's internal phases map to plan entries:

| Roko Phase | Plan Entry Status |
|-----------|-------------------|
| Enriching | in_progress |
| Implementing | in_progress |
| Gating | in_progress |
| Verifying | in_progress |
| Reviewing | in_progress |
| Merging | in_progress |
| (completed phases) | completed |
| (future phases) | pending |

### Functions

1. **`build_plan_entries(current_phase: &str, task_list: &[TaskInfo]) -> Vec<PlanEntry>`**
   - Builds a flat list of plan entries from the current phase and task list
   - Each task becomes an entry with appropriate priority and status
   - Current phase tasks are `in_progress`, completed are `completed`, future are `pending`

2. **`phase_transition_notification(phase: &str, entries: &[PlanEntry]) -> SessionUpdate`**
   - Returns `SessionUpdate::Plan { entries }`

3. **`task_status_to_plan_status(task_status: &str) -> PlanStatus`**
   - Maps internal task statuses to ACP plan statuses

### TaskInfo struct (local bridge type)

```rust
pub struct TaskInfo {
    pub name: String,
    pub phase: String,
    pub status: String,
    pub priority: Priority,
}
```

### Example notification

```json
{
    "sessionUpdate": "plan",
    "entries": [
        {"content": "Analyze codebase structure", "priority": "high", "status": "completed"},
        {"content": "Implement session manager", "priority": "high", "status": "in_progress"},
        {"content": "Gate: compile check", "priority": "high", "status": "pending"},
        {"content": "Gate: run tests", "priority": "high", "status": "pending"},
        {"content": "Review and refine", "priority": "medium", "status": "pending"}
    ]
}
```

## Verification

```bash
cargo check -p roko-acp
cargo clippy -p roko-acp --no-deps -- -D warnings
```

## Done when

- Plan entries reflect current execution state
- Phase transitions produce correct plan notifications
- All phases are mapped correctly
- Priority and status values are correct
