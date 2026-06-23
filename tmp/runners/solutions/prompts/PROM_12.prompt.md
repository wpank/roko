# PROM_12: Replace Binary Effectiveness Threshold with Graduated Scaling

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-12`](../ISSUE-TRACKER.md#prom-12)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.12
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_12 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Replace the binary `should_include()` (score < 0.1 = excluded,
line 185-190) with proportional per-section budget scaling.

## Exact Changes

1. Remove or soften the hard 0.1 threshold in `should_include()` (keep threshold at 0.0 to still exclude truly zero-value sections)
2. Add `fn section_budget_multiplier(&self, section_name: &str) -> f64`:
   - Returns the effectiveness score for the section, clamped to [0.0, 1.5]
   - Default 1.0 when no effectiveness data exists
3. In assembly, each section's character cap becomes `base_cap * section_budget_multiplier(name)`
4. A section at 0.05 effectiveness gets 5% of its normal cap (nearly excluded)
5. A section at 0.3 gets 30% of its cap (included at reduced size)

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

- [ ] Section with effectiveness 0.05 gets ~5% of normal budget (not hard-excluded)
- [ ] Section with effectiveness 0.3 gets ~30%
- [ ] Section with effectiveness 1.0 gets full budget
- [ ] Section with no effectiveness data gets full budget (safe default)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_12 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Section with effectiveness 0.05 gets ~5% of normal budget (not hard-excluded)
- Section with effectiveness 0.3 gets ~30%
- Section with effectiveness 1.0 gets full budget
- Section with no effectiveness data gets full budget (safe default)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_12 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
