# RNNR_14: Implement structured handoff documents for multi-role workflows

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-14`](../ISSUE-TRACKER.md#rnnr-14)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.14
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_14 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: For multi-pass workflows (strategist -> implementer -> reviewer),
generate structured handoff documents instead of passing raw strings.

## Exact Changes

1. Add `StrategyBrief` struct: `approach`, `key_constraints`, `files_to_modify`,
   `files_not_to_modify`, `estimated_complexity`
2. Add `ReviewFindings` struct: `must_fix: Vec<Finding>`, `nits: Vec<Finding>`
   where `Finding` has `file`, `line`, `description`
3. Add `fn format_strategy_brief(brief: &StrategyBrief) -> String` and
   `fn format_review_findings(findings: &ReviewFindings) -> String`
4. Parse agent output into these structures using regex patterns for common
   formats (numbered lists, `file:line` patterns)
5. Fallback to raw string when parsing fails (no crash on unusual formats)

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Strategist output parsed into `StrategyBrief` with structured fields
- [ ] Review findings parsed into `must_fix` and `nit` categories
- [ ] Implementer receives formatted brief with clear scope boundaries
- [ ] Fallback to raw string when parsing fails

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_14 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Strategist output parsed into `StrategyBrief` with structured fields
- Review findings parsed into `must_fix` and `nit` categories
- Implementer receives formatted brief with clear scope boundaries
- Fallback to raw string when parsing fails
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_14 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
