# W10-C: Unify Memory/Learn Paths and Fix Index Tracking

**Priority**: P1 -- dual data stores cause split-brain, stale indexes mislead operators
**Effort**: 2-3 hours
**Files to modify**: 5-6
**Dependencies**: None

## Problem

Four related data path and tracking bugs:

1. **14.15**: `.roko/memory/` and `.roko/learn/` are both used as roots for `LearningRuntime`. Some code paths open `.roko/memory/` (e.g., `util.rs:1553` for ad-hoc agent runs) while others open `.roko/learn/` (e.g., `orchestrate.rs:4442` for plan runs). Both directories get populated with identical data structures (cascade-router.json, costs, efficiency.jsonl, etc.), creating a split-brain where learning data diverges depending on which code path wrote it.

2. **14.16**: The cascade router silently drops outcome records for model slugs not in its initialization list. If the router was initialized with hardcoded defaults (`claude-sonnet-4-5`, `claude-haiku-4-5`), and the actual model used is `gpt-5.4-mini`, the outcome is silently discarded (`return false`). Learning data for non-default models is lost.

3. **14.18**: Plans INDEX.md counts `status = "done"` strings in `tasks.toml` on disk, but `plan run` never writes task status back to `tasks.toml` -- completion state lives only in `executor.json`. Result: INDEX.md always shows 0/N done even after a successful run.

4. **14.19**: The PRD `plans_generated` frontmatter field is never updated after `prd plan` successfully generates a plan. The field stays as `[]` even though a plan directory was created.

## Root Cause

### 14.15
Two different entry points to `LearningRuntime::open_under()` use different base paths. The runner v2 event loop (plan execution) uses `.roko/learn/`. The `record_agent_episode` helper in `util.rs` (used by `roko run`, `roko prd`, etc.) uses `.roko/memory/`.

Additional occurrences of `.roko/memory` exist in: `status.rs:252`, `research.rs:653`, `dispatch/prompt_builder.rs:1085`, `dispatch/prompt_cache.rs:136`, `tui/dashboard.rs:45`, `main.rs:3449`, `main.rs:4163`.

### 14.16
`CascadeRouter::record_confidence_outcome()` calls `self.model_index_for_slug(slug)` which returns `None` for unregistered slugs, then returns `false` without logging. The `model_slugs` field is a plain `Vec<String>` (not behind a lock), so auto-registration is not possible with `&self`. A `tracing::warn!` is the correct minimal fix.

### 14.18
`count_top_level_tasks()` in `index.rs` parses `tasks.toml` and counts entries with `status = "done"`. But the runner never modifies `tasks.toml` -- completion data lives in `.roko/state/executor.json` as `RunStateSnapshot.completed_tasks: HashMap<String, Vec<String>>`.

### 14.19
`cmd_prd_plan()` in `prd.rs` writes `tasks.toml` and `plan.md` to the plan directory, but never reads back the PRD file to update `plans_generated: []` in the frontmatter.

## Exact Code to Change

### File 1: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs`

#### Change 1 (14.15): Standardize on `.roko/learn/` path

**Find this code** (line 1553):
```rust
    let mut runtime = LearningRuntime::open_under(workdir.join(".roko").join("memory"))
```

**Replace with:**
```rust
    tracing::debug!(workdir = %workdir.display(), "opening learning runtime under .roko/learn/");
    let mut runtime = LearningRuntime::open_under(workdir.join(".roko").join("learn"))
```

### File 2: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/status.rs`

#### Change 2 (14.15): Fix episode path in status

**Find this code** (lines 251-254):
```rust
fn read_episode_summary(workdir: &Path) -> (usize, Option<bool>) {
    let primary = workdir.join(".roko").join("memory").join("episodes.jsonl");
    let fallback = workdir.join(".roko").join("episodes.jsonl");
    let path = if primary.exists() { primary } else { fallback };
```

**Replace with:**
```rust
fn read_episode_summary(workdir: &Path) -> (usize, Option<bool>) {
    let primary = workdir.join(".roko").join("episodes.jsonl");
    let fallback = workdir.join(".roko").join("learn").join("episodes.jsonl");
    let path = if primary.exists() { primary } else { fallback };
```

