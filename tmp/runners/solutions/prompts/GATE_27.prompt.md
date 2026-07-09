# GATE_27: Update GateService documentation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-27`](../ISSUE-TRACKER.md#gate-27)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.27
- Priority: **P2**
- Effort: 1 hour
- Depends on: `GATE_03` (source 4.3), `GATE_04` (source 4.4), `GATE_08` (source 4.8), `GATE_11` (source 4.11), `GATE_12` (source 4.12), `GATE_13` (source 4.13), `GATE_14` (source 4.14)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

After all the convergence work, GateService's module-level documentation and struct-level documentation should reflect its new capabilities: rung selection, feedback generation, failure classification, SPC draining, Hotelling observation, domain profiles, temperament, custom gates, and event emission.

## Exact Changes

1. Update the module-level doc comment in `gate_service.rs` to describe the full feature set.
2. Update the `GateService` struct doc comment to list all builder methods and their purpose.
3. Update the `run_gates()` method doc comment to describe the full pipeline: rung selection -> ordering -> adaptive skip -> execution -> feedback -> classification -> SPC drain -> Hotelling -> report.
4. Add a "Usage" section showing the builder pattern:
   ```rust
   /// let svc = GateService::new()
   ///     .with_adaptive_thresholds(thresholds)
   ///     .with_temperament(Temperament::Conservative)
   ///     .with_custom_gates(custom_specs)
   ///     .with_event_sink(tx, run_id);
   ```

## Write Scope

- `crates/roko-gate/src/gate_service.rs`

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

- [ ] `cargo doc -p roko-gate --no-deps` generates clean documentation
- [ ] GateService documentation describes all builder methods
- [ ] run_gates() documentation describes the full pipeline

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- GateService documentation describes all builder methods
- run_gates() documentation describes the full pipeline
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
