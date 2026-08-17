# PROM_18: Lower VCG Warmup and Wire Observation Recording

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#prom-18`](../ISSUE-TRACKER.md#prom-18)
- Source: `tmp/solutions/roko/tasks/06-PROMPT-ASSEMBLY.md` — Task 6.18
- Priority: **??**
- Effort: ?
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: PROM_18 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

: Lower `DEFAULT_VCG_WARMUP_OBSERVATIONS` from 10 to 5 (line 10)
and ensure observations are recorded per bidder during dispatch.

## Exact Changes

1. Change `DEFAULT_VCG_WARMUP_OBSERVATIONS` from 10 to 5 (line 10)
2. In `orchestrate.rs`, after each dispatch, increment the bidder observation
   count for each `AttentionBidder` that contributed sections to the prompt
3. Persist bidder observations alongside existing learning state
4. Update the test at line 79 that asserts on the warmup threshold

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

- [ ] After 5 tasks (not 10), VCG allocation activates when strategy is `Auto`
- [ ] `CompositionStrategy::auto_select()` returns `Vcg` with 5+ observations per bidder
- [ ] Observation counts persist across runs

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: PROM_18 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- After 5 tasks (not 10), VCG allocation activates when strategy is `Auto`
- `CompositionStrategy::auto_select()` returns `Vcg` with 5+ observations per bidder
- Observation counts persist across runs
- No files outside the Write Scope are modified.
- Commit message contains `tracker: PROM_18 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
