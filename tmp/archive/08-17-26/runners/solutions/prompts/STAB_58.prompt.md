# STAB_58: Fix `--share` without `--serve` producing dead URL

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-58`](../ISSUE-TRACKER.md#stab-58)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.58
- Priority: **P2**
- Effort: 2 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_58 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`roko run --share` prints `http://localhost:6677/runs/{token}` which is inaccessible
without serve running.

## Exact Changes

1. When `--share` without `--serve`: generate self-contained HTML artifact.
2. Write to `.roko/shared/{token}.html`.
3. Print local file path instead of dead URL.
4. When `--serve` IS active: print serve URL as before.

## Write Scope

- `crates/roko-cli/src/share.rs`
- `crates/roko-cli/src/run.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko run --share "hello"` without serve prints local file path
- [ ] HTML file opens in browser showing transcript

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_58 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run --share "hello"` without serve prints local file path
- HTML file opens in browser showing transcript
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_58 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
