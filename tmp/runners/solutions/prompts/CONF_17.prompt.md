# CONF_17: Add Config Validation on Load

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-17`](../ISSUE-TRACKER.md#conf-17)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.17
- Priority: **P1**
- Effort: Medium
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_17 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`RokoConfig::from_toml()` at `crates/roko-core/src/config/schema.rs:171` accepts any
syntactically valid TOML without semantic validation. `validate_references()` at line
974 exists and checks provider/model references, but it is not called automatically
on load.

Missing checks: `default_model` resolves to a known model, provider `api_key_env`
values have corresponding env vars, budget values are non-negative, `tier_models`
values are valid model keys.

## Exact Changes

1. Extend `validate_references()` to also check:
   - `default_model` resolves to a model in `[models]` table or is a known alias.
   - Provider `api_key_env` values have corresponding env vars set (warn if not).
   - Budget values are non-negative.
   - `tier_models` values are valid model keys.
2. Call `validate_references()` after loading config and print warnings to stderr.
3. `roko config validate` runs the full validation and returns a structured report.

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

- [ ] Setting `default_model = "nonexistent"` produces a startup warning.
- [ ] `roko config validate` reports all detected issues.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_17 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Setting `default_model = "nonexistent"` produces a startup warning.
- `roko config validate` reports all detected issues.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_17 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
