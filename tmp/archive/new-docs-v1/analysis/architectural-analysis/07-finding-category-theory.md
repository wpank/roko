# Finding: Category Theory Grounding

> The pipeline as Kleisli composition, Score as a commutative monoid, Verdict as a filtered
> monoid, and cross-cuts as endofunctors. These are structural guarantees, not metaphors.

**Status**: Analysis (informational — structural properties already hold)
**Crate**: `roko-core`
**Depends on**: [Six Synapse Traits](../../reference/05-operators/README.md), [Score](../../reference/10-types/score.md)
**Last reviewed**: 2026-04-13

---

## TL;DR

The Synapse Architecture's composability is not accidental — it is a structural property. The
six traits are morphisms in the Engram category. Score is a commutative monoid (under both
addition and multiplication). Verdict is a filtered monoid (the Maybe monad dual). The full
pipeline is Kleisli composition in the `Result<T, RokoError>` monad. Cross-cuts are
endofunctors with natural transformations between them.

**Practical implication**: Any new trait implementation that preserves these categorical
properties will compose correctly with existing code. Any implementation that violates them
will break composition.

---

## The Engram Category (Eng)

**Objects**: Types in the pipeline — `Vec<Signal>`, `Signal`, `Score`, `Selection`, `Verdict`

**Morphisms**: Trait operations, parameterized by `Context`:
- `query_ctx : 1 → Vec<Signal>` (Substrate)
- `score_ctx : Signal → Score` (Scorer)
- `select_ctx : Vec<Signal> → Option<Selection>` (Router)
- `compose_ctx : (Vec<Signal>, Budget) → Signal` (Composer)
- `verify_ctx : Signal → Verdict` (Gate)
- `decide_ctx : Vec<Signal> → Vec<Signal>` (Policy)

**Identity morphisms**: NoOp implementations (NoOpScorer, NoOpRouter, NoOpComposer, etc.)

**Composition**: Pipeline steps compose via standard function composition. The pipeline
`query >> select >> compose >> verify >> persist >> decide` is an arrow in the category
of "Eng-valued computations."

---

## Score as a Commutative Monoid

```
(Score, +, Score::ZERO)     — additive identity: {confidence: 0, novelty: 0, utility: 0, reputation: 0}
(Score, ×, Score::NEUTRAL)  — multiplicative identity: {confidence: 1, novelty: 0, utility: 0, reputation: 1}
```

Both operations are:
- **Associative**: `(a + b) + c = a + (b + c)`
- **Commutative**: `a + b = b + a`
- **Have identity**: `a + 0 = a`, `a × 1 = a`

The multiplicative monoid is particularly important for the effective score formula:

```
effective = confidence × (1 + novelty) × (1 + utility) × reputation
```

This is a **monoid homomorphism** from the product monoid `(Score, ×)` to the positive reals
`(ℝ⁺, ×)`. Monoid homomorphisms preserve composition, which means composing Scores and then
computing effective is the same as computing effective on each and multiplying.

**Practical implication**: Any new Scorer implementation that produces Score values respecting
these algebraic laws will compose correctly with SumScorer, MulScorer, and the effective
score formula — without any modification to existing code.

---

## Verdict as a Filtered Monoid

Verdicts form a monoid under sequential composition (pipeline of gates):

```
verdict₁ ∘ verdict₂ = {
    passed: verdict₁.passed && verdict₂.passed,
    score: min(verdict₁.score, verdict₂.score),
    // other fields merged
}
```

This is a **filtered monoid**: the `passed` field acts as a filter, and once any gate fails
(`passed = false`), the pipeline short-circuits. This is the categorical dual of the Maybe
monad — composition stops on the first failure.

**Practical implication**: Any new Gate implementation that returns a `Verdict` respecting
this monoid structure will compose correctly in a `GatePipeline` without changes to the
pipeline combinator.

---

## Pipeline as Kleisli Composition

The full pipeline involves effects (async I/O, failure, state) and can be modeled as Kleisli
composition in a monad:

```
Pipeline = Substrate.query >=> Router.select >=> Composer.compose >=> Gate.verify >=> Substrate.put >=> Policy.decide
```

Where `>=>` is Kleisli composition in the `Result<T, RokoError>` monad. Each step may fail,
and failure short-circuits the pipeline (like the Verdict monoid, but at the pipeline level).

**Practical implication**: Adding a new pipeline step is safe if the step is a Kleisli arrow
— it takes the previous step's output and returns `Result<NextType, RokoError>`. The monad
laws guarantee that the new step composes correctly with all existing steps.

---

## Functorial Cross-Cuts

Define the cross-cut functors:

```
N : Eng → Eng    (Neuro: enrich with knowledge)
D : Eng → Eng    (Daimon: modulate with affect)
R : Eng → Eng    (Dreams: consolidate with replay)
```

The claim: N, D, and R are endofunctors. This requires:
1. **Identity preservation**: `N(id) = id` (enriching a no-op produces a no-op)
2. **Composition preservation**: `N(f ∘ g) = N(f) ∘ N(g)` (enriching a pipeline = enriching each step)

Both hold because cross-cuts inject additional information without changing the pipeline
structure. The NoOp implementations serve as witnesses for identity preservation.

**Natural transformations** between cross-cuts:
```
η : N → D    (knowledge outcomes update affect)
ε : D → N    (affect biases knowledge retrieval)
```

These form an adjunction if the arbitration protocol correctly resolves conflicts — the
priority hierarchy (Daimon > Neuro > Dreams) establishes the adjunction's unit and counit.
See [06-finding-crosscut-isolation.md](06-finding-crosscut-isolation.md) for the arbitration
protocol gap.

---

## The Design Rule

The categorical analysis yields a concrete design rule:

> Every new trait implementation must:
> 1. Accept and return types that are objects in the Engram category
> 2. Preserve the monoidal structure of Score (no Score that breaks associativity)
> 3. Be implementable as a natural transformation on the pipeline (no hidden side effects)

Any feature that violates these rules (e.g., a Gate that mutates shared state outside the
Substrate) will break composition in ways that may not be immediately obvious but will
manifest as test failures in higher-order composition tests.

---

## Related Findings

- [F1 — Trait Sufficiency](02-finding-trait-sufficiency.md): The six traits are the morphisms.
- [F10 — Cross-Cut Isolation](06-finding-crosscut-isolation.md): The arbitration protocol is
  the formal requirement for functorial commutativity.
- [08 — Novel Proposals](08-novel-proposals.md): Proposal 8.3 uses monoid homomorphisms to
  formalize hierarchical speed composition.

## References

- Milewski, B. (2014). "Category Theory for Programmers." bartoszmilewski.com
- Seemann, M. (2017). "From Design Patterns to Category Theory." blog.ploeh.dk
- Clarke, B. et al. (2020). "Profunctor Optics: A Categorical Update." arXiv:2001.07488
- Gonzalez, G. (2012). "The Functor Design Pattern." haskellforall.com

## Open Questions

- Should the categorical properties be enforced as compile-time tests (property-based tests
  verifying monad laws for the Kleisli pipeline)?
- Is the adjunction between N and D (affect ↔ knowledge) strong enough to warrant a formal
  proof?
