# INNO_35: Add `roko knowledge export/import` commands

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-35`](../ISSUE-TRACKER.md#inno-35)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.35
- Priority: **P3**
- Effort: 8 hours
- Depends on: `INNO_33` (source 11.33), `INNO_34` (source 11.34)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_35 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. `roko knowledge export [--domain <tag>] [--tier <n>] -o <file>`:
   - Export entries matching filters with full scrubbing.
   - Output as JSON.
2. `roko knowledge import <file> [--tier <n>]`:
   - Validate entries. Import at specified tier (default: Tier 1).
   - Merge with existing (boost confidence if duplicate).
3. `roko knowledge domains`: list all domain stores with entry counts.

## Write Scope

- `crates/roko-cli/src/commands/knowledge.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko knowledge export` produces a JSON file with no absolute paths
- [ ] `roko knowledge import` adds entries to the appropriate store
- [ ] `roko knowledge domains` lists domains with counts

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_35 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko knowledge export` produces a JSON file with no absolute paths
- `roko knowledge import` adds entries to the appropriate store
- `roko knowledge domains` lists domains with counts
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_35 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
