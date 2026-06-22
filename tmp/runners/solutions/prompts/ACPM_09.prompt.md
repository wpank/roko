# ACPM_09: Add VerdictMerge Phase to Pipeline

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#acpm-09`](../ISSUE-TRACKER.md#acpm-09)
- Source: `tmp/solutions/roko/tasks/09-ACP-MCP.md` — Task 9.9
- Priority: **P1**
- Effort: 3 hours
- Depends on: `ACPM_08` (source 9.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ACPM_09 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

After parallel review agents complete, their outputs need to be merged into a single approve/revise verdict. This is a new pipeline phase that the runner implements by parsing review outputs.

## Exact Changes

1. Add `VerdictMerge` variant to `PipelinePhase`:
   ```rust
   VerdictMerge { outputs: Vec<(String, String)> }  // (role, output)
   ```
2. Add `MergeComplete { merged_verdict: String }` to `PipelineEvent`.
3. Add `MergeVerdicts { outputs: Vec<(String, String)> }` to `PipelineAction`.
4. Transition from `ParallelExecution` when barrier met:
   - Collect all `(agent_id, output)` pairs from the completed list
   - Emit `MergeVerdicts` action, transition to `VerdictMerge` phase
5. Transition from `VerdictMerge` on `MergeComplete`:
   - If verdict contains "approve" -> `Committing` + `Commit`
   - If verdict contains "revise" -> `Implementing` + `SpawnImplementer` (if iterations remain) or `Committing` + `Commit` (accept with caveats)

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

- [ ] Unit test: Full pipeline with parallel review completes: `Strategizing -> Implementing -> Gating -> ParallelExecution -> VerdictMerge -> Committing`
- [ ] Unit test: merged revise verdict sends back to Implementing with accumulated findings
- [ ] All existing pipeline tests pass

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ACPM_09 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test: Full pipeline with parallel review completes: `Strategizing -> Implementing -> Gating -> ParallelExecution -> VerdictMerge -> Committing`
- Unit test: merged revise verdict sends back to Implementing with accumulated findings
- All existing pipeline tests pass
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ACPM_09 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
