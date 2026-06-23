# INNO_43: Add knowledge provenance tags

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-43`](../ISSUE-TRACKER.md#inno-43)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.43
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_43 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: Karpathy's LLM Wiki pattern (April 2026) -- provenance tags
(extracted, inferred, ambiguous) and a lint pass that flags drift to speculation.

## Exact Changes

1. Add `provenance: Provenance` field to `KnowledgeEntry`.
2. Define `Provenance` enum: `Extracted`, `Inferred`, `Ambiguous`.
3. Default all new entries to `Extracted`.
4. When dream consolidation synthesizes knowledge, tag as `Inferred`.
5. When two sources disagree (HDC similarity > 0.9 but contradictory), tag
   as `Ambiguous`.
6. Implement `lint_provenance() -> Vec<ProvenanceWarning>` flagging entries
   drifting from Extracted to Inferred without acknowledgment.

## Write Scope

- `crates/roko-neuro/src/knowledge_store.rs`

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

- [ ] New knowledge entries from direct observation are tagged `Extracted`
- [ ] Synthesized entries from dream cycle are tagged `Inferred`
- [ ] `lint_provenance()` returns warnings for unacknowledged Inferred entries
- [ ] Provenance is visible in `roko knowledge query` output

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_43 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- New knowledge entries from direct observation are tagged `Extracted`
- Synthesized entries from dream cycle are tagged `Inferred`
- `lint_provenance()` returns warnings for unacknowledged Inferred entries
- Provenance is visible in `roko knowledge query` output
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_43 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
