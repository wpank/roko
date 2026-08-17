# INNO_02: Implement memory retrieval with token budget

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-02`](../ISSUE-TRACKER.md#inno-02)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.2
- Priority: **P0**
- Effort: 8 hours
- Depends on: `INNO_01` (source 11.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_02 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `query_for_task()` stub needs three-tier retrieval logic with a 2K token
budget. EpisodeLogger already has `hdc_fingerprint` per episode
(`crates/roko-learn/src/episode_logger.rs:247`). KnowledgeStore has
`query_similar()` for HDC-based retrieval
(`crates/roko-neuro/src/knowledge_store.rs:651`). PlaybookStore has search by
category.

HdcVector at `crates/roko-primitives/src/hdc.rs` provides `fingerprint()` and
`hamming_similarity()`. Use these for Tier 2 matching.

## Exact Changes

1. In `query_for_task()`, accept a `TaskContext` with `task_id`, `domain_tags`,
   `description`, and optional `hdc_fingerprint`.
2. Tier 1 (exact match): query EpisodeLogger for episodes matching `task_id`.
   Return the most recent attempt's outcome, error patterns, and tool calls.
3. Tier 2 (HDC similarity): if `hdc_fingerprint` is available on the task
   context, scan recent episodes (last 100) for `hamming_similarity > 0.7`.
   Weight by recency using half-life decay (configurable, default 7 days).
4. Tier 3 (semantic): query KnowledgeStore by domain tags using `query_kind()`.
   Include anti-knowledge entries (where `is_anti_knowledge == true`) in the
   `anti_patterns` field.
5. Query PlaybookStore for playbooks matching task category. Include top-3 by
   confidence score.
6. Enforce 2048-token budget: rank all results by relevance score (exact match
   > HDC similarity > semantic > playbook). Estimate token count per item (use
   `roko_compose::token_counter` if available, else heuristic of
   `text.len() / 4`). Truncate to fit budget.
7. Return populated `MemoryInjection` with `total_tokens` field.

## Write Scope

- `crates/roko-learn/src/memory_layer.rs`

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

- [ ] After 10+ recorded episodes, `query_for_task` returns the 3-5 most relevant items, not all items
- [ ] Anti-knowledge entries appear in the `anti_patterns` field
- [ ] Total token count of returned injection is <= 2048 tokens
- [ ] Unit test: insert 20 episodes, query, verify budget constraint
- [ ] Unit test: verify recency weighting -- recent episodes rank higher than old ones

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_02 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 10+ recorded episodes, `query_for_task` returns the 3-5 most relevant items, not all items
- Anti-knowledge entries appear in the `anti_patterns` field
- Total token count of returned injection is <= 2048 tokens
- Unit test: insert 20 episodes, query, verify budget constraint
- Unit test: verify recency weighting -- recent episodes rank higher than old ones
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_02 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
