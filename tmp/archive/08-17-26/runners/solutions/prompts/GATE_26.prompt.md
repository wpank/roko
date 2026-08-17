# GATE_26: Add GateService integration tests

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-26`](../ISSUE-TRACKER.md#gate-26)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.26
- Priority: **P1**
- Effort: 4 hours
- Depends on: `GATE_03` (source 4.3), `GATE_04` (source 4.4), `GATE_08` (source 4.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Existing integration tests at `crates/roko-gate/tests/gate_truth.rs` (6 tests) and `crates/roko-gate/tests/rungs.rs` (9 tests) test GateService and rung dispatch separately. After the convergence work, we need integration tests that verify the full pipeline including feedback, classification, SPC alerts, rung selection, and temperament.

## Exact Changes

1. Create `crates/roko-gate/tests/gate_service_integration.rs`.
2. Add tests:
   - `test_feedback_generated_on_failure`: GateService returns `feedback: Some(...)` with classified errors when compile fails.
   - `test_rung_selection_trivial`: `complexity: Some(0)` runs only compile.
   - `test_rung_selection_escalation`: `prior_failures: Some(3)` escalates Trivial to Complex.
   - `test_stub_verdicts_are_skipped`: Missing inputs produce skipped (not passed) verdicts.
   - `test_adaptive_skip_respected`: After 20 consecutive passes, rung is skipped (except compile).
   - `test_temperament_conservative_no_skip`: Conservative temperament never skips.
   - `test_spc_alerts_in_report`: After regime change, SPC alerts appear in report.
   - `test_custom_gates`: Custom gate spec executes via ShellGate.
3. Use real cargo scaffolds (tempdir with Cargo.toml + src/lib.rs) for compile/clippy/test gates.

## Write Scope

- `crates/roko-gate/tests/gate_service_integration.rs`

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

- [ ] All new integration tests pass
- [ ] Tests cover the full task scope (feedback, rung selection, stubs, adaptive, SPC, custom)
- [ ] Tests do not depend on external services (all local)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All new integration tests pass
- Tests cover the full task scope (feedback, rung selection, stubs, adaptive, SPC, custom)
- Tests do not depend on external services (all local)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
