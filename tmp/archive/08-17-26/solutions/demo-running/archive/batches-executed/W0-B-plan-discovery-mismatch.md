# W0-B: Fix Plan Discovery Mismatch Between `prd plan` and `plan run`

**Priority**: P0 — demo pipeline breaks: plans generated but not found
**Effort**: 30 minutes
**Files to modify**: 2 files
**Dependencies**: None

## Problem

`roko prd plan <slug>` generates plans and writes `tasks.toml` successfully, but `roko plan run .roko/plans` reports "No plans found in .roko/plans". The plan generation and plan execution use **different discovery mechanisms**.

## Root Cause

### Two discovery systems that don't agree:

1. **`prd plan` writes plans via** `crates/roko-cli/src/prd.rs` line 1071-1080:
   - Creates `.roko/plans/<slug>/tasks.toml`
   - Also writes optional `plan.md` to the same directory
   - This is correct — plans go into subdirectories

2. **`plan run` loads plans via** `crates/roko-cli/src/runner/plan_loader.rs` line 50-83:
   - Checks if `<dir>/tasks.toml` exists (single plan mode)
   - Scans immediate subdirectories for `tasks.toml` files
   - This DOES find plans in subdirectories — **this part is fine**

3. **BUT `plan run` ALSO calls** `validate_before_run()` in `crates/roko-cli/src/commands/plan.rs` which calls `discover_plans()` from `roko-orchestrator`:
   - `discover_plans()` in `crates/roko-orchestrator/src/plan_discovery.rs` line 161-209
   - This looks for **`plan.md` files** inside subdirectories (line 199-201)
   - It ALSO looks for `.md` files directly in the plans dir
   - It does NOT look for `tasks.toml` files

4. **The real blocker is the `validate_before_run()` early exit** in `plan.rs` line 222:
   ```rust
   if let Some(exit_code) = validate_before_run(&plans_dir) {
       return Ok(exit_code);
   }
   ```
   This calls `discover_plans()` which only finds `plan.md` files. If `prd plan` didn't generate a `plan.md` (the fenced block extraction only requires `toml`, `plan.md` is optional), then `discover_plans()` finds nothing and `validate_before_run()` returns `Some(1)`, causing "No plans found".

### The secondary issue: fenced block extraction may fail

The agent output may not contain properly fenced ```toml blocks, especially with weaker models like glm51. The extraction at `prd.rs:1063-1064` only looks for ```` ```toml ```` and ```` ```tasks.toml ```` — not loose TOML that isn't in code fences.

## Exact Code to Change

### Fix 1: Make `validate_before_run()` use the same loader as `plan run`

**File**: `crates/roko-cli/src/commands/plan.rs`

Find `validate_before_run()` (around line 965). It currently uses `roko_orchestrator::discover_plans()` which only finds `plan.md` files. Change it to check for `tasks.toml` files instead.

**Current pattern:**
```rust
fn validate_before_run(plans_dir: &Path) -> Option<i32> {
    // ... uses discover_plans() which finds plan.md files
}
```

**New pattern:**
```rust
fn validate_before_run(plans_dir: &Path) -> Option<i32> {
    // Use the same loader that plan run uses
    match roko_cli::runner::plan_loader::load_plans(plans_dir) {
        Ok(plans) if plans.is_empty() => {
            eprintln!("error: No plans found in {}", plans_dir.display());
            Some(1)
        }
        Ok(plans) => {
            // Plans found — continue with validation
            // Run the existing validation logic on each plan
            for plan in &plans {
                // ... existing validation (field checks, etc.)
            }
            None // validation passed
        }
        Err(e) => {
            eprintln!("error: cannot load plans from {}: {e}", plans_dir.display());
            Some(1)
        }
    }
}
```

**Alternatively (simpler):** Just bypass `validate_before_run()` entirely and let `load_plans()` in the actual run path do the checking. The validation is redundant since the run path will fail anyway if plans are missing. You could delete the call at line 222 or make it non-blocking:

```rust
// BEFORE:
if let Some(exit_code) = validate_before_run(&plans_dir) {
    return Ok(exit_code);
}

// AFTER:
// Validation uses plan_validate which may not find all plan formats.
// Let the actual loader report errors — it scans tasks.toml files.
```

### Fix 2: Ensure `prd plan` writes plan.md alongside tasks.toml

**File**: `crates/roko-cli/src/prd.rs` — around line 1081-1092

The code already tries to extract `plan.md` from agent output. But if the agent doesn't produce a fenced `plan.md` block, no `plan.md` is written. Add a minimal `plan.md` fallback:

```rust
        if let Some(toml_content) = toml_content {
            let plan_dir = plans_root.join(slug);
            std::fs::create_dir_all(&plan_dir)?;
            std::fs::write(plan_dir.join("tasks.toml"), toml_content)?;

            // Write plan.md (from agent output or minimal fallback)
            let plan_md_content = extract_fenced_block(&output, "plan.md")
                .or_else(|| extract_fenced_block(&output, "markdown"))
                .or_else(|| extract_fenced_block(&output, "md"));

            if let Some(plan_md) = plan_md_content {
                std::fs::write(plan_dir.join("plan.md"), plan_md)?;
            } else {
                // Write minimal plan.md so discover_plans() finds this directory
                let minimal_plan_md = format!(
                    "---\nplan: {slug}\ntitle: {slug}\n---\n\n# {slug}\n\nGenerated plan.\n"
                );
                std::fs::write(plan_dir.join("plan.md"), minimal_plan_md)?;
            }
        }
```

### Fix 3: Improve fenced block extraction robustness

**File**: `crates/roko-cli/src/prd.rs` — `extract_fenced_block()` at line 1424-1457

Add fallback: if no fenced `toml` block is found, try to find TOML content by looking for `[meta]` and `[[task]]` markers in the raw output:

```rust
fn extract_toml_content_fallback(output: &str) -> Option<&str> {
    // Look for [meta] section start
    let meta_start = output.find("[meta]")?;
    // Find the start of the line containing [meta]
    let line_start = output[..meta_start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    // Take everything from [meta] to the end, trimming any trailing non-TOML
    let candidate = &output[line_start..];
    // Verify it has at least one [[task]]
    if candidate.contains("[[task]]") {
        Some(candidate.trim())
    } else {
        None
    }
}
```

Then in the extraction code:
```rust
        let toml_content = extract_fenced_block(&output, "toml")
            .or_else(|| extract_fenced_block(&output, "tasks.toml"))
            .or_else(|| extract_toml_content_fallback(&output));  // ← ADD fallback
```

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W0-B-plan-discovery-mismatch.md and implement all changes described in it. The key fixes are: (1) make validate_before_run() use the same loader as plan run (tasks.toml based, not plan.md based), (2) ensure prd plan always writes a plan.md alongside tasks.toml, (3) add TOML fallback extraction when fenced blocks aren't found. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with Wave 0 (critical pipeline fixes). Do not commit individually.

## Checklist

- [x] Fix `validate_before_run()` to use `plan_loader::load_plans()` (not `discover_plans()`)
- [x] Or remove/bypass `validate_before_run()` call entirely
- [x] Add minimal `plan.md` fallback when agent doesn't produce one
- [x] Add `extract_toml_content_fallback()` for non-fenced TOML extraction
- [x] Add fallback to the extraction chain in `generate_plan_from_prd_with_outcome()`
- [ ] Verify: `prd plan` creates both `tasks.toml` and `plan.md` in `.roko/plans/<slug>/`
- [ ] Verify: `plan run .roko/plans` finds plans after `prd plan` runs
