# PROM_01: Add `model_slug` and `context_tier` to PromptAssemblyService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-01`](../ISSUE-TRACKER.md#prom-01)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.1
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_01 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Add optional `model_slug` and derived `context_tier` fields. When
set, the tier's `default_token_budget()` overrides the static `token_budget`.

## Exact Changes

1. Add fields to `PromptAssemblyService`:
   ```rust
   model_slug: Option<String>,
   context_tier: Option<ContextTier>,
   ```
2. Add builder methods:
   - `with_model_slug(slug: String) -> Self` -- sets `model_slug`
   - `with_context_tier(tier: ContextTier) -> Self` -- sets `context_tier`
3. In `assemble()`, compute `effective_budget`:
   - If `context_tier` is set: use `tier.default_token_budget()`
   - Else if `model_slug` is set: derive tier via `ContextTier::from_task_and_model()`, use its budget
   - Else if `token_budget` is set: use that
   - Else: no budget (unbounded, existing behavior)
4. Use `effective_budget` where `self.token_budget` was previously used (line 469)
5. Initialize both fields to `None` in `new()`

## Write Scope

_None — this is a documentation/verification-only batch._

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

- [ ] `PromptAssemblyService::new().with_model_slug("ollama/llama3.2".into())` produces assembly with budget <= 4000 tokens
- [ ] `PromptAssemblyService::new().with_model_slug("claude-sonnet-4-20250514".into())` produces budget <= 12000 tokens
- [ ] `PromptAssemblyService::new().with_model_slug("claude-opus-4-20250514".into())` produces budget <= 24000 tokens
- [ ] Existing callers that set `token_budget` continue to work unchanged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_01 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `PromptAssemblyService::new().with_model_slug("ollama/llama3.2".into())` produces assembly with budget <= 4000 tokens
- `PromptAssemblyService::new().with_model_slug("claude-sonnet-4-20250514".into())` produces budget <= 12000 tokens
- `PromptAssemblyService::new().with_model_slug("claude-opus-4-20250514".into())` produces budget <= 24000 tokens
- Existing callers that set `token_budget` continue to work unchanged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_01 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
