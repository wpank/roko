# XCUT_25: Add roko.toml Config Schema Validation

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#xcut-25`](../ISSUE-TRACKER.md#xcut-25)
- Source: `tmp/solutions/roko/tasks/19-CROSS-CUTTING.md` — Task 19.25
- Priority: **P4**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: XCUT_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko.toml` accepts any keys without validation. The `serde_ignored` crate is not in any `Cargo.toml` in the workspace. Typos like `defalt_model` instead of `default_model` are silently ignored. The config schema divergence between `[[gate]]` (written by `roko init`) and `[gates]` (read by `roko plan run`) documented in `06-IMPLEMENTATION-PLANS.md` Plan 5 is one symptom. `crates/roko-core/src/config/compat.rs` exists (references `schema_version`), suggesting some migration infrastructure is present.

## Exact Changes

1. Add `serde_ignored` dependency to `roko-core/Cargo.toml`.
2. After deserializing `RokoConfig` from TOML, collect unknown keys using `serde_ignored::deserialize()`.
3. For each unknown key, compute Levenshtein distance to known keys.
4. If distance <= 2, suggest the correct key: `warning: unknown key 'defalt_model', did you mean 'default_model'?`.
5. If distance > 2, warn: `warning: unknown key 'xyz' in section [agent]`.
6. Add `roko config validate` subcommand that runs validation and reports all issues.
7. On `roko plan run`, validate config and warn (do not fail) for unknown keys.
8. Accept both `[[gate]]` and `[gates]` formats per Plan 5, with deprecation warning for `[[gate]]`.

## Write Scope

- `crates/roko-core/src/config/mod.rs`
- `crates/roko-cli/src/commands/config_cmd.rs`

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

- [ ] `roko config validate` detects typos and suggests corrections
- [ ] `defalt_model` in roko.toml produces: `warning: unknown key 'defalt_model', did you mean 'default_model'?`
- [ ] Both gate config formats are accepted with deprecation warning
- [ ] Validation is non-blocking (warns, does not fail)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: XCUT_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko config validate` detects typos and suggests corrections
- `defalt_model` in roko.toml produces: `warning: unknown key 'defalt_model', did you mean 'default_model'?`
- Both gate config formats are accepted with deprecation warning
- Validation is non-blocking (warns, does not fail)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: XCUT_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
