# INNO_42: Implement pheromone signal sanitization

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-42`](../ISSUE-TRACKER.md#inno-42)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.42
- Priority: **P1**
- Effort: 4 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_42 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: Multi-agent attack amplification (arxiv 2504.16489) -- structured
prompt rewriting raises mean harmfulness from 28.14% to 80.34% in Multi-Agent
Debate. Infectious Jailbreak: one adversarial image propagates to ~100% of
agents.

## Exact Changes

1. Before injecting pheromone signals into an agent's context, pass through
   a sanitization pipeline:
   - Strip any executable content (code blocks that look like tool calls).
   - Validate signal format against expected schema.
   - Truncate to maximum pheromone size (configurable, default 500 tokens).
2. Log sanitization events: what was stripped, from which source.
3. If a pheromone fails validation entirely, quarantine it and log a warning.

## Write Scope

- `crates/roko-cli/src/orchestrate.rs`

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

- [ ] A pheromone containing a fake tool call is sanitized (tool call stripped)
- [ ] A pheromone exceeding 500 tokens is truncated
- [ ] An invalid pheromone is quarantined, not injected
- [ ] Sanitization events are logged

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_42 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- A pheromone containing a fake tool call is sanitized (tool call stripped)
- A pheromone exceeding 500 tokens is truncated
- An invalid pheromone is quarantined, not injected
- Sanitization events are logged
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_42 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
