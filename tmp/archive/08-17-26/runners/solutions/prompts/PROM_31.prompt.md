# PROM_31: Wire Dependency Chain Context into Prompt Assembly

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-31`](../ISSUE-TRACKER.md#prom-31)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.31
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_31 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When a task depends on completed prior tasks, inject a structured
summary of what those tasks produced and their gate outcomes.

## Exact Changes

1. Create struct (in context_provider.rs or a new helper):
   ```rust
   pub struct DependencyContext {
       pub task_id: String,
       pub summary: String,
       pub gate_outcome: String,  // "PASSED", "FAILED (clippy)", etc.
       pub files_modified: Vec<String>,
   }
   ```
2. In orchestrate.rs, after a task completes, store its `DependencyContext`
3. Before dispatching a dependent task, collect all predecessor `DependencyContext` entries
4. Format as a "Completed Dependencies" section and inject into Layer 3
5. Only for Focused and Full tiers (Surgical skips this -- check `tier.is_eligible("context")`)

## Write Scope

- `crates/roko-compose/src/context_provider.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] A task with dependencies receives a "Completed Dependencies" section listing predecessors
- [ ] The section includes gate outcomes
- [ ] Surgical tier tasks do not receive dependency context

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_31 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A task with dependencies receives a "Completed Dependencies" section listing predecessors
- The section includes gate outcomes
- Surgical tier tasks do not receive dependency context
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_31 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
