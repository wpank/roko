# Finding: Three Cognitive Speeds

> Domain mapping completeness, comparison to classical architectures, speed interaction model,
> and the case for Delta speed as a genuine innovation.

**Status**: Analysis
**Crate**: `roko-core` (loop), `roko-dreams` (Delta)
**Depends on**: [Three Cognitive Speeds](../../reference/07-speeds/README.md), [Universal Loop](../../reference/06-loop/README.md)
**Last reviewed**: 2026-04-13

---

## TL;DR

The three cognitive speeds (Gamma/Theta/Delta) map cleanly to all four operational domains
(coding, chain, research, orchestration). The speeds are domain-agnostic because they are
defined in terms of the universal cognitive loop, not domain-specific operations. Delta speed
(offline consolidation via Dreams) is a genuine architectural innovation with no direct analog
in established cognitive architectures.

---

## Domain Mapping Completeness

| Domain | Gamma (~5-15s) | Theta (~75s) | Delta (~hours) | Clean? |
|---|---|---|---|---|
| **Coding** | Compile check, quick fix, cached lookup | Summarize progress, check predictions, update PAD | Dreams replay of failed compilations, knowledge promotion | **Yes** |
| **Chain** | Gas check, balance check, price lookup | Portfolio assessment, hedging check, prediction calibration | MEV incident analysis, strategy consolidation | **Yes** |
| **Research** | Citation lookup, fact check | Research direction assessment, contradiction detection | Cross-domain hypothesis generation, literature synthesis | **Yes** |
| **Orchestration** | Task status check, agent health probe | Plan progress summary, re-planning assessment | Full plan retrospective, skill library update | **Yes** |

All domains map cleanly. The key insight: the three speeds are **domain-agnostic** because they
are defined in terms of the universal cognitive loop, not domain-specific operations. Any
operation that can be expressed as `query → score → route → compose → act → verify → persist → react`
can run at any of the three speeds.

---

## Comparison to Classical Architectures

| Architecture | Number of Speeds | Roko Equivalent |
|---|---|---|
| **SOAR** | 1 (~50ms decision cycle) | Roughly Gamma, with impasses escalating to deeper reasoning |
| **ACT-R** | 1 (~50ms production fire) | Roughly Gamma; no explicit reflective or consolidation speed |
| **LIDA** | 1 (~260-390ms cognitive cycle) | Roughly Gamma; deliberation is a subphase, not a separate speed |
| **SOFAI** | 2 (Fast/Slow) | Fast ≈ Gamma, Slow ≈ Theta; no Delta equivalent |
| **Roko** | 3 (Gamma/Theta/Delta) | Extends dual-process with offline consolidation |

Roko's three speeds are a genuine architectural innovation. The Delta speed (offline
consolidation via Dreams) has no direct analog in established cognitive architectures.
It is inspired by sleep neuroscience (McClelland et al. 1995, CLS theory) rather than
cognitive architecture tradition.

---

## Speed Interaction Model

The three speeds are not independent — they interact through the cross-cuts:

```
Gamma ticks produce episodes → stored in Substrate
    │
    ├── Theta reads recent episodes → summarizes → updates Daimon PAD
    │       │
    │       └── PAD changes may trigger speed escalation or consolidation
    │
    └── Delta reads accumulated episodes → Dreams replay → Neuro promotion
            │
            └── Promoted knowledge available to next Gamma tick
```

This is a **hierarchical prediction error cascade**: Gamma handles immediate surprises, Theta
handles accumulated pattern changes, and Delta handles deep structural learning. Each speed's
output feeds the next speed's input, creating the autocatalytic loop described in
`16-autocatalytic-and-cybernetics.md`.

The formal claim: if `TickOutcome` forms a monoid (under concatenation of written hashes),
then `fold_outcomes` is a monoid homomorphism. Theta's processing of Gamma outcomes is
well-defined regardless of how many Gamma ticks produced how many outcomes. See
[07-finding-category-theory.md](07-finding-category-theory.md) for the full categorical treatment.

---

## Related Findings

- [F7 — Engram Universality](05-finding-engram-universality.md): The universal Engram type is
  what makes the same data structure work across all three speeds.
- [08 — Novel Proposals](08-novel-proposals.md): Proposal 8.3 formalizes hierarchical pipeline
  composition as monoid homomorphisms across speeds.
- [Integration Map: learning×dreams](../integration-map/learning-x-dreams.md): The Gamma→Delta
  feedback path is partially wired (episodes written) but lacks Dreams' catch-up mechanism.

## References

- Kahneman, D. (2011). "Thinking, Fast and Slow." Farrar, Straus and Giroux.
- Sun, R. (2002). "Duality of the Mind." (CLARION architecture)
- Fabiano, F. et al. (2025). "SOFAI: A multi-component cognitive architecture." npj Artificial Intelligence.
- Laird, J. E. (2012). "The Soar Cognitive Architecture." MIT Press.
- Franklin, S. et al. (2016). "LIDA." IEEE Trans. AMD 6(1).
- McClelland, J. et al. (1995). "Complementary Learning Systems." Psychological Review 102(3).

## Open Questions

- What is the correct Theta interval for the Research domain? Is 75s still appropriate for
  operations with multi-minute wall-clock costs?
- Should the speed interaction model be expressed as a formal diagram in the reference docs?
