# ORCH_26: Audit Feature Parity Before Retirement

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#orch-26`](../ISSUE-TRACKER.md#orch-26)
- Source: `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` — Task 2.26
- Priority: **P2**
- Effort: 4 hours
- Depends on: `ORCH_08` (source 2.8), `ORCH_12` (source 2.12)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: ORCH_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Before deleting orchestrate.rs (22,522 lines), every feature in the "Features Only in Dead Code" table (from the AUDIT) must be classified as: ported to WorkflowEngine, explicitly deprecated, or documented as a gap.

Features from the audit table (Section 10):
- Dream consolidation (line 7589+)
- Daimon affect engine (full, line 266+)
- Knowledge routing (`build_knowledge_routing_advice()`)
- VCG auction (`vcg_allocate()`)
- Custody audit chain (`CustodyLogger`)
- Skill extraction (`SkillLibrary::extract()`)
- Anophily remediation
- C-factor computation (`CFactorSummary`)
- 30+ enrichment steps
- Predictive calibration (`CalibrationTracker`)
- Section effectiveness (`SectionEffectivenessRegistry`)
- Error pattern queries (`ErrorPatternStore`)
- Model experiments (`ModelExperimentStore`)
- Heartbeat monitoring (`HeartbeatClock`)
- Routing decision log (`RoutingDecisionLog`)

## Exact Changes

1. For each feature in the table, determine:
   - Is it ported to WorkflowEngine? (via Tasks 2.10-2.12 or prior work)
   - Is it used by Runner v2? (still active)
   - Is it dead code with no consumers?
2. Create a classification table in `.roko/GAPS.md`.
3. Features that are ported -> mark as "ported, verified".
4. Features that are only in orchestrate.rs and have no consumer -> mark as "deprecated".
5. Features that are valuable but not yet ported -> mark as "gap, track for future" with specific task IDs.

## Design Guidance

Do not block deletion on porting every feature. Some features (VCG auction, anophily remediation) may be genuinely unused. The goal is to make a conscious decision about each feature, not to port them all.

## Write Scope

_None — this is a documentation/verification-only batch._

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/02-ORCHESTRATION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Every feature in the "Features Only in Dead Code" table is classified
- [ ] Classification table written to `.roko/GAPS.md`
- [ ] No feature is accidentally lost -- each is either ported, deprecated, or tracked

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: ORCH_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Every feature in the "Features Only in Dead Code" table is classified
- Classification table written to `.roko/GAPS.md`
- No feature is accidentally lost -- each is either ported, deprecated, or tracked
- No files outside the Write Scope are modified.
- Commit message contains `tracker: ORCH_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
