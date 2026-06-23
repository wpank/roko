# ORCH_18: Wire Anti-Pattern Checks as Pre-Gate Step

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-18`](../ISSUE-TRACKER.md#orch-18)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.18
- Priority: **P1**
- Effort: 3 hours
- Depends on: `ORCH_17` (source 2.17)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Anti-pattern checks should run before any compilation gate. They execute in milliseconds and catch structural mistakes that compilation/tests do not detect (e.g., stubs that always return success, inline prompt strings).

The EffectDriver's `run_gates()` method (line 280-333) runs gates via `self.services.gate_runner.run_gates(config)`. Anti-pattern checks should run before this call.

## Exact Changes

1. Add `anti_pattern_registry: Option<Arc<AntiPatternRegistry>>` to `EffectServices`.
2. In `EffectDriver::run_gates()`, if `anti_pattern_registry` is `Some`:
   - Run AP checks first
   - If any AP check has `Severity::Error`, return `PipelineInput::GateFailed` immediately (before compilation)
   - If only `Severity::Warning`, include violations in the gate report but do not fail
3. Include AP check results in the `GateReport` verdicts as `ap:<id>` named gates.
4. Pass the task's `ap_exempt` list (from tasks.toml) through to the AP runner.

## Design Guidance

AP checks are "rung -1" -- they run before rung 0 (compile). They should be reported as gate verdicts so the affect policy and learning subsystem can track them. Use the existing `GateVerdict` struct with `gate_name: "ap:AP-7"`.

## Write Scope

- `crates/roko-runtime/src/effect_driver.rs`
- `crates/roko-runtime/src/workflow_engine.rs`

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

- [ ] AP checks run before compilation gates
- [ ] AP Error violations fail the gate immediately (no wasted compile time)
- [ ] AP Warning violations are reported but do not fail the gate
- [ ] AP violations appear in `GateReport` as `ap:<id>` verdicts
- [ ] `ap_exempt` list suppresses specific checks

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- AP checks run before compilation gates
- AP Error violations fail the gate immediately (no wasted compile time)
- AP Warning violations are reported but do not fail the gate
- AP violations appear in `GateReport` as `ap:<id>` verdicts
- `ap_exempt` list suppresses specific checks
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
