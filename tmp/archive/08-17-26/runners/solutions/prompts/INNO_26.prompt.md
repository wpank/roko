# INNO_26: Implement swarm collaboration pattern

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-26`](../ISSUE-TRACKER.md#inno-26)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.26
- Priority: **P3**
- Effort: 16 hours
- Depends on: `INNO_21` (source 11.21)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_26 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

EventBus at `crates/roko-runtime/src/event_bus.rs` provides typed broadcast
channels with replay support. The swarm pattern uses this for inter-agent
communication.

Research: CodeCRDT (arxiv 2510.18893) -- 600-trial study shows up to 21.1%
speedup but also 39.4% slowdown depending on task structure. Parallelism is
not a free lunch.

## Exact Changes

1. Create `crates/roko-orchestrator/src/swarm.rs`.
2. Define `SwarmRunner` with `roles: Vec<AgentRole>`.
3. Implement shared message channel (tokio broadcast) for inter-agent communication.
4. Implementer agent runs in its worktree. Reviewer watches the diff stream.
5. If reviewer detects an issue mid-implementation, inject a `Redirect`
   steering action into the implementer's context.
6. Task completes when implementer finishes AND reviewer approves.

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

- [ ] Swarm mode: reviewer catches a bug mid-implementation, implementer receives the correction before finishing
- [ ] If reviewer never objects, task completes at normal speed (no overhead)
- [ ] Signal bus messages are recorded in the episode log

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_26 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Swarm mode: reviewer catches a bug mid-implementation, implementer receives the correction before finishing
- If reviewer never objects, task completes at normal speed (no overhead)
- Signal bus messages are recorded in the episode log
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_26 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
