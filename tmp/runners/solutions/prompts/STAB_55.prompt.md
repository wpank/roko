# STAB_55: Unify StateHub types between serve and CLI

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-55`](../ISSUE-TRACKER.md#stab-55)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.55
- Priority: **P2**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_55 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko-serve` at line 68 includes the `state_hub.rs` via `#[path]`:
```rust
#[path = "../../roko-core/src/state_hub.rs"]
pub mod state_hub_compat;
```
This creates two copies of the `StateHub` type (one in serve, one in core), preventing
zero-cost sharing between the two crates.

## Exact Changes

1. Export `StateHub` as a public type from `roko-core`.
2. Remove the `#[path]` include from `roko-serve`.
3. Import `roko_core::StateHub` in both `roko-serve` and `roko-cli`.
4. Share a single instance when running together (`roko run --serve`).

## Write Scope

- `crates/roko-serve/src/lib.rs`
- `crates/roko-core/src/state_hub.rs`
- `crates/roko-cli/src/state_hub.rs`

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

- [ ] `roko run --serve "hello"` SSE endpoint emits real-time events
- [ ] No `#[path]` includes of `state_hub.rs`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_55 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run --serve "hello"` SSE endpoint emits real-time events
- No `#[path]` includes of `state_hub.rs`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_55 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
