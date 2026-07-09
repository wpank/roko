# W2-D: Add Spinners for Long Operations

**Priority**: P1 — makes demo presentable
**Effort**: 1-2 hours
**Files to modify**: 3-4 files
**Dependencies**: None (but pairs well with W2-A tracing-to-file)

## Problem

Long-running operations (`prd draft new`, `prd plan`, `plan run`) produce no user-facing output for 30-300 seconds. The user sees silence or tracing noise.

## Fix

Add `indicatif` spinners to wrap LLM dispatch and plan execution.

## Files to Modify

### 1. Add dependency: `crates/roko-cli/Cargo.toml`

```toml
[dependencies]
indicatif = "0.17"
```

### 2. File: `crates/roko-cli/src/prd.rs`

Find the `generate_plan_from_prd_with_outcome()` function (line ~926). Before the agent dispatch call, start a spinner. After it completes, finish the spinner.

```rust
use indicatif::{ProgressBar, ProgressStyle};

// Before agent dispatch:
let spinner = ProgressBar::new_spinner()
    .with_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg} ({elapsed})")
            .unwrap()
    )
    .with_message(format!("Generating plan from PRD: {slug}"));
spinner.enable_steady_tick(std::time::Duration::from_millis(80));

// ... agent dispatch ...

// After success:
spinner.finish_with_message(format!("Plan generated: {}", plans_root.display()));

// After failure:
spinner.finish_with_message("Plan generation failed");
```

Similarly, wrap the `cmd_draft_new` function (find where `prd draft new` dispatches an agent):

```rust
let spinner = ProgressBar::new_spinner()
    .with_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg} ({elapsed})").unwrap())
    .with_message("Generating PRD draft...");
spinner.enable_steady_tick(std::time::Duration::from_millis(80));
// ... agent dispatch ...
spinner.finish_with_message("Draft generated");
```

### 3. File: `crates/roko-cli/src/commands/plan.rs` (or wherever plan run execution happens)

For `plan run`, use `indicatif::MultiProgress` to show per-task progress:

```rust
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

let multi = MultiProgress::new();
let overall = multi.add(ProgressBar::new_spinner()
    .with_style(ProgressStyle::default_spinner().template("{spinner:.cyan} {msg} ({elapsed})").unwrap())
    .with_message(format!("Running plan: {} ({} tasks)", plan_name, task_count)));
overall.enable_steady_tick(std::time::Duration::from_millis(80));

// For each task as it starts:
let task_bar = multi.add(ProgressBar::new_spinner()
    .with_style(ProgressStyle::default_spinner().template("  {spinner:.dim} {msg}").unwrap())
    .with_message(format!("[{}/{}] {} — executing...", i+1, total, task_id)));
task_bar.enable_steady_tick(std::time::Duration::from_millis(80));

// When task completes:
task_bar.finish_with_message(format!("[{}/{}] {} — done", i+1, total, task_id));

// When all done:
overall.finish_with_message(format!("Plan complete: {} tasks", task_count));
```

The exact integration point depends on how the plan runner exposes progress. Search for where tasks are dispatched in the runner:
```bash
grep -n 'dispatch\|execute.*task\|run_task' crates/roko-cli/src/orchestrate.rs | head -20
grep -n 'dispatch\|execute.*task\|run_task' crates/roko-cli/src/runner.rs | head -20
```

### 4. Helper: Create a spinner utility

To avoid duplication, create a small helper:

```rust
// In crates/roko-cli/src/spinner.rs (new file) or inline
pub fn cli_spinner(msg: impl Into<String>) -> ProgressBar {
    let pb = ProgressBar::new_spinner()
        .with_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg} ({elapsed})")
                .unwrap()
        )
        .with_message(msg.into());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}
```

## Important: Spinner vs Tracing Interaction

If W2-A (tracing to file) is NOT done yet, spinners and tracing will fight over stderr. The spinner will be interrupted by WARN/INFO lines.

**Workaround if W2-A not done**: Suspend the spinner before tracing can write, or accept some visual noise. `indicatif` handles this reasonably — spinners redraw after other output.

If W2-A IS done (tracing goes to file), spinners will have clean stderr.

## Agent Prompt

```
Read /Users/will/dev/nunchi/roko/roko/tmp/solutions/demo-running/batches/W2-D-spinners.md and implement all changes described in it. Add indicatif = "0.17" to crates/roko-cli/Cargo.toml. Add spinners to prd.rs (draft new + plan generation) and to the plan run execution path. Do NOT run cargo build/test/clippy/fmt — compilation is deferred. Mark the checklist items as done.
```

## Commit

This batch is committed with all Wave 2 batches together. Do not commit individually.

## Checklist

- [x] Add `indicatif = "0.17"` to roko-cli Cargo.toml
- [x] Add spinner to `prd draft new` agent dispatch
- [x] Add spinner to `prd plan` agent dispatch
- [x] Add spinner/multi-progress to `plan run` execution
- [x] Spinners show elapsed time
- [x] Spinners finish with success/failure message
- [ ] Verify: long operations show animated spinner
- [ ] Pre-commit checks pass
