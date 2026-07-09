# STAB_25: Wire runner v2 section effectiveness updates

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-25`](../ISSUE-TRACKER.md#stab-25)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.25
- Priority: **P1**
- Effort: 1 hour
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`SectionEffectivenessRegistry` tracks lift per prompt section but receives no observations
from runner v2 plan execution.

## Exact Changes

1. On task completion, call:
   ```rust
   section_registry.observe(sections_included, gate_passed);
   ```
2. `sections_included` can be derived from the `PromptAssemblyService` output.
3. Persist registry during periodic flush.

## Design Guidance

Section names should match across assemblies so observations accumulate correctly.
Use the canonical section names from `PromptAssemblyService`.

## Write Scope

- `crates/roko-cli/src/runner/event_loop.rs`
- `crates/roko-learn/src/section_effect.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After plan run, `.roko/learn/section-effects.json` has entries with `observation_count > 0`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After plan run, `.roko/learn/section-effects.json` has entries with `observation_count > 0`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
