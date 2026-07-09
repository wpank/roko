# F — Advanced Allocation And Scoring

Coverage for:

- `docs/03-composition/07-active-inference-context-selection.md`
- `docs/03-composition/09-predictive-foraging-mvt.md`
- `docs/03-composition/10-vcg-attention-auction.md`
- `docs/03-composition/11-distributed-context-engineering.md`
- `docs/03-composition/12-affect-modulated-retrieval.md`

---

## Verdict

`defer`

Keep the small amount of shipped scoring code. Defer the rest.

---

## What Exists

- `SectionScorer` is real at `crates/roko-compose/src/scorer.rs:21-90`.
- `ActiveInferenceScorer` is real at `crates/roko-compose/src/scorer.rs:98-229`.
- `RoleSystemPromptSpec::composition_scorer()` selects between them at `crates/roko-compose/src/role_prompts.rs:475-480`.
- `PadState` and affect-guidance hooks are real inputs elsewhere in composition and neuro code.

---

## What Must Be Reframed

`ActiveInferenceScorer` is the main truthfulness problem.

The implementation uses:

- hashed lightweight text embeddings,
- token overlap,
- simple prior beliefs,
- a pragmatic/epistemic scoring heuristic.

It does **not** demonstrate a formal expected-free-energy implementation. The name is stronger than the code.

The correct parity language is:

- shipped heuristic scorer,
- goal-directed and information-aware,
- not evidence of a full active-inference subsystem.

---

## What Is Deferred

These should be labeled as deferred or theory-only in parity materials:

- VCG attention auction
- predictive foraging / MVT as a composition subsystem
- distributed context engineering
- RAGAS
- CLEAR
- CIV
- Meta-Harness

There is some related code surface:

- `MARGINAL_VALUE_STOP_RATIO` exists in `crates/roko-neuro/src/context.rs:235`,
- semantic-similarity helpers exist at `crates/roko-neuro/src/context.rs:924-931`,

but that is not enough to describe the full theory stack as implemented.

---

## Follow-On Batch Shape

The only realistic near-term batch here is small:

1. rename or relabel `ActiveInferenceScorer`,
2. tighten comments/tests around what it actually does,
3. leave VCG/MVT/distributed/eval work deferred.
