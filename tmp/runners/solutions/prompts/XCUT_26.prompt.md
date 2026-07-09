# XCUT_26: Add Deprecation Warnings for Config Migrations

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-26`](../ISSUE-TRACKER.md#xcut-26)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.26
- Priority: **P9**
- Effort: 3 hours
- Depends on: `XCUT_25` (source 19.25)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Config format changes need deprecation warnings so users know to update. Currently, old formats either silently work (confusing) or silently break (worse). The `[[gate]]` to `[gates]` migration is the first concrete case.

## Exact Changes

1. Add `DeprecationWarning { field: String, message: String, since_version: String, removal_version: Option<String> }` struct.
2. During config parsing, collect deprecation warnings into `Vec<DeprecationWarning>`.
3. Return warnings alongside the parsed config: `fn load_config() -> Result<(RokoConfig, Vec<DeprecationWarning>)>`.
4. In CLI entry points, display warnings with color: yellow for deprecations, red for upcoming removals.
5. Add `#[deprecated_field(since = "0.5", use_instead = "gates")]` attribute macro for config struct fields (or use a simpler HashMap-based approach).

## Write Scope

- `crates/roko-core/src/config/mod.rs`
- `crates/roko-core/src/config/schema.rs`

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

- [ ] `[[gate]]` format produces a deprecation warning naming `[gates]` as the replacement
- [ ] Warnings include the version where the old format will be removed
- [ ] `roko config show` displays active deprecation warnings
- [ ] Warnings do not prevent execution

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `[[gate]]` format produces a deprecation warning naming `[gates]` as the replacement
- Warnings include the version where the old format will be removed
- `roko config show` displays active deprecation warnings
- Warnings do not prevent execution
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
