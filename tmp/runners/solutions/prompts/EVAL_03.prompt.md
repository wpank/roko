# EVAL_03: Define `Criterion` trait

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-03`](../ISSUE-TRACKER.md#eval-03)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.3
- Priority: **P0**
- Effort: 4 hours
- Depends on: `EVAL_01` (source 5.1)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_03 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

Currently, each gate in `roko-gate` both collects evidence and evaluates it in a single `verify()` call. The `Criterion` trait separates evaluation: it receives pre-collected evidence via `EvidenceBag` and produces a `CriterionResult`.

## Exact Changes

1. Define the `Criterion` trait:
   ```rust
   #[async_trait]
   pub trait Criterion: Send + Sync {
       fn name(&self) -> &str;
       fn criterion_kind(&self) -> CriterionKind;
       fn is_hard(&self) -> bool;
       fn required_evidence(&self) -> &[EvidenceKind];
       fn optional_evidence(&self) -> &[EvidenceKind] { &[] }
       fn default_threshold(&self) -> f64 { 0.5 }
       async fn evaluate(
           &self,
           artifact: &ArtifactRef,
           evidence: &EvidenceBag,
           ctx: &Context,
       ) -> Result<CriterionResult, EvalError>;
   }
   ```
2. Implement `fn check_evidence(criterion: &dyn Criterion, bag: &EvidenceBag) -> Result<(), EvalError>` as a standalone function that verifies all `required_evidence()` kinds exist in the bag, returning `EvalError::EvidenceUnavailable` if any are missing.
3. Add `pub mod criterion;` to `lib.rs` and re-export.

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

- [ ] Unit test with a mock criterion and a pre-populated `EvidenceBag`
- [ ] `check_evidence` returns error when required evidence is missing
- [ ] `check_evidence` returns Ok when required evidence is present

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_03 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Unit test with a mock criterion and a pre-populated `EvidenceBag`
- `check_evidence` returns error when required evidence is missing
- `check_evidence` returns Ok when required evidence is present
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_03 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
