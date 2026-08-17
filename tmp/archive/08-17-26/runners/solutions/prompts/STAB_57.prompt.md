# STAB_57: Wire knowledge candidate ingestion post-run

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-57`](../ISSUE-TRACKER.md#stab-57)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.57
- Priority: **P2**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_57 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Knowledge candidates written to `.roko/learn/knowledge_candidates.jsonl` are never ingested
into `KnowledgeStore`.

## Exact Changes

1. After each plan run, read new candidates.
2. Validate and deduplicate against existing entries.
3. Ingest validated candidates.
4. Mark ingested candidates (or truncate file).

## Write Scope

- `crates/roko-learn/src/knowledge_ingestion.rs`
- `crates/roko-cli/src/runner/event_loop.rs`

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

- [ ] After run producing candidates, `roko knowledge stats` shows new entries

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_57 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After run producing candidates, `roko knowledge stats` shows new entries
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_57 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
