# PERF_16: Git Diff Cache Per Gate Phase

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#perf-16`](../ISSUE-TRACKER.md#perf-16)
- Source: `tmp/solutions/roko/tasks/10-PERFORMANCE.md` — Task 10.16
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PERF_16 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Compute git diff once per gate phase, store in `GatePhaseContext`,
pass to all gates that need it.

## Exact Changes

1. Define:
   ```rust
   struct GatePhaseContext {
       diff_stat: String,
       diff_full: String,
       modified_files: Vec<String>,
       computed_at: Instant,
   }
   ```
2. Add `async fn compute(workdir: &Path) -> Self` that runs `git diff --stat HEAD`
   and `git diff HEAD` in parallel via `tokio::join!`
3. Parse modified file paths from diff stat output
4. Replace the gate-phase git subprocess spawns (around lines 17484-17537,
   17666, 18700) with reads from `GatePhaseContext`
5. Pass `GatePhaseContext` through the gate dispatch path

## Write Scope

- `crates/roko-cli/src/orchestrate.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/10-PERFORMANCE.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] During gate phase, at most ONE `git diff --stat` and ONE `git diff HEAD`
- [ ] Gate verdicts unchanged from before

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PERF_16 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- During gate phase, at most ONE `git diff --stat` and ONE `git diff HEAD`
- Gate verdicts unchanged from before
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PERF_16 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
