# INNO_29: Implement speculative prefetch for DAG tasks

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-29`](../ISSUE-TRACKER.md#inno-29)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.29
- Priority: **P2**
- Effort: 12 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

DAG executor at `crates/roko-orchestrator/src/dag.rs` (2,557 LOC) manages task
ordering. While task N executes, task N+1's context can be pre-built.

## Exact Changes

1. In the DAG executor's main loop, identify the next task(s) after the current batch.
2. For each candidate next task:
   - Resolve dependencies from the DAG.
   - Pre-build system prompt layers 1-3 (stable across tasks).
   - Pre-spawn a warm agent (connect to LLM, don't send prompt yet).
   - Pre-fetch code context for the candidate task.
3. If current task succeeds, hand off to the pre-warmed agent immediately.
4. If current task fails, discard the prefetch (context may have changed).
5. Add `--speculate` flag to `plan run` to enable speculative prefetch.

## Write Scope

- `crates/roko-orchestrator/src/dag.rs`

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

- [ ] DAG with 5 sequential tasks: speculative prefetch prepares task N+1 while N executes
- [ ] Discarded prefetches do not leak resources (agents, worktrees)
- [ ] Prefetch is disabled by default; enabled with `--speculate`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- DAG with 5 sequential tasks: speculative prefetch prepares task N+1 while N executes
- Discarded prefetches do not leak resources (agents, worktrees)
- Prefetch is disabled by default; enabled with `--speculate`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
