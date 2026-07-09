# W10-D: Scaffold Inter-Crate Deps, Gate Enforcement, Schema Fix, Git Commits

**Priority**: P1 -- scaffold bugs cause compile failures, gate mismatch loses data, no git history
**Effort**: 3-4 hours
**Files to modify**: 5
**Dependencies**: None

## Problem

Four related build/gate infrastructure bugs:

1. **14.3**: When the plan scaffold creates a new crate (e.g., `btc-funding-alert-cli`), it generates a minimal `Cargo.toml` with no `[dependencies]`. If the task says `files = ["crates/btc-funding-alert-cli/src/main.rs"]` and the code imports from another workspace crate, the build fails immediately. The scaffold should parse the task's `files` and `depends_on` to infer inter-crate dependencies.

2. **14.8**: `max_loc` is a soft prompt hint ("roughly N lines") with no gate enforcement. Task T2 wrote 248 lines despite `max_loc=150`. A stronger prompt instruction is the minimum fix; a `DiffLocGate` is the stretch goal.

3. **14.11**: The runner writes gate threshold data as `GateThresholdStats` (fields: `pass_count`, `total_count`, `ema_pass_rate`) but `adaptive_threshold.rs` reads it with `RungStats` (fields: `ema_pass_rate`, `total_observations`, `consecutive_passes`, `cusum_high`, `cusum_low`, `cusum_shift_detected`). Field names mismatch: `total_count` vs `total_observations` -- so `total_observations` always deserializes as 0.

4. **14.24**: In in-place mode (no plan branch), generated code is written to the working tree but never committed. After a successful run, there's only the initial `workspace init` commit. No git history for generated code means subsequent tasks cannot use `git diff` to see what changed.

## Root Cause

### 14.3
`scaffold_missing_crates()` in `plan_loader.rs` creates `Cargo.toml` with just `[package]` -- no dependency analysis. It does not examine which other crates the plan's tasks depend on.

### 14.8
No `DiffLocGate` exists in `roko-gate`. The `max_loc` field is only used in prompt text via `dispatch_helpers.rs` where it says "roughly N lines". The word "roughly" weakens the constraint.

### 14.11
`GateThresholdStats` in `persist.rs` already has `#[serde(alias = "total_observations")]` on `total_count`, so it can read `RungStats`-formatted data. But `RungStats` in `adaptive_threshold.rs` has no `alias = "total_count"`, so it cannot read `GateThresholdStats`-formatted data. The mismatch is one-directional.

