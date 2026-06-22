# ACPM_12: Implement VerdictMerge in Runner

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-12`](../ISSUE-TRACKER.md#acpm-12)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.12
- Priority: **P1**
- Effort: 4 hours
- Depends on: `ACPM_11` (source 9.11)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`parse_structured_review_verdict()` from `roko_gate` (`crates/roko-gate/src/review_verdict.rs`) parses agent output into a `ReviewVerdictContext` with verdict (approve/revise/reject), findings with severity, and suggested changes. The runner needs to merge multiple verdicts.

## Exact Changes

1. Add `handle_merge_verdicts()` method to the runner.
2. Parse each output through `parse_structured_review_verdict()`.
3. Merge strategy:
   - If any reviewer rejects -> revise
   - If all approve -> approve
   - Mixed -> take majority; if tied, revise (conservative)
4. Concatenate findings from all reviewers, deduplicate by description similarity.
5. Feed `MergeComplete` with the merged verdict back to the pipeline.

## Write Scope

- `crates/roko-acp/src/runner.rs`

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

- [ ] Unit test: 3 approve -> merged approve
- [ ] Unit test: 1 reject + 2 approve -> merged revise with reject findings
- [ ] Unit test: 2 revise + 1 approve -> merged revise with combined findings

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: 3 approve -> merged approve
- Unit test: 1 reject + 2 approve -> merged revise with reject findings
- Unit test: 2 revise + 1 approve -> merged revise with combined findings
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
