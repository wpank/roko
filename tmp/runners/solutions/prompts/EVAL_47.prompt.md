# EVAL_47: Built-in profiles (TOML)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-47`](../ISSUE-TRACKER.md#eval-47)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.47
- Priority: **P1**
- Effort: 4 hours
- Depends on: `EVAL_04` (source 5.4)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_47 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. `rust-strict`: compile -> lint -> test -> format -> diff. Sequential. All deterministic.
2. `code-review`: compile -> lint -> test -> format -> diff -> substance -> judge_panel. Sequential.
3. Profile loader: read `.roko/eval/profiles/*.toml`, merge with built-in profiles. Built-in profiles can be overridden by same-named TOML files.

## Write Scope

- `crates/roko-eval/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Test profile loading and criterion resolution
- [ ] Test built-in override by TOML file

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_47 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test profile loading and criterion resolution
- Test built-in override by TOML file
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_47 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
