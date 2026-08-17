# DISP_25: Extract Gate Failure Replan to roko-orchestrator

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-25`](../ISSUE-TRACKER.md#disp-25)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.25
- Priority: **P2**
- Effort: 4 hours
- Depends on: `DISP_24` (source 3.24)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`build_gate_failure_plan_revision()` in orchestrate.rs generates a revised plan when gate checks fail. This is a valuable pattern for the live `WorkflowEngine` path -- when a task fails gates, the system should be able to generate a fix-up plan automatically.

The function likely takes gate failure details (which gates failed, error messages, diff context) and produces a revised task list. It should live in `roko-orchestrator` alongside the existing plan execution infrastructure.

## Exact Changes

1. Create `/Users/will/dev/nunchi/roko/roko/crates/roko-orchestrator/src/replan.rs`
2. Extract `build_gate_failure_plan_revision()` from orchestrate.rs into the new module
3. Generalize the function signature to not depend on orchestrate.rs-specific types:
   ```rust
   pub struct GateFailureContext {
       pub task_id: String,
       pub gate_name: String,
       pub failure_message: String,
       pub diff_context: Option<String>,
       pub prior_attempts: u32,
   }

   pub fn build_replan(context: &GateFailureContext) -> Vec<TaskSpec> { ... }
   ```
4. Re-export from `crates/roko-orchestrator/src/lib.rs`
5. If orchestrate.rs still exports this function (for backward compat), delegate to the new location
6. Add a unit test for the replan function

## Design Guidance

The extracted function should be pure (no I/O, no state mutation). It takes failure context and returns new task specs. The caller (WorkflowEngine or runner) handles persistence, state updates, and dispatch. This separation allows the replan logic to be tested independently.

## Write Scope

- `crates/roko-orchestrator/src/replan.rs`
- `crates/roko-orchestrator/src/lib.rs`
- `crates/roko-cli/src/orchestrate.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `crates/roko-orchestrator/src/replan.rs` exists and is re-exported
- [ ] orchestrate.rs callers (if any) delegate to the new location

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `crates/roko-orchestrator/src/replan.rs` exists and is re-exported
- orchestrate.rs callers (if any) delegate to the new location
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
