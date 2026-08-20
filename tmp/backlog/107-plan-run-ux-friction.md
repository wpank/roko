# 107 — Plan Run UX Friction

**Priority**: P1 — first-run blocker; a stale worktree silently kills all tasks with no user-facing error
**Size**: M (2-3 days)
**Crates**: `roko-cli` (`/Users/will/dev/nunchi/roko/roko/crates/roko-cli/`)
**Depends on**: None

---

## Background

`roko plan run` is the primary command for executing autonomous agent plans. During dogfood
testing with a non-default provider (z.ai / GLM-5.1), eight distinct UX problems were
encountered that together make the first-run experience very difficult. Each issue is small in
isolation, but they cascade: a new user hits them sequentially with no single error message
pointing to the root cause.

The most critical issue (#1) is that stale git worktrees from previous runs silently prevent all
task dispatch. When a worktree branch is already checked out in an isolated directory, `plan run
--fresh` fails to acquire worktrees for every task, producing "Plan complete: 0/5 tasks" — which
looks like silent success rather than an infrastructure failure. This is a blocker for any user
who has ever run a plan before.

The remaining issues (#2-#8) are documentation and CLI ergonomics gaps discovered in the same
session.

## Current State

1. **Stale worktree silently kills execution (BLOCKER)**

   When `--fresh` is passed, the runner archives old state but does NOT prune git worktrees.
   Worktree acquisition fails with:
   ```
   ERROR: failed to acquire isolated task-attempt worktree:
   cannot safely reattach worktree `attempt-0ff536f7d38f1196fee0`:
   branch `roko/attempt/attempt-0ff536f7d38f1196fee0` is already checked out
   ```
   This error is logged at the `ERROR` level but does NOT propagate to the user as a CLI error.
   The plan exits with "Plan complete: 0/5 tasks" — looks like success, not failure.

   Relevant files:
   - Worktree manager: search for `WorktreeManager` in `crates/roko-cli/src/runner/event_loop.rs`
     (referenced at line 25 `use crate::dispatch::worktrees::WorktreeManager`)
   - `--fresh` handling: `crates/roko-cli/src/commands/plan.rs` lines 349-376

2. **`plan show` vs `plan run` argument asymmetry**

   `plan show` accepts a plan ID string (e.g., `demo-multistage`).
   `plan run` accepts a directory path (e.g., `plans/demo-multistage`).

   Handler in `crates/roko-cli/src/commands/plan.rs` line 116-122:
   ```rust
   PlanCmd::Show { plan_id, workdir } => {
       let Some(plan_info) =
           roko_cli::plan::discover_plan_by_id(&wd, &plan_id)
               .map_err(|e| anyhow!("{e}"))?
       else {
           anyhow::bail!("plan '{plan_id}' not found");
       };
   ```
   `discover_plan_by_id` takes a plan ID string. If you pass a path like
   `plans/demo-multistage`, it looks for a plan named `plans/demo-multistage` and fails with
   "not found". There is no path-to-ID resolution fallback.

3. **`--model` flag not visible on `plan run --help`**

   The `--model` flag is defined as a GLOBAL flag before the subcommand in `main.rs`. It is not
   listed in `plan run --help`. Users trying `plan run --model glm51 plans/...` get an error
   because the flag must come before the subcommand: `roko --model glm51 plan run plans/...`.
   There is no `--force-backend` alias on the CLI even though that term is used internally.

4. **`cargo run -p roko-cli` requires `--bin roko`**

   `crates/roko-cli/Cargo.toml` has two `[[bin]]` entries:
   - `name = "roko"` at line 13
   - `name = "layer_check"` at line 17

   No `default-run` key is set (verified: not present in `Cargo.toml`). Running `cargo run -p
   roko-cli` fails with "could not determine which binary to run." Users must use
   `cargo run -p roko-cli --bin roko`.

5. **Running from wrong directory gives confusing error**

   Running `plan run` from inside `plans/demo-multistage/` instead of the workspace root
   produces: `plans directory does not exist: .../plans/demo-multistage/.roko/plans`. The
   resolver appends `.roko/plans` to the cwd, which is wrong. No hint to run from workspace
   root.

6. **Silent provider fallback shows "unknown" model in TUI**

   When the configured model isn't available, the cascade router falls back to another provider
   silently. The TUI shows model as "unknown" in the F3 Agents tab. No warning is logged at a
   user-visible level. Relevant: `crates/roko-cli/src/dispatch/model_routing.rs`.

7. **`--approval` flag required for inline TUI, but not obvious**

   `plan run` without `--approval` outputs plain text. The TUI only appears with `--approval`
   (defined in `crates/roko-cli/src/main.rs` line 1506 as `#[arg(long)] approval: bool`).
   The flag name "approval" implies task approval workflow, not "show TUI." Many users expect
   the TUI to appear automatically in an interactive terminal.

8. **Zero-token/zero-cost episodes don't indicate failure reason**

   When a task fails to dispatch (e.g., worktree error), it appears in the plan summary as
   `STAGE-1A: 0 tokens, 0 calls, $0.0000` with result "orphaned." This is indistinguishable
   from a task that succeeded with zero cost.

## Implementation Plan

### Fix 1: Auto-prune worktrees when `--fresh` is passed (BLOCKER)

In `crates/roko-cli/src/commands/plan.rs`, inside the `--fresh` branch (around line 349), after
archiving old state, add a `git worktree prune` call:

```rust
// After: archive old state for --fresh
if fresh {
    // ... existing archive logic (lines 349-376) ...

    // NEW: prune stale worktrees so task dispatch can acquire fresh ones
    let prune_result = std::process::Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(&wd)
        .output();
    match prune_result {
        Ok(out) if out.status.success() => {
            if !cli.quiet {
                println!("▸ --fresh: pruned stale git worktrees");
            }
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            tracing::warn!("git worktree prune failed: {stderr}");
        }
        Err(e) => {
            tracing::warn!("could not run git worktree prune: {e}");
        }
    }
}
```

Also: when worktree acquisition fails (wherever `WorktreeManager::acquire` or equivalent is
called), propagate the error up as a user-facing message instead of only logging it. Find the
worktree acquisition call site and ensure it returns `Err(...)` that surfaces as a CLI error,
not just `tracing::error!(...)`.

### Fix 2: Make `plan show` accept path arguments

In `crates/roko-cli/src/commands/plan.rs`, in the `PlanCmd::Show` handler (line 116), add
path-to-ID resolution before calling `discover_plan_by_id`:

```rust
PlanCmd::Show { plan_id, workdir } => {
    let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));

    // NEW: if the argument looks like a path (contains '/' or '\'), strip it to just the dir name
    let resolved_id = if plan_id.contains('/') || plan_id.contains('\\') {
        std::path::Path::new(&plan_id)
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or(plan_id.clone())
    } else {
        plan_id.clone()
    };

    let Some(plan_info) = roko_cli::plan::discover_plan_by_id(&wd, &resolved_id)
        .map_err(|e| anyhow!("{e}"))? else {
        // NEW: suggest the path-stripped form
        if resolved_id != plan_id {
            anyhow::bail!("plan '{plan_id}' not found (tried ID '{resolved_id}')");
        }
        anyhow::bail!("plan '{plan_id}' not found");
    };
```

### Fix 3: Add `--model` as a local alias on `plan run`, mention it in help

In `crates/roko-cli/src/main.rs`, in the `PlanCmd::Run` variant (around line 1491), add a local
`--model` flag that forwards to the global:

```rust
/// Override the model for this plan run (alias for the global --model flag).
/// Example: roko plan run plans/ --model claude-opus-4-5
#[arg(long, value_name = "MODEL_SLUG")]
model: Option<String>,
```

In the `PlanCmd::Run` handler in `commands/plan.rs`, when `model` is `Some`, merge it into the
`cli_model_override` field that gets passed to `RunConfig` (line 590). This avoids requiring
users to put `--model` before the subcommand.

Also add a visible alias `--force-backend` since the internal code uses that term. Add it as
`#[arg(long, alias = "force-backend", hide_short_help = true)]` on the same field.

### Fix 4: Set `default-run = "roko"` in `Cargo.toml`

In `crates/roko-cli/Cargo.toml`, add after `[package]`:

```toml
[package]
name = "roko-cli"
# ... existing fields ...
default-run = "roko"
```

This is a one-line change. After this, `cargo run -p roko-cli -- plan show demo` works without
`--bin roko`.

### Fix 5: Detect wrong working directory and suggest fix

In `crates/roko-cli/src/commands/plan.rs`, in the `PlanCmd::Run` handler where `resolved_plans_dir`
is checked for existence (around line 309), add a parent-directory check:

```rust
if !resolved_plans_dir.exists() {
    // Check if we're inside a plans directory
    if wd.join("tasks.toml").exists() || wd.join("plan.md").exists() {
        anyhow::bail!(
            "plans directory not found at {:?}. \
             It looks like you're inside a plan directory. \
             Run from the workspace root instead:\n  cd {:?}",
            resolved_plans_dir,
            wd.ancestors().find(|p| p.join("Cargo.toml").exists() || p.join(".roko").exists())
                .unwrap_or(&wd)
        );
    }
    anyhow::bail!("plans directory does not exist: {}", resolved_plans_dir.display());
}
```

### Fix 6: Log a warning when cascade router selects non-configured provider

In `crates/roko-cli/src/dispatch/model_routing.rs`, where cascade router selection is logged,
add a comparison against the configured default:

```rust
// After selecting the cascade route target:
if let Some(configured_default) = &config.default_model {
    if selected_model != configured_default {
        tracing::warn!(
            configured = %configured_default,
            selected = %selected_model,
            "cascade router selected a different model than configured default"
        );
    }
}
```

### Fix 7: Document `--approval` clearly, add `--tui` alias

In `crates/roko-cli/src/main.rs`, on the `approval` field (line 1505-1506):

```rust
/// Launch the connected inline TUI while Runner-v2 runs.
/// Use this to monitor agent output, tokens, and gate progress in real time.
/// Alias: --tui. Without this flag, plan run outputs plain text logs.
#[arg(long, alias = "tui")]
approval: bool,
```

This makes `roko plan run plans/ --tui` work and makes the help text self-explanatory. No
behavior change — just an alias and better docs.

### Fix 8: Include failure reason in zero-token episodes

In the plan completion summary code in `crates/roko-cli/src/commands/plan.rs` (around line
829-835, where per-task results are printed), add failure reason extraction:

```rust
// When printing task result and tokens == 0 and outcome == "orphaned":
if tokens == 0 && matches!(outcome.as_deref(), Some("orphaned")) {
    println!("  {task_id}: FAILED (orphaned — no dispatch occurred; check worktree/provider errors above)");
} else {
    // ... existing printing ...
}
```

## Acceptance Criteria

1. `roko plan run plans/ --fresh` with stale worktrees succeeds: worktrees are pruned before dispatch, not after failure.
2. `roko plan show plans/demo-multistage` (path form) returns the same result as `roko plan show demo-multistage` (ID form).
3. `cargo run -p roko-cli -- plan show demo` works without `--bin roko`.
4. `roko plan run plans/ --tui` is equivalent to `roko plan run plans/ --approval`.
5. `roko plan run plans/ --model claude-sonnet-4-5` works without putting `--model` before the subcommand.
6. Running `roko plan run plans/` from inside a plan directory gives a clear error mentioning the workspace root.
7. When cascade router selects a different model than the config default, a `WARN` log entry appears.
8. A plan where all tasks fail due to worktree errors shows "FAILED (orphaned)" in the summary, not "0 tokens 0 calls $0.00."

## Verification Checklist

- [ ] Create a stale worktree: `git worktree add /tmp/test-wt roko/attempt/stale-branch 2>/dev/null || true`
- [ ] Run `cargo run -p roko-cli --bin roko -- plan run plans/demo-multistage --fresh --engine runner-v2`
- [ ] Verify plan does NOT output "Plan complete: 0/5 tasks" on first real task
- [ ] Run `cargo run -p roko-cli -- plan show plans/demo-multistage` and verify it returns plan info
- [ ] Run `cargo run -p roko-cli -- plan run plans/ --tui` and verify TUI appears (same as `--approval`)
- [ ] `grep -n 'default-run' crates/roko-cli/Cargo.toml` returns `default-run = "roko"`
- [ ] `cargo test -p roko-cli 2>&1 | tail -5` passes

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/Cargo.toml` | Add `default-run = "roko"` to `[package]` section |
| `crates/roko-cli/src/main.rs` | Add `--tui` alias to `approval` flag (line 1505); add local `--model` / `--force-backend` flag to `PlanCmd::Run` (after line 1506) |
| `crates/roko-cli/src/commands/plan.rs` | Add `git worktree prune` call in `--fresh` branch (after line 376); add path-to-ID resolution in `PlanCmd::Show` handler (line 116); add wrong-cwd detection (line 309); update plan completion summary to show "FAILED (orphaned)" when tokens=0 |
| `crates/roko-cli/src/dispatch/model_routing.rs` | Add warning log when cascade router selects non-default model |
