# W9-B: Cross-Task Output Injection

**Priority**: P0 -- T2 does not know what T1 produced; `depends_on` only gates execution order
**Effort**: 2 hours
**Files to modify**: 4 files
**Dependencies**: None (can be done in parallel with W9-A)

## Problem

When task T2 depends on T1 via `depends_on = ["T1"]`, the only effect is that T2 waits for T1 to finish before starting. T2's prompt contains zero information about what T1 actually produced -- which files it created, what types it defined, what APIs it exposed. The agent starts blind and frequently duplicates work or invents incompatible types.

The runner v2 event loop (`event_loop.rs`) marks tasks completed in `RunState.completed_tasks` but never records what files each task modified. The `PromptAssembler` in `prompt_builder.rs` has no concept of dependency outputs.

## Root Cause

### 1. No task output tracking in RunState
**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/state.rs` (lines 92-94)

`RunState` tracks `completed_tasks: HashMap<String, Vec<String>>` (plan_id -> list of completed task IDs) but has no field for recording what files each task modified. After a task passes its gates, the modified file list is never captured.

### 2. PromptContext/DispatchContext have no dependency output fields
**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/prompt_builder.rs` (lines 60-77)
**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/mod.rs` (lines 71-92)

Neither `PromptContext` nor `DispatchContext` carries dependency output information. The PromptAssembler cannot inject "what files T1 modified" because the data never reaches it.

### 3. No git diff after task completion
**File**: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs` (lines 700-725)

After a task passes gates (line 700 `if completion.passed`), the event loop marks the task completed and advances the DAG, but never runs `git diff` to record which files were modified by the task.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/state.rs`

#### Change 1.1: Add task_outputs field to RunState (after line 94)

**Find this code:**
```rust
    // ─── Task DAG ───────────────────────────────────────────────────
    /// Completed task IDs per plan (for DAG dependency resolution).
    pub completed_tasks: HashMap<String, Vec<String>>,
```

**Replace with:**
```rust
    // ─── Task DAG ───────────────────────────────────────────────────
    /// Completed task IDs per plan (for DAG dependency resolution).
    pub completed_tasks: HashMap<String, Vec<String>>,

    // ─── Cross-task outputs ─────────────────────────────────────────
    /// Files modified by each completed task, keyed by "{plan_id}:{task_id}".
    /// Populated after each task passes gates so downstream tasks can see
    /// what their dependencies produced.
    pub task_outputs: HashMap<String, Vec<String>>,
```

#### Change 1.2: Initialize task_outputs in RunState::new()

**Find this code:**
```rust
            completed_tasks: HashMap::new(),
            snapshot_fail_streak: 0,
```

**Replace with:**
```rust
            completed_tasks: HashMap::new(),
            task_outputs: HashMap::new(),
            snapshot_fail_streak: 0,
```

#### Change 1.3: Add helper methods for task output recording and retrieval

Add these methods at the end of `impl RunState`, just before the closing `}` of the impl block (after the `take_replan_context` method around line 552):

```rust
    /// Record the files modified by a completed task.
    pub fn record_task_outputs(&mut self, plan_id: &str, task_id: &str, files: Vec<String>) {
        let key = format!("{plan_id}:{task_id}");
        tracing::debug!(
            plan_id,
            task_id,
            file_count = files.len(),
            "recording task output files"
        );
        self.task_outputs.insert(key, files);
    }

    /// Get files modified by a specific task (for cross-task injection).
    pub fn task_output_files(&self, plan_id: &str, task_id: &str) -> &[String] {
        let key = format!("{plan_id}:{task_id}");
        self.task_outputs
            .get(&key)
            .map(|v| v.as_slice())
            .unwrap_or_default()
    }

    /// Get output summaries for all dependency tasks.
    pub fn dependency_outputs(
        &self,
        plan_id: &str,
        depends_on: &[String],
    ) -> Vec<(String, Vec<String>)> {
        depends_on
            .iter()
            .filter_map(|dep_id| {
                let files = self.task_output_files(plan_id, dep_id);
                if files.is_empty() {
                    None
                } else {
                    Some((dep_id.clone(), files.to_vec()))
                }
            })
            .collect()
    }
```

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

#### Change 2.1: Record modified files after task passes gates (line 723)

**Find this code:**
```rust
                    tui.task_completed(&completion.plan_id, &completion.task_id, "passed");

                    let total_task_ms = state.task_elapsed_ms();
