# ORCH_09: Add Rung and Confidence Fields to GateVerdict (ORCH-018)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-09`](../ISSUE-TRACKER.md#orch-09)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.9
- Priority: **P1**
- Effort: 3 hours
- Depends on: `ORCH_08` (source 2.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`GateVerdict` in `crates/roko-core/src/foundation.rs:284-296` carries `gate_name`, `passed`, `skipped`, `skip_reason`, `output`, `duration_ms` but not `rung` or `confidence`. The EffectDriver re-derives rung from the gate name (ORCH-008) and uses hardcoded confidence (1.0 for deterministic, 0.5 for heuristic) at lines 308-314:
```rust
let rung = rung_for_gate_name(&verdict.gate_name);
let confidence = if rung <= 4 { 1.0_f64 } else { 0.5_f64 };
```

The code includes a TODO:
```rust
// TODO: add `rung: u8` and `confidence: f64` to GateVerdict in
// roko-core/src/foundation.rs so callers don't need to re-derive them.
```

## Exact Changes

1. Add two fields to `GateVerdict` in `crates/roko-core/src/foundation.rs`:
   ```rust
   #[serde(default)]
   pub rung: u8,
   #[serde(default = "default_confidence")]
   pub confidence: f64,
   ```
2. Use `#[serde(default)]` on both fields for backward compatibility with existing serialized verdicts.
3. Update `GateRunner` implementations in `crates/roko-gate/` to populate `rung` and `confidence` when creating verdicts.
4. Update the EffectDriver's `run_gates()` method to read `verdict.rung` and `verdict.confidence` directly instead of re-deriving them.
5. Remove the TODO comment from `effect_driver.rs`.

## Design Guidance

Use `#[serde(default)]` so old serialized verdicts (without rung/confidence) deserialize correctly with `rung: 0, confidence: 0.0`. Callers should check if `confidence == 0.0` and re-derive if needed, as a migration path.

## Write Scope

- `crates/roko-core/src/foundation.rs`
- `crates/roko-runtime/src/effect_driver.rs`

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

- [ ] `GateVerdict` has `rung: u8` and `confidence: f64` fields
- [ ] Deserialization of old JSON without these fields works (default values)
- [ ] EffectDriver reads `verdict.rung` and `verdict.confidence` directly
- [ ] Both TODO comments removed

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `GateVerdict` has `rung: u8` and `confidence: f64` fields
- Deserialization of old JSON without these fields works (default values)
- EffectDriver reads `verdict.rung` and `verdict.confidence` directly
- Both TODO comments removed
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
