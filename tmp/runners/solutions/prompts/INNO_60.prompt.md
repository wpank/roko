# INNO_60: Implement CMP scoring for agent variants

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#inno-60`](../ISSUE-TRACKER.md#inno-60)
- Source: `tmp/solutions/roko/tasks/11-INNOVATIONS.md` — Task 11.60
- Priority: **P3**
- Effort: 8 hours
- Depends on: **none**

When this batch lands, the commit message MUST contain the trailer:

```
tracker: INNO_60 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Research: Huxley Godel Machine (ICLR 2026 oral) -- CMP scores agent variants
by aggregate descendant performance, not the variant's own output.

## Exact Changes

1. Track agent lineage: which configuration produced which outcomes, and which
   configurations descended from which.
2. Define CMP score: average gate pass rate of all tasks dispatched by agents
   using this configuration AND all descendant configurations.
3. When evaluating which configuration to use, prefer higher CMP scores.
4. Store CMP scores in `.roko/learn/agent-variants.json`.
5. Add `roko learn agents` CLI showing variant CMP scores.

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

- [ ] An agent configuration with good outcomes AND good descendant performance has a higher CMP score than one with only good individual performance
- [ ] CMP scores persist across runs
- [ ] `roko learn agents` displays variant lineage with CMP scores

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: INNO_60 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- An agent configuration with good outcomes AND good descendant performance has a higher CMP score than one with only good individual performance
- CMP scores persist across runs
- `roko learn agents` displays variant lineage with CMP scores
- No files outside the Write Scope are modified.
- Commit message contains `tracker: INNO_60 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