```

**Replace with:**
```rust
                    tui.task_completed(&completion.plan_id, &completion.task_id, "passed");

                    // Record files modified by this task for cross-task injection.
                    let modified_files = git_diff_names_since_task_start(&config.workdir);
                    if !modified_files.is_empty() {
                        debug!(
                            plan_id = %completion.plan_id,
                            task_id = %completion.task_id,
                            file_count = modified_files.len(),
                            "recording task output files for downstream tasks"
                        );
                    }
                    state.record_task_outputs(
                        &completion.plan_id,
                        &completion.task_id,
                        modified_files,
                    );

                    let total_task_ms = state.task_elapsed_ms();
```

#### Change 2.2: Add the git_diff_names helper function

Add this function near the end of the file, before the `#[cfg(test)]` module (or if there is no test module, at the end). It should be a module-level function, not inside any impl block.

```rust
/// Get list of files modified since the last commit (or in the working tree).
///
/// Uses `git diff --name-only HEAD` to capture both staged and unstaged changes.
/// Falls back to `git status --porcelain` if HEAD doesn't exist.
fn git_diff_names_since_task_start(workdir: &Path) -> Vec<String> {
    // Try git diff --name-only HEAD first
    let output = std::process::Command::new("git")
        .args(["diff", "--name-only", "HEAD"])
        .current_dir(workdir)
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut files: Vec<String> = stdout
                .lines()
                .filter(|l| !l.trim().is_empty())
                .map(|l| l.trim().to_string())
                .collect();

            // Also check for new untracked files
            if let Ok(status_out) = std::process::Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(workdir)
                .output()
            {
                let status_str = String::from_utf8_lossy(&status_out.stdout);
                for line in status_str.lines() {
                    if line.starts_with("??") || line.starts_with("A ") {
                        if let Some(file) = line.get(3..) {
                            let file = file.trim().to_string();
                            if !files.contains(&file) {
                                files.push(file);
                            }
                        }
                    }
                }
            }

            // Limit to 50 files to avoid bloating the prompt
            files.truncate(50);
            files
        }
        _ => {
            // Fallback: list all files in working tree changes
            let status = std::process::Command::new("git")
                .args(["status", "--porcelain"])
                .current_dir(workdir)
                .output();
            match status {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    stdout
                        .lines()
                        .filter(|l| !l.trim().is_empty())
                        .filter_map(|l| l.get(3..).map(|s| s.trim().to_string()))
                        .take(50)
                        .collect()
                }
                Err(_) => Vec::new(),
            }
        }
    }
}
```

### File 3: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/mod.rs`

#### Change 3.1: Add dependency_outputs to DispatchContext (line 71)

**Find this code:**
```rust
#[derive(Debug, Clone)]
pub struct DispatchContext {
    /// Plan id this task belongs to.
    pub plan_id: String,
    /// Logical role name (`"implementer"`, `"reviewer"`, ...).
    pub role: String,
    /// Working directory for the agent.
    pub workdir: std::path::PathBuf,
    /// Optional explicit model override from CLI / config (`task.model_hint`).
    pub model_hint: Option<String>,
    /// Optional `force_backend` override (manual operator decision).
    pub force_backend: Option<String>,
    /// Remaining USD budget for the plan; the router uses this to bias
    /// toward cheaper models when the budget is nearly exhausted.
    pub budget_remaining_usd: f64,
    /// Attempt number for this task (0 = first try, > 0 = retry).
    pub attempt: u32,
    /// Optional structured feedback from a previous gate failure.
    pub gate_feedback: Option<GateFeedback>,
    /// Routing context for the CascadeRouter. Built at the dispatch site
    /// from task + runner state, threaded through to `RoutingInputs`.
    pub routing_context: Option<RoutingContext>,
}
```

**Replace with:**
```rust
#[derive(Debug, Clone)]
pub struct DispatchContext {
    /// Plan id this task belongs to.
    pub plan_id: String,
    /// Logical role name (`"implementer"`, `"reviewer"`, ...).
    pub role: String,
    /// Working directory for the agent.
    pub workdir: std::path::PathBuf,
    /// Optional explicit model override from CLI / config (`task.model_hint`).
    pub model_hint: Option<String>,
    /// Optional `force_backend` override (manual operator decision).
    pub force_backend: Option<String>,
    /// Remaining USD budget for the plan; the router uses this to bias
    /// toward cheaper models when the budget is nearly exhausted.
    pub budget_remaining_usd: f64,
    /// Attempt number for this task (0 = first try, > 0 = retry).
    pub attempt: u32,
    /// Optional structured feedback from a previous gate failure.
    pub gate_feedback: Option<GateFeedback>,
    /// Routing context for the CascadeRouter. Built at the dispatch site
    /// from task + runner state, threaded through to `RoutingInputs`.
    pub routing_context: Option<RoutingContext>,
    /// Files modified by dependency tasks, for cross-task context injection.
    /// Each entry is (task_id, vec_of_modified_files).
    pub dependency_outputs: Vec<(String, Vec<String>)>,
}
```

