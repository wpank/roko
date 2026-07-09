# Roko Documentation

> The specification, implementation guide, operational playbook, and research
> archive for the Roko cognitive agent runtime.

---

## What This Documentation Is

This tree is a **reference work**, not a tour. Every concept in Roko has exactly one
canonical home. Pages are as long as the concept requires — no longer, no shorter. The
tree is organized so that a first-time reader can enter at any leaf and build context
outward via `Depends on` headers, `See also` footers, and the top-level entry points
below.

The discipline is deliberate:

- **One concept, one page.** Finding any authoritative answer requires opening exactly
  one file, not three.
- **Status is always explicit.** Every page declares whether its subject is
  Shipping, Built, Scaffold, Specified, or Deferred. No prose is ambiguous about what
  exists today.
- **Source-of-truth prose, not templates.** Pages are written from the perspective of
  a reader with no prior context. Every term is defined on first use, every claim
  points at code or a paper.
- **Cross-references over duplication.** Related concepts link; they do not restate
  each other.

---

## Entry Points by Reader

| If you are… | Start here |
|---|---|
| **New to Roko** | [`00-architecture/README.md`](00-architecture/README.md) → [`00-architecture/vision.md`](00-architecture/vision.md) |
| **Learning the core types** | [`00-architecture/concepts/engram.md`](00-architecture/concepts/engram.md) → [`00-architecture/concepts/score.md`](00-architecture/concepts/score.md) → [`00-architecture/concepts/substrate.md`](00-architecture/concepts/substrate.md) |
| **Implementing a new operator** | [`00-architecture/concepts/operators.md`](00-architecture/concepts/operators.md) → [`00-architecture/crate-map.md`](00-architecture/crate-map.md) |
| **Understanding the cognitive model** | [`00-architecture/loop.md`](00-architecture/loop.md) → [`00-architecture/speeds.md`](00-architecture/speeds.md) → [`00-architecture/dual-process.md`](00-architecture/dual-process.md) |
| **Operating Roko in production** | [`operations/configuration.md`](operations/configuration.md) → [`operations/performance.md`](operations/performance.md) → [`operations/error-handling.md`](operations/error-handling.md) |
| **Evaluating Roko (investor / technical buyer)** | [`00-architecture/README.md`](00-architecture/README.md) → [`00-architecture/analysis/readiness-audit.md`](00-architecture/analysis/readiness-audit.md) |
| **Contributing research** | [`00-architecture/perspectives/`](00-architecture/perspectives/) → [`00-architecture/innovations/README.md`](00-architecture/innovations/README.md) |
| **Reviewing the refactor plan** | [`strategy/refactor-phases.md`](strategy/refactor-phases.md) → [`strategy/roadmap.md`](strategy/roadmap.md) |
| **Debugging a failure** | [`operations/error-handling.md`](operations/error-handling.md) |
| **Writing tests** | [`testing/strategy.md`](testing/strategy.md) |

---

## Top-Level Layout

```
new-docs/
  README.md                          ← you are here
  CONVENTIONS.md                     writing rules: templates, status, cross-references
  GLOSSARY.md                        flat A-Z term lookup (authoritative vocabulary)
  ALIASES.md                         public-facing aliases ↔ canonical internal terms

  00-architecture/                   the foundational specification
    README.md                        architecture-at-a-glance + reading order
    vision.md                        thesis: "the scaffold IS the product"
    naming.md                        canonical vocabulary, retired terms, status tags

    concepts/                        the eight core concepts
      engram.md                      durable content-addressed record (with Kind, Body,
                                     ContentHash, HDC fingerprint, compositional kinds)
      pulse.md                       ephemeral event medium (target state)
      score.md                       seven-axis appraisal + arithmetic + constants
      decay.md                       decay variants, demurrage, tier matrix
      provenance.md                  attestation levels, Taint, Custody
      substrate.md                   durable storage fabric + Synapse traits
      bus.md                         target-state transport fabric (Topic, TopicFilter)
      operators.md                   Scorer, Gate, Router, Composer, Policy

    loop.md                          the universal cognitive loop (loop_tick)
    speeds.md                        three cognitive speeds (T0 / T1 / T2)
    dual-process.md                  System-1 / System-2 + active inference
    layers.md                        five-layer dependency taxonomy
    cross-cuts.md                    Neuro, Daimon, Dreams
    design-principles.md             architectural principles
    frontier-summary.md              state of the frontier + open problems
    crate-map.md                     which crate ships which concept

    foundations/                     theoretical anchors
      active-inference.md            Friston free-energy principle applied to Roko
      cybernetics.md                 Ashby, Beer, Conant-Ashby (requisite variety, VSM)
      autocatalysis.md               Kauffman autocatalytic sets applied to memory
      c-factor.md                    Woolley collective-intelligence factor

    perspectives/                    six lens essays on the architecture
      attention-as-currency.md
      immune-system.md
      temporal-topology.md
      emergent-goals.md
      energy-model.md
      collective-intelligence.md

    innovations/                     eight frontier ideas, one per file
      README.md
      hdc-active-inference.md
      code-somatic-markers.md
      stigmergic-bandits.md
      dream-token-economy.md
      knowledge-morphogenesis.md
      witness-world-model.md
      affect-causal-discovery.md
      dream-verification.md

    analysis/                        meta-documentation about the architecture
      architectural-analysis.md      top architectural improvements + refactors
      integration-map.md             cross-section interaction matrix
      readiness-audit.md             implementation readiness scorecard
      synergy-map.md                 interaction-density moat

  strategy/                          forward-looking plans
    refactor-phases.md               phased landing sequence
    roadmap.md                       consolidated Q1–Q4 roadmap
    refinements.md                   open refinement backlog

  operations/                        running Roko in practice
    configuration.md                 config schema + validation
    performance.md                   latency budgets + numerical stability
    error-handling.md                error taxonomy + recovery strategies

  testing/
    strategy.md                      comprehensive test strategy (tier + subsystem)

  _migration/
    section-00.md                    source-to-target map + coverage audit
```

