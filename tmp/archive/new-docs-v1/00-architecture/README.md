# Architecture

> The foundational specification of Roko: the one durable datum (`Engram`), the one
> ephemeral datum (`Pulse`), the two fabrics that move them (`Substrate` and the
> target-state `Bus`), and the six operators (Scorer, Gate, Router, Composer, Policy, and
> the Substrate/Bus operators) that coordinate the whole system.

**Status**: Shipping (with explicitly marked target-state extensions)
**Last reviewed**: 2026-04-19
**Prerequisites**: None. This folder is where the documentation begins.

---

## What This Folder Is

This is Roko's architecture: what the system *is*, not what it *does*. Every concept in
the runtime — every type, every trait, every composition rule, every layer boundary —
has its canonical home here. If you need to look up what an Engram is, or how the
universal cognitive loop fits together, or why Roko has three cognitive speeds instead
of one, this is the place.

This folder does *not* contain subsystem implementations (orchestrator, composer,
agent dispatch, gates) — those land in their own sections in subsequent refactor
passes. It also does not contain guides or tutorials; it is reference.

---

## Architecture at a Glance

Roko is a Rust toolkit for building cognitive agents that improve through use. Its
architecture is built around:

- **Two mediums.** [`Engram`](concepts/engram.md) is the durable, content-addressed
  record of cognition. [`Pulse`](concepts/pulse.md) is the ephemeral event that flows
  through the system and, on graduation, becomes an Engram.
- **Two fabrics.** [`Substrate`](concepts/substrate.md) is the storage fabric for
  Engrams. [`Bus`](concepts/bus.md) is the target-state transport fabric for Pulses.
  In the current codebase, `EventBus<E>` is the shipping transport; `Bus` is the
  architectural generalisation it will converge on.
- **Six operators.** [`Scorer`](concepts/operators.md), [`Gate`](concepts/operators.md),
  [`Router`](concepts/operators.md), [`Composer`](concepts/operators.md), and
  [`Policy`](concepts/operators.md), plus the
  [`Substrate` and `Bus` operators](concepts/substrate.md) themselves. The Synapse
  traits. Every capability in the system is an implementation of one of these.
- **Five layers.** [Runtime, Framework, Scaffold, Harness, Orchestration](layers.md),
  with strictly downward dependencies.
- **Three speeds.** [T0 Reflex, T1 Habit, T2 Deliberate](speeds.md) — the tier
  routing that mirrors the dual-process distinction from cognitive science.
- **Three cross-cuts.** [Neuro, Daimon, Dreams](cross-cuts.md) — cognitive
  subsystems injected across layers rather than pinned to any single one.
- **One loop.** [The universal cognitive loop](loop.md) — nine steps that every
  agent runs, scaled along the three speeds.

Everything in this folder is an elaboration of those pieces.

---

## Reading Order

There is no single correct path through this folder — it is reference, and different
readers need different entry points. The paths below are suggestions.

### Path A: The thesis first

For readers who want to understand why Roko exists before diving into how it works.

1. [`vision.md`](vision.md) — the scaffold thesis, empirical evidence, and the
   Synapse Architecture at a high level.
2. [`naming.md`](naming.md) — the canonical vocabulary you will meet everywhere else.
3. [`concepts/engram.md`](concepts/engram.md) — the core datum.
4. [`loop.md`](loop.md) — the nine-step cognitive loop.
5. [`layers.md`](layers.md) — the five architectural layers.

### Path B: The datatypes first

For readers who want to understand what moves through the system.

1. [`concepts/engram.md`](concepts/engram.md) — durable content-addressed records.
2. [`concepts/pulse.md`](concepts/pulse.md) — ephemeral events.
3. [`concepts/score.md`](concepts/score.md) — the seven-axis appraisal vector.
4. [`concepts/decay.md`](concepts/decay.md) — how records fade, freeze, and thaw.
5. [`concepts/provenance.md`](concepts/provenance.md) — attestation and audit.

### Path C: The operators first

For readers building or modifying Roko behaviour.

1. [`concepts/substrate.md`](concepts/substrate.md) — the storage fabric and the Synapse traits.
2. [`concepts/bus.md`](concepts/bus.md) — the transport fabric.
3. [`concepts/operators.md`](concepts/operators.md) — Scorer, Gate, Router, Composer, Policy.
4. [`crate-map.md`](crate-map.md) — which crate ships which concept.
5. [`design-principles.md`](design-principles.md) — the seven guiding principles.

