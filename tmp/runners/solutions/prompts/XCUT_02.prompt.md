# XCUT_02: Implement Structured Error Context Chain

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-02`](../ISSUE-TRACKER.md#xcut-02)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.2
- Priority: **P5**
- Effort: 4 hours
- Depends on: `XCUT_01` (source 19.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Error messages like `"substrate error: failed"` lose the causal chain. When a gate failure triggers an autofix agent, the agent sees the final error string but not nested cause information (e.g., clippy lint X in file Y at line Z). The `RokoError` variants carry `String` payloads, not structured context. The gate dispatch in Runner v2 (`crates/roko-cli/src/runner/gate_dispatch.rs`) propagates error strings that the autofix agent must parse.

## Exact Changes

1. Add `ErrorContext` struct to `crates/roko-core/src/error/mod.rs`:
   ```rust
   pub struct ErrorContext {
       pub subsystem: &'static str,  // "gate", "agent", "substrate"
       pub operation: String,         // "clippy", "compile", "dispatch"
       pub detail: String,            // human-readable
       pub source_file: Option<String>,
       pub source_line: Option<u32>,
       pub suggestions: Vec<String>,
   }
   ```
2. Add `RokoError::Contextual { context: ErrorContext, #[source] source: Box<RokoError> }` variant.
3. Add `RokoError::with_context(self, subsystem, operation)` builder method.
4. In `GateService::run_gates()`, wrap gate errors with `ErrorContext` containing gate name, rung index, and stderr snippet.
5. In `gate_dispatch.rs`, propagate structured context to the autofix agent prompt.

## Write Scope

- `crates/roko-core/src/error/mod.rs`
- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-cli/src/runner/gate_dispatch.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Gate failure errors contain subsystem, operation, and detail fields
- [ ] Autofix agent receives structured error context (not just flat string)
- [ ] Error `Display` impl still produces a human-readable chain

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Gate failure errors contain subsystem, operation, and detail fields
- Autofix agent receives structured error context (not just flat string)
- Error `Display` impl still produces a human-readable chain
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
