# INNO_53: Wire knowledge store consultation into CascadeRouter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-53`](../ISSUE-TRACKER.md#inno-53)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.53
- Priority: **P1**
- Effort: 4 hours
- Depends on: `INNO_01` (source 11.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_53 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

CascadeRouter already has `route_with_knowledge()` at line 856 and
`KnowledgeRoutingAdvice` at `crates/roko-learn/src/cascade/types.rs`. But the
live dispatch path does not construct `KnowledgeRoutingAdvice` from the neuro
store (CLAUDE.md item 13: "Knowledge-informed agent routing -- neuro store not
yet consulted for model selection").

## Exact Changes

1. Before routing, query `KnowledgeStore` for entries about model performance
   on the current task's domain/type using `query_kind()`.
2. Construct `KnowledgeRoutingAdvice` from matching entries.
3. Pass to `route_with_knowledge()` (already implemented).
4. If knowledge contradicts bandit observations, weight bandit higher.

## Write Scope

- `crates/roko-learn/src/cascade_router.rs`

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

- [ ] If knowledge store contains "cerebras fails on async Rust" with high confidence, CascadeRouter avoids cerebras for async Rust tasks
- [ ] Knowledge consultation adds < 5ms to routing latency
- [ ] Bandit observations override stale knowledge

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_53 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- If knowledge store contains "cerebras fails on async Rust" with high confidence, CascadeRouter avoids cerebras for async Rust tasks
- Knowledge consultation adds < 5ms to routing latency
- Bandit observations override stale knowledge
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_53 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
