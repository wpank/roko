# ORCH_23: Wire DaimonState as AffectPolicy for WorkflowEngine

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-23`](../ISSUE-TRACKER.md#orch-23)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.23
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_23 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

EffectDriver supports `AffectPolicy` via `EffectServices::affect_policy` (line 49) but Runner v2 only provides `DaimonPolicy::default()`. The full `DaimonState` affect engine in orchestrate.rs (`load_or_new` at line 299) provides somatic signals, strategy coordinates, and dispatch modulation.

`roko-daimon` crate has `AffectEngine`, `DaimonState`, `StrategyCoordinates`. These need to be wrapped in an `AffectPolicy` impl and wired into EffectServices at CLI entry points.

## Exact Changes

1. Create a `DaimonAffectPolicy` adapter in `crates/roko-daimon/` that wraps `DaimonState` and implements `roko_core::foundation::AffectPolicy`.
2. `pre_dispatch()` -> query DaimonState for current behavioral state and PAD values.
3. `modulate_dispatch()` -> compute exploration rate, tier bias, turn limit factor from DaimonState.
4. `on_task_outcome()` -> feed outcome back to DaimonState for learning.
5. `on_gate_result()` -> feed gate results for somatic marker updates.
6. Wire into CLI entry points (`crates/roko-cli/src/run.rs`) where `EffectServices` is constructed -- load `DaimonState` from `.roko/state/daimon.json` and wrap in `DaimonAffectPolicy`.

## Design Guidance

The DaimonState should persist across runs (load from disk, save on completion). The affect modulation should be conservative -- start with small adjustments (exploration_rate 0.0-0.3) until the system is validated. The fallback should be `DaimonPolicy::default()` which provides neutral modulation.

## Write Scope

- `crates/roko-daimon/src/lib.rs`
- `crates/roko-cli/src/run.rs`
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

- [ ] `DaimonAffectPolicy` implements `AffectPolicy` trait
- [ ] EffectDriver receives non-default modulation values from DaimonState
- [ ] DaimonState persists to `.roko/state/daimon.json` between runs
- [ ] Fallback: when DaimonState fails to load, neutral modulation is used

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_23 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `DaimonAffectPolicy` implements `AffectPolicy` trait
- EffectDriver receives non-default modulation values from DaimonState
- DaimonState persists to `.roko/state/daimon.json` between runs
- Fallback: when DaimonState fails to load, neutral modulation is used
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_23 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
