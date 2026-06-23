# INNO_34: Implement knowledge tier promotion logic

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-34`](../ISSUE-TRACKER.md#inno-34)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.34
- Priority: **P3**
- Effort: 8 hours
- Depends on: `INNO_33` (source 11.33)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_34 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. On run completion, scan new knowledge entries from Tier 0.
2. Filter: exclude entries containing project-specific paths, variable names, or secrets.
3. Classify remaining by tier:
   - About model/tool behavior -> Tier 2
   - About language/framework patterns -> Tier 1
   - Project-specific -> stay at Tier 0
4. Promote if: confidence > 0.8, pattern matches domain tags, not path-dependent,
   for Tier 2: confirmed across 2+ domains.
5. Implement path/secret scrubbing: remove absolute paths, replace with
   placeholders, strip anything matching known secret patterns.

## Write Scope

- `crates/roko-neuro/src/tiered_store.rs`

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

- [ ] After roko learns "Cerebras fails on async trait impls" in project A, the entry appears in `~/.roko/meta/model-knowledge.jsonl`
- [ ] Promoted entries contain no absolute paths or project-specific identifiers
- [ ] An entry with confidence < 0.8 is not promoted

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_34 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After roko learns "Cerebras fails on async trait impls" in project A, the entry appears in `~/.roko/meta/model-knowledge.jsonl`
- Promoted entries contain no absolute paths or project-specific identifiers
- An entry with confidence < 0.8 is not promoted
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_34 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
