# DISP_35: Unify dispatch_v2.rs Config Construction

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-35`](../ISSUE-TRACKER.md#disp-35)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.35
- Priority: **P1**
- Effort: 3 hours
- Depends on: `DISP_10` (source 3.10)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_35 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`dispatch_via_model_call_service()` at `/Users/will/dev/nunchi/roko/roko/crates/roko-cli/src/dispatch_v2.rs:53-100` manually constructs a `RokoConfig` by copying fields from the layered config one by one (lines 67-80). This is fragile -- new config fields must be added to this copy loop.

The same manual copy pattern exists in `run.rs` (lines 430-449). Both should use a shared config preparation function.

## Exact Changes

1. Extract the config construction logic (lines 62-86 of `dispatch_v2.rs`) into a shared function:
   ```rust
   pub fn prepare_model_config(workdir: &Path) -> anyhow::Result<(Config, RokoConfig)> {
       let config = crate::config::load_layered(workdir)
           .map(|r| r.config)
           .unwrap_or_default();
       let mut model_config = RokoConfig::default();
       model_config.merge_from(&config);  // or field-by-field, but in one place
       Ok((config, model_config))
   }
   ```
2. Use this shared function in both `dispatch_v2.rs` and `run.rs`
3. Verify that all config fields are properly forwarded (compare the field-by-field copy in both files)
4. Add any missing fields from `run.rs` that `dispatch_v2.rs` doesn't copy (or vice versa)

## Design Guidance

Config preparation should happen in exactly one place. If `RokoConfig` gains a new field, only one function needs updating. The shared function should be in a `config_helpers.rs` or `config_prep.rs` module, not duplicated across dispatch files.

## Write Scope

- `crates/roko-cli/src/dispatch_v2.rs`

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

- [ ] `dispatch_v2.rs` and `run.rs` both use the shared config function
- [ ] No field-by-field config copying remains in dispatch files

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_35 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `dispatch_v2.rs` and `run.rs` both use the shared config function
- No field-by-field config copying remains in dispatch files
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_35 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