### File 3: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/research.rs`

#### Change 3 (14.15): Fix episode path in research analyze

**Find this code** (line 653):
```rust
            let episodes_path = workdir.join(".roko/memory/episodes.jsonl");
```

**Replace with:**
```rust
            let episodes_path = workdir.join(".roko/episodes.jsonl");
```

**Find this code** (line 660):
```rust
            let task_prompt = "Read .roko/memory/episodes.jsonl and analyze: \
```

**Replace with:**
```rust
            let task_prompt = "Read .roko/episodes.jsonl and analyze: \
```

### Additional .roko/memory references (14.15)

The following files also reference `.roko/memory` and should be updated. These are lower-priority because they use fallback patterns (try `.roko/learn` first, then `.roko/memory`):

- `dispatch/prompt_builder.rs:1085` -- already has `.roko/learn` as primary, `.roko/memory` as fallback. Keep the fallback for backward compat.
- `dispatch/prompt_cache.rs:136` -- same pattern. Keep fallback.
- `tui/dashboard.rs:45` -- `MEMORY_DIR` constant. Change to `"learn"` if episodes are there, but episodes are at `.roko/episodes.jsonl` not inside either subdirectory. Review this file to determine the correct path.
- `main.rs:3449` and `main.rs:4163` -- test-only code. Update for consistency but not critical.

For this batch, focus on the functional paths (util.rs, status.rs, research.rs, index.rs). Leave the fallback patterns in prompt_builder.rs and prompt_cache.rs as-is since they already try `.roko/learn` first.

### File 4: `/Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs`

#### Change 4 (14.16): Log warning on unknown model slug

The `model_slugs` field is a plain `Vec<String>` (line 92), not behind a lock. The method takes `&self`, so auto-registration would require `&mut self` or interior mutability. The simplest correct fix is a `tracing::warn!`.

**Find this code** (lines 1075-1078):
```rust
    pub fn record_confidence_outcome(&self, model_slug: &str, success: bool) -> bool {
        let Some(model_idx) = self.model_index_for_slug(model_slug) else {
            return false;
        };
```

**Replace with:**
```rust
    pub fn record_confidence_outcome(&self, model_slug: &str, success: bool) -> bool {
        let Some(model_idx) = self.model_index_for_slug(model_slug) else {
            tracing::warn!(
                slug = %model_slug,
                success,
                "cascade router: unknown model slug -- outcome dropped. \
                 Add this model to [models] in roko.toml or provider config \
                 so the router can track it."
            );
            return false;
        };
```

### File 5: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/index.rs`

#### Change 5 (14.18): Read run-state.json for plan completion status

The `RunStateSnapshot` struct (defined in `persist.rs` line 81) has `completed_tasks: HashMap<String, Vec<String>>` mapping plan_id to completed task IDs. This struct is persisted to `.roko/state/run-state.json` (NOT `executor.json` -- that holds `ExecutorSnapshot` which has different fields).

**Find this code** (lines 152-162):
```rust
    plan_dirs.sort();

    let mut total_tasks = 0u32;
    let mut total_done = 0u32;

    for dir in &plan_dirs {
        let name = dir.file_name().unwrap_or_default().to_string_lossy();
        let tasks_path = dir.join("tasks.toml");
        let content = std::fs::read_to_string(&tasks_path).unwrap_or_default();

        let (tasks, done, ready) = count_top_level_tasks(&content);
```

**Replace with:**
```rust
    plan_dirs.sort();

    // Load run-state for real completion data. tasks.toml is never
    // updated by plan run -- completion state lives in run-state.json only.
    // NOTE: This is run-state.json (RunStateSnapshot), NOT executor.json
    // (ExecutorSnapshot). Only RunStateSnapshot has completed_tasks.
    let run_state_path = workdir.join(".roko/state/run-state.json");
    let run_state_completed: std::collections::HashMap<String, Vec<String>> =
        if run_state_path.exists() {
            std::fs::read_to_string(&run_state_path)
                .ok()
                .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
                .and_then(|val| {
                    val.get("completed_tasks")
                        .and_then(|ct| serde_json::from_value(ct.clone()).ok())
                })
                .unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        };

    let mut total_tasks = 0u32;
    let mut total_done = 0u32;

    for dir in &plan_dirs {
        let name = dir.file_name().unwrap_or_default().to_string_lossy();
        let tasks_path = dir.join("tasks.toml");
        let content = std::fs::read_to_string(&tasks_path).unwrap_or_default();

        let (tasks, mut done, ready) = count_top_level_tasks(&content);

        // Overlay real completion data from run-state.json if available.
        if let Some(completed_ids) = run_state_completed.get(name.as_ref()) {
            if !completed_ids.is_empty() {
                done = completed_ids.len() as u32;
            }
        }
```

