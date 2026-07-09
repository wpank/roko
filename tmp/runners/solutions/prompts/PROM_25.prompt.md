# PROM_25: Tier-Adaptive Knowledge Confidence Thresholds

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-25`](../ISSUE-TRACKER.md#prom-25)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.25
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Make knowledge store confidence thresholds dependent on
`ContextTier` instead of hardcoded 0.5/0.3/0.2.

## Exact Changes

1. Add `fn confidence_thresholds(tier: Option<ContextTier>) -> (f64, f64, f64)` returning (domain_facts, techniques, anti_patterns):
   - Surgical: `(0.8, 0.7, 0.5)` -- only proven knowledge
   - Focused: `(0.5, 0.3, 0.2)` -- current defaults
   - Full: `(0.3, 0.2, 0.1)` -- include speculative knowledge
   - None: `(0.5, 0.3, 0.2)` -- Focused as safe default
2. Replace the hardcoded `>= 0.5` in `relevant_knowledge_for_spec()` (line 547) with `domain_threshold`
3. Replace `>= 0.3` in `query_techniques()` (line 228) with `technique_threshold`
4. Replace `>= 0.2` in `query_anti_patterns()` (line 241) with `anti_pattern_threshold`

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

- [ ] Surgical tier assembly includes fewer knowledge entries (higher threshold)
- [ ] Full tier assembly includes more knowledge entries (lower threshold)

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Surgical tier assembly includes fewer knowledge entries (higher threshold)
- Full tier assembly includes more knowledge entries (lower threshold)
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
