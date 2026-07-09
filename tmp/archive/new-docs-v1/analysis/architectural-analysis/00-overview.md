# Architectural Coherence Analysis — Overview

> A comprehensive evaluation of Roko's Synapse Architecture against modern research in
> cognitive architectures, trait-based systems, category theory, and active inference.

**Status**: Analysis (informing future architectural decisions)
**Date**: 2026-04-12
**Crate**: —
**Depends on**: [Six Synapse Traits](../../reference/05-operators/README.md), [Five-Layer Taxonomy](../../reference/08-layers/README.md)
**Last reviewed**: 2026-04-13

---

## TL;DR

Roko's Synapse Architecture is **remarkably coherent**. The "one noun, six verbs" model holds
up under scrutiny. The architecture has one confirmed dependency violation, six unclassified
crates, and several documentation inconsistencies — all fixable. Its deepest strength is
**categorical composability**: the pipeline is a morphism composition, Score is a monoid, and
cross-cuts are endofunctors. These are structural guarantees, not metaphors.

---

## Scope

This analysis covers:

- All 24 architecture documents in `docs/00-architecture/`
- All 22 section INDEX files across the full `docs/` tree
- STATUS.md, QUICKSTART.md, COMPARISON.md
- Actual Cargo.toml dependency graphs across all 28 crates
- Trait definitions in `roko-core/src/traits.rs`
- All 131 trait implementations found in the codebase

---

## Methodology

**Step 1: Trait audit** — Searched the codebase for all implementations of the six Synapse
traits. Counted 131 total (Substrate 4, Scorer 7, Gate 33, Router 7, Composer 5, Policy 4).
Also searched for TODO/HACK/FIXME markers near trait usage to find awkward boundaries.

**Step 2: Layer audit** — Ran full Cargo.toml analysis across all 28 crates, tracing each
dependency to its target crate's layer assignment. Checked for upward dependencies.

**Step 3: Domain mapping** — For each of the three cognitive domains (coding, chain, research),
mapped which operations occur at Gamma, Theta, and Delta speed.

**Step 4: Edge case census** — Enumerated all data categories that the Engram type must
represent, including binary blobs, structured multi-part data, real-time streams, and
confidential data.

**Step 5: Category theory grounding** — Applied formal definitions of morphisms, monoids,
functors, and Kleisli composition to the pipeline structure.

**Step 6: Literature survey** — Compared findings against recent publications in cognitive
architectures (CoALA, LIDA, SOAR, ACT-R, SOFAI), active inference (Friston, VERSES Genius),
agent data protocols (ADP, arXiv:2510.24702), and hyperdimensional computing.

---

## Key Findings (Summary)

1. **Six traits are sufficient.** No 7th trait is needed. Boundary operations fit as degenerate
   Composer and Policy. See [02-finding-trait-sufficiency.md](02-finding-trait-sufficiency.md).

2. **One dependency violation exists.** `roko-conductor` → `roko-learn` breaks the L3→L2 rule.
   Fixable with a `HealthMetrics` trait in L0. See [03-finding-layer-taxonomy.md](03-finding-layer-taxonomy.md).

3. **Six crates unclassified.** `roko-neuro`, `roko-daimon`, `roko-dreams`, `roko-golem`,
   `roko-chain`, `roko-plugin` need formal layer assignment.
   See [03-finding-layer-taxonomy.md](03-finding-layer-taxonomy.md).

4. **Three cognitive speeds are domain-agnostic and complete.** All three domains map cleanly.
   Delta speed is a genuine innovation with no classical architecture equivalent.
   See [04-finding-cognitive-speeds.md](04-finding-cognitive-speeds.md).

5. **Engram/Signal is universal.** Edge cases are handled by existing extension mechanisms.
   Engram is strictly richer than the Agent Data Protocol (ADP).
   See [05-finding-engram-universality.md](05-finding-engram-universality.md).

6. **Cross-cut isolation has two gaps.** Daimon is not injected via trait object; Dreams
   imports Neuro and Learn directly. Both fixable.
   See [06-finding-crosscut-isolation.md](06-finding-crosscut-isolation.md).

7. **Category theory provides formal composability guarantees.** The pipeline is Kleisli
   composition; Score is a commutative monoid; Verdict is a filtered monoid; cross-cuts are
   endofunctors. See [07-finding-category-theory.md](07-finding-category-theory.md).

8. **Documentation has five inconsistencies.** `roko-fs` layer assignment is wrong; Score
   documentation promises 7 axes but ships 4; two other mismatches.
   See [09-finding-inconsistencies.md](09-finding-inconsistencies.md).

---

## Overall Verdict

The architecture is sound and theoretically well-grounded. The most impactful improvement is
**gradient gate feedback** (see [08-novel-proposals.md](08-novel-proposals.md)): connecting
the existing Gate pipeline to active inference's prediction-error minimization framework enables
continuous learning from every verification attempt rather than binary pass/fail signals.

The most urgent fixes are the dependency violation and the documentation inconsistencies, both
of which are small. The most important enhancement is gradient gate feedback. The most
intellectually interesting finding is the category theory grounding in
[07-finding-category-theory.md](07-finding-category-theory.md), which shows that the
architecture's composability is a structural property, not an accident.

---

## References

Full bibliography in [10-prioritized-improvements.md](10-prioritized-improvements.md).

Key papers:
- Sumers et al. (2023). "CoALA." arXiv:2309.02427
- Franklin et al. (2016). "LIDA: A Systems-level Architecture." IEEE Trans. AMD 6(1)
- Friston (2010). "The free-energy principle." Nature Reviews Neuroscience 11(2)
- Kleyko et al. (2022). "A Survey on Hyperdimensional Computing." AI Review 56
- Phan-Ba et al. (2025). "Agent Data Protocol (ADP)." arXiv:2510.24702
- McClelland et al. (1995). "Complementary Learning Systems." Psychological Review 102(3)

## Open Questions

- Should the architectural analysis be re-run after the Signal→Engram rename lands?
- Does the category theory analysis change if the Pipeline struct is formalized per Proposal 8.3?
