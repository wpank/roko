# EVAL_20: Judge panel construction with family exclusion

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-20`](../ISSUE-TRACKER.md#eval-20)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.20
- Priority: **P1**
- Effort: 5 hours
- Depends on: `EVAL_18` (source 5.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_20 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The critical composition rule: never use the same model family as both generator and judge. Panel construction integrates with the existing cascade router at `crates/roko-learn/src/cascade_router.rs` to determine the generator family from `AgentEfficiencyEvent.model`.

The `slug_family()` function at `crates/roko-learn/src/cascade/helpers.rs` maps model slugs to families (e.g., "claude-opus-4-6" -> "anthropic", "gpt-4o" -> "openai").

## Exact Changes

1. Define `JudgePanelConfig { min_panel_size: usize, preferred_panel_size: usize, exclude_generator_family: bool }` with defaults (3, 3, true).
2. Define `JudgeSpec { model_id: String, family: String, endpoint: Option<String>, max_tokens: u32, temperature: f64 }`.
3. Implement `construct_panel(available_models: &[JudgeSpec], generator_family: Option<&str>, config: &JudgePanelConfig) -> Result<Vec<JudgeSpec>, EvalError>`:
   - Exclude models from generator family.
   - Select one model per remaining family.
   - Sort by priority (configurable or hardcoded: frontier closed > rubric-conditioned > open).
   - Take top `preferred_panel_size`.
   - Error if fewer than `min_panel_size` available.
4. Implement `generator_family_from_model_slug(slug: &str) -> Option<String>` reusing the `slug_family()` logic from `crates/roko-learn/src/cascade/helpers.rs`.

## Write Scope

- `crates/roko-eval-judge/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Test with 4 model families, exclude one, assert panel = 3
- [ ] Test error when fewer than min_panel_size available

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_20 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test with 4 model families, exclude one, assert panel = 3
- Test error when fewer than min_panel_size available
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_20 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
