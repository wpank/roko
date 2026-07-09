# EVAL_29: Pattern library and neuro store promotion

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-29`](../ISSUE-TRACKER.md#eval-29)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.29
- Priority: **P2**
- Effort: 6 hours
- Depends on: `EVAL_05` (source 5.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_29 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Patterns extracted from successful evaluation traces are promoted into the `roko-neuro` knowledge store as engrams. Queried at dispatch time for system prompt enrichment.

## Exact Changes

1. Define `PatternEntry { id, name, category, fingerprint: Option<String>, polarity: PatternPolarity, support_count: u32, avg_score: f64, template: Option<String>, anti_pattern_description: Option<String>, tags: Vec<String>, updated_at }`.
2. Define `PatternPolarity { Positive, Negative }`.
3. Define `PatternLibrary` storing to `.roko/eval/patterns.json` with `add(entry)`, `query(category, limit) -> Vec<PatternEntry>`.
4. Define `NeuroBridgeOutput` as a type that the orchestrator can use to create engrams in the neuro store (avoiding direct `roko-neuro` dependency from `roko-eval`).

## Write Scope

- `crates/roko-eval/src/lib.rs`

## Read-Only Context

The following files contain context that informs this change but **must
not** be modified by this batch. If you find yourself wanting to edit
them, file a follow-up tracker row instead.

- `tmp/runners/solutions/context-pack/00-RULES.md`
- `tmp/runners/solutions/context-pack/02-FILE-INVENTORY.md`
- `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` (the full task list this came from)

## Verification Checklist

These criteria come straight from the source task. Tick each one before
opening the merge:

- [ ] Test pattern creation and query
- [ ] Persistence round-trip test

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_29 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test pattern creation and query
- Persistence round-trip test
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_29 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
