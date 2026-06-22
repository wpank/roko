# CONF_26: Add Config Migration for Gate Format

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-26`](../ISSUE-TRACKER.md#conf-26)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.26
- Priority: **P3**
- Effort: Medium
- Depends on: `CONF_02` (source 16.2)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `[[gate]]` to `[gates]` format change (16.2) creates a breaking change for
existing workspaces. Config versioning already exists (`config_version` at
`schema.rs:50`, `schema_version` at `schema.rs:52`) but no migration logic
handles the gate format change.

## Exact Changes

1. In `RokoConfig::from_toml()`, detect `[[gate]]` array syntax in the raw TOML
   before deserialization. If present, normalize to `[gates]` format.
2. Print a one-time migration hint: `hint: run "roko config migrate" to update format`.
3. `roko config migrate` rewrites the file, creates `roko.toml.bak` backup.
4. Bump `config_version` to indicate the new format.

## Write Scope

- `crates/roko-core/src/config/schema.rs`
- `crates/roko-cli/src/config.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko config migrate` on a roko.toml with `[[gate]]` arrays produces valid `[gates]`.
- [ ] A backup exists at `roko.toml.bak`.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko config migrate` on a roko.toml with `[[gate]]` arrays produces valid `[gates]`.
- A backup exists at `roko.toml.bak`.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