Note: The `RunStateSnapshot` serializes `completed_tasks` as a top-level field (not nested under `"plans"`). The JSON format is:
```json
{
  "run_id": "...",
  "completed_tasks": {
    "plan-slug": ["T1", "T2"]
  },
  ...
}
```

### File 6: `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs`

#### Change 6 (14.19): Update PRD plans_generated after successful plan generation

After the plan files are written (line ~1209, before the `} else {` branch at line 1210), add the PRD update call.

**Find this code** (lines 1202-1209):
```rust
            } else {
                // Write minimal plan.md so plan discovery tools can find this directory.
                let minimal_plan_md = format!(
                    "---\nplan: {slug}\ntitle: {slug}\n---\n\n# {slug}\n\nGenerated plan.\n"
                );
                std::fs::write(plan_dir.join("plan.md"), &minimal_plan_md)
                    .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
            }
```

**Replace with:**
```rust
            } else {
                // Write minimal plan.md so plan discovery tools can find this directory.
                let minimal_plan_md = format!(
                    "---\nplan: {slug}\ntitle: {slug}\n---\n\n# {slug}\n\nGenerated plan.\n"
                );
                std::fs::write(plan_dir.join("plan.md"), &minimal_plan_md)
                    .with_context(|| format!("write plan.md to {}", plan_dir.display()))?;
            }

            // Update PRD frontmatter: record the generated plan slug.
            if let Err(err) = update_prd_plans_generated(prd_path, slug) {
                eprintln!("warning: failed to update PRD plans_generated: {err}");
                tracing::warn!(
                    slug = %slug,
                    error = %err,
                    "failed to update PRD plans_generated field"
                );
            } else {
                tracing::info!(slug = %slug, "updated PRD plans_generated field");
            }
```

**Add this helper function** at the end of the file (or near other PRD helpers). Search for a good location such as after the `extract_fenced_block` function or near the end of the `impl` block.

```rust
/// Update the PRD frontmatter to record that a plan was generated.
fn update_prd_plans_generated(prd_path: &std::path::Path, plan_slug: &str) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(prd_path)?;

    let updated = if content.contains("plans_generated: []") {
        content.replace(
            "plans_generated: []",
            &format!("plans_generated: [\"{plan_slug}\"]"),
        )
    } else if let Some(pos) = content.find("plans_generated: [") {
        let after_bracket = pos + "plans_generated: [".len();
        if let Some(close) = content[after_bracket..].find(']') {
            let close_pos = after_bracket + close;
            let existing = content[after_bracket..close_pos].trim();
            if existing.is_empty() {
                format!(
                    "{}\"{plan_slug}\"{}",
                    &content[..after_bracket],
                    &content[close_pos..]
                )
            } else if existing.contains(plan_slug) {
                // Already listed
                return Ok(());
            } else {
                format!(
                    "{}, \"{plan_slug}\"{}",
                    &content[..close_pos],
                    &content[close_pos..]
                )
            }
        } else {
            return Ok(()); // Malformed, skip
        }
    } else {
        // No plans_generated field found -- don't modify
        return Ok(());
    };

    std::fs::write(prd_path, updated)?;
    Ok(())
}
```

## Verification

