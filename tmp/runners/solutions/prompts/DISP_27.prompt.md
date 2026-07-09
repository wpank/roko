# DISP_27: Delete Dead PlanRunner from orchestrate.rs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-27`](../ISSUE-TRACKER.md#disp-27)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.27
- Priority: **P2**
- Effort: 8 hours
- Depends on: `DISP_24` (source 3.24), `DISP_25` (source 3.25), `DISP_26` (source 3.26)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

After extracting valuable patterns (Tasks 3.25 and 3.26), the dead `PlanRunner` struct and its associated methods can be deleted. The goal is to reduce orchestrate.rs from 22,522 lines to under 5,000 (or less, ideally under 2,000).

The analysis from Task 3.24 identifies which exports are live. Everything else can be deleted.

## Exact Changes

1. Using the categorized export list from Task 3.24, identify all dead code
2. Delete the `PlanRunner` struct and all its methods
3. Delete `dispatch_agent_with()` and all private helpers that only serve PlanRunner
4. Delete `run_task_plans()` and callers
5. Keep all live exports (identified in Task 3.24)
6. For test-only exports, move them to a `#[cfg(test)]` block or a test helper module
7. Run `cargo check --workspace` after each major deletion to catch breakage early
8. Run `cargo test --workspace` after all deletions

## Design Guidance

Delete in stages: start with the largest clearly-dead functions, verify compilation after each stage. The `#[cfg(feature = "legacy-orchestrate")]` feature gate may already protect some sections. Check if the feature is enabled in any CI configuration before removing gated code.

Do NOT delete anything that is imported by production code. The analysis task (3.24) must be completed first.

## Write Scope

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `wc -l crates/roko-cli/src/orchestrate.rs` shows < 5000 lines
- [ ] No live production imports are broken
- [ ] All extracted patterns (replan, context bidding) are accessible from their new locations

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `wc -l crates/roko-cli/src/orchestrate.rs` shows < 5000 lines
- No live production imports are broken
- All extracted patterns (replan, context bidding) are accessible from their new locations
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
