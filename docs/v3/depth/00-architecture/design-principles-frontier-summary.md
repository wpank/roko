# 00-ARCH -- Design Principles and Frontier Summary

> **Parent**: [00-ARCHITECTURE](../../00-ARCHITECTURE.md)
>
> Roko is guided by seven design principles (P1-P7) that constrain architectural
> decisions and prevent feature drift. This depth file preserves the principles with
> their theoretical foundations, catalogs the net-new innovations (primitive vs.
> composed) with current implementation status as of 2026-09-15, and documents the
> anti-patterns explicitly rejected.

---

## 1. The Seven Design Principles

### P1: Composition Over Configuration

**Statement**: Every capability is a trait implementation. New behaviors emerge from
composing existing traits, not from configuration flags or feature gates.

**Rationale**: Configuration systems grow unboundedly -- a dozen flags become a
hundred, interactions between flags become untestable, and users cannot reason about
behavior. Trait composition is bounded by the type system: if it compiles, the
composition is valid.

**Theoretical basis**: Parnas (1972, CACM 15(12)) established that modules should hide
design decisions likely to change. Each of the 12 protocol traits hides exactly one
design decision: `Scorer` hides the scoring algorithm, `Gate` hides the verification
mechanism, `Router` hides the selection algorithm, `Composer` hides the assembly
strategy. Ousterhout (2018, "A Philosophy of Software Design", Yaknyam Press) defines
module depth = benefit / interface cost. Each trait has a 1-3 method interface hiding
arbitrarily complex implementations -- maximally deep design.

**Application**: A new verification strategy is a new `Gate` implementation, not a
`verification_strategy` config key. Domain-specific behavior is a trait implementation
plugged into the universal loop.

**Anti-pattern**: `if config.mode == "chain" { ... } else if config.mode == "coding" { ... }`.

### P2: Verify Everything

**Statement**: Every agent output passes through the Gate pipeline before being
persisted or acted upon. No unverified Signals enter the audit DAG as trusted.

**Rationale**: LLMs hallucinate. Tools fail silently. External data sources lie.
Without systematic verification, errors compound through the lineage DAG. The Gate
pipeline is the firewall between "the agent produced something" and "the system
trusts it." This addresses the Hallucination Amplification anti-pattern from the MAST
taxonomy (Cemri et al. 2025, arXiv:2503.13657): 41.8% of multi-agent failures stem
from specification issues where unverified outputs cascade.

**Application**: 19 gates in a 7-rung pipeline. Code passes through compile -> test ->
clippy -> diff gates. Knowledge claims pass through confidence threshold -> source
verification -> consistency check.

**Escape hatch**: Signals can be marked with low confidence and stored as Transient
tier knowledge without full verification, allowing speculative hypotheses to exist in
the system while clearly flagged as unverified.

### P3: Budget-Aware by Default

**Statement**: Every operation has a budget (tokens, time, cost, signals). The system
degrades gracefully when budgets are exhausted, never crashes or produces unbounded
output.

**Application**: `Composer.compose()` takes a `Budget { max_tokens, max_signals,
max_bytes, max_wall_ms }`. `CascadeRouter` selects the cheapest model tier sufficient
for the current prediction error (cf. Chen et al. 2023, arXiv:2305.05176 --
FrugalGPT: up to 98% cost reduction via cascade). The Graph engine enforces
paid-failure-aware cost budgets with atomic reservations. E36 (Payments, 8/8) adds
x402 batching, MPP sessions, and reputation pricing.

### P4: Content-Addressed Everything

**Statement**: Every Signal has a BLAKE3 content hash computed from its identity
fields (kind, body, author, tainted, lineage, tags). The hash is the Signal's
identity. Two Signals with identical content have identical hashes.

**Properties**:
1. **Deduplication**: Identical content is automatically deduplicated
2. **Integrity verification**: Any modification changes the hash, making tampering
   detectable
3. **Lineage verification**: The audit DAG is a hash chain -- replaying the lineage
   verifies that no intermediate Signal was modified

This is the foundation of Forensic AI -- causal replay of agent decisions with
cryptographic verification. `roko replay <hash>` walks the DAG backward from any
Signal to its root inputs.

### P5: Decay as Feature, Not Bug

**Statement**: Signals decay by default. Information that is not reinforced fades.
This is a feature that prevents knowledge hoarding and ensures the system's working
memory stays relevant.

