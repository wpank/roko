# SAFE_19: ASR (Attack Success Rate) Tracking

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#safe-19`](../ISSUE-TRACKER.md#safe-19)
- Source: `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` — Task 17.19
- Priority: **P3**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: SAFE_19 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Track the ratio of blocked vs. total safety checks. Surface as a
metric for monitoring safety enforcement effectiveness.

## Exact Changes

1. Define `SecurityMetrics` with counters per category (contract, permission,
   sanitization, rate_limit, network)
2. Increment on every safety check from the audit trail
3. Persist to `.roko/learn/security-metrics.json`
4. Add `roko learn security` CLI command showing metrics
5. Alert when ASR rises above 5% threshold

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/17-SAFETY-SECURITY.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko learn security` shows safety check breakdown by category
- [ ] Metrics persist across sessions
- [ ] ASR > 5% triggers a warning in `roko doctor`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: SAFE_19 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko learn security` shows safety check breakdown by category
- Metrics persist across sessions
- ASR > 5% triggers a warning in `roko doctor`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: SAFE_19 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