### File 4: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch/prompt_builder.rs`

#### Change 4.1: Add dependency_outputs field to PromptContext

If W9-A has already been applied, add this field after `prd_excerpt`. If not, add after `attempt`.

**Find this code** (the closing fields of PromptContext -- use the `attempt` field as anchor):
```rust
    /// Attempt number (0 = first, > 0 = retry).
    pub attempt: u32,
```

**Add after it** (either after `attempt` or after `prd_excerpt` if W9-A applied):
```rust
    /// Files modified by dependency tasks (for cross-task context injection).
    /// Each entry is (task_id, vec_of_files).
    pub dependency_outputs: Vec<(String, Vec<String>)>,
```

#### Change 4.2: Populate dependency_outputs in from_task

In `PromptContext::from_task()`, add the field to the Self constructor:

**Find this code** (the last field in the Self constructor):
```rust
            gate_feedback: ctx.gate_feedback.clone(),
            attempt: ctx.attempt,
        }
```

**Replace with** (if W9-A not yet applied):
```rust
            gate_feedback: ctx.gate_feedback.clone(),
            attempt: ctx.attempt,
            dependency_outputs: ctx.dependency_outputs.clone(),
        }
```

Or (if W9-A already applied -- the last three fields will be workspace_map, tasks_toml, prd_excerpt):
```rust
            workspace_map,
            tasks_toml,
            prd_excerpt,
            dependency_outputs: ctx.dependency_outputs.clone(),
        }
```

#### Change 4.3: Inject dependency outputs section in assemble()

In `PromptAssembler::assemble()`, after the allowlist section push (or after the prd_excerpt/workspace_map/tasks_toml pushes if W9-A applied), add:

**Find this code:**
```rust
        let mut diagnostics = PromptDiagnostics::default();
```

**Add BEFORE it:**
```rust
        // Cross-task outputs — what dependency tasks produced
        if !ctx.dependency_outputs.is_empty() {
            let mut dep_text = String::from(
                "# Prior Task Outputs\n\n\
                 These tasks have already completed. Use their output files \
                 instead of reimplementing.\n",
            );
            for (task_id, files) in &ctx.dependency_outputs {
                dep_text.push_str(&format!(
                    "\n## Completed by task {task_id}:\nFiles created/modified:\n"
                ));
                for f in files {
                    dep_text.push_str(&format!("- `{f}`\n"));
                }
            }
            sections.push(PromptSection::new(
                "dependency_outputs",
                dep_text,
                7, // same priority as prd_excerpt — important context
            ));
            tracing::debug!(
                dep_count = ctx.dependency_outputs.len(),
                "injected dependency outputs into prompt"
            );
        }

```

#### Change 4.4: Add dependency_outputs to canonical section ordering

**Find this code:**
```rust
        let canonical: &[&str] = &[
            "role",
            "task",
            "files",
            "acceptance",
            "verify",
            "retry",
            "allowlist",
            "knowledge",
            "episode_knowledge",
            "playbooks",
            "section_effectiveness",
        ];
```

**Replace with** (if W9-A not yet applied):
```rust
        let canonical: &[&str] = &[
            "role",
            "task",
            "files",
            "acceptance",
            "verify",
            "dependency_outputs",
            "retry",
            "allowlist",
            "knowledge",
            "episode_knowledge",
            "playbooks",
            "section_effectiveness",
        ];
```

Or (if W9-A already applied, which adds prd_excerpt/workspace_map/tasks_toml):
```rust
        let canonical: &[&str] = &[
            "role",
            "task",
            "prd_excerpt",
            "files",
            "acceptance",
            "verify",
            "dependency_outputs",
            "retry",
            "allowlist",
            "workspace_map",
            "tasks_toml",
            "knowledge",
            "episode_knowledge",
            "playbooks",
            "section_effectiveness",
        ];
```

### File 5: Fix ALL DispatchContext constructor sites

Every place that constructs `DispatchContext { ... }` MUST add `dependency_outputs: Vec::new()` (or the real value).

#### 5a. Production constructor in event_loop.rs (line 2144)

