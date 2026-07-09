# PAH_02: Enrich task detail modal with full Mori-parity fields

## Task
Extend the TUI task detail modal to show all task metadata fields, not just gate results.

## Runner Context
Runner PAH (UX & Data Model), batch 2 of 3. Depends on PAH_01.

## Problem
UX-3 gap: The TUI task detail modal exists but only shows gate results. Missing: parallel_group, exclusive_files, dependencies, blocked_by, files, acceptance criteria, context_files, model_hint, role, elapsed time, retry count.

## Current Code

**TaskRow** — `crates/roko-cli/src/tui/state.rs:886-895`:
```rust
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub elapsed_secs: Option<f64>,
}
```
Only 4 fields displayed.

**Task detail rendering** — find `task_detail` or `detail_modal` in `crates/roko-cli/src/tui/`.

## Exact Changes

### Step 1: Extend TaskRow with more fields from TaskSummary (PAH_01)

If PAH_01 has run, use `TaskSummary`. Otherwise extend TaskRow:

```rust
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub elapsed_secs: Option<f64>,
    // New fields:
    pub role: Option<String>,
    pub model_hint: Option<String>,
    pub depends_on: Vec<String>,
    pub files: Vec<String>,
    pub acceptance: Option<String>,
    pub gate_results: Vec<GateResultRow>,
    pub retry_count: u32,
    pub max_retries: u32,
}
```

### Step 2: Populate new fields from TaskDef

In the TUI state update path where TaskRow is constructed:

```rust
TaskRow {
    id: task.id.clone(),
    title: task.title.clone(),
    status: ...,
    elapsed_secs: ...,
    role: task.role.as_ref().map(|r| r.to_string()),
    model_hint: task.model_hint.clone(),
    depends_on: task.depends_on.clone().unwrap_or_default(),
    files: task.files.clone().unwrap_or_default(),
    acceptance: task.acceptance.clone(),
    gate_results: vec![],  // populated after gate runs
    retry_count: 0,
    max_retries: task.max_retries.unwrap_or(3),
}
```

### Step 3: Render new fields in detail modal

```rust
// In the task detail modal rendering:
fn render_task_detail(task: &TaskRow, area: Rect, buf: &mut Buffer) {
    let sections = vec![
        ("Role", task.role.as_deref().unwrap_or("default")),
        ("Model", task.model_hint.as_deref().unwrap_or("auto")),
        ("Dependencies", &task.depends_on.join(", ")),
        ("Files", &task.files.join(", ")),
        ("Acceptance", task.acceptance.as_deref().unwrap_or("none")),
        ("Retries", &format!("{}/{}", task.retry_count, task.max_retries)),
    ];

    // Render as table or block list
    for (label, value) in sections {
        // ... ratatui rendering ...
    }

    // Gate results section
    if !task.gate_results.is_empty() {
        // ... existing gate rendering ...
    }
}
```

## Write Scope
- `crates/roko-cli/src/tui/state.rs` (extend TaskRow)
- `crates/roko-cli/src/tui/` (task detail modal rendering)

## Read-Only Context
- `crates/roko-cli/src/task_parser.rs` (TaskDef fields available)


## Verify
```bash
cargo build -p roko-cli 2>&1 | head -30
cargo test -p roko-cli 2>&1 | tail -20
```
## Acceptance Criteria
- Task detail modal shows: role, model, dependencies, files, acceptance, retries
- All new fields sourced from TaskDef at construction time
- Missing optional fields show "none" or "auto" (not empty)
- Existing gate results display unchanged
- Modal renders within the existing layout

## Do NOT
- Change the task detail modal's open/close behavior
- Add editable fields (read-only display)
- Remove existing gate result rendering
