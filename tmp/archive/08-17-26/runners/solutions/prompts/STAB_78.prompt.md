# STAB_78: Consolidate 4 agent dispatch implementations

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-78`](../ISSUE-TRACKER.md#stab-78)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.78
- Priority: **P2**
- Effort: 10 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_78 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Four dispatch implementations with different features, error handling, timeout logic, token
counting, and safety checks. Bug fixes don't propagate across copies.

## Exact Changes

1. Consolidate into EffectDriver's `ModelCaller` + `PromptAssembler` trait pattern.
2. Add service traits for safety, custody, knowledge routing.
3. Compose into `EffectServices`.
4. All paths delegate to EffectDriver.
5. Delete redundant implementations.

## Write Scope

- `crates/roko-acp/src/runner.rs`
- `crates/roko-cli/src/dispatch_v2.rs`
- `crates/roko-cli/src/orchestrate.rs`
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

- [ ] Single dispatch code path handles all cases
- [ ] Changing dispatch behavior affects all surfaces

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_78 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Single dispatch code path handles all cases
- Changing dispatch behavior affects all surfaces
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_78 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
