# STAB_52: Replace ACP inline review prompts with templates

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-52`](../ISSUE-TRACKER.md#stab-52)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.52
- Priority: **P2**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_52 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`run_multi_role_review()` hardcodes full role descriptions in `format!()` strings that
partially duplicate `ReviewerTemplate`.

## Exact Changes

1. Replace inline prompts with calls to `ReviewerTemplate::architect()` and `::security()`.
2. Add template methods if they don't exist.
3. Remove inline role description strings.

## Write Scope

- `crates/roko-acp/src/runner.rs`
- `crates/roko-compose/src/templates/reviewer.rs`

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

- [ ] `grep -rn 'Architect Reviewer' crates/roko-acp/` returns zero matches in non-template code

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_52 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `grep -rn 'Architect Reviewer' crates/roko-acp/` returns zero matches in non-template code
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_52 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
