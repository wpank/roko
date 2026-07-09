# INNO_04: Wire memory update on task completion

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-04`](../ISSUE-TRACKER.md#inno-04)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.4
- Priority: **P0**
- Effort: 4 hours
- Depends on: `INNO_01` (source 11.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`RuntimeFeedback` at `crates/roko-learn/src/runtime_feedback.rs` handles
post-task feedback. This is the natural place to update the memory layer after
each task attempt. KnowledgeStore has `ingest()` and anti-knowledge support.
PlaybookStore has `Playbook` and `PlaybookStep` types.

Research: Dohmatob 2025 -- accumulate (synthetic added to real) gives bounded
error vs replace scenarios. All new knowledge must be additive.

## Exact Changes

1. In the post-task feedback handler, check task outcome (success/failure).
2. On success:
   - Call `PlaybookStore::upsert()` (or create equivalent) with the successful
     approach as a new playbook.
   - Call `KnowledgeStore::ingest()` with extracted facts at Transient tier.
3. On failure:
   - Call `KnowledgeStore::ingest()` with error pattern as anti-knowledge
     (set `is_anti_knowledge: true`).
4. On either outcome:
   - Compute HDC fingerprint from task context + outcome using
     `roko_primitives::hdc::fingerprint()`.
   - Store fingerprint on the episode via `EpisodeLogger`.
5. If the ingested knowledge matches an existing entry (HDC similarity > 0.9),
   boost the existing entry's confidence instead of creating a duplicate.

## Write Scope

- `crates/roko-learn/src/runtime_feedback.rs`

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

- [ ] Run a plan with 5 tasks. 3 succeed, 2 fail
- [ ] After run: KnowledgeStore has 3 new entries (successes) and 2 anti-knowledge entries (failures)
- [ ] PlaybookStore has at least 1 new playbook from a successful task
- [ ] A second run on the same plan shows memory injection from first run's data
- [ ] Duplicate knowledge entries are merged (confidence boosted), not duplicated

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run a plan with 5 tasks. 3 succeed, 2 fail
- After run: KnowledgeStore has 3 new entries (successes) and 2 anti-knowledge entries (failures)
- PlaybookStore has at least 1 new playbook from a successful task
- A second run on the same plan shows memory injection from first run's data
- Duplicate knowledge entries are merged (confidence boosted), not duplicated
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