```bash
cd /Users/will/dev/nunchi/roko/roko

# Build check
cargo check -p roko-cli -p roko-learn 2>&1 | tail -5

# Verify .roko/memory is no longer in critical paths
grep -rn '\.roko.*memory' crates/roko-cli/src/commands/util.rs --include='*.rs'
# Should return no results

# Verify status.rs uses correct primary path
grep -n 'memory.*episodes' crates/roko-cli/src/status.rs
# Should return no results

# Verify cascade router logs on unknown slug
grep -n 'unknown model slug' crates/roko-learn/src/cascade_router.rs
# Should show the new warning

# Verify run-state.json is consulted for plan completion
grep -n 'run_state_completed\|run-state.json' crates/roko-cli/src/index.rs
# Should show the new code

# Verify PRD update function exists
grep -n 'update_prd_plans_generated' crates/roko-cli/src/prd.rs
# Should show function definition and call site
```

## Agent Prompt

```
You are fixing four data path and tracking bugs in the roko codebase. This is a Rust project at /Users/will/dev/nunchi/roko/roko.

IMPORTANT: Read the source files FIRST before making changes. The batch file has exact find/replace pairs but line numbers may drift if other changes have been applied.

### Fix 1 (14.15): Unify .roko/memory/ and .roko/learn/ paths

The LearningRuntime is opened with different base paths depending on the code path. Standardize on `.roko/learn/`.

a) Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/util.rs line 1553
   Change `.roko/memory` to `.roko/learn` in the LearningRuntime::open_under call.

b) Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/status.rs lines 251-254
   Change the primary episode path from `.roko/memory/episodes.jsonl` to `.roko/episodes.jsonl`.
   Keep a fallback to `.roko/learn/episodes.jsonl`.

c) Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/commands/research.rs lines 653 and 660
   Change `.roko/memory/episodes.jsonl` to `.roko/episodes.jsonl` in the analyze command.

d) Do NOT change `.roko/episodes.jsonl` references -- that path is correct as-is.
   Do NOT change the fallback patterns in dispatch/prompt_builder.rs or prompt_cache.rs -- they already try `.roko/learn` first.

### Fix 2 (14.16): Cascade router drops unknown model slugs silently

Read /Users/will/dev/nunchi/roko/roko/crates/roko-learn/src/cascade_router.rs lines 1075-1078.
The `model_slugs` field is a plain Vec<String> (not behind a lock), so auto-registration requires &mut self. Add a `tracing::warn!` with the slug name and a hint about adding the model to config.

### Fix 3 (14.18): Plans INDEX.md shows 0/N done

Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/index.rs lines 152-162
Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/runner/persist.rs lines 81-113 (RunStateSnapshot struct)

The runner never writes completion status back to tasks.toml -- it writes to `.roko/state/run-state.json` as RunStateSnapshot which has `completed_tasks: HashMap<String, Vec<String>>` (plan_id -> task IDs). NOTE: this is run-state.json, NOT executor.json (executor.json holds ExecutorSnapshot which does not have completed_tasks).

In `rebuild_plans_index`, load run-state.json before the loop, parse the `completed_tasks` field, and overlay the real completion count onto the `done` value from `count_top_level_tasks`.

### Fix 4 (14.19): PRD plans_generated never updated

Read /Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/prd.rs lines 1202-1209
After the plan.md write block (before the `} else {` branch at line 1210), add a call to `update_prd_plans_generated(prd_path, slug)`.
Add the `update_prd_plans_generated` helper function that does string replacement on the PRD frontmatter.

After all changes, run:
```bash
cargo check -p roko-cli -p roko-learn 2>&1 | tail -20
```
Then run the verification grep commands.
```

## Commit

This batch is committed with Wave 10. Do not commit individually.

## Checklist

- [ ] 14.15: `util.rs` LearningRuntime path changed from `.roko/memory` to `.roko/learn`
- [ ] 14.15: `status.rs` episode path changed from `.roko/memory/episodes.jsonl` to `.roko/episodes.jsonl`
- [ ] 14.15: `research.rs` episode path changed from `.roko/memory/episodes.jsonl` to `.roko/episodes.jsonl`
- [ ] 14.16: Cascade router logs `tracing::warn!` on unknown model slugs
- [ ] 14.18: Plans INDEX reads `run-state.json` for real completion counts
- [ ] 14.19: PRD `plans_generated` updated after successful plan generation
- [ ] `cargo check -p roko-cli -p roko-learn` passes

## Audit Status

Audited: 2026-05-05. PASS no changes needed
