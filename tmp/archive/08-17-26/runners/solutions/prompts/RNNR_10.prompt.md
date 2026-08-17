# RNNR_10: Implement wave gate failure bisection

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-10`](../ISSUE-TRACKER.md#rnnr-10)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.10
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_08` (source 14.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When a wave gate fails, determine which task(s) in the wave caused
the regression. The mega-parity runner used `git log` + bisect across merge
commits.

## Exact Changes

1. Add `async fn bisect_wave_failure(integration_dir: &Path, wave_task_ids: &[String], gate_fn: F) -> Vec<String>`
2. Retrieve list of merge commits in the wave from `git log --merges`
3. For each merge commit, check if reverting it fixes the gate failure:
   `git revert --no-commit <merge_sha>` -> run gates -> `git reset --hard`
4. Return the task IDs whose merge commits, when reverted, fix the failure
5. Mark offending tasks for retry with gate failure output as context
6. Log the bisection process for debugging

## Write Scope

- `crates/roko-cli/src/runner/gate_dispatch.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Bisection correctly identifies the task that introduced a compile error
- [ ] Offending task is retried with failure context from gate output
- [ ] Non-offending tasks in the wave are not retried
- [ ] Bisection works for multiple simultaneous offenders

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Bisection correctly identifies the task that introduced a compile error
- Offending task is retried with failure context from gate output
- Non-offending tasks in the wave are not retried
- Bisection works for multiple simultaneous offenders
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
