# ORCH_06: Cumulative Context Buffer

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-06`](../ISSUE-TRACKER.md#orch-06)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.6
- Priority: **P0**
- Effort: 6 hours
- Depends on: `ORCH_03` (source 2.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

EffectDriver passes a flat `context` string to agents via `spawn_agent()` (line 143-146):
```rust
let user_content = context.map_or_else(
    || user_prompt.to_string(),
    |ctx| format!("{user_prompt}\n\n## Additional Context\n\n{ctx}"),
);
```

There is no mechanism to build cumulative context showing what other agents in the same plan have changed. The mega-parity runner identified this as the single most impactful context improvement (merge conflicts reduced from ~50% to ~30%).

orchestrate.rs has `load_prior_task_outputs()` and `with_task_failure_context()` (not ported) that provide similar functionality. These should be the reference for the port.

## Exact Changes

1. Create a `CumulativeContext` struct:
   ```rust
   pub struct CumulativeContext {
       changes: Vec<TaskChangeSummary>,
       max_tokens: usize,  // default 4000
   }
   pub struct TaskChangeSummary {
       pub task_id: String,
       pub files_changed: Vec<String>,
       pub diff_stat: String,           // "+45 -12"
       pub functions_added: Vec<String>,
       pub functions_modified: Vec<String>,
   }
   ```
2. After each task completes, compute a git diff summary in the task's worktree via `git diff --stat HEAD~1` and `git diff --name-only HEAD~1`.
3. Add a `render()` method that produces a markdown section:
   ```markdown
   ## What Changed Before You
   Tasks completed in this plan before your task:
   ### T1: Wire compile gate
   - `src/gate/compile.rs` (+45 -12)
   ```
4. Implement token budget management: truncate oldest task summaries when exceeding `max_tokens`.
5. Pass the rendered context into `spawn_agent_in_worktree()` as the `context` parameter.

## Design Guidance

The context buffer should be per-plan, not global. Each plan execution holds its own `CumulativeContext` that grows as tasks complete. For large plans (20+ tasks), the token truncation is critical -- signature-only views (function name + parameter types, no body) keep overhead manageable. Consider using `roko-index` for function signature extraction when available, with a fallback to `git diff --stat`.

## Write Scope

- `crates/roko-runtime/src/workflow_engine.rs`
- `crates/roko-runtime/src/effect_driver.rs`

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

- [ ] `CumulativeContext::render()` produces valid markdown with file changes
- [ ] Token budget truncation removes oldest summaries first
- [ ] After 3 task completions, the 4th task receives context about all 3 prior tasks
- [ ] Unit test: context with 20 tasks truncates to stay within 4000 tokens

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `CumulativeContext::render()` produces valid markdown with file changes
- Token budget truncation removes oldest summaries first
- After 3 task completions, the 4th task receives context about all 3 prior tasks
- Unit test: context with 20 tasks truncates to stay within 4000 tokens
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
