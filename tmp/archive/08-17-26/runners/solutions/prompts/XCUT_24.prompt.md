# XCUT_24: Add Schema Version to All Persisted JSON Files

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-24`](../ISSUE-TRACKER.md#xcut-24)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.24
- Priority: **P9**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

34 files reference `schema_version` but coverage is partial. `RuntimeEventEnvelope` has it (line 15 of `runtime_event.rs`). `RuntimeSnapshot` has it (`crates/roko-orchestrator/src/runtime_snapshot.rs`). But persisted learning files (`cascade-router.json`, `gate-thresholds.json`, `experiments.json`) may lack it. The `crates/roko-cli/src/snapshot_migrate.rs` file exists, suggesting migration infrastructure is partially built.

## Exact Changes

1. For each persisted JSON file type, ensure the root object has `"schema_version": N`.
2. On load, check the schema version. If unknown (newer than expected), log a warning and attempt best-effort parsing.
3. If the version is older, apply migration functions: `migrate_v0_to_v1(data: Value) -> Value` per file type.
4. Add `roko config migrate` subcommand that migrates all persisted files to the latest schema.
5. Document schema changes in each file's module docs.

## Write Scope

- `crates/roko-learn/src/runtime_feedback.rs`
- `crates/roko-learn/src/playbook.rs`
- `crates/roko-learn/src/contextual_bandit.rs`
- `crates/roko-learn/src/section_outcome.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] All persisted JSON files have `schema_version` field
- [ ] Older schemas are auto-migrated on load
- [ ] Newer schemas produce a warning, not a crash
- [ ] `roko config migrate` succeeds on a workspace with v0 files

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- All persisted JSON files have `schema_version` field
- Older schemas are auto-migrated on load
- Newer schemas produce a warning, not a crash
- `roko config migrate` succeeds on a workspace with v0 files
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