Each non-leaf directory has a `README.md` that indexes its contents with a suggested
reading order and a short paragraph explaining what the folder is for and — just as
importantly — what it is not.

---

## Status Vocabulary

Every concept carries an implementation tier. This is the authoritative definition and
it is enforced everywhere in the tree.

| Tier | Meaning |
|---|---|
| **Shipping** | End-to-end wired, tested, used in the self-hosting workflow. Reachable from the CLI. |
| **Built** | Code exists, compiles, has tests — but not yet called from the runtime or CLI. |
| **Scaffold** | Struct or trait stubs exist. No meaningful implementation behind them. |
| **Specified** | Described in these docs only. No code. |
| **Deferred** | Intentionally postponed (Phase 2+, chain-dependent, or research-only). |

Status always appears in the frontmatter block at the top of every substantive page.
It is never threaded through prose as a `[target-state]` disclaimer.

---

## The Universal Page Template

Substantive pages follow a consistent skeleton. Not every slot is filled on every page,
but the headings are predictable so readers know where to look.

```markdown
# <Concept>

> One-sentence definition.

**Status**: Shipping | Built | Scaffold | Specified | Deferred
**Crate**: `roko-<name>` — or `—` for pure concepts
**Depends on**: [A](path), [B](path)
**Used by**: [C](path), [D](path)
**Last reviewed**: YYYY-MM-DD

## TL;DR
Two to four sentences. What this is, what it does, why it matters.

## The Idea
Concrete motivating prose, with an example. No code yet.

## Specification
Formal definition: types, signatures, invariants. Quote the actual Rust.

## Semantics
What operations mean step-by-step. Pre- and post-conditions.

## Implementation
Current Rust implementation, annotated, with source pointers.

## API Reference
Every public method, every parameter, every return type. Mechanical.

## Invariants
What must always be true; where it is enforced.

## Failure Modes
What can go wrong; how the system recovers; observable signals.

## Performance
Complexity, hot paths, allocation behavior, benchmarks.

## Examples
Worked examples, minimal to complex.

## Edge Cases
Rarely hit behaviors that matter when they do.

## Interactions
How this composes with other concepts. Cross-linked.

## Rationale
Why this design; alternatives considered; what was rejected.

## History
How this concept evolved; retired names; migration notes.

## References
Papers and prior art.

## Open Questions
First-class content. Unresolved design decisions live here, not in spec prose.

## See Also
Three to seven curated links. Never a link dump.
```

See [`CONVENTIONS.md`](CONVENTIONS.md) for the complete writing rules, the folder-index
template, and the integration-page template.

---

## How This Tree Came To Be

This target-state tree is the product of a staged refactor of the legacy
`docs/` tree. Section 00 (architecture) is the foundation and has already been
consolidated here. Subsequent sections (01 Orchestration through 21 References) land
in later refactor passes following the same conventions.

The full source-to-target map and coverage audit for the Section 00 refactor lives in
[`_migration/section-00.md`](_migration/section-00.md). Nothing from the source tree
was discarded — content was consolidated, re-ordered, and in many places expanded.
