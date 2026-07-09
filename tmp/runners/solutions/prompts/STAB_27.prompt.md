# STAB_27: Wire section effectiveness into PromptAssemblyService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-27`](../ISSUE-TRACKER.md#stab-27)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.27
- Priority: **P1**
- Effort: 2 hours
- Depends on: `STAB_25` (source 1.25)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`PromptAssemblyService` already has a `section_weights` concept. Section effectiveness data
is collected (after Task 1.25) but not read back during assembly.

## Exact Changes

1. On construction, load section effectiveness from `.roko/learn/section-effects.json`.
2. Apply weights during section budget allocation:
   - Sections with score < 0.1: exclude entirely
   - Sections with score 0.1-0.5: reduce budget proportionally
   - Sections with score > 0.5: full budget
3. Log when a section is deprioritized due to negative effectiveness.

## Design Guidance

Use a minimum observation threshold (e.g., 10) before applying effectiveness scores. Below
that threshold, use equal weights (all sections get full budget). This prevents premature
optimization from small sample sizes.

## Write Scope

- `crates/roko-compose/src/prompt_assembly_service.rs`
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

- [ ] After 10+ runs with section effectiveness data, low-lift sections get less budget
- [ ] High-lift sections get full budget
- [ ] Sections with < 10 observations use default weights

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 10+ runs with section effectiveness data, low-lift sections get less budget
- High-lift sections get full budget
- Sections with < 10 observations use default weights
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
