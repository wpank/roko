# ORCH_08: Export Gate Rung Mapping from roko-gate (ORCH-008)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-08`](../ISSUE-TRACKER.md#orch-08)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.8
- Priority: **P1**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_08 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`EffectDriver` at `crates/roko-runtime/src/effect_driver.rs:645-656` has a duplicate `rung_for_gate_name()` function that mirrors the mapping in `crates/roko-gate/src/gate_service.rs`. The code includes a TODO:
```rust
/// TODO: expose this mapping from roko-gate as a public function so this duplicate is not needed.
```

This is a straightforward deduplication. The canonical mapping lives in roko-gate. The EffectDriver should import it.

## Exact Changes

1. Find the `rung_for_name` function (or equivalent) in `crates/roko-gate/src/gate_service.rs` and make it `pub`.
2. Re-export it from `crates/roko-gate/src/lib.rs`.
3. Add `roko-gate` as a dependency in `crates/roko-runtime/Cargo.toml` (it is already a dev-dependency, move to `[dependencies]`).
4. Replace the local `rung_for_gate_name()` in `effect_driver.rs` with an import from `roko_gate`.
5. Remove the TODO comment.

## Design Guidance

If the roko-gate function has a different signature, create a thin wrapper. The important thing is one source of truth for rung assignments.

## Write Scope

- `crates/roko-gate/src/gate_service.rs`
- `crates/roko-gate/src/lib.rs`
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

- [ ] `rung_for_gate_name` in `effect_driver.rs` is replaced with import from `roko-gate`
- [ ] The TODO comment is removed

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_08 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `rung_for_gate_name` in `effect_driver.rs` is replaced with import from `roko-gate`
- The TODO comment is removed
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_08 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
