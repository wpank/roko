# EVAL_28: Auto-grade bridge to existing `FeedbackService`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-28`](../ISSUE-TRACKER.md#eval-28)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.28
- Priority: **P1**
- Effort: 5 hours
- Depends on: `EVAL_05` (source 5.5)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_28 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Converts EvalTrace verdicts into `KnowledgeOutcome` records for the existing `FeedbackService` at `crates/roko-learn/src/feedback_service.rs`. Also wires into `ExperimentStore`: when trace carries a `prompt_variant`, report pass/fail to `VariantStats` for UCB1 convergence.

The existing `KnowledgeOutcome` enum at `crates/roko-learn/src/feedback_service.rs:27-33`:
```rust
pub enum KnowledgeOutcome { Success, Failure, Partial }
```

## Exact Changes

1. Implement `eval_trace_to_knowledge_outcome(trace: &EvalTrace) -> KnowledgeOutcome`: passed -> Success, failed -> Failure.
2. Implement `bridge_to_experiment_store(trace: &EvalTrace, store: &ExperimentStore)`: if `trace.pipeline_context.prompt_variant` is Some, record pass/fail to `VariantStats`.
3. The bridge functions are free-standing (not a trait impl) -- they are called from the orchestrator wiring in Task 5.46.

## Design Guidance

Do NOT add the `roko-learn` dependency to `roko-eval` (circular risk). Instead, define the bridge as a conversion function that returns intermediate types. The actual `FeedbackService` calls happen in the orchestrator (which already depends on both crates). Define a `FeedbackBridgeOutput { knowledge_outcomes: Vec<(String, KnowledgeOutcome)>, experiment_outcome: Option<(String, bool)> }` that the orchestrator can consume.

Wait -- re-check: `roko-eval` depending on `roko-learn` may create a cycle since `roko-learn` may later depend on `roko-eval`. Safer approach: define the bridge output types in `roko-eval` and let the orchestrator do the actual FeedbackService/ExperimentStore calls.

## Write Scope

- `crates/roko-eval/src/lib.rs`
- `crates/roko-eval/Cargo.toml`

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

- [ ] Test that a passing trace produces Success outcome
- [ ] Test that a failing trace produces Failure outcome
- [ ] Test experiment bridge with a trace carrying prompt_variant

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_28 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Test that a passing trace produces Success outcome
- Test that a failing trace produces Failure outcome
- Test experiment bridge with a trace carrying prompt_variant
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_28 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
