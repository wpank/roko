# DISP_21: Remove Hardcoded Model Strings from run.rs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#disp-21`](../ISSUE-TRACKER.md#disp-21)
- Source: `tmp/solutions/roko/tasks/03-INFERENCE-DISPATCH.md` — Task 3.21
- Priority: **P1**
- Effort: 3 hours
- Depends on: `DISP_03` (source 3.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: DISP_21 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`run.rs` has hardcoded model strings at:
- Line 530: `"claude-sonnet-4-6"` used as fallback
- Line 657: `"llama3.1:8b"` used for local model detection

These bypass the user's `default_model` configuration in `roko.toml`.

## Exact Changes

1. Replace the hardcoded `"claude-sonnet-4-6"` at line 530 with `config.agent.default_model.clone()` or the result from `resolve_effective_model()`
2. Replace `"llama3.1:8b"` at line 657 with a check against configured local models rather than a hardcoded string
3. Add constants for any fallback model strings that must remain (e.g., `const BUILT_IN_DEFAULT_MODEL: &str = "claude-sonnet-4-6";`) and use them from the `BuiltInDefault` precedence tier in `model_selection.rs`
4. Verify that changing `default_model` in `roko.toml` actually affects the `roko run` dispatch path

## Design Guidance

Model strings should flow from config, not from source code. The only acceptable hardcoded model is the `BuiltInDefault` fallback (used when no config is available at all). Every other path should read from `RokoConfig`. This ensures `roko config set agent.default_model=my-model` actually takes effect.

## Write Scope

- `crates/roko-cli/src/run.rs`

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

- [ ] `grep -n '"claude-sonnet-4-6"\|"llama3.1:8b"' crates/roko-cli/src/run.rs` returns zero results (outside comments/constants)
- [ ] Changing `default_model` in `roko.toml` changes which model `roko run` uses

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: DISP_21 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -n '"claude-sonnet-4-6"\|"llama3.1:8b"' crates/roko-cli/src/run.rs` returns zero results (outside comments/constants)
- Changing `default_model` in `roko.toml` changes which model `roko run` uses
- No files outside the Write Scope are modified.
- Commit message contains `tracker: DISP_21 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
