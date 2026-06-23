# PROM_30: Wire Shared Vocabulary Injection for Multi-Agent Plans

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-30`](../ISSUE-TRACKER.md#prom-30)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.30
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When multiple agents work on tasks in the same plan, inject shared
vocabulary definitions. The `ContextMesh` struct already exists in
`context_mesh.rs`.

## Exact Changes

1. Add `shared_vocabulary: Option<Vec<(String, String)>>` to `PromptAssemblyService`
2. Builder method: `with_shared_vocabulary(vocab: Vec<(String, String)>) -> Self`
3. In assembly, if vocabulary is present, inject as a section:
   ```
   ## Shared Vocabulary (plan coordination)
   - "tier" = ContextTier (Surgical/Focused/Full)
   - "budget" = token budget, not character budget
   ```
4. In orchestrate.rs, extract vocabulary from plan metadata and pass to assembly

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

- [ ] Plan with `shared_vocabulary` in metadata injects vocabulary into agent prompts
- [ ] All agents in the plan see the same vocabulary definitions

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Plan with `shared_vocabulary` in metadata injects vocabulary into agent prompts
- All agents in the plan see the same vocabulary definitions
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
