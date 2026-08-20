# 140 — Merge Success/Conflict Proof via Active Runner

**Priority**: P2 — `PlanMerger` and `GitMergeBackend` exist but have never been proven correct end-to-end; multi-agent plan execution on the same repo cannot be trusted until merge correctness is demonstrated under both success and conflict conditions.
**Size**: S (1 day)
**Crates**: `crates/roko-cli/src/runner/`, `tests/`
**Depends on**: #139 (per-plan agent-handle map — concurrent execution is a prerequisite for meaningful merge testing)
**Sources**: `tmp/backlog/_mori-diffs-gaps.md` §D-3 (suggested 120)

---

## Background

`PlanMerger` and `GitMergeBackend` implement the merge-to-branch step that runs after each plan completes its gate pipeline. The merger takes the plan's worktree branch and merges it into the batch branch, then continues with the next plan. This is how roko produces a coherent combined output from multiple concurrent plans.

The audit identified that the original runner had a "merge auto-success stub" path that marked merges as succeeded without performing a real git merge. Even if that stub is retired, the merge machinery has not been exercised under controlled proof conditions. A reproducible proof is needed covering:
1. Non-conflicting merge: two plans touch different files → merge succeeds → events show `MergeSucceeded`.
2. Conflicting merge: two plans touch the same line → merge fails → events show `MergeConflict` with conflict evidence.
3. Post-merge regression gate: merge succeeds at the git level but a subsequent `cargo check` fails → the gate failure is not converted to `MergeSucceeded`.

## Current State

- `PlanMerger` — present in the runner; exact file path requires inspection.
- `GitMergeBackend` — performs the actual `git merge` operation.
- `RunnerEvent::MergeSucceeded` / `MergeConflict` — may or may not exist; needs verification.
- Auto-success stub — may still be present in the legacy runner; need to confirm retirement.
- No end-to-end merge proof exists.

## Implementation Plan

1. **Audit auto-success stub**: Grep for `MergeSucceeded` and any path that bypasses `GitMergeBackend`. If found, confirm it is only in the legacy runner (which is being retired via #132). If present in runner-v2, remove it.

2. **Proof harness for non-conflicting merge**: Create `tests/merge_proof/non_conflicting.rs`:
   - Set up two plan worktrees that modify different files.
   - Run both plans to gate-pass state.
   - Trigger the merge sequence.
   - Assert: `MergeSucceeded` event in `.roko/events.jsonl`, batch branch contains both changes.

3. **Proof harness for conflicting merge**: Create `tests/merge_proof/conflicting.rs`:
   - Set up two plans that modify the same line in the same file.
   - Run both plans to gate-pass state.
   - Trigger the merge sequence.
   - Assert: `MergeConflict` event in `.roko/events.jsonl` with conflict evidence (file name, line range).
   - Assert: the batch branch is NOT corrupted (no partial merge state).

4. **Proof for post-merge regression gate**: Create `tests/merge_proof/regression_gate.rs`:
   - Set up two plans that individually pass `cargo check` but together fail (e.g., plan A adds a function, plan B adds a call to a different version of that function).
   - Merge plan A successfully.
   - Attempt to merge plan B.
   - Assert: the post-merge regression gate fires and the event is `GateFailed`, not `MergeSucceeded`.

5. **HTTP evidence**: Each proof script asserts the same events are queryable via `GET /api/runs/<id>/events` (from #131 when available, or from JSONL file read in the interim).

6. **Fix any bugs found**: Expect that running these proofs will expose bugs. Fix them as part of this item.

## Acceptance Criteria

1. Non-conflicting merge proof passes: `MergeSucceeded` event in events log, batch branch has both changes.
2. Conflicting merge proof passes: `MergeConflict` event in events log, no partial merge state in batch branch.
3. Regression gate proof passes: post-merge gate failure produces `GateFailed`, not `MergeSucceeded`.
4. All three proof scripts are deterministic (not flaky).
5. Auto-success stub is confirmed absent from runner-v2.

## Verification Checklist

- [ ] Run non-conflicting proof; verify `MergeSucceeded` in `events.jsonl`.
- [ ] Run conflicting proof; verify `MergeConflict` in `events.jsonl` with file name.
- [ ] Run regression gate proof; verify `GateFailed` after merge.
- [ ] Check `events.jsonl` for any `MergeSucceeded` that does not have a corresponding git log entry; verify zero such entries.

## Files to Modify

| File | Change |
|---|---|
| `crates/roko-cli/src/runner/` | Fix merge bugs discovered during proof |
| `tests/merge_proof/` | New directory with three proof scripts |
| `crates/roko-cli/src/runner/types.rs` | Verify `MergeSucceeded`/`MergeConflict` event variants exist |
