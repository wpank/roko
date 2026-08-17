# GATE_01: Add complexity and prior_failures to GateConfig

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-01`](../ISSUE-TRACKER.md#gate-01)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.1
- Priority: **P0**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`GateConfig` at `crates/roko-core/src/foundation.rs:271` currently has 4 fields: `workdir`, `enabled_gates`, `shell_gates`, `max_rung`. Each caller implements rung selection independently. Making complexity and prior_failures part of GateConfig lets GateService perform rung selection internally via `rung_selector::select_rungs()`.

`PlanComplexity` is defined at `crates/roko-gate/src/rung_selector.rs:25` with variants: Trivial, Simple, Standard, Complex. `roko-core` already depends on `roko-gate` types (via re-exports) or can reference the enum by path. If circular dependency is an issue, the fields should be typed as `Option<u8>` (0-3 mapping to complexity) and `Option<u32>` for prior_failures, avoiding direct dependency on roko-gate from roko-core.

## Exact Changes

1. Open `crates/roko-core/src/foundation.rs` and locate `pub struct GateConfig` at line 271.
2. Add two optional fields after `max_rung`:
   ```rust
   /// Plan complexity for rung selection (0=Trivial, 1=Simple, 2=Standard, 3=Complex).
   /// When Some, GateService uses rung_selector::select_rungs() to determine which gates to run.
   /// When None, GateService runs all enabled_gates as-is.
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub complexity: Option<u8>,
   /// Number of prior gate failures for this task. Used for escalation ladder
   /// (each failure promotes complexity one tier, saturating at Complex).
   #[serde(default, skip_serializing_if = "Option::is_none")]
   pub prior_failures: Option<u32>,
   ```
3. Find all sites constructing `GateConfig` (search `GateConfig {` in `crates/`). Add `complexity: None, prior_failures: None` to each. Known sites:
   - `crates/roko-runtime/src/effect_driver.rs:285` in `run_gates()`
   - `crates/roko-runtime/src/effect_driver.rs` test mocks
   - `crates/roko-runtime/src/workflow_engine.rs` test mocks
   - `crates/roko-gate/src/gate_service.rs` tests
   - `crates/roko-gate/tests/gate_truth.rs`

## Design Guidance

Use `Option<u8>` for complexity rather than importing `PlanComplexity` from roko-gate into roko-core. roko-core is the kernel crate and must not depend on roko-gate (circular). GateService will convert `u8 -> PlanComplexity` internally. Document the 0-3 mapping in the field doc-comment.

## Write Scope

- `crates/roko-core/src/foundation.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All construction sites updated (grep `GateConfig {` returns no compile errors)
- [ ] New fields are `Option` with serde defaults (backward-compatible deserialization)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All construction sites updated (grep `GateConfig {` returns no compile errors)
- New fields are `Option` with serde defaults (backward-compatible deserialization)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
