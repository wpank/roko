# INNO_17: Define FailureKind taxonomy for agent debugging

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-17`](../ISSUE-TRACKER.md#inno-17)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.17
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Agent failures are currently unclassified. A failure from token exhaustion looks
the same as a failure from wrong model selection. Systematic debugging requires
a taxonomy.

## Exact Changes

1. Create `crates/roko-learn/src/failure_taxonomy.rs`.
2. Define `FailureKind` enum:
   - `QualityFailure { gate_rung: String, error_hash: String, is_recurring: bool }`
   - `ConvergenceFailure { iterations: usize, repeated_error_hashes: Vec<String> }`
   - `ResourceFailure { kind: ResourceKind, used: f64, limit: f64 }`
   - `ToolFailure { tool_name: String, error: String, is_permission: bool }`
   - `ComprehensionFailure { evidence: Vec<String> }`
3. Define `ResourceKind` enum: `Tokens`, `Budget`, `Time`, `Context`.
4. Implement `classify(task_result: &TaskResult, episodes: &[Episode]) -> FailureKind`:
   - Check for repeated error hashes -> ConvergenceFailure
   - Check for budget/context exceeded -> ResourceFailure
   - Check for tool errors -> ToolFailure
   - Check for wrong-direction changes -> ComprehensionFailure
   - Default to QualityFailure
5. Add `is_recurring` check: compare error hash against past episodes.
6. Add `pub mod failure_taxonomy;` to `crates/roko-learn/src/lib.rs`.

## Write Scope

- `crates/roko-learn/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] A task that fails 3 times with the same error is classified as ConvergenceFailure
- [ ] A task that runs out of tokens is classified as ResourceFailure
- [ ] A task where `bash` returns permission denied is classified as ToolFailure with `is_permission: true`
- [ ] Unit tests for each variant

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A task that fails 3 times with the same error is classified as ConvergenceFailure
- A task that runs out of tokens is classified as ResourceFailure
- A task where `bash` returns permission denied is classified as ToolFailure with `is_permission: true`
- Unit tests for each variant
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
