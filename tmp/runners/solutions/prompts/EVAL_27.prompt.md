# EVAL_27: Preference mining and `PreferenceTriple`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-27`](../ISSUE-TRACKER.md#eval-27)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.27
- Priority: **P1**
- Effort: 5 hours
- Depends on: `EVAL_05` (source 5.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_27 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Every preference signal is logged as a `PreferenceTriple`. This collection is a private arena for pairwise learning.

## Exact Changes

1. Define `PreferenceTriple { id, prompt, candidate_a, candidate_b, preferred: PreferenceChoice, source: PreferenceSource, trace_id_a, trace_id_b, task_id, timestamp, criterion_deltas: Vec<CriterionDelta>, confidence: f64 }`.
2. Define `PreferenceSource { UserEdit, UserSelection, JudgePanel, ExternalBenchmark, RegressionComparison }`.
3. Define `CriterionDelta { criterion: String, score_a: f64, score_b: f64 }`.
4. Define `PreferenceStore` appending to `.roko/learn/preferences.jsonl` (same pattern as `EpisodeLogger`).
5. Implement mining: `mine_from_judge_panel(trace: &EvalTrace) -> Vec<PreferenceTriple>` -- every pairwise comparison produces a triple.

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

- [ ] Write/read round-trip test
- [ ] Mining from a trace with 3 judge comparisons produces 3 triples

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_27 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Write/read round-trip test
- Mining from a trace with 3 judge comparisons produces 3 triples
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_27 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
