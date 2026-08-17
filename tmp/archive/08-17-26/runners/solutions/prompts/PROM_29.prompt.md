# PROM_29: Add ReasoningDepth Tier-Based Default

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-29`](../ISSUE-TRACKER.md#prom-29)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.29
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Add a `ReasoningDepth` enum and include tier-appropriate
reasoning instructions in the role identity layer.

## Exact Changes

1. Add enum:
   ```rust
   pub enum ReasoningDepth {
       Suppress,  // "Do not explain. Just implement."
       Brief,     // "Briefly explain your approach, then implement."
       Deep,      // "Think step by step. Analyze, explain, implement."
   }
   ```
2. Add `pub fn with_reasoning_depth(mut self, depth: ReasoningDepth) -> Self` builder method
3. Inject reasoning instructions into Layer 1 (role identity) based on depth
4. Default: derive from `ContextTier` (Surgical -> Suppress, Focused -> Brief, Full -> Deep)
5. Allow experiment override from Task 6.28

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

- [ ] Surgical tier prompts contain "Do not explain" or equivalent
- [ ] Full tier prompts contain "Think step by step" or equivalent
- [ ] Experiment override changes the reasoning depth regardless of tier

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Surgical tier prompts contain "Do not explain" or equivalent
- Full tier prompts contain "Think step by step" or equivalent
- Experiment override changes the reasoning depth regardless of tier
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
