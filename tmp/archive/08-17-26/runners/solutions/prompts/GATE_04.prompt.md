# GATE_04: Add rung selection to GateService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#gate-04`](../ISSUE-TRACKER.md#gate-04)
- Source: `tmp/solutions/roko/tasks/04-GATE-PIPELINE.md` — Task 4.4
- Priority: **P0**
- Effort: 3 hours
- Depends on: `GATE_01` (source 4.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: GATE_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Currently GateService runs all `enabled_gates` from GateConfig in rung order. With `complexity` and `prior_failures` now available on GateConfig (Task 4.1), GateService can perform rung selection internally using `rung_selector::select_rungs()` (at `crates/roko-gate/src/rung_selector.rs`).

When `GateConfig.complexity` is `Some`, GateService should compute the effective rung set and filter `enabled_gates` to only those in the selected set. When `None`, behavior is unchanged (all enabled_gates run).

## Exact Changes

1. Import `crate::rung_selector::{PlanComplexity, RungCaps, select_rungs, Rung}` in gate_service.rs.
2. Add a helper to convert `u8 -> PlanComplexity`:
   ```rust
   fn complexity_from_u8(v: u8) -> PlanComplexity {
       match v {
           0 => PlanComplexity::Trivial,
           1 => PlanComplexity::Simple,
           2 => PlanComplexity::Standard,
           _ => PlanComplexity::Complex,
       }
   }
   ```
3. At the beginning of `run_gates()`, after `ordered_gate_names()`, filter the gate list when complexity is provided:
   ```rust
   let gate_names = Self::ordered_gate_names(&config);
   let gate_names = if let Some(complexity_u8) = config.complexity {
       let complexity = complexity_from_u8(complexity_u8);
       let prior_failures = config.prior_failures.unwrap_or(0);
       let caps = RungCaps::all(); // TODO: detect from environment
       let selected_rungs = select_rungs(complexity, &caps, prior_failures);
       let selected_indices: HashSet<u8> = selected_rungs.iter().map(|r| r.index()).collect();
       gate_names.into_iter().filter(|name| {
           Self::rung_for_name(name).map_or(true, |rung| selected_indices.contains(&rung))
       }).collect()
   } else {
       gate_names
   };
   ```
4. Add tests: verify that `complexity: Some(0)` (Trivial) only runs compile, `complexity: Some(3)` (Complex) runs all rungs, and `prior_failures: Some(2)` escalates Trivial -> Standard.

## Design Guidance

Rung selection is pure computation with no I/O. `select_rungs()` returns a `Vec<Rung>` in ~0.01ms. The escalation ladder is: each prior failure promotes complexity one tier, saturating at Complex. This means a Trivial task that fails twice gets Standard-level gates automatically.

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

- [ ] `complexity: Some(0)` with `enabled_gates: ["compile", "clippy", "test"]` runs only compile
- [ ] `complexity: Some(3)` runs all enabled gates
- [ ] `prior_failures: Some(2)` escalates Trivial to Standard

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: GATE_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `complexity: Some(0)` with `enabled_gates: ["compile", "clippy", "test"]` runs only compile
- `complexity: Some(3)` runs all enabled gates
- `prior_failures: Some(2)` escalates Trivial to Standard
- No files outside the Write Scope are modified.
- Commit message contains `tracker: GATE_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
