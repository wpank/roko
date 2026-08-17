# LERN_10: Wire Section Effectiveness to Prompt Assembly

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#lern-10`](../ISSUE-TRACKER.md#lern-10)
- Source: `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` — Task 7.10
- Priority: **P1**
- Effort: 3 hours
- Depends on: `LERN_05` (source 7.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: LERN_10 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`SectionEffectivenessRegistry` is already wired into `SystemPromptBuilder` and `RoleSystemPromptSpec`:
- `SystemPromptBuilder::with_section_effectiveness()` (at `system_prompt_builder.rs:335`)
- `RoleSystemPromptSpec::build_with_section_effectiveness()` (at `role_prompts.rs:445`)
- `RoleSystemPromptSpec::compose_build_with_budget_and_section_effectiveness()` (at `role_prompts.rs:518`)
- `PromptAssemblyService::with_section_effectiveness()` (at `prompt_assembly_service.rs:180`)

`FeedbackService::section_effectiveness()` (at `feedback_service.rs:277`) returns `HashMap<String, f64>` with lift-based weights per section.

`SectionEffectivenessRegistry::load_or_new(path)` (at `context_provider.rs:463`) loads from `.roko/learn/section-effects.json`.

But `run.rs` never loads or passes section effectiveness data to prompt assembly.

## Exact Changes

1. In the prompt composition path in `run.rs` (around line 1110-1174 where the prompt is built), load `SectionEffectivenessRegistry::load_or_new(&roko_dir.join("learn/section-effects.json"))`.
2. Pass the registry to the prompt builder via `with_section_effectiveness()`.
3. If using `PromptAssemblyService`, call `.with_section_effectiveness(feedback.section_effectiveness())`.
4. Log sections with weight < 0.7 (deprioritized) and > 1.3 (boosted) at DEBUG level.

## Write Scope

- `crates/roko-cli/src/run.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/07-LEARNING-FEEDBACK.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] After 50+ tasks with varying gate results, `section-effects.json` has entries
- [ ] Prompt assembly uses non-default section weights
- [ ] Low-effectiveness sections get reduced token allocation

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: LERN_10 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 50+ tasks with varying gate results, `section-effects.json` has entries
- Prompt assembly uses non-default section weights
- Low-effectiveness sections get reduced token allocation
- No files outside the Write Scope are modified.
- Commit message contains `tracker: LERN_10 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
