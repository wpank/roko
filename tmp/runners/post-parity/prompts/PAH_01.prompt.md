# PAH_01: Create canonical TaskState shared between TUI/serve/orchestrator

## Task
Introduce a single canonical `TaskState` type that all consumers (TUI, serve routes, runner, snapshots) reference instead of 5+ separate copies.

## Runner Context
Runner PAH (UX & Data Model), batch 1 of 3. No dependencies.

## Problem
UX-1 anti-pattern: "5+ task state representations, zero shared type." Adding a field to task state requires changes in 5+ places. Current copies:

| Type | Location | Fields |
|------|----------|--------|
| `TaskDef` | `task_parser.rs:49-98` | 22 fields (canonical definition) |
| `TaskRow` | `tui/state.rs:886-895` | 4 fields (id, title, status, elapsed_secs) |
| `TaskState` | `dashboard_snapshot.rs:172` | Dashboard projection |
| `PlanTaskSnapshot` | `tui/dashboard.rs:1145` | TUI snapshot |
| `TaskStatus` (4 copies) | roko_core::task, roko_runtime::task_scheduler, tui::state, plan.rs | All different enums |
| `PlanTask` | `plan.rs:113`, `plan_types.rs:68` | Plan display types |

## Exact Changes

### Step 1: Create shared TaskSummary in roko-core

Add a lightweight shared type that all consumers can use:

```rust
// In crates/roko-core/src/task.rs (or extend existing):

/// Canonical task state shared between TUI, serve routes, and runner.
/// All consumers convert FROM their internal types TO this for display/API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub title: String,
    pub status: UnifiedTaskStatus,
    pub role: Option<String>,
    pub model_hint: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub gate_status: Option<GateSummary>,
    pub depends_on: Vec<String>,
    pub files: Vec<String>,
    pub acceptance: Option<String>,
}

/// Unified status enum (superset of all existing status types)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UnifiedTaskStatus {
    Pending,
    Blocked,     // waiting on dependencies
    Ready,       // dependencies met, can run
    Running,
    GateRunning, // agent done, gates in progress
    Passed,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateSummary {
    pub gates_run: u32,
    pub gates_passed: u32,
    pub gates_total: u32,
}
```

### Step 2: Add From implementations for existing types

```rust
impl From<&TaskDef> for TaskSummary {
    fn from(task: &TaskDef) -> Self { ... }
}

impl From<&TaskRow> for TaskSummary {
    fn from(row: &TaskRow) -> Self { ... }
}
```

### Step 3: Use TaskSummary in TUI rendering

```rust
// In tui/state.rs, replace TaskRow usage with TaskSummary
// Or keep TaskRow as a view type that wraps TaskSummary:
pub struct TaskRow {
    pub summary: TaskSummary,
    pub ui_state: TaskRowUiState,  // selection, scroll position, etc.
}
```

### Step 4: Use TaskSummary in serve route responses

```rust
// In serve route handlers, return TaskSummary in JSON responses
// instead of ad-hoc structs
```

## Write Scope
- `crates/roko-core/src/task.rs` (TaskSummary, UnifiedTaskStatus, GateSummary)
- `crates/roko-cli/src/tui/state.rs` (use TaskSummary)
- `crates/roko-serve/src/plan_types.rs` (use TaskSummary)

## Read-Only Context
- `crates/roko-cli/src/task_parser.rs` (TaskDef — the canonical input type)


## Verify
```bash
cargo build --workspace 2>&1 | head -30
cargo test --workspace 2>&1 | tail -20
```
## Acceptance Criteria
- Single `TaskSummary` type in roko-core
- `UnifiedTaskStatus` is superset of all existing status enums
- TUI and serve routes can consume TaskSummary
- Adding a new display field requires change in ONE place
- Existing TaskDef not changed (it's the input type; TaskSummary is the display type)

## Do NOT
- Replace TaskDef (it's the input/parsing type, not the display type)
- Remove the existing status enums immediately (deprecate, then migrate)
- Add TaskSummary fields that aren't needed by at least 2 consumers
