# W1-A: Fix `plan run` Path Resolution with `--repo`

**Priority**: P0 — blocks demo pipeline
**Effort**: 15-30 minutes
**Files to modify**: 1 file
**Dependencies**: None

## Problem

`roko --repo /tmp/xyz plan run plans/` fails with "No plans found in plans/" because `plans_dir` is used raw without resolving it relative to `--repo`. The `validate_before_run()` function also uses `std::env::current_dir()` instead of the workdir.

## Root Cause

In `crates/roko-cli/src/commands/plan.rs`, the `PlanCmd::Run` handler calls `validate_before_run(&plans_dir)` BEFORE resolving the workdir. And `validate_before_run()` internally calls `std::env::current_dir()` to find `roko.toml`, not the `--repo` path.

## Exact Code to Change

### File: `crates/roko-cli/src/commands/plan.rs`

### Change 1: PlanCmd::Run handler (lines 209-240)

**Current code** (lines 209-231):
```rust
PlanCmd::Run {
    plans_dir,
    workdir,
    resume_plan,
    approval,
    max_retries,
    max_tasks,
    dry_run,
    fresh,
    force_resume,
} => {
    // Mandatory validation: reject malformed plans before execution
    if let Some(exit_code) = validate_before_run(&plans_dir) {
        return Ok(exit_code);
    }

    // Dry-run mode: parse plans + show summary without executing
    if dry_run {
        return cmd_plan_dry_run(&plans_dir, cli).await;
    }

    let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));
```

**New code**:
```rust
PlanCmd::Run {
    plans_dir,
    workdir,
    resume_plan,
    approval,
    max_retries,
    max_tasks,
    dry_run,
    fresh,
    force_resume,
} => {
    // Resolve workdir FIRST (before using plans_dir)
    let wd = workdir.unwrap_or_else(|| resolve_workdir(cli));

    // Resolve plans_dir relative to workdir if not absolute
    let resolved_plans_dir = if plans_dir.is_absolute() {
        plans_dir.clone()
    } else {
        wd.join(&plans_dir)
    };

    // Mandatory validation: reject malformed plans before execution
    if let Some(exit_code) = validate_before_run(&resolved_plans_dir, &wd) {
        return Ok(exit_code);
    }

    // Dry-run mode: parse plans + show summary without executing
    if dry_run {
        return cmd_plan_dry_run(&resolved_plans_dir, cli).await;
    }
```

Then replace ALL remaining uses of `plans_dir` below this point in the Run handler with `resolved_plans_dir`.

### Change 2: validate_before_run() signature + body (lines ~965-990)

**Current signature**:
```rust
fn validate_before_run(plans_dir: &Path) -> Option<i32> {
```

**New signature**:
```rust
fn validate_before_run(plans_dir: &Path, workdir: &Path) -> Option<i32> {
```

**Current body** (uses `std::env::current_dir()`):
```rust
fn validate_before_run(plans_dir: &Path) -> Option<i32> {
    let current_dir = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(error) => {
            eprintln!("error: cannot resolve cwd for validation: {error}");
            return Some(1);
        }
    };

    let config_path = current_dir.join("roko.toml");
```

**New body** (uses passed workdir):
```rust
fn validate_before_run(plans_dir: &Path, workdir: &Path) -> Option<i32> {
    let config_path = workdir.join("roko.toml");
```

Remove the `current_dir` resolution entirely — use the passed `workdir` parameter wherever `current_dir` was used in this function.

### Change 3: Also fix the PlanCmd::Validate handler

Find the `PlanCmd::Validate` handler (around line 197-208). It likely has a similar pattern where `dir` (the plans directory argument) is not resolved relative to workdir. Apply the same pattern:

```rust
PlanCmd::Validate { dir } => {
    let wd = resolve_workdir(cli);
    let resolved_dir = if dir.is_absolute() {
        dir.clone()
    } else {
        wd.join(&dir)
    };
    // Use resolved_dir instead of dir
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W1-A-plan-run-path.md and implement all changes described in it. Follow the code changes exactly as specified. Do NOT run cargo build/test/clippy/fmt — compilation is deferred to a later phase. Just make the code changes and mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 1 batches together. Do not commit individually.

## Verification (deferred to Phase 2)

After all waves are implemented and compiled:
```bash
mkdir -p /tmp/roko-path-test && cd /tmp/roko-path-test
cargo run -p roko-cli -- init
mkdir -p .roko/plans
echo '[meta]\nplan = "test"\n\n[[task]]\nid = "t1"\ntitle = "Test"\nrole = "engineer"\nprompt = "hello"' > .roko/plans/tasks.toml
cd /
cargo run -p roko-cli -- --repo /tmp/roko-path-test plan validate .roko/plans
# Should find the plan (not "No plans found")
```

## Checklist

- [x] Move `resolve_workdir()` call above `validate_before_run()` in PlanCmd::Run
- [x] Resolve `plans_dir` relative to workdir when not absolute
- [x] Update `validate_before_run()` to accept `workdir` param
- [x] Remove `std::env::current_dir()` from `validate_before_run()`
- [x] Fix PlanCmd::Validate handler similarly
- [x] Replace all uses of raw `plans_dir` with `resolved_plans_dir` in Run handler
- [ ] Verify: `--repo /tmp/test plan run plans/` finds plans
- [ ] Verify: absolute paths still work
- [ ] Pre-commit checks pass
