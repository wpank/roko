# STAB_18: Wire ContextTier into dispatch for small models

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#stab-18`](../ISSUE-TRACKER.md#stab-18)
- Source: `tmp/solutions/roko/tasks/01-STABILITY-AND-FIXES.md` — Task 1.18
- Priority: **P1**
- Effort: 3 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: STAB_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`ContextTier` in `context_provider.rs` defines correct budgets (Surgical: 4K, Focused: 12K,
Full: 24K tokens) and `is_local_model()` correctly identifies small models. But the dispatch
path never calls `ContextTier::from_task_and_model()`. Small models (Ollama gemma4, Cerebras
llama 8b) receive 200K-context prompts, causing silent truncation.

## Exact Changes

1. In the dispatch path (wherever prompt is assembled before agent call), resolve context tier:
   ```rust
   let tier = ContextTier::from_model(model_slug);
   let budget = tier.token_budget();
   ```
2. Pass budget to `PromptAssemblyService::with_token_budget(budget)`.
3. In `PromptAssemblyService`, enforce the budget:
   - Surgical (4K): identity + role + task + constraints only
   - Focused (12K): add conventions and limited context
   - Full (24K+): include all sections
4. Log the selected tier: `tracing::info!(tier = ?tier, budget, "context tier selected for {model}")`.

## Design Guidance

The tier should be derived from the model's context window, not hardcoded per model name.
Add a `context_window` field to `ModelProfile` in config, and derive tier from that. This
makes new models automatically get the right tier.

## Write Scope

- `crates/roko-compose/src/context_provider.rs`
- `crates/roko-compose/src/prompt_assembly_service.rs`
- `crates/roko-cli/src/run.rs`

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

- [ ] With `default_model = "ollama/gemma4"`, assembled prompt is under 4K tokens
- [ ] With `default_model = "claude-sonnet-4"`, assembled prompt uses full budget
- [ ] System log shows tier selection

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: STAB_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- With `default_model = "ollama/gemma4"`, assembled prompt is under 4K tokens
- With `default_model = "claude-sonnet-4"`, assembled prompt uses full budget
- System log shows tier selection
- No files outside the Write Scope are modified.
- Commit message contains `tracker: STAB_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
