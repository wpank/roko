# RNNR_18: Add anti-pattern false-positive tracking and exemption learning

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#rnnr-18`](../ISSUE-TRACKER.md#rnnr-18)
- Source: `tmp/solutions/roko/tasks/14-RUNNER-PATTERNS.md` — Task 14.18
- Priority: **??**
- Effort: ?
- Depends on: `RNNR_16` (source 14.16)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: RNNR_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Track false positive rates per rule and per file pattern. After N
false positives for a rule+file combination, auto-suggest an exemption.

## Exact Changes

1. Add `AntiPatternStats` persisted to `.roko/learn/anti-pattern-stats.json`:
   ```rust
   pub struct AntiPatternStats {
       pub per_rule: HashMap<String, RuleStats>,
   }
   pub struct RuleStats {
       pub total_fires: u64,
       pub false_positives: u64,
       pub auto_exemptions: Vec<String>,
   }
   ```
2. When task succeeds on retry after AP failure, mark the prior AP firing
   as a potential false positive
3. When false positive rate for a rule+file exceeds 50% over 10+ firings,
   suggest an exemption (log at warn level)
4. Persist stats after each plan run

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

- [ ] False positive rate tracked per rule
- [ ] Stats survive across runs (persisted to disk)
- [ ] Auto-exemption suggestions appear in logs when rate is high
- [ ] Manual exemption override works via task config

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: RNNR_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- False positive rate tracked per rule
- Stats survive across runs (persisted to disk)
- Auto-exemption suggestions appear in logs when rate is high
- Manual exemption override works via task config
- No files outside the Write Scope are modified.
- Commit message contains `tracker: RNNR_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
