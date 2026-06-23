# PROM_04: Wire Tier Eligibility into PromptAssemblyService

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-04`](../ISSUE-TRACKER.md#prom-04)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.4
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: When `context_tier` is set, hard-exclude sections that the tier
does not support, before the effectiveness threshold check.

## Exact Changes

1. In `should_include()` (line 185), add a tier eligibility check before the
   effectiveness check:
   ```rust
   if let Some(tier) = self.context_tier {
       if !tier.is_eligible(section) {
           tracing::debug!(section, ?tier, "excluded by tier ineligibility");
           return false;
       }
   }
   ```
2. Make `WORKSPACE_MAP_LINE_LIMIT` tier-dependent:
   - Surgical: 0 (no workspace map)
   - Focused: 100
   - Full: 300
   - No tier set: 200 (current default)
3. Use the tier-aware limit in `workspace_map_for_spec()` or `workspace_map_from_file_listing()`

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

- [ ] Assembly with `ContextTier::Surgical` produces a prompt containing only identity, task, tools, anti-patterns, and verification content
- [ ] Assembly with `ContextTier::Surgical` has zero workspace_map content
- [ ] Assembly with `ContextTier::Full` includes all sections that pass effectiveness threshold

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Assembly with `ContextTier::Surgical` produces a prompt containing only identity, task, tools, anti-patterns, and verification content
- Assembly with `ContextTier::Surgical` has zero workspace_map content
- Assembly with `ContextTier::Full` includes all sections that pass effectiveness threshold
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
