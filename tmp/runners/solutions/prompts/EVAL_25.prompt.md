# EVAL_25: `JudgePanelCriterion` and `PanelJudgeOracle` gate adapter

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-25`](../ISSUE-TRACKER.md#eval-25)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.25
- Priority: **P1**
- Effort: 8 hours
- Depends on: `EVAL_19` (source 5.19), `EVAL_20` (source 5.20), `EVAL_21` (source 5.21), `EVAL_22` (source 5.22), `EVAL_23` (source 5.23), `EVAL_24` (source 5.24)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_25 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

The full evaluation flow. `JudgePanelCriterion` implements the `Criterion` trait from `roko-eval`. `PanelJudgeOracle` implements the existing `JudgeOracle` trait from `roko-gate` for backward compatibility.

The existing `JudgeOracle` trait at `crates/roko-gate/src/llm_judge_gate.rs:53-56`:
```rust
pub trait JudgeOracle: Send + Sync {
    async fn judge(&self, prompt: &str) -> Result<f32, String>;
}
```

## Exact Changes

1. `JudgePanelCriterion` implements `Criterion`:
   - `criterion_kind()` = `CriterionKind::JudgePanel`
   - `required_evidence()` = `[EvidenceKind::Diff]` (minimum)
   - `evaluate()` flow: construct panel -> for each judge run pairwise with position swap -> discard inconsistent -> aggregate -> BT model -> return CriterionResult.
   - The actual LLM calls are delegated to a `JudgeInvoker` trait (abstracts the HTTP call).
2. `PanelJudgeOracle` implements `JudgeOracle`:
   - Wraps the full panel flow.
   - Returns a normalized f32 score.
3. Define `JudgeInvoker` trait:
   ```rust
   #[async_trait]
   pub trait JudgeInvoker: Send + Sync {
       async fn invoke(&self, spec: &JudgeSpec, prompt: &JudgePrompt) -> Result<String, EvalError>;
   }
   ```
   This is the adapter point for real LLM backends. A mock implementation is provided for testing.

## Design Guidance

The `JudgeInvoker` trait is critical for testability. Tests inject a mock invoker that returns predetermined responses. The real implementation (wiring to `roko-agent` backends) is done in Task 5.46 when the orchestrator integrates the full eval service.

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

- [ ] Integration test with mock judge invokers returning consistent preferences
- [ ] Test that inconsistent judges are discarded
- [ ] Test `PanelJudgeOracle` returns a score in [0, 1]

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_25 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Integration test with mock judge invokers returning consistent preferences
- Test that inconsistent judges are discarded
- Test `PanelJudgeOracle` returns a score in [0, 1]
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_25 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
