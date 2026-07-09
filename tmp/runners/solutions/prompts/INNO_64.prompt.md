# INNO_64: Add C2PA-aligned metadata to agent outputs

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-64`](../ISSUE-TRACKER.md#inno-64)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.64
- Priority: **P3**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_64 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

_(no context section in source)_

## Exact Changes

1. On each agent output, attach metadata fields: `ai.generated: true`,
   `ai.model`, `ai.timestamp`, `ai.agent_id`, `ai.confidence`,
   `ai.provenance_version: "c2pa-draft-2026"`.
2. Include metadata in JSONL events.
3. Include metadata in A2A task artifacts (if A2A wired).

## Write Scope

- `crates/roko-runtime/src/jsonl_logger.rs`

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

- [ ] Every agent output event in JSONL includes provenance metadata
- [ ] Metadata fields match C2PA-aligned naming conventions

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_64 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every agent output event in JSONL includes provenance metadata
- Metadata fields match C2PA-aligned naming conventions
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_64 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
