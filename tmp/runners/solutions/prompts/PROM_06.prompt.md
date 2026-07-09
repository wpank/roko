# PROM_06: Thread model_slug Through run.rs Path

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-06`](../ISSUE-TRACKER.md#prom-06)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.6
- Priority: **??**
- Effort: 1-2 days | **Impact**: Critical (enables budget convergence)
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_06 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: `BudgetPredictor` is fully built (EMA-based, failure
inflation, partial-match fallback, persistence, 679 LOC) but nobody calls
`predictor.predict()` before assembly or `predictor.record()` after gate results.

## Exact Changes

1. In the `roko run` handler, resolve the model slug from config or the
   `EffectiveModelSelection`
2. Compute ContextTier from model slug
3. Pass tier budget as the `context_window_tokens` parameter (currently
   hardcoded from `config.prompt.token_budget` at line 1398)
4. The same ContextTier logic from Task 6.5 applies here

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

- [ ] `roko run "test prompt"` with a configured Ollama model produces system prompt <= 4K tokens
- [ ] `roko run "test prompt"` with Opus produces system prompt <= 24K tokens

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_06 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko run "test prompt"` with a configured Ollama model produces system prompt <= 4K tokens
- `roko run "test prompt"` with Opus produces system prompt <= 24K tokens
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_06 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
