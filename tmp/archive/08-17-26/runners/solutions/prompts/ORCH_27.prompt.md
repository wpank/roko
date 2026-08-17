# ORCH_27: Delete orchestrate.rs and Remove Dead Imports

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-27`](../ISSUE-TRACKER.md#orch-27)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.27
- Priority: **P2**
- Effort: 3 hours
- Depends on: `ORCH_26` (source 2.26)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

orchestrate.rs is 22,522 lines. Its import block (lines 1-210) pulls from every crate in the workspace. After feature extraction and parity audit, the file can be deleted.

The legacy `roko plan run` entry point that calls orchestrate.rs (if any remains) must be redirected to Runner v2 or WorkflowEngine.

## Exact Changes

1. Remove `mod orchestrate;` (or equivalent) from the CLI crate's module tree.
2. Delete `crates/roko-cli/src/orchestrate.rs`.
3. Run `cargo check --workspace` to find orphaned imports and fix them.
4. Remove any `use` statements in other files that reference `orchestrate::`.
5. Check `crates/roko-cli/Cargo.toml` for dependencies that were only used by orchestrate.rs. Remove orphaned dependencies.
6. Run `cargo test --workspace` to verify nothing breaks.
7. Update `CLAUDE.md` to remove references to orchestrate.rs file paths and mark it as retired.

## Design Guidance

Do this in a single commit for clean git history. The commit message should reference the feature parity audit (Task 2.26) that verified all valuable features are ported or tracked.

## Write Scope

- `crates/roko-cli/src/orchestrate.rs`
- `crates/roko-cli/src/lib.rs`
- `crates/roko-cli/Cargo.toml`

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

- [ ] `crates/roko-cli/src/orchestrate.rs` no longer exists
- [ ] No orphaned imports referencing orchestrate
- [ ] CLAUDE.md updated

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `crates/roko-cli/src/orchestrate.rs` no longer exists
- No orphaned imports referencing orchestrate
- CLAUDE.md updated
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
