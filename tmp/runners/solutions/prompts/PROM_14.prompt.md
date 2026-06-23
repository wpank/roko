# PROM_14: Wire PromptAssemblyService into chat_session.rs System Prompt

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-14`](../ISSUE-TRACKER.md#prom-14)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.14
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: The `build_chat_system_prompt()` function (line 1350) already uses
`SystemPromptBuilder` directly. Upgrade it to use `PromptAssemblyService`
with model-aware tier selection.

## Exact Changes

1. In `build_chat_system_prompt()`, resolve the model slug from the config
2. Create a `PromptAssemblyService` with:
   - Default conventions from workspace detection (already done at line 1355)
   - Model slug for tier selection
   - No episodes or playbooks (cold start for chat)
3. Call `assemble()` with role = None (defaults to Implementer) and task = None
4. If assembly fails, fall back to the existing `SystemPromptBuilder` path
5. The resulting prompt replaces the current `builder.build()` output

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

- [ ] `roko chat` starts with a system prompt containing role identity and conventions
- [ ] The system prompt is tier-appropriate for the configured model
- [ ] Existing chat behavior is not broken

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko chat` starts with a system prompt containing role identity and conventions
- The system prompt is tier-appropriate for the configured model
- Existing chat behavior is not broken
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
