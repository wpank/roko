# INNO_30: Implement density-threshold gating for multi-agent

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-30`](../ISSUE-TRACKER.md#inno-30)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.30
- Priority: **P2**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_30 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: Stigmergic phase transition (arxiv 2512.10166) -- above agent density
rho_c = 0.230, trace-based coordination dominates by 36-41%. Below rho_c = 0.10,
stigmergy fails completely.

MAST taxonomy (NeurIPS 2025): 41-86.7% failure rates across multi-agent systems.
Princeton NLP: single agent matches multi-agent on 64% of tasks.

## Exact Changes

1. Define `agent_density(num_agents: usize, num_tasks: usize,
   interaction_edges: usize) -> f64`.
2. Before spawning parallel agents, compute density.
3. If density < 0.23, log warning and fall back to sequential execution.
4. Track density vs outcome in efficiency events for future calibration.
5. Add `multi_agent.density_threshold` to configuration (default 0.23).

## Write Scope

- `crates/roko-orchestrator/src/dag.rs`

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

- [ ] A plan with 2 agents and 20 tasks (density ~0.1) falls back to sequential
- [ ] A plan with 5 agents and 10 tasks (density ~0.5) proceeds with multi-agent
- [ ] Warning logged when density is below threshold

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_30 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A plan with 2 agents and 20 tasks (density ~0.1) falls back to sequential
- A plan with 5 agents and 10 tasks (density ~0.5) proceeds with multi-agent
- Warning logged when density is below threshold
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_30 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
