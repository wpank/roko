# RNNR_19: Add custom anti-pattern rules via roko.toml

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-19`](../ISSUE-TRACKER.md#rnnr-19)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.19
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_16` (source 14.16)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Allow users to define custom anti-pattern rules in `roko.toml` in
addition to the built-in 10 rules.

## Exact Changes

1. Add `[[anti_pattern]]` section to `roko.toml` schema:
   ```toml
   [[anti_pattern]]
   id = "AP-CUSTOM-1"
   name = "hardcoded_api_key"
   pattern = 'sk-[a-zA-Z0-9]{32,}'
   severity = "error"
   file_glob = "*.rs"
   ```
2. Parse custom rules alongside built-in rules in `AntiPatternChecker::new()`
3. Custom rules use same `AntiPatternRule` struct and exemption system
4. Built-in rules can be disabled via `[anti_pattern_defaults] disable = ["AP-10"]`
5. Invalid regex in custom rules produces clear error at config load time

## Write Scope

- `crates/roko-gate/src/anti_pattern.rs`
- `crates/roko-core/src/config/mod.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Custom rules defined in `roko.toml` loaded and applied
- [ ] Built-in rules can be disabled per-project
- [ ] Custom rules participate in false-positive tracking
- [ ] Invalid regex produces clear error at config load time

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Custom rules defined in `roko.toml` loaded and applied
- Built-in rules can be disabled per-project
- Custom rules participate in false-positive tracking
- Invalid regex produces clear error at config load time
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
