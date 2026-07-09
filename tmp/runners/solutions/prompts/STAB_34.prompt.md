# STAB_34: Fix `signals.jsonl` dead path (writes to `engrams.jsonl`)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-34`](../ISSUE-TRACKER.md#stab-34)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.34
- Priority: **P1**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_34 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`file_substrate.rs` writes to `engrams.jsonl` (line 48: `root.join("engrams.jsonl")`).
`layout.rs` defines both paths:
- Line 168: `engrams_log() -> root.join("engrams.jsonl")` -- the one actually used
- Line 177: `signals_log() -> root.join("signals.jsonl")` -- defined but never populated

The `roko status` command reads signals -- if it reads from `signals.jsonl`, it finds nothing.

## Exact Changes

1. Determine the canonical name. `engrams.jsonl` is the historical name from mori; `signals.jsonl`
   is the roko convention.
2. Option A: Change `file_substrate.rs` to write to `signals.jsonl`. Update `layout.rs` to
   remove `engrams_log()` or make it a deprecated alias.
3. Option B: Update all readers to use `engrams_log()` consistently. Deprecate `signals_log()`.
4. Add a migration helper that renames `engrams.jsonl` to `signals.jsonl` on first run.
5. Update `roko status` to read from the correct file.

## Design Guidance

Prefer option A (use `signals.jsonl`). The roko naming convention should be consistent.
Add a fallback that checks both paths during migration.

## Write Scope

- `crates/roko-fs/src/file_substrate.rs`
- `crates/roko-fs/src/layout.rs`

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

- [ ] `roko run "hello"` writes to the canonical signal log path
- [ ] `roko status` reads from the same path and shows signal count

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_34 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run "hello"` writes to the canonical signal log path
- `roko status` reads from the same path and shows signal count
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_34 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