**Find this code:**
```rust
            let dispatch_ctx = DispatchContext {
                plan_id: plan_id.clone(),
                role: role.to_string(),
                workdir: ctx.config.workdir.clone(),
                model_hint: Some(ctx.config.model.clone()),
                force_backend: ctx.config.cli_model_override.clone(),
                budget_remaining_usd: if ctx.config.max_plan_usd > 0.0 {
                    (ctx.config.max_plan_usd - ctx.state.plan_cost(plan_id)).max(0.0)
                } else {
                    f64::INFINITY
                },
                attempt: attempt_num.saturating_sub(1),
                gate_feedback,
                routing_context: Some(routing_context),
            };
```

**Replace with:**
```rust
            let dependency_outputs = ctx.state.dependency_outputs(
                plan_id,
                &task_def.depends_on,
            );
            if !dependency_outputs.is_empty() {
                info!(
                    plan_id = %plan_id,
                    task = %task_id,
                    dep_count = dependency_outputs.len(),
                    "injecting cross-task dependency outputs"
                );
            }
            let dispatch_ctx = DispatchContext {
                plan_id: plan_id.clone(),
                role: role.to_string(),
                workdir: ctx.config.workdir.clone(),
                model_hint: Some(ctx.config.model.clone()),
                force_backend: ctx.config.cli_model_override.clone(),
                budget_remaining_usd: if ctx.config.max_plan_usd > 0.0 {
                    (ctx.config.max_plan_usd - ctx.state.plan_cost(plan_id)).max(0.0)
                } else {
                    f64::INFINITY
                },
                attempt: attempt_num.saturating_sub(1),
                gate_feedback,
                routing_context: Some(routing_context),
                dependency_outputs,
            };
```

#### 5b. Test constructor in dispatch/mod.rs (line 342)

**Find this code:**
```rust
    fn make_ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p1".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            gate_feedback: None,
            routing_context: None,
        }
    }
```

**Replace with:**
```rust
    fn make_ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p1".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            gate_feedback: None,
            routing_context: None,
            dependency_outputs: Vec::new(),
        }
    }
```

#### 5c. Test constructor in dispatch/mod.rs (line 402)

**Find this code:**
```rust
        let ctx = DispatchContext {
            force_backend: Some("gpt-5".into()),
            ..make_ctx()
        };
```

No change needed -- uses `..make_ctx()` which now has the field.

#### 5d. Test constructor in prompt_builder.rs (line 1192)

**Find this code:**
```rust
    fn ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            gate_feedback: None,
            routing_context: None,
        }
    }
```

**Replace with:**
```rust
    fn ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            gate_feedback: None,
            routing_context: None,
            dependency_outputs: Vec::new(),
        }
    }
```

#### 5e. Test constructor in model_routing.rs (line 250)

**Find this code:**
```rust
    fn ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            gate_feedback: None,
            routing_context: None,
        }
    }
```

**Replace with:**
```rust
    fn ctx() -> DispatchContext {
        DispatchContext {
            plan_id: "p".into(),
            role: "implementer".into(),
            workdir: PathBuf::from("/tmp"),
            model_hint: None,
            force_backend: None,
            budget_remaining_usd: 5.0,
            attempt: 0,
            gate_feedback: None,
            routing_context: None,
            dependency_outputs: Vec::new(),
        }
    }
```

#### 5f. Catch-all: search for any other DispatchContext constructors

Run: `grep -rn 'DispatchContext {' crates/roko-cli/src/ --include='*.rs' | grep -v target/`

For each match not covered above, add `dependency_outputs: Vec::new(),`.

## Verification

```bash
# 1. Build
cd /Users/will/dev/nunchi/roko/roko
cargo check -p roko-cli 2>&1 | head -30

# 2. Run tests
cargo test -p roko-cli --lib runner::state 2>&1 | tail -20
cargo test -p roko-cli --lib dispatch 2>&1 | tail -20

# 3. Search for any remaining DispatchContext constructors missing the new field
grep -rn 'DispatchContext {' crates/roko-cli/src/ --include='*.rs' | grep -v target/

# 4. Clippy
cargo clippy -p roko-cli --no-deps -- -D warnings 2>&1 | tail -20
```

## Agent Prompt

