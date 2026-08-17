# ORCH_19: Priority-Based Task Ordering in TaskScheduler

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-19`](../ISSUE-TRACKER.md#orch-19)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.19
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`TaskScheduler::next_batch()` (at `task_scheduler.rs:81-135`) iterates `self.status` in HashMap order (non-deterministic) and picks ready tasks based on file exclusion and parallelism constraints. It does not consider:
- Downstream dependency count (tasks that unblock more work should run first)
- Estimated cost (cheaper tasks first to secure early wins)
- Critical path membership (zero-slack tasks are time-critical)

## Exact Changes

1. Add a `priority_score()` method to `TaskScheduler`:
   ```rust
   fn priority_score(&self, task_id: &str) -> f64 {
       let dependents = self.count_downstream_dependents(task_id);
       let dependents_score = dependents as f64 * 0.5;
       // Critical path bonus would require DAG integration -- defer to later
       dependents_score
   }
   ```
2. Add `count_downstream_dependents()`: count all tasks that transitively depend on this task.
3. In `next_batch()`, collect all Ready tasks, sort by `priority_score()` descending, then apply file exclusion and parallelism constraints in sorted order.
4. Add a `tier` field to `SchedulableTask` for cost estimation:
   ```rust
   pub tier: Option<String>,  // "mechanical", "focused", "integrative", "architectural"
   ```
5. Factor tier into priority score: mechanical tasks get a +0.3 bonus (cheap, quick wins).

## Design Guidance

The priority scoring should be deterministic given the same inputs. Use `BTreeMap` or sort by (score DESC, task_id ASC) for deterministic tie-breaking. The score function can be extended later with critical path integration from `UnifiedTaskDag::slack()`.

## Write Scope

- `crates/roko-runtime/src/task_scheduler.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Tasks with more downstream dependents are dispatched first
- [ ] Mechanical-tier tasks are prioritized over architectural-tier tasks (when dependency count is equal)
- [ ] File exclusion still prevents conflicting tasks from running concurrently
- [ ] Deterministic ordering: same inputs produce same batch order
- [ ] Existing tests pass with new ordering (may need updates for deterministic expectations)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Tasks with more downstream dependents are dispatched first
- Mechanical-tier tasks are prioritized over architectural-tier tasks (when dependency count is equal)
- File exclusion still prevents conflicting tasks from running concurrently
- Deterministic ordering: same inputs produce same batch order
- Existing tests pass with new ordering (may need updates for deterministic expectations)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
