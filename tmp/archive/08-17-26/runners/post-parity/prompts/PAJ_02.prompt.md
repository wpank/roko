# PAJ_02: Wire durable BackgroundTaskSupervisor for serve routes

## Task
Add durable tracking for background tasks spawned by serve routes so they survive server restarts and orphaned tasks are detected.

## Runner Context
Runner PAJ (Projection & Background Tasks), batch 2 of 3. No dependencies.

## Problem
PB-2 anti-pattern: "Routes spawn tasks without durable ownership." Serve routes spawn background operations via `tokio::spawn` with volatile in-memory tracking in `AppState`. On restart, all in-flight operations are lost.

## Current Code

**AppState** — `crates/roko-serve/src/state.rs:345`:
```rust
pub struct AppState {
    // ...
    pub active_runs: RwLock<HashMap<String, RunHandle>>,       // L381
    pub active_plans: RwLock<HashMap<String, PlanHandle>>,     // L383
    pub operations: RwLock<HashMap<String, OperationHandle>>,  // L385
    // ... all volatile, lost on restart
}
```

**Background spawn sites** (each uses `tokio::spawn` with no persistence):

| File | Line | What |
|------|------|------|
| `routes/run.rs` | 102 | One-shot run spawn |
| `routes/run.rs` | 216 | Plan run execute |
| `routes/run.rs` | 374 | Alternative plan execution |
| `routes/run.rs` | 506 | Plan mutation handler |
| `routes/run.rs` | 1159 | Plan generation background task |
| `routes/research.rs` | 213 | Research operation dispatch |
| `routes/plans.rs` | multiple | Background plan execution |

All register handles in `state.active_runs`/`active_plans`/`operations` via `write().await`, but these maps are in-memory only.

## Exact Changes

### Step 1: Add BackgroundTaskRecord types

In `crates/roko-serve/src/supervisor.rs` (new file):

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundTaskRecord {
    pub id: String,
    pub kind: TaskKind,
    pub started_at: u64,
    pub status: BackgroundTaskStatus,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskKind {
    PlanRun { plan_dir: PathBuf },
    BenchRun { bench_id: String },
    Research { topic: String },
    OneShot { prompt_preview: String },
    PlanGenerate { slug: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackgroundTaskStatus {
    Running,
    Completed { at: u64 },
    Failed { at: u64, error: String },
    Orphaned,
}

pub struct BackgroundTaskSupervisor {
    tasks: HashMap<String, BackgroundTaskRecord>,
    workdir: PathBuf,
}
```

### Step 2: Implement persistence methods

```rust
impl BackgroundTaskSupervisor {
    pub fn new(workdir: PathBuf) -> Self {
        let tasks = Self::load_from_disk(&workdir).unwrap_or_default();
        Self { tasks, workdir }
    }

    fn record_path(workdir: &Path) -> PathBuf {
        workdir.join(".roko/state/background-tasks.json")
    }

    fn load_from_disk(workdir: &Path) -> anyhow::Result<HashMap<String, BackgroundTaskRecord>> {
        let path = Self::record_path(workdir);
        let json = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&json)?)
    }

    pub fn register(&mut self, record: BackgroundTaskRecord) -> anyhow::Result<()> {
        self.tasks.insert(record.id.clone(), record);
        self.persist()
    }

    pub fn complete(&mut self, id: &str) -> anyhow::Result<()> {
        if let Some(task) = self.tasks.get_mut(id) {
            task.status = BackgroundTaskStatus::Completed {
                at: chrono::Utc::now().timestamp_millis() as u64,
            };
        }
        self.persist()
    }

    pub fn fail(&mut self, id: &str, error: String) -> anyhow::Result<()> {
        if let Some(task) = self.tasks.get_mut(id) {
            task.status = BackgroundTaskStatus::Failed {
                at: chrono::Utc::now().timestamp_millis() as u64,
                error,
            };
        }
        self.persist()
    }

    fn persist(&self) -> anyhow::Result<()> {
        let path = Self::record_path(&self.workdir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&self.tasks)?;
        std::fs::write(&path, json.as_bytes())?;
        Ok(())
    }

    /// On startup, mark any Running tasks as Orphaned
    pub fn recover(&mut self) -> Vec<BackgroundTaskRecord> {
        let mut orphaned = Vec::new();
        for task in self.tasks.values_mut() {
            if matches!(task.status, BackgroundTaskStatus::Running) {
                task.status = BackgroundTaskStatus::Orphaned;
                orphaned.push(task.clone());
            }
        }
        if !orphaned.is_empty() {
            let _ = self.persist();
        }
        orphaned
    }
}
```

### Step 3: Add supervisor to AppState

In `crates/roko-serve/src/state.rs`, add field to AppState (around line 385):

```rust
pub task_supervisor: Arc<Mutex<BackgroundTaskSupervisor>>,
```

In `AppState::new()` (around line 526):
```rust
let mut supervisor = BackgroundTaskSupervisor::new(workdir.to_path_buf());
let orphaned = supervisor.recover();
for task in &orphaned {
    tracing::warn!(task_id = %task.id, kind = ?task.kind, "orphaned background task");
}
let task_supervisor = Arc::new(Mutex::new(supervisor));
```

### Step 4: Register at each spawn site

At each `tokio::spawn` call in the route handlers, register before spawn and complete/fail after:

Example for `routes/run.rs:102`:
```rust
// Before tokio::spawn:
let task_id = uuid::Uuid::new_v4().to_string();
{
    let mut sup = state.task_supervisor.lock().await;
    sup.register(BackgroundTaskRecord {
        id: task_id.clone(),
        kind: TaskKind::OneShot { prompt_preview: prompt[..80.min(prompt.len())].to_string() },
        started_at: chrono::Utc::now().timestamp_millis() as u64,
        status: BackgroundTaskStatus::Running,
        pid: None,
    })?;
}

// Inside tokio::spawn, after completion:
let result = do_work().await;
{
    let mut sup = state.task_supervisor.lock().await;
    match &result {
        Ok(_) => { let _ = sup.complete(&task_id); }
        Err(e) => { let _ = sup.fail(&task_id, e.to_string()); }
    }
}
```

Repeat for all 7+ spawn sites listed above.

## Write Scope
- `crates/roko-serve/src/supervisor.rs` (new: BackgroundTaskSupervisor, record types)
- `crates/roko-serve/src/lib.rs` (add `pub mod supervisor;`)
- `crates/roko-serve/src/state.rs` (add supervisor to AppState)
- `crates/roko-serve/src/routes/run.rs` (register at spawn sites: lines 102, 216, 374, 506, 1159)
- `crates/roko-serve/src/routes/research.rs` (register at line 213)
- `crates/roko-serve/src/routes/plans.rs` (register at spawn sites)

## Read-Only Context
- `crates/roko-serve/src/state.rs:345-385` (AppState struct)
- `crates/roko-serve/src/state.rs:526-528` (AppState::new)

## Verify
```bash
cargo build -p roko-serve 2>&1 | head -30
cargo test -p roko-serve 2>&1 | tail -20
```

## Acceptance Criteria
- Background tasks tracked in `.roko/state/background-tasks.json`
- On server restart, orphaned tasks detected and logged with `tracing::warn`
- Records include: id, kind, timing, pid, status
- Register before spawn, complete/fail inside spawn closure
- Missing file → empty task map (no crash)
- `cargo build --workspace` passes

## Do NOT
- Auto-resume orphaned tasks (just detect and report)
- Add task cancellation via the supervisor (use existing cancel mechanisms)
- Change the route handler response API
- Hold the supervisor lock across `tokio::spawn` boundaries
