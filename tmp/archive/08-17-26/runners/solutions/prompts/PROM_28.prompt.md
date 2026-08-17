# PROM_28: Wire Prompt A/B Testing via ExperimentStore

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-28`](../ISSUE-TRACKER.md#prom-28)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.28
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Support A/B testing prompt variants through the existing
`ExperimentStore`.

## Exact Changes

1. Add `experiment_store: Option<Arc<ExperimentStore>>` to `PromptAssemblyService`
2. Add builder method `with_experiment_store(store: Arc<ExperimentStore>) -> Self`
3. Define prompt experiment types: `reasoning_depth`, `anti_pattern_format`, `section_ordering`
4. In assembly, if an active experiment covers a prompt dimension, use the experiment's selected variant
5. After gate results (in orchestrate.rs), record the outcome against the variant
6. Periodically (every 50 observations), promote the winning variant

## Write Scope

- `crates/roko-compose/src/prompt_assembly_service.rs`

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

- [ ] Can define an experiment: `reasoning_depth` with variants `["suppress", "brief", "deep"]`
- [ ] Assembly uses the experiment's assigned variant for the current task
- [ ] Gate outcomes are recorded per variant
- [ ] After 50 observations, the experiment has a `current_winner`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Can define an experiment: `reasoning_depth` with variants `["suppress", "brief", "deep"]`
- Assembly uses the experiment's assigned variant for the current task
- Gate outcomes are recorded per variant
- After 50 observations, the experiment has a `current_winner`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