### Path D: The cognition first

For readers interested in the cognitive-science grounding.

1. [`loop.md`](loop.md) — the nine-step universal cognitive loop.
2. [`speeds.md`](speeds.md) — three cognitive speeds.
3. [`dual-process.md`](dual-process.md) — System-1 / System-2 and active inference.
4. [`cross-cuts.md`](cross-cuts.md) — Neuro / Daimon / Dreams.
5. [`foundations/`](foundations/) — active inference, cybernetics, autocatalysis,
   collective intelligence.

### Path E: The frontier first

For readers evaluating the research contribution.

1. [`perspectives/`](perspectives/) — six lens essays on the architecture.
2. [`innovations/`](innovations/) — eight frontier ideas, one per file.
3. [`analysis/architectural-analysis.md`](analysis/architectural-analysis.md) — audits and improvements.
4. [`analysis/integration-map.md`](analysis/integration-map.md) — cross-section interaction matrix.
5. [`analysis/readiness-audit.md`](analysis/readiness-audit.md) — implementation status scorecard.

---

## Contents

### Top-level

| Page | What it covers | Status |
|---|---|---|
| [vision.md](vision.md) | The scaffold thesis, empirical evidence, CoALA/active-inference/autocatalytic/cybernetic foundations, Synapse Architecture overview, domain-agnosticism, self-hosting proof, seven design principles, current gaps | Shipping |
| [naming.md](naming.md) | Canonical A–Z vocabulary, retired terms, public aliases, capitalisation rules | Shipping |
| [loop.md](loop.md) | The universal cognitive loop — the nine-step procedure every agent runs | Shipping |
| [speeds.md](speeds.md) | Three cognitive speeds (T0 Reflex, T1 Habit, T2 Deliberate), tier routing, adaptive clock | Shipping |
| [dual-process.md](dual-process.md) | System-1 / System-2 mapping onto tier routing, EFE, prediction-error drive | Shipping |
| [layers.md](layers.md) | The five-layer taxonomy (Runtime, Framework, Scaffold, Harness, Orchestration), dependency rules, VSM mapping | Shipping |
| [cross-cuts.md](cross-cuts.md) | Neuro (knowledge), Daimon (motivation), Dreams (offline learning) | Shipping + Scaffold |
| [design-principles.md](design-principles.md) | Seven architectural principles (P1–P7) | Shipping |
| [frontier-summary.md](frontier-summary.md) | State of the frontier and open problems | Shipping |
| [crate-map.md](crate-map.md) | Which crate ships which concept | Shipping |

### [`concepts/`](concepts/) — the eight core concepts

| Page | What it covers | Status |
|---|---|---|
| [engram.md](concepts/engram.md) | Durable content-addressed record: fields, `Kind`, `Body`, `ContentHash`, HDC fingerprint, lineage DAG, builder, compositional kinds, tier matrix | Shipping (as `Signal` today) |
| [pulse.md](concepts/pulse.md) | Ephemeral event medium, graduation rules, Substrate↔Bus boundary | Specified |
| [score.md](concepts/score.md) | Seven-axis appraisal (4 stable + 3 extended), effective formula, arithmetic, constants | Shipping (4 axes) / Specified (3 extended) |
| [decay.md](concepts/decay.md) | Decay variants, demurrage, reinforcement, cold-tier freeze/thaw, tier matrix | Shipping |
| [provenance.md](concepts/provenance.md) | Provenance record, one-way Taint, Custody, attestation levels | Specified |
| [substrate.md](concepts/substrate.md) | The storage fabric: trait surface, fingerprint population, similarity query, backends, concurrency, pruning | Shipping |
| [bus.md](concepts/bus.md) | The transport fabric: Bus trait, Topic/TopicFilter, replay/ring semantics, backend families | Specified |
| [operators.md](concepts/operators.md) | Scorer, Gate, Router, Composer, Policy — specs for the five non-Substrate operators | Shipping |

### [`foundations/`](foundations/) — theoretical anchors

| Page | What it covers |
|---|---|
| [active-inference.md](foundations/active-inference.md) | Friston's free-energy principle, EFE, prediction error, predictive coding |
| [cybernetics.md](foundations/cybernetics.md) | Ashby's law of requisite variety, Conant-Ashby good regulator, Beer's VSM |
| [autocatalysis.md](foundations/autocatalysis.md) | Kauffman autocatalytic sets, compound improvement math |
| [c-factor.md](foundations/c-factor.md) | Woolley collective intelligence factor |

