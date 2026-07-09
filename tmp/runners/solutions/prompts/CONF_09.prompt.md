# CONF_09: Remove `unsafe { std::env::set_var() }` for Provider Override

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#conf-09`](../ISSUE-TRACKER.md#conf-09)
- Source: `tmp/solutions/roko/tasks/16-CONFIG-AND-WIRING.md` — Task 16.9
- Priority: **P1**
- Effort: Small
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: CONF_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `--provider` CLI flag uses `unsafe { std::env::set_var("ROKO_PROVIDER", p) }` at
`crates/roko-cli/src/commands/util.rs:236`. Rust 2024 edition marks `set_var` as
`unsafe` because it is unsound in multi-threaded contexts. Two other `set_var` calls
exist in `main.rs:2225` (`ROKO_HIGH_CONTRAST`) and `main.rs:2229` (`ROKO_REDUCED_MOTION`).

## Exact Changes

1. For `--provider`: thread the override through a field on the command context or
   `ServiceConfig` struct. Remove the `set_var` call.
2. For `ROKO_HIGH_CONTRAST` and `ROKO_REDUCED_MOTION`: these are set very early
   (before any threads spawn) so they are safe in practice. Move them to before
   tokio runtime creation, or replace with config struct fields. At minimum, add
   a `// SAFETY:` comment explaining why the call is sound (single-threaded at this
   point).

## Write Scope

- `crates/roko-cli/src/commands/util.rs`
- `crates/roko-cli/src/main.rs`

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

- [ ] `roko run --provider anthropic "hello"` works without calling `set_var`.
- [ ] No `unsafe { std::env::set_var }` remains for `ROKO_PROVIDER`.

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: CONF_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run --provider anthropic "hello"` works without calling `set_var`.
- No `unsafe { std::env::set_var }` remains for `ROKO_PROVIDER`.
- No files outside the Write Scope are modified.
- Commit message contains `tracker: CONF_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
