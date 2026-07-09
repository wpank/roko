# ACPM_10: Wire Full Template to Parallel Review

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-10`](../ISSUE-TRACKER.md#acpm-10)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.10
- Priority: **P1**
- Effort: 3 hours
- Depends on: `ACPM_09` (source 9.9)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The `Gating + GatesPassed` transition at `pipeline.rs:288-293` currently checks `self.template.has_review()` and always emits `SpawnReviewer` (single agent). The Full template should emit `SpawnParallelAgents` instead.

## Exact Changes

1. Modify the `Gating + GatesPassed` transition for `Full` template:
   - Instead of `SpawnReviewer`, emit `SpawnParallelAgents` with 3 specs:
     - Architect: deep architectural review, read-only
     - Auditor: security and correctness audit, read-only
     - Scribe: documentation coverage check, read-only
   - Set barrier to `AllComplete`
2. Keep Standard template unchanged (single `SpawnReviewer`).
3. Keep Express template unchanged (skips review).
4. Add `has_parallel_review(&self) -> bool` method to `WorkflowTemplate` that returns `true` only for `Full`.

## Write Scope

- `crates/roko-acp/src/pipeline.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/09-ACP-MCP.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Unit test: Full template emits `SpawnParallelAgents` with 3 specs after gates pass
- [ ] Unit test: Standard template still emits `SpawnReviewer` (single agent)
- [ ] Full transition table test covers the parallel path end-to-end

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: Full template emits `SpawnParallelAgents` with 3 specs after gates pass
- Unit test: Standard template still emits `SpawnReviewer` (single agent)
- Full transition table test covers the parallel path end-to-end
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
