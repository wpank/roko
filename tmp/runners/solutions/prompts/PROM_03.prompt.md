# PROM_03: Add Tier-Dependent Section Eligibility to ContextTier

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-03`](../ISSUE-TRACKER.md#prom-03)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.3
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Add `eligible_sections()` and `is_eligible()` methods to
`ContextTier` (currently defined at line 39).

## Exact Changes

1. Add method to `ContextTier` impl block (starts at line 48):
   ```rust
   pub fn eligible_sections(&self) -> &'static [&'static str]
   ```
2. Return values:
   - Surgical: `["identity", "task", "tools", "anti_patterns", "verification"]`
   - Focused: Surgical + `["conventions", "playbooks", "gate_feedback", "brief", "file_context"]`
   - Full: Focused + `["domain", "context", "workspace_map", "prd", "research", "episodes", "affect"]`
3. Add convenience method:
   ```rust
   pub fn is_eligible(&self, section_name: &str) -> bool
   ```
4. Add unit tests for all tier/section combinations

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

- [ ] `ContextTier::Surgical.is_eligible("workspace_map")` returns false
- [ ] `ContextTier::Surgical.is_eligible("task")` returns true
- [ ] `ContextTier::Full.is_eligible("workspace_map")` returns true

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `ContextTier::Surgical.is_eligible("workspace_map")` returns false
- `ContextTier::Surgical.is_eligible("task")` returns true
- `ContextTier::Full.is_eligible("workspace_map")` returns true
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