### [`perspectives/`](perspectives/) — six lens essays

| Page | What it covers |
|---|---|
| [attention-as-currency.md](perspectives/attention-as-currency.md) | Attention allocation modelled as an internal economy |
| [immune-system.md](perspectives/immune-system.md) | Gates, quarantine, and adaptive thresholds as an immune system |
| [temporal-topology.md](perspectives/temporal-topology.md) | Knowledge aging, decay topology, and temporal consolidation |
| [emergent-goals.md](perspectives/emergent-goals.md) | Goal structures emerging from scoring and policy interaction |
| [energy-model.md](perspectives/energy-model.md) | A thermodynamic reading of budgets, tiers, and attention |
| [collective-intelligence.md](perspectives/collective-intelligence.md) | c-factor, multi-agent mesh, stigmergic coordination |

### [`innovations/`](innovations/) — eight frontier ideas

See [`innovations/README.md`](innovations/README.md) for the index. Each file is a
self-contained research sketch: HDC × active inference, code as somatic markers,
stigmergic bandits, dream token economy, knowledge morphogenesis, witness world model,
affect × causal discovery, dream verification.

### [`analysis/`](analysis/) — meta-documentation

| Page | What it covers |
|---|---|
| [architectural-analysis.md](analysis/architectural-analysis.md) | Top architectural improvements and refactors, with rationale |
| [integration-map.md](analysis/integration-map.md) | Cross-section interaction matrix: what flows where |
| [readiness-audit.md](analysis/readiness-audit.md) | Per-component implementation-readiness scorecard |
| [synergy-map.md](analysis/synergy-map.md) | Interaction-density moat — why N components do not equal N capabilities |

---

## Related Sections

- [`../strategy/refactor-phases.md`](../strategy/refactor-phases.md) — how the concepts in
  this folder land as code, in phases.
- [`../strategy/roadmap.md`](../strategy/roadmap.md) — consolidated Q1–Q4 plan.
- [`../operations/`](../operations/) — configuration, performance, and error handling for
  the running system.
- [`../testing/strategy.md`](../testing/strategy.md) — the test strategy built on top of
  these concepts.
- [`../_migration/section-00.md`](../_migration/section-00.md) — what moved where from
  the legacy `docs/00-architecture/` tree.

---

## Key Academic Foundations

Roko is grounded in a specific literature. Papers are cited on the pages where they
apply most directly; this is the master list.

- Sumers et al. 2023 — [CoALA: Cognitive Architectures for Language Agents](https://arxiv.org/abs/2309.02427).
- Friston 2010 — The Free Energy Principle, Nature Reviews Neuroscience 11(2).
- Clark 2013 — Predictive Processing, Behavioral and Brain Sciences 36(3).
- de Vries et al. 2025 — [EFE as variational inference](https://arxiv.org/abs/2504.14898).
- Kauffman 1993 — *The Origins of Order*, Oxford University Press.
- Conant & Ashby 1970 — Good Regulator Theorem, IJSS 1(2).
- Ashby 1956 — *An Introduction to Cybernetics*.
- Beer 1972 — *Brain of the Firm* (Viable System Model).
- Kanerva 2009 — Hyperdimensional Computing, Cognitive Computation 1(2).
- Woolley et al. 2010 — Collective Intelligence Factor, Science 330(6004).
- Vovk et al. 2005 — *Algorithmic Learning in a Random World*.
- Lee et al. 2026 — [Meta-Harness](https://arxiv.org/abs/2603.28052).
- Chen et al. 2023 — [FrugalGPT](https://arxiv.org/abs/2305.05176).
- Khattab et al. 2024 — DSPy.
- Jimenez et al. 2024 — SWE-bench Verified.
- Li et al. 2022 — AlphaCode, DeepMind.
- Zaharia et al. 2024 — Compound AI Systems, BAIR.
- Mehrabian & Russell 1974 — PAD emotional model.
- Damasio 1994 — *Descartes' Error* (somatic marker hypothesis).
- Mattar & Daw 2018 — Prioritized replay, Nature Neuroscience 21.
- Lacaux et al. 2021 — Hypnagogia and creativity, Science Advances 7(50).
- Pearl 2009 — *Causality*.
- Hu et al. 2025 — ADAS: meta-agent architecture search, ICLR.
- Grassé 1959 — Stigmergy.
- Dorigo et al. 2000 — Ant colony optimization.

Each citation is linked from the pages where it does work; this list is the
consolidated index.