```
You are implementing cross-task output injection for the Roko plan runner so
downstream tasks know what their dependencies produced.

## Context

Roko is a Rust agent toolkit with 18 crates. When task T2 depends on T1 via
`depends_on = ["T1"]`, T2 only waits for T1 to complete -- T2's prompt contains
zero information about what T1 actually produced. This causes agents to duplicate
types, invent incompatible APIs, and miss files created by prior tasks.

## Architecture

The dispatch chain is:

1. `event_loop.rs` line 2144: constructs `DispatchContext`
2. `event_loop.rs` line 2162: calls `dispatcher.plan(task_def, &dispatch_ctx)`
3. `dispatch/mod.rs` line 153: calls `PromptContext::from_task(task, ctx)`
4. `prompt_builder.rs` line 321: `PromptAssembler::assemble()` builds sections

Task completion is handled at `event_loop.rs` line 700-723:
- Line 703: `state.mark_task_completed(&completion.plan_id, &completion.task_id)`
- Line 704: `state.task_completed()`
- Line 723: `tui.task_completed(...)`
- Then line 725+: timing, DAG advancement

## What to do

Follow the "Exact Code to Change" section precisely. The changes are:

### Step 1: Add `task_outputs` to RunState (state.rs)
- Add `pub task_outputs: HashMap<String, Vec<String>>` field after `completed_tasks`
- Initialize as `HashMap::new()` in `RunState::new()`
- Add methods: `record_task_outputs()`, `task_output_files()`, `dependency_outputs()`

### Step 2: Record modified files after task completion (event_loop.rs)
- After `tui.task_completed(...)` on line 723, call `git_diff_names_since_task_start()`
- Store result via `state.record_task_outputs()`
- Add `git_diff_names_since_task_start()` function that runs `git diff --name-only HEAD`
  and `git status --porcelain` to capture all modified and new files

### Step 3: Add dependency_outputs to DispatchContext (dispatch/mod.rs)
- Add `pub dependency_outputs: Vec<(String, Vec<String>)>` to `DispatchContext`

### Step 4: Add dependency_outputs to PromptContext (prompt_builder.rs)
- Add field, populate from `ctx.dependency_outputs.clone()` in `from_task()`

### Step 5: Inject dependency outputs section in assemble()
- Before `let mut diagnostics`, push a "Prior Task Outputs" section

### Step 6: Wire dependency_outputs at dispatch site (event_loop.rs line 2144)
- Before constructing DispatchContext, call `ctx.state.dependency_outputs(plan_id, &task_def.depends_on)`
- Add the result as `dependency_outputs` field

### Step 7: Fix ALL DispatchContext constructor sites
- Run `grep -rn 'DispatchContext {' crates/roko-cli/src/ --include='*.rs'`
- Add `dependency_outputs: Vec::new()` to every struct literal found
- There are at least 5 sites: event_loop.rs:2144 (production), dispatch/mod.rs:343 (test),
  dispatch/mod.rs:402 (test), prompt_builder.rs:1193 (test), model_routing.rs:251 (test)

### Verification
Run: `cargo check -p roko-cli && cargo test -p roko-cli --lib runner::state && cargo test -p roko-cli --lib dispatch && cargo clippy -p roko-cli --no-deps -- -D warnings`
```

## Commit

This batch is committed with Wave 9 (Systemic Pipeline Quality). Do not commit individually.

## Checklist

- [ ] `RunState` has `task_outputs: HashMap<String, Vec<String>>` field
- [ ] `RunState::new()` initializes `task_outputs: HashMap::new()`
- [ ] `record_task_outputs()` method with `tracing::debug!` added
- [ ] `task_output_files()` method added
- [ ] `dependency_outputs()` method added
- [ ] `git_diff_names_since_task_start()` helper function added in event_loop.rs
- [ ] After task passes gates (line 723), modified files are recorded via `state.record_task_outputs()`
- [ ] `DispatchContext` has `dependency_outputs: Vec<(String, Vec<String>)>` field
- [ ] `PromptContext` has `dependency_outputs` field, populated from `DispatchContext`
- [ ] `PromptAssembler::assemble()` injects "Prior Task Outputs" section with tracing
- [ ] `dependency_outputs` added to canonical section ordering
- [ ] Production DispatchContext at event_loop.rs:2144 calls `state.dependency_outputs()` with `tracing::info!`
- [ ] Test DispatchContext at dispatch/mod.rs:343 has `dependency_outputs: Vec::new()`
- [ ] Test DispatchContext at prompt_builder.rs:1193 has `dependency_outputs: Vec::new()`
- [ ] Test DispatchContext at model_routing.rs:251 has `dependency_outputs: Vec::new()`
- [ ] `grep 'DispatchContext {' crates/roko-cli/src/` shows no missing fields
- [ ] `cargo check -p roko-cli` passes
- [ ] `cargo test -p roko-cli --lib dispatch` passes
- [ ] `cargo clippy -p roko-cli --no-deps -- -D warnings` passes

## Audit Status

Audited: 2026-05-05. PASS no changes needed