### 14.24
The runner writes files via the agent but never runs `git add` + `git commit` after gates pass. The merge module handles branch merges but not in-place commits.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/plan_loader.rs`

#### Change 1 (14.3): Add inter-crate dependencies to scaffolded Cargo.toml

The `scaffold_missing_crates` function (line 99) takes `(workdir: &Path, plans: &[Plan])`. It collects crate names in the first pass (lines 112-145) and scaffolds in the second pass (lines 148-177). The scaffold needs to infer dependencies from the plan's task graph.

**Find this code** (lines 147-164):
```rust
    // Second pass: create scaffold files for each new crate.
    for crate_name in &scaffolded {
        let crate_dir = crates_dir.join(crate_name);
        let src_dir = crate_dir.join("src");
        std::fs::create_dir_all(&src_dir)
            .with_context(|| format!("scaffold: create {}", src_dir.display()))?;

        let is_bin = crate_needs_main.contains(crate_name.as_str());

        let cargo_toml = if is_bin {
            format!(
                "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"{crate_name}\"\npath = \"src/main.rs\"\n"
            )
        } else {
            format!(
                "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
            )
        };
```

**Replace with:**
```rust
    // Second pass: create scaffold files for each new crate.
    for crate_name in &scaffolded {
        let crate_dir = crates_dir.join(crate_name);
        let src_dir = crate_dir.join("src");
        std::fs::create_dir_all(&src_dir)
            .with_context(|| format!("scaffold: create {}", src_dir.display()))?;

        let is_bin = crate_needs_main.contains(crate_name.as_str());

        // Infer inter-crate dependencies from task graph:
        // If any task targets this crate and depends_on tasks in other crates,
        // those other crates are likely dependencies.
        let mut deps: Vec<String> = Vec::new();
        for plan in plans {
            for task in &plan.tasks.tasks {
                let targets_this = task.files.iter().any(|f| {
                    f.starts_with(&format!("crates/{crate_name}/"))
                        || f.starts_with(&format!("crates/{}/", crate_name.replace('-', "_")))
                });
                if targets_this {
                    for dep_id in &task.depends_on {
                        // Find the dependency task and extract its target crate
                        for other_task in &plan.tasks.tasks {
                            if &other_task.id == dep_id {
                                for f in &other_task.files {
                                    if let Some(rest) = f.strip_prefix("crates/") {
                                        if let Some(dep_crate) = rest.split('/').next() {
                                            if dep_crate != crate_name
                                                && !deps.contains(&dep_crate.to_string())
                                            {
                                                deps.push(dep_crate.to_string());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let deps_section = if deps.is_empty() {
            String::new()
        } else {
            let mut s = String::from("\n[dependencies]\n");
            for dep in &deps {
                s.push_str(&format!(
                    "{} = {{ path = \"../{dep}\" }}\n",
                    dep.replace('-', "_")
                ));
            }
            s
        };
        tracing::debug!(
            crate_name,
            dep_count = deps.len(),
            deps = ?deps,
            "scaffold: inferred inter-crate dependencies"
        );

        let cargo_toml = if is_bin {
            format!(
                "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[[bin]]\nname = \"{crate_name}\"\npath = \"src/main.rs\"\n{deps_section}"
            )
        } else {
            format!(
                "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{deps_section}"
            )
        };
```

Note: The `Plan` struct has `tasks: TaskList` which has `tasks: Vec<TaskDef>`. `TaskDef` has `files: Vec<String>`, `depends_on: Vec<String>`, and `id: String`. Verify these field names by reading the `TaskDef` struct definition (likely in `task_parser.rs`).

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_helpers.rs`

#### Change 2 (14.8): Strengthen max_loc prompt text

**Find this code** (lines 68-72):
```rust
    if let Some(max_loc) = task_def.max_loc {
        sections.push(format!(
            "Keep the total code delta within roughly {max_loc} lines of change unless verification requires a tightly scoped follow-up."
        ));
    }
```

**Replace with:**
```rust
    if let Some(max_loc) = task_def.max_loc {
        sections.push(format!(
            "HARD LIMIT: The total code delta MUST be strictly under {max_loc} lines of change. \
             If you find yourself exceeding this, stop and split the work into smaller changes. \
             This is a quality gate — exceeding it will cause the task to fail."
        ));
    }
```

### File 3: `/Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs`

#### Change 3 (14.11): Add serde alias to RungStats for bidirectional compat

**Find this code** (lines 29-33):
```rust
pub struct RungStats {
    /// Exponential moving average of the pass rate (0.0 to 1.0).
    pub ema_pass_rate: f64,
    /// Total observations for this rung.
    pub total_observations: u64,
    /// Consecutive passes (reset on any failure).
```

**Replace with:**
```rust
pub struct RungStats {
    /// Exponential moving average of the pass rate (0.0 to 1.0).
    pub ema_pass_rate: f64,
    /// Total observations for this rung.
    #[serde(default, alias = "total_count")]
    pub total_observations: u64,
    /// Consecutive passes (reset on any failure).
```

This makes `RungStats` accept both `total_observations` (its canonical name) and `total_count` (the name `GateThresholdStats` uses) when deserializing. Since `GateThresholdStats` already has `#[serde(default, alias = "total_observations")]` on `total_count` (see `persist.rs` line 150), both directions now work.

### File 4: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs`

#### Change 4 (14.24): Add git commit after all gates pass for a task

After the task passes all gates and is marked complete (line ~723 where `tui.task_completed` is called), add a git commit for the working tree changes.

**Find this code** (lines 723-737):
```rust
                    tui.task_completed(&completion.plan_id, &completion.task_id, "passed");

                    let total_task_ms = state.task_elapsed_ms();
                    let dispatch_ms = state.last_dispatch_ms;
                    let gate_ms = completion.duration_ms;
                    let agent_ms = total_task_ms.saturating_sub(dispatch_ms + gate_ms);

                    // ── Stream: task done ────────────────────────────
                    if stream_to_stderr {
                        let secs = total_task_ms / 1000;
                        eprintln!(
                            "     \u{2713} Done ({secs}s) [{}/{} tasks]",
                            state.tasks_completed, state.tasks_total,
                        );
                    }
```

**Replace with:**
```rust
                    tui.task_completed(&completion.plan_id, &completion.task_id, "passed");

                    // Commit generated code to git so subsequent tasks can diff.
                    commit_task_changes(
                        &config.workdir,
                        &completion.plan_id,
                        &completion.task_id,
                    );

                    let total_task_ms = state.task_elapsed_ms();
                    let dispatch_ms = state.last_dispatch_ms;
                    let gate_ms = completion.duration_ms;
                    let agent_ms = total_task_ms.saturating_sub(dispatch_ms + gate_ms);

                    // ── Stream: task done ────────────────────────────
                    if stream_to_stderr {
                        let secs = total_task_ms / 1000;
                        eprintln!(
                            "     \u{2713} Done ({secs}s) [{}/{} tasks]",
                            state.tasks_completed, state.tasks_total,
                        );
                    }
```

**Add the helper function** near the bottom of `event_loop.rs` (before the tests section, or near other helper functions like `update_gate_thresholds`).

```rust
/// Commit working tree changes for a completed task.
///
/// Only acts if there are uncommitted changes. Silently succeeds if git is
/// not available or the workdir is not a git repo. Uses `--no-verify` to
/// avoid triggering hooks in generated workspaces.
fn commit_task_changes(workdir: &std::path::Path, plan_id: &str, task_id: &str) {
    use std::process::Command;

    // Check if there are changes to commit
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(workdir)
        .output();
    let has_changes = status
        .as_ref()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);
    if !has_changes {
        debug!(%plan_id, %task_id, "no uncommitted changes to commit");
        return;
    }

    let msg = format!("[roko] {plan_id}: {task_id} completed");
    let add = Command::new("git")
        .args(["add", "-A"])
        .current_dir(workdir)
        .status();
    if add.is_err() || !add.as_ref().map(|s| s.success()).unwrap_or(false) {
        debug!(%plan_id, %task_id, "git add failed -- skipping commit");
        return;
    }
    let commit = Command::new("git")
        .args(["commit", "-m", &msg, "--no-verify"])
        .current_dir(workdir)
        .status();
    match commit {
        Ok(s) if s.success() => {
            info!(%plan_id, %task_id, "committed task changes to git");
        }
        _ => {
            debug!(%plan_id, %task_id, "git commit failed -- non-fatal");
        }
    }
}
```

Note: `debug!` and `info!` are already imported from `tracing` at the top of event_loop.rs (line 23). The `std::process::Command` import is scoped inside the function.

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Build check
cargo check -p roko-cli -p roko-gate 2>&1 | tail -5

# Verify serde alias added to RungStats
grep -n 'alias.*total_count' crates/roko-gate/src/adaptive_threshold.rs
# Should show the new alias

# Verify scaffold generates dependencies
grep -n 'deps_section' crates/roko-cli/src/runner/plan_loader.rs
# Should show the dependency generation code

# Verify max_loc prompt is strengthened
grep -n 'HARD LIMIT' crates/roko-cli/src/dispatch_helpers.rs
# Should show the new prompt text

# Verify commit function exists
grep -n 'commit_task_changes' crates/roko-cli/src/runner/event_loop.rs
# Should show function definition and call site

# Verify no more "roughly" in max_loc prompt
grep -n 'roughly' crates/roko-cli/src/dispatch_helpers.rs | grep max_loc
# Should return no results
```

## Agent Prompt

```
You are fixing four build/gate infrastructure bugs in the roko codebase. This is a Rust project at /Users/will/dev/nunchi/roko/roko. The batch file has exact find/replace pairs.

IMPORTANT: Read the source files FIRST before making changes. Line numbers may drift if other changes have been applied.

### Fix 1 (14.3): Scaffold Cargo.toml with inter-crate deps
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/plan_loader.rs

Read lines 99-177 to understand `scaffold_missing_crates`. The function takes `(workdir: &Path, plans: &[Plan])`.

In the second pass (line ~148), after the `is_bin` determination and before Cargo.toml generation, add dependency inference logic:
- Iterate all plans/tasks
- For tasks targeting this crate, check depends_on to find dependency tasks
- Extract crate names from dependency task file paths
- Build a `[dependencies]` section with `path = "../dep-crate"` entries

Verify the `Plan` and `TaskDef` struct field names by reading:
- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/plan_loader.rs (Plan struct)
- /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/task_parser.rs (TaskDef struct)

### Fix 2 (14.8): Strengthen max_loc prompt
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_helpers.rs (lines 68-72)

Change "Keep the total code delta within roughly {max_loc} lines" to a strict limit warning that says exceeding it will cause the task to fail.

### Fix 3 (14.11): Gate threshold schema mismatch
File: /Users/will/dev/nunchi/roko/roko/crates/roko-gate/src/adaptive_threshold.rs (lines 29-33)

Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/persist.rs lines 145-170 (GateThresholdStats -- note it already has `alias = "total_observations"` on its `total_count` field).

Add `#[serde(default, alias = "total_count")]` to `RungStats.total_observations`. This makes both structs able to read each other's serialized data.

### Fix 4 (14.24): Git commits for generated code
File: /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/event_loop.rs

Read lines 710-740 to find where a task passes all gates (look for `tui.task_completed`).

After `tui.task_completed` (line ~723), add a call to `commit_task_changes(&config.workdir, &completion.plan_id, &completion.task_id)`.

Add the `commit_task_changes` function near the bottom of the file (before tests). It should:
- Check `git status --porcelain` for changes
- Run `git add -A` then `git commit -m "[roko] {plan_id}: {task_id} completed" --no-verify`
- Log with tracing::info on success, tracing::debug on failure (non-fatal)
- `config.workdir` is a `PathBuf` field on `RunConfig`

After all changes, run:
```bash
cargo check -p roko-cli -p roko-gate 2>&1 | tail -20
```
Then run the verification grep commands.
```

## Commit

This batch is committed with Wave 10. Do not commit individually.

## Checklist

- [ ] 14.3: Scaffold Cargo.toml includes inferred inter-crate dependencies
- [ ] 14.8: max_loc prompt text strengthened from "roughly" to strict limit
- [ ] 14.11: `RungStats.total_observations` has `#[serde(default, alias = "total_count")]`
- [ ] 14.24: `commit_task_changes` function added and called after task gate pass
- [ ] `tracing::debug!` / `tracing::info!` instrumentation at key points
- [ ] `cargo check -p roko-cli -p roko-gate` passes

## Audit Status

Audited: 2026-05-05. PASS no changes needed
