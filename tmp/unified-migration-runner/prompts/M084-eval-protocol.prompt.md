# M084 — Eval Protocol

**[BLOCKED:depth]** -- This item depends on `tmp/unified-depth/19-arenas/` depth docs. The depth docs specify ground truth source interfaces, scoring calibration, and Variance Inequality measurement methods.

## Objective
Implement the Eval protocol: calibration against ground truth sources (test suites, oracles, human review, chain state, benchmarks). Enforce the Variance Inequality: the verifier must be spectrally cleaner than the generator. No LLM-judging-itself. Evals are the mechanism by which arena scores become trustworthy.

## Scope
- Crates: `roko-learn`
- Files: `crates/roko-learn/src/arena/eval.rs` (new)
- Phase ref: `tmp/unified-migration/04-PHASE-3-ECONOMY.md` SS3.5
- Spec ref: `tmp/unified/19-ARENAS-EVALS-BOUNTIES.md` SS5
- Depth docs: `tmp/unified-depth/19-arenas/` (pending)

## Steps
1. Define the Eval types:
   ```rust
   pub struct Eval {
       pub id: String,
       pub arena_id: String,
       pub ground_truth: GroundTruthSource,
       pub scoring: ScoringFunction,
       pub variance_check: VarianceCheckConfig,
   }

   pub struct EvalResult {
       pub eval_id: String,
       pub agent_id: String,
       pub score: f64,
       pub ground_truth_used: String,
       pub verifier_accuracy: f64,
       pub timestamp: DateTime<Utc>,
   }
   ```

2. Implement ground truth evaluation:
   - Test suite: run tests, count pass/fail
   - Oracle: query external oracle, compare output
   - Human review: collect human verdict
   - Chain state: query on-chain state, verify correctness

3. Enforce Variance Inequality: check that verifier accuracy exceeds generator improvement rate before accepting eval results.

4. Write tests: eval correctly scores agent output against ground truth.

## Verification
```bash
cargo check -p roko-learn
cargo clippy -p roko-learn --no-deps -- -D warnings
cargo test -p roko-learn -- arena::eval
```

## What NOT to do
- Do NOT use LLM output to judge LLM output -- ground truth is external only
- Do NOT proceed without depth docs
- Do NOT skip Variance Inequality checks
- Do NOT implement real oracle connections -- use mock ground truth for now
