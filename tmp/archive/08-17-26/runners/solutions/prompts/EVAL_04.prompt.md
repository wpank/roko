# EVAL_04: Define `Profile` and `EvalService`

## Tracker

- Issue tracker row: [`ISSUE-TRACKER.md#eval-04`](../ISSUE-TRACKER.md#eval-04)
- Source: `tmp/solutions/roko/tasks/05-GATE-EVOLUTION.md` — Task 5.4
- Priority: **P0**
- Effort: 8 hours
- Depends on: `EVAL_01` (source 5.1), `EVAL_02` (source 5.2), `EVAL_03` (source 5.3)

When this batch lands, the commit message MUST contain the trailer:

```
tracker: EVAL_04 done
```

`bin/sync-tracker.py --apply` flips the corresponding `[ ]` to `[x]`.

## Problem

`Profile` composes criteria into a named evaluation strategy. `EvalService` orchestrates the full evaluation lifecycle: resolve profile -> collect evidence -> run criteria -> aggregate verdict -> emit trace.

The existing `GatePipeline` at `crates/roko-gate/src/gate_pipeline.rs` implements sequential composition of `Verify` impls with short-circuit on failure. `EvalService` does the same but over `Criterion` impls with typed evidence and richer aggregation.

## Exact Changes

1. Define `Profile`:
   ```rust
   pub struct Profile {
       pub id: String,
       pub name: String,
       pub tags: Vec<String>,
       pub strategy: EvalStrategy,
       pub criteria: Vec<CriterionRef>,
   }
   pub enum EvalStrategy {
       Sequential,
       ConjunctiveHardParetoSoft,
       WeightedSum { weights: Vec<f64> },
   }
   pub struct CriterionRef {
       pub name: String,
       pub hard: Option<bool>,
       pub threshold: Option<f64>,
       pub params: serde_json::Value,
   }
   ```
2. Define `EvalService`:
   ```rust
   pub struct EvalService {
       pub collectors: Vec<Box<dyn EvidenceCollector>>,
       pub criteria: Vec<Box<dyn Criterion>>,
   }
   impl EvalService {
       pub async fn evaluate(
           &self,
           artifact: &ArtifactRef,
           profile: &Profile,
           ctx: &Context,
       ) -> Result<EvalTrace, EvalError>;
   }
   ```
3. `evaluate()` flow:
   a. Determine which evidence kinds are needed from the profile's criteria.
   b. Run only the collectors that produce those kinds.
   c. Run criteria in profile order. For `Sequential` strategy, short-circuit on hard failure. For `ConjunctiveHardParetoSoft`, run all hard criteria first (short-circuit on failure), then run soft criteria and aggregate.
   d. Aggregate into `EvalVerdict`: passed = all hard criteria pass AND soft score >= profile threshold.
   e. Return `EvalTrace` (defined in Task 5.5).

## Design Guidance

Mirror the short-circuit pattern from `GatePipeline::verify()` at `crates/roko-gate/src/gate_pipeline.rs:224-244`. The evidence optimization (only collecting what criteria need) is critical -- it avoids running expensive collectors (e.g., coverage) when no criterion requires that evidence. Use `HashSet<EvidenceKind>` to compute the union of all criteria's required evidence kinds, then filter collectors to those whose `produces()` intersects with the needed set.

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

- [ ] Integration test: ProcessCollector -> mock CompileCriterion -> EvalService -> EvalVerdict
- [ ] Short-circuit test: second criterion not called when first hard criterion fails
- [ ] Evidence optimization test: unused collector not invoked

## Verify Recipe

```bash
# Spot-check with ripgrep / git on the touched files.
# Do NOT run cargo — the merge-back pipeline does that.
git diff --stat
```

Then check the commit-message trailer:

```bash
git log -1 --format=%B | rg "^tracker: EVAL_04 done"
```

## Acceptance Criteria

- All Verify checkboxes pass on inspection.
- Integration test: ProcessCollector -> mock CompileCriterion -> EvalService -> EvalVerdict
- Short-circuit test: second criterion not called when first hard criterion fails
- Evidence optimization test: unused collector not invoked
- No files outside the Write Scope are modified.
- Commit message contains `tracker: EVAL_04 done` trailer.
- Pre-commit pipeline (fmt + clippy + test) passes when run by the merge-back stage.

## Do NOT

- Compile or run tests inside the batch (`cargo check/test/clippy/build`).
- Touch files outside the Write Scope above.
- Bundle this batch with another tracker row, even if both touch the same file.
- Add `unwrap()`, `panic!()`, or `unreachable!()` to changed code.
- Add a second dispatch / prompt-assembly / chat-state path. See
  `context-pack/00-RULES.md` §"Universal anti-patterns".
- Refactor neighbours opportunistically. File a separate tracker row.
