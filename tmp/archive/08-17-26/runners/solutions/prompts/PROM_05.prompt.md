# PROM_05: Thread model_slug Through dispatch_agent_with()

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-05`](../ISSUE-TRACKER.md#prom-05)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.5
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_05 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Pass the CascadeRouter-selected model slug into prompt assembly
so `ContextTier` is consulted before building the system prompt. The
function is at line 14469.

## Exact Changes

1. In `dispatch_agent_with()`, after model selection (CascadeRouter), extract `model_slug`
2. Compute `ContextTier::from_task_and_model(&task_tier_string, &model_slug)`
3. Pass model slug/tier into prompt assembly (via `PromptAssemblyService::with_model_slug()` or directly into `build_system_prompt_with_context_validated()`)
4. Use `tier_scaled_budget()` to scale the per-role budget before passing to the builder
5. Log the selected tier at `info!` level: `"context_tier={tier:?} model={model_slug}"`

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

- [ ] Run `roko plan run` with a task configured for Ollama backend: system prompt <= 4K tokens (verify via log)
- [ ] Run with Opus backend: system prompt <= 24K tokens
- [ ] Log line shows "context_tier=Surgical" or "context_tier=Full" as appropriate

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_05 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Run `roko plan run` with a task configured for Ollama backend: system prompt <= 4K tokens (verify via log)
- Run with Opus backend: system prompt <= 24K tokens
- Log line shows "context_tier=Surgical" or "context_tier=Full" as appropriate
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_05 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
