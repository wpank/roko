# STAB_37: Export `rung_for_gate_name` from roko-gate

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-37`](../ISSUE-TRACKER.md#stab-37)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.37
- Priority: **P2**
- Effort: 30 minutes
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_37 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`rung_for_gate_name()` is defined at line 645 of `effect_driver.rs` as a local function.
This duplicates logic from `roko-gate`. The comment at line 309 references it.

## Exact Changes

1. Add or export `pub fn rung_for_gate_name(name: &str) -> u8` from `roko-gate/src/lib.rs`.
2. In `effect_driver.rs`, delete the local `rung_for_gate_name()` function (line 645).
3. Import from `roko_gate::rung_for_gate_name`.

## Write Scope

- `crates/roko-gate/src/lib.rs`
- `crates/roko-runtime/src/effect_driver.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `rung_for_gate_name` defined in one place only (roko-gate)
- [ ] `effect_driver.rs` imports it

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_37 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `rung_for_gate_name` defined in one place only (roko-gate)
- `effect_driver.rs` imports it
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_37 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
