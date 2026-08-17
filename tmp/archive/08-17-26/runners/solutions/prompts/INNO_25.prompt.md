# INNO_25: Implement competitive proposals (Best-of-N)

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-25`](../ISSUE-TRACKER.md#inno-25)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.25
- Priority: **P2**
- Effort: 16 hours
- Depends on: `INNO_08` (source 11.8)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

WorktreeManager at `crates/roko-orchestrator/src/worktree.rs` (1,203 LOC) can
isolate parallel attempts. CancelToken at `crates/roko-runtime/src/cancel.rs`
can stop losing agents.

Research: up to 70% higher success rates with multi-agent collaboration vs
single-agent (AWS Strands Agents, OpenAI Agents SDK, Swarms framework).
But Princeton NLP: single well-tooled agent matches or outperforms multi-agent
on 64% of tasks. Competitive proposals are the exception where multi-agent
reliably wins.

## Exact Changes

1. Create `crates/roko-orchestrator/src/competitive.rs`.
2. Define `CompetitiveRunner` struct with `proposal_count: usize` (default 3).
3. Implement `run_competitive(task: &Task, n: usize) -> Vec<ProposalResult>`:
   - Allocate N worktrees via WorktreeManager.
   - Spawn N agents concurrently (different models or different prompts).
   - Run gate pipeline on each completed proposal.
   - Rank by gate score. Select winner.
   - Clean up losing worktrees.
4. Wire into the dispatch path: if `--collaboration=competitive` is set,
   use CompetitiveRunner instead of single dispatch.
5. Track all proposals in the episode log with a `proposal_group` field.

## Write Scope

- `crates/roko-orchestrator/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/11-INNOVATIONS.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] `roko plan run --collaboration=competitive --proposals=3` spawns 3 implementers in separate worktrees
- [ ] Gate pipeline scores all 3. Best wins. Others are cleaned up
- [ ] TUI shows all proposals with scores
- [ ] Episode log records all 3 proposals with the same `proposal_group`

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- `roko plan run --collaboration=competitive --proposals=3` spawns 3 implementers in separate worktrees
- Gate pipeline scores all 3. Best wins. Others are cleaned up
- TUI shows all proposals with scores
- Episode log records all 3 proposals with the same `proposal_group`
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
