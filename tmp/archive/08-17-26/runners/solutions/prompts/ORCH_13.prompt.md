# ORCH_13: Failure Policy Configuration

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-13`](../ISSUE-TRACKER.md#orch-13)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.13
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_13 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Gate failure recovery in `PipelineStateV2::step()` (lines 662-687) is hardcoded:
```rust
if self.autofix_attempts < self.config.max_autofix_attempts {
    // autofix
} else if self.iteration < self.config.max_iterations {
    // re-implement
} else {
    // halt
}
```

All gate failures are treated identically. A trivial clippy warning triggers the same recovery as a type error. The mega-parity runner's gate-specific recovery table shows that compile failures -> autofix, test failures -> reimplement, clippy -> autofix (trivial), fmt -> run formatter.

## Exact Changes

1. Add a `FailurePolicy` config struct:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct FailurePolicy {
       pub default_action: FailureAction,
       pub default_max_attempts: u32,
       pub per_gate: HashMap<String, GateFailurePolicy>,
   }
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct GateFailurePolicy {
       pub action: FailureAction,
       pub max_attempts: u32,
   }
   #[derive(Debug, Clone, Copy, Serialize, Deserialize)]
   pub enum FailureAction {
       AutoFix,
       Reimplement,
       Skip,
       Halt,
       Escalate,
   }
   ```
2. Add `failure_policy: FailurePolicy` to `WorkflowConfig` with a sensible default.
3. Add TOML parsing for `[workflow.failure]` and `[workflow.failure.<gate>]` tables.
4. Replace the hardcoded match arms in `step()` with a lookup against the failure policy:
   ```rust
   let policy = self.config.failure_policy.policy_for(&gate);
   match policy.action {
       FailureAction::AutoFix if self.autofix_attempts < policy.max_attempts => { ... }
       FailureAction::Reimplement if self.iteration < self.config.max_iterations => { ... }
       FailureAction::Skip => { /* advance past gates */ }
       FailureAction::Escalate => { /* emit EscalateModel action */ }
       _ => { /* halt */ }
   }
   ```
5. Add a new `PipelineOutput::EscalateModel` variant for model escalation.

## Design Guidance

The default failure policy should match current behavior exactly (autofix first, then reimplement, then halt) so this is a zero-regression change. Per-gate overrides are additive. The `Escalate` action is a new concept that the EffectDriver will need to handle (switch to a stronger model and retry). This task only adds the state machine support; the EffectDriver handling is a separate task.

## Write Scope

- `crates/roko-runtime/src/pipeline_state.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Default `FailurePolicy` produces identical behavior to current hardcoded logic
- [ ] Per-gate override: compile -> AutoFix(3), test -> Reimplement(2), clippy -> AutoFix(1)
- [ ] TOML `[workflow.failure.compile]` parses correctly
- [ ] All existing `PipelineStateV2` tests pass unchanged
- [ ] New test: custom policy routes test failures to Reimplement

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_13 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Default `FailurePolicy` produces identical behavior to current hardcoded logic
- Per-gate override: compile -> AutoFix(3), test -> Reimplement(2), clippy -> AutoFix(1)
- TOML `[workflow.failure.compile]` parses correctly
- All existing `PipelineStateV2` tests pass unchanged
- New test: custom policy routes test failures to Reimplement
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_13 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