**Rationale**: Without decay, the Substrate grows unboundedly. Old, irrelevant Signals
dilute search results. Biological memory systems (Ebbinghaus 1885) demonstrate that
forgetting is essential for efficient recall.

**Application**: Demurrage-weighted retrieval (E24 10/10, E25 10/10). Ebbinghaus decay
curves with configurable strength and scale. Five reinforcement signals: citation,
retrieval, gating, surprise, and agent quotation. Cold-tier archival for signals below
minimum balance.

### P6: Self-Model Is Mandatory

**Statement**: Every agent maintains a self-model (the Daimon PAD vector) that tracks
its own cognitive state. This self-model modulates behavior -- it is not cosmetic.

**Rationale**: Good Regulator Theorem (Conant & Ashby 1970, IJSS 1(2)): every good
regulator must contain a model of the system it regulates. An agent without
self-awareness cannot adaptively allocate cognitive resources, cannot detect when it is
struggling, and cannot decide when to escalate to a stronger model.

**Application**: E23 (Agent Cognitive Autonomy, 10/10) adds lifecycle type-state,
behavioral vitality, CorticalState energy fields, energy accounting, adaptive
timescales, energy/affect coupling, EFE routing, GoalTree, SlotManager, revisioned
mode owners, and phase-aware runner dispatch. Somatic markers (positive/negative
valence from past experiences) bias Router selection before analytical reasoning
engages (Damasio 1994, Descartes' Error).

### P7: Observable by Default

**Statement**: Every Signal, trait invocation, gate verdict, and tier selection is
observable. The system is transparent to its operators at every level of abstraction.

**Application**: `roko replay <hash>` walks the lineage DAG. Gate verdicts include
gate name, score, reason, test counts, and error digest. CascadeRouter logs every
tier selection decision with confidence and cost. E33 (Telemetry Lens, 9/9, 39/39
ingress variants) provides all 11 built-in Lens executors, bounded queued delivery,
breaker controls, typed StateHub aggregation, REST/SSE, restart-durable projection
history, resolution queries, and configurable 7-day time retention.

---

## 2. Theoretical Foundations

### 2.1 Information Hiding (Parnas 1972)

Parnas, D. L. (1972). "On the Criteria to Be Used in Decomposing Systems into
Modules." Communications of the ACM 15(12):1053-1058.

Each trait passes the Parnas test: the interface reveals as little as possible about
the implementation. The danger is "interface leakage" -- when a shared representation
exposes internal details through the trait interface, coupling all traits through one
overloaded representation.

### 2.2 Module Depth (Ousterhout 2018)

Ousterhout, J. (2018). "A Philosophy of Software Design." Yaknyam Press.

Module depth = benefit / interface cost. Deep modules have large implementations
behind small interfaces. Each protocol trait has a 1-3 method interface hiding
arbitrarily complex implementations.

### 2.3 Clean Architecture / Hexagonal Architecture

Martin, R. C. (2017). "Clean Architecture." Prentice Hall.
Cockburn, A. (2005). "Hexagonal Architecture." Technical Report.

Dependencies point inward. Domain types (Signal, Score, Kind) are in the center.
Infrastructure (Substrate backends, LLM APIs) is on the outside.

- **Ports** = Protocol traits (Store, Score, Verify, Route, Compose, React, ...)
- **Adapters** = Concrete implementations (FileSubstrate, CompileGate, CascadeRouter)

Multiple adapters per port is the norm: MemorySubstrate for testing, FileSubstrate
for production, ChainSubstrate for on-chain state.

### 2.4 Algebraic Effects

Plotkin, G. & Power, J. (2001). "Adequacy for Algebraic Effects." FoSSaCS, LNCS 2030.
Bauer, A. & Pretnar, M. (2015). "Programming with Algebraic Effects." JLAMP 84(1).

The 12-trait system approximates a 12-effect system constrained by Rust's lack of
native effect polymorphism. Handlers can be stacked: a caching handler wraps a
neural-model handler, enabling compositional interpretation without trait inheritance.

### 2.5 Functional Core / Imperative Shell (Bernhardt 2012)

Bernhardt, G. (2012). "Boundaries." SCNA Conference.

- **Functional core** (pure): Scorer, Router, Composer, Policy -- operate on immutable
  inputs, return new values. No I/O, no state mutation.
- **Imperative shell** (effectful): Substrate (persistence), Agent dispatch (LLM
  calls) -- the only impure operations.

This creates a clean testing boundary: functional core traits can be tested with pure
inputs/outputs and no mocking. Imperative shell traits require integration testing.

---

## 3. Anti-Patterns: Designs Explicitly Rejected

Based on the MAST taxonomy (Cemri et al. 2025, arXiv:2503.13657) and empirical
agent failure analysis (OWASP ASI08 2026).

| Anti-Pattern | MAST Finding | Roko's Design |
|---|---|---|
| **God Agent** | 41.8% of failures from specification issues; role ambiguity is the leading cause | Explicit roles via `RoleSystemPromptSpec` (11 templates). Each role has defined responsibilities AND exclusions |
| **Hallucination Amplification** | Chain-of-thought amplification (OWASP ASI08) | P2: Gate pipeline on every output. Lineage DAG traces every Signal to its verified sources |
| **Unbounded Context Sharing** | FM-1.4: context window overflow from shared growing context | P3: Budget with VCG attention auction. `Budget { max_tokens, max_signals, max_bytes, max_wall_ms }` |
| **Implicit Termination** | FM-1.5: 8.2% premature or failed termination | Machine-readable completion criteria + budget hard stops (`warn_threshold`, `block_threshold`) |
| **Self-Verification** | FM-3.3: 9.1% incorrect verification -- agents rationalize their own output | Gates structurally separate from producing agent. `CompileGate` runs external subprocess. `JudgeGate` uses separate LLM |
| **Configuration Over Composition** | -- | P1: Trait implementations, not config flags |
| **Verification as Afterthought** | -- | P2: Verification is a load-bearing loop phase, not an appendable afterthought |

---

## 4. Frontier Innovations

### 4.1 Seven Primitive Innovations

These are the net-new architectural primitives -- proposed or implemented invariants
that are genuinely novel in the agent systems landscape.

| # | Innovation | What is net-new | Closest prior art | Status (2026-09-15) |
|---|---|---|---|---|
| 1 | **Signal / Pulse / Bus / Substrate** | Clean separation of durable record, ephemeral transport, transport fabric, storage fabric | Event sourcing, actor systems, hexagonal arch | **Shipping** |
| 2 | **HDC fingerprint** | 10,240-bit deterministic fingerprint for similarity, clustering, consensus, analogy | HDC (Kanerva 2009), sparse distributed memory | **Shipping** -- per-episode fingerprints wired |
| 3 | **Demurrage** | Continuous holding cost on idle Signals with reinforcement on use | Gesell demurrage, Ebbinghaus forgetting | **Shipping** -- E24 balance/reinforcement wired |
| 4 | **Heuristics / falsifiers commons** | Heuristics with explicit falsifiers, recalibration, exportable calibration history | Scientific method, prediction markets | **Shipping** -- E25 when/then playbooks, significance, early stopping |
| 5 | **C-factor** | Cohort-process scalar learned from Bus/Substrate evidence | Woolley et al. 2010, team-process metrics | **Partial** -- CFactorSummary wired, full Bus measurement target-state |
| 6 | **Replication ledger** | Durable record of replications, failures, and outcome conditions | Provenance graphs, audit logs | **Partial** -- content-addressed DAG exists |
| 7 | **Plugin SPI** | Stable plugin service provider interface | Rust trait plugins, ports/adapters | **8/8** -- E32 signed dependency graphs, bounded WASM hooks, strict admission |

### 4.2 Eleven Composed Innovations

These are higher-level capabilities composed from the primitives plus prior art. Their
novelty is not that each subpiece is unprecedented in isolation; their novelty is that
the same architecture can wire them together.

| # | Innovation | Built from | Status (2026-09-15) |
|---|---|---|---|
| 1 | 16 T0 Probes | Probe registry + router + budgets + prediction-error aggregation | Specified, partially wired |
| 2 | VCG Attention Auction | Budgeting + competing subsystems + truthful allocation | **Wired** -- LearningBidder, CompositionStrategy::Auto |
| 3 | Somatic Landscape | PAD/self-model + HDC neighborhood search + strategy memory | **Wired** -- DaimonFunctor queries 8-dimensional strategy coordinates |
| 4 | Hypnagogia Engine | HDC recall + episodic recombination + guarded partial completions | Scaffolded in roko-dreams |
| 5 | Predictive Foraging | Heuristics + falsifiers + retrieval prediction + stopping rules | Partially wired |
| 6 | Collective Calibration Loop | c-factor + replication ledger + verified outcomes | CFactorSummary wired; full loop target-state |
| 7 | Forensic AI | Signal lineage + Bus history + Gate verdicts + replay tooling | Content-addressed DAG + `roko replay` exist |
| 8 | EvoSkills / ADAS | Plugin SPI + gates + replication ledger + trait search | R04 scoped lifecycle complete; full ADAS target-state |
| 9 | Cross-Domain Insight Resonance | HDC fingerprint + heuristic commons + diverse evidence | HDC vectors built; cross-domain detection wiring partial |
| 10 | Generative Interfaces (A2UI) | Plugin SPI + design system + runtime descriptions | Named surfaces (E37 9/9) wired; full A2UI target-state |
| 11 | Knowledge Futures Market | Replication ledger + plugin SPI + chain settlement | Deferred (chain dependency) |

---

## 5. Innovation Interconnection Map

The moat is the composition across the seven primitives. Each reinforces the others:

| Primitive | Enables | Moat effect |
|---|---|---|
| Signal/Pulse/Bus/Substrate | Forensic AI, live coordination | Auditable without freezing runtime traffic |
| HDC fingerprint | Somatic Landscape, Hypnagogia, Resonance | Similarity as shared computation surface |
| Demurrage | Predictive Foraging, memory hygiene | Durable medium stays live, not dead weight |
| Heuristics/falsifiers | Collective calibration, VCG, EvoSkills | Learned rules stay falsifiable and exportable |
| C-factor | Cohort routing, team diagnostics | Group process measurable and transportable |
| Replication ledger | Evidence inheritance, failure replay | Deployments start from accumulated evidence |
| Plugin SPI | Domain extensions, ADAS search | Platform open without giving up architectural control |

The structural lesson: competitors can copy a feature surface, but they do not
automatically inherit the linked primitives that make the surface self-reinforcing.
P1 keeps the system compositional, P2 and P7 make it auditable, P3 keeps accumulation
bounded, P4 and P5 keep memory durable but selective, and P6 keeps the policy layer
able to model itself well enough to exploit the evidence the loop returns.

---

## Academic Foundations

| Citation | Contribution |
|---|---|
| Parnas 1972, CACM 15(12) | Information hiding: modules hide design decisions likely to change |
| Ousterhout 2018, Yaknyam Press | Module depth: simple interfaces, powerful implementations |
| Martin 2017, Prentice Hall | Clean Architecture: dependencies point inward |
| Cockburn 2005 | Hexagonal Architecture: ports (traits) and adapters (impls) |
| Plotkin & Power 2001, FoSSaCS, LNCS 2030 | Algebraic effects: adequacy for effect algebras |
| Bauer & Pretnar 2015, JLAMP 84(1) | Programming with algebraic effects and handlers |
| Leijen 2017, POPL | Row-typed algebraic effects: efficient compilation |
| Bernhardt 2012, SCNA | Functional Core / Imperative Shell pattern |
| Cemri et al. 2025, arXiv:2503.13657 | MAST: 14 failure modes across 150 multi-agent system traces |
| OWASP ASI08 2026 | Cascading Failures in Agentic AI |
| Conant & Ashby 1970, IJSS 1(2) | Good Regulator Theorem |
| Chen et al. 2023, arXiv:2305.05176 | FrugalGPT: cascade cost optimization, up to 98% reduction |
| Vickrey 1961; Clarke 1971; Groves 1973 | VCG mechanism: truthful bidding for resource allocation |
| Damasio 1994, Descartes' Error | Somatic marker hypothesis |
| Kanerva 2009, Cognitive Computation 1(2) | HDC: hyperdimensional computing |
| Kleyko et al. 2022, Artificial Intelligence Review 56 | Survey of HDC applications |
| Hu et al. 2025, ICLR | ADAS: meta-agent architecture search, +14% ARC Challenge |
| Lee et al. 2026, arXiv:2603.28052 | Meta-Harness: harness optimization alone produces outsized gains |
| Ebbinghaus, H. 1885, "Uber das Gedachtnis" | Forgetting curve: memory decay without reinforcement |

---

## Cross-References

- [00-ARCHITECTURE](../../00-ARCHITECTURE.md) -- Parent chapter
- [cognitive-cross-cuts.md](cognitive-cross-cuts.md) -- E44 functor model
- [autocatalytic-and-cybernetics.md](autocatalytic-and-cybernetics.md) -- Compounding loops
- [c-factor-collective-intelligence.md](c-factor-collective-intelligence.md) -- C-factor details
- [compositional-kinds.md](compositional-kinds.md) -- Kind algebra
- [configuration-schema.md](configuration-schema.md) -- P1 in practice: config vs. composition
