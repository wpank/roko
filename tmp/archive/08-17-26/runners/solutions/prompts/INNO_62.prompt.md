# INNO_62: Implement distributed causal discovery over episodes

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-62`](../ISSUE-TRACKER.md#inno-62)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.62
- Priority: **P3**
- Effort: 12 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_62 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: DCILP (AAAI 2025) -- ~270x speedup over DAGMA. Each Block estimates
its local Markov blanket; a merge produces the global structural causal model.

`episode_completion.rs` exists at `crates/roko-neuro/src/episode_completion.rs`.

## Exact Changes

1. For each task, estimate its local Markov blanket from episode outcomes:
   which other tasks' outcomes statistically predict this task's success.
2. Merge local blankets into a global structural causal model (DAG).
3. Compare causal DAG with the declared dependency DAG in plans.
4. Flag spurious dependencies: tasks declared as dependent but with no causal
   relationship in the data.
5. Flag missing dependencies: tasks with causal relationships not declared.
6. Output recommendations: "task-07 does not actually depend on task-05
   (p = 0.02); consider parallelizing."
7. Persist to `.roko/learn/causal-model.json`.

## Write Scope

- `crates/roko-neuro/src/episode_completion.rs`

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

- [ ] After 20+ plan runs, the causal model identifies at least one spurious dependency
- [ ] Recommendations are actionable: include task IDs and p-values

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_62 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 20+ plan runs, the causal model identifies at least one spurious dependency
- Recommendations are actionable: include task IDs and p-values
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_62 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
