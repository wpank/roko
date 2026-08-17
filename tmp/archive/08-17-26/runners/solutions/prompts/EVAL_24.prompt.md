# EVAL_24: Judge prompt templates and rubrics

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-24`](../ISSUE-TRACKER.md#eval-24)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.24
- Priority: **P1**
- Effort: 5 hours
- Depends on: `EVAL_18` (source 5.18)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_24 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Code evaluation 7-dimension rubric: correctness (0.30), maintainability (0.20), safety (0.15), performance (0.10), test_coverage (0.10), api_design (0.10), documentation (0.05). All prompts use analyze-before-rate format, JSON-only output.

## Exact Changes

1. Define `Rubric { dimensions: Vec<RubricDimension> }` and `RubricDimension { name: String, weight: f64, description: String, min_score: f64 }`.
2. Implement `code_rubric() -> Rubric` with the 7 code dimensions.
3. Define `JudgePrompt { system: String, user: String }` and implement `render_pairwise_prompt(rubric, artifact_a, artifact_b) -> JudgePrompt`.
4. Implement `parse_judge_response(response: &str) -> Result<JudgeRating, EvalError>` parsing the JSON output.
5. Define `JudgeRating { analysis_a: String, analysis_b: String, rubric_a: HashMap<String, f64>, rubric_b: HashMap<String, f64>, findings: Vec<Finding>, preference: PairwiseVerdict, confidence: f64, reasoning: String }`.

## Write Scope

- `crates/roko-eval-judge/src/lib.rs`

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

- [ ] Test prompt rendering contains all 7 dimensions
- [ ] Test JSON parsing of a sample response

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_24 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test prompt rendering contains all 7 dimensions
- Test JSON parsing of a sample response
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_24 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
