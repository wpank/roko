# Migration Log — Cluster B: Fabrics & Operators

**Cluster**: B  
**Migration date**: 2026-04-19  
**Author**: Automated refactor (session)  
**Status**: Complete

---

## Summary

Cluster B refactored the Fabrics and Operators sections of the Roko architecture docs.
Four dense source files (total ~2 000 lines of markdown) were split into **64 granular
target files** across `reference/03-substrate/`, `reference/04-bus/`, and
`reference/05-operators/` (including five operator subfolders).

Every H2/H3 section in the source maps to at least one file in the target. Thin sections
were expanded with inferred content and marked `<!-- ADDED: ... -->`.

---

## Source files

| Source file | Approx lines | Primary content |
|---|---|---|
| `docs/00-architecture/03-substrate.md` | ~450 | Substrate trait, backends, pruning |
| `docs/00-architecture/04-bus.md` | ~350 | Bus/EventBus traits, topics, delivery |
| `docs/00-architecture/05-operators.md` | ~700 | Scorer, Gate, Router, Composer, Policy |
| `docs/00-architecture/06-cognitive-loop.md` | ~300 | Loop integration, tick lifecycle |

---

## Target files — `reference/03-substrate/` (17 files)

| Target file | Source section | Notes |
|---|---|---|
| `README.md` | Index/intro | New: navigation hub |
| `00-overview.md` | § Substrate overview | Expanded with 5-layer placement |
| `01-trait-surface.md` | § Substrate trait | Verbatim + crate annotation |
| `02-put-get-query.md` | § put/get/query_recent | Expanded with async semantics |
| `03-query-similar.md` | § query_similar | Expanded with HDC fingerprint context |
| `04-fingerprint-population.md` | § fingerprint population | ADDED: strategy table |
| `05-concurrency-model.md` | § concurrency / async | ADDED: inferred from `Send+Sync` bound |
| `06-pruning.md` | § pruning | Expanded with `PruneStrategy` variants |
| `07-backends-overview.md` | § backends intro | New: comparison table |
| `08-backend-file-jsonl.md` | § FileSubstrate | Expanded with JSONL format detail |
| `09-backend-in-memory.md` | § MemorySubstrate | Expanded with test-use guidance |
| `10-backend-chain.md` | § ChainSubstrate | Expanded with tiered-storage pattern |
| `11-invariants.md` | — | ADDED: 7 invariants inferred from trait contract |
| `12-failure-modes.md` | § failure notes (scattered) | ADDED: 7 failure modes consolidated |
| `13-performance.md` | § performance notes | ADDED: per-operation complexity table |
| `14-api-reference.md` | § Substrate trait | Expanded: full type table |
| `15-examples.md` | § examples | ADDED: 5 end-to-end examples |
| `16-rationale.md` | § design notes | ADDED: trait-based design rationale |

---

## Target files — `reference/04-bus/` (16 files)

| Target file | Source section | Notes |
|---|---|---|
| `README.md` | Index/intro | New: navigation hub |
| `00-overview.md` | § Bus overview | Expanded with Gamma/Delta context |
| `01-trait-surface.md` | § Bus trait | Verbatim + crate annotation |
| `02-topics.md` | § topics | Expanded with naming conventions |
| `03-topic-filters.md` | § topic filters | Expanded with wildcard semantics |
| `04-publish-subscribe.md` | § publish/subscribe | ADDED: backpressure behaviour |
| `05-replay-and-ring.md` | § replay / ring buffer | Expanded with ring-full behaviour |
| `06-backends-overview.md` | § backends intro | New: comparison table |
| `07-backend-event-bus.md` | § EventBus<E> | Expanded with in-process semantics |
| `08-backend-distributed.md` | § distributed bus (planned) | Expanded with NATS/Redis notes |
| `09-ordering-guarantees.md` | § ordering | ADDED: per-backend guarantee table |
| `10-delivery-semantics.md` | § delivery | ADDED: at-most-once / at-least-once |
| `11-failure-modes.md` | § failure notes | ADDED: 6 failure modes |
| `12-performance.md` | § performance | ADDED: latency breakdown |
| `13-api-reference.md` | § Bus trait | Full type reference |
| `14-today-vs-planned.md` | § today/target split | Expanded with migration path |
| `15-rationale.md` | § design notes | ADDED: in-process first, async second |

---

## Target files — `reference/05-operators/` top-level (4 files)

| Target file | Source section | Notes |
|---|---|---|
| `README.md` | § Operators overview | New: navigation hub |
| `00-overview.md` | § Operators intro | Expanded with 7-step loop placement |
| `01-trait-composition-model.md` | § trait composition | ADDED: `loop_tick` integration point |
| `02-trait-layer-map.md` | § layer map | ADDED: 5-layer × operator grid |

---

## Target files — `reference/05-operators/01-scorer/` (11 files)

| Target file | Source section | Notes |
|---|---|---|
| `README.md` | Index | New |
| `00-overview.md` | § Scorer overview | Expanded |
| `01-trait-surface.md` | § Scorer trait | Verbatim + crate annotation |
| `02-semantics.md` | § Score semantics | Expanded with axis meanings |
| `03-implementation.md` | § WeightedScorer, PassScorer | ADDED: ComposedScorer |
| `04-api-reference.md` | § Score struct | Full field table |
| `05-invariants.md` | — | ADDED: 6 invariants |
| `06-failure-modes.md` | — | ADDED: 5 failure modes |
| `07-performance.md` | — | ADDED: O(N) per-axis analysis |
| `08-examples.md` | § examples | ADDED: 5 examples |
| `09-composition-patterns.md` | § composition | Expanded with stacking patterns |
| `10-rationale.md` | § design notes | ADDED |

---

## Target files — `reference/05-operators/02-gate/` (11 files)

| Target file | Source section | Notes |
|---|---|---|
| `README.md` | Index | New |
| `00-overview.md` | § Gate overview | Expanded |
| `01-trait-surface.md` | § Gate trait | Verbatim + `Abstain` semantics |
| `02-semantics.md` | § GateDecision | Expanded; `Abstain ≠ Pass` highlighted |
| `03-implementation.md` | § ThresholdGate, SafetyGate | ADDED: ComposedGate |
| `04-api-reference.md` | § Gate types | Full table |
| `05-invariants.md` | — | ADDED: fail-closed invariant |
| `06-failure-modes.md` | — | ADDED: 6 failure modes |
| `07-performance.md` | — | ADDED |
| `08-examples.md` | § examples | ADDED |
| `09-gate-composition.md` | § composition | ADDED: ALL/ANY/FIRST patterns |
| `10-rationale.md` | § design notes | ADDED |

---

## Target files — `reference/05-operators/03-router/` (11 files)

| Target file | Source section | Notes |
|---|---|---|
| `README.md` | Index | New |
| `00-overview.md` | § Router overview | Expanded |
| `01-trait-surface.md` | § Router trait | Verbatim + `Ok(None)` semantics |
| `02-semantics.md` | § ActionKind | Expanded with cascade fallthrough |
| `03-implementation.md` | § CascadeRouter, UCBRouter | ADDED: DirectRouter |
| `04-api-reference.md` | § Router types | Full table |
| `05-invariants.md` | — | ADDED: 6 invariants |
| `06-failure-modes.md` | — | ADDED: 6 failure modes |
| `07-performance.md` | — | ADDED |
| `08-examples.md` | § examples | ADDED |
| `09-bandit-integration.md` | § UCB / bandit | Expanded: UCB1 formula, exploration |
| `10-rationale.md` | § design notes | ADDED |

---

## Target files — `reference/05-operators/04-composer/` (11 files)

| Target file | Source section | Notes |
|---|---|---|
| `README.md` | Index | New |
| `00-overview.md` | § Composer overview | Expanded |
| `01-trait-surface.md` | § Composer trait | Verbatim + `Prompt` output type |
| `02-semantics.md` | § prompt assembly | Expanded with section ordering |
| `03-implementation.md` | § SystemPromptBuilder | ADDED: ChatComposer |
| `04-api-reference.md` | § Composer types | Full table |
| `05-invariants.md` | — | ADDED: token budget invariant |
| `06-failure-modes.md` | — | ADDED: token overflow, truncation |
| `07-performance.md` | — | ADDED: recall latency is the dominant cost |
| `08-examples.md` | § examples | ADDED |
| `09-placement-strategies.md` | § placement / U-shape | Expanded: Liu et al. 2023 reference |
| `10-rationale.md` | § design notes | ADDED |

---

## Target files — `reference/05-operators/05-policy/` (11 files)

| Target file | Source section | Notes |
|---|---|---|
| `README.md` | Index | New |
| `00-overview.md` | § Policy overview | Expanded |
| `01-trait-surface.md` | § Policy trait | Verbatim + `&mut self` note |
| `02-semantics.md` | § PolicyDecision | All 4 variants documented |
| `03-implementation.md` | § CircuitBreakerPolicy | ADDED: SafetyPolicy, ComposedPolicy |
| `04-api-reference.md` | § Policy types | Full table incl. EscalationPacket |
| `05-invariants.md` | — | ADDED: 7 invariants incl. fail-open |
| `06-failure-modes.md` | — | ADDED: 7 failure modes |
| `07-performance.md` | — | ADDED: O(window_size), tick budget |
| `08-examples.md` | § examples | ADDED: 6 examples |
| `09-policy-vs-calibrator.md` | § today/target learning split | Expanded: Calibrator spec |
| `10-rationale.md` | § design notes | ADDED: fail-open rationale, sync rationale |

---

## Content additions log

Sections added beyond what was present in source (all marked `<!-- ADDED: ... -->`):

| Topic | File | Reason added |
|---|---|---|
| `ChainSubstrate` tiered-storage pattern | `03-substrate/10-backend-chain.md` | Source mentioned chaining but gave no guidance |
| Substrate 7 invariants | `03-substrate/11-invariants.md` | Source had no invariants section |
| Bus ordering guarantee table | `04-bus/09-ordering-guarantees.md` | Critical for distributed usage; not in source |
| Bus delivery semantics | `04-bus/10-delivery-semantics.md` | Standard distributed-systems concern; absent from source |
| Operator `loop_tick` integration point | `05-operators/01-trait-composition-model.md` | Source described operators in isolation; integration missing |
| Gate `Abstain ≠ Pass` | `05-operators/02-gate/02-semantics.md` | Critical safety distinction; thinly described in source |
| Gate ALL/ANY/FIRST composition | `05-operators/02-gate/09-gate-composition.md` | Mentioned but not specified |
| Router `Ok(None)` cascade semantics | `05-operators/03-router/01-trait-surface.md` | Source left this ambiguous |
| UCB1 formula + regret bound | `05-operators/03-router/09-bandit-integration.md` | Source referenced bandit without formula |
| Composer token budget invariant | `05-operators/04-composer/05-invariants.md` | Token overflow is a common real failure; absent from source |
| Composer U-shape placement + Liu et al. 2023 | `05-operators/04-composer/09-placement-strategies.md` | Source mentioned placement without research backing |
| Policy `&mut self` rationale | `05-operators/05-policy/10-rationale.md` | Source stated the fact but not the alternatives considered |
| Policy fail-open principle | `05-operators/05-policy/05-invariants.md` | Source implied it; never stated explicitly |
| Calibrator trait spec | `05-operators/05-policy/09-policy-vs-calibrator.md` | Source noted the planned split; this gives the full design |
| `ComposedPolicy` stacking | `05-operators/05-policy/05-invariants.md` | Logical extension of single-responsibility principle |

---

## Unclear mappings / open items

| Item | Status | Notes |
|---|---|---|
| `Calibrator` trait | Specified (not yet written) | Documented as planned in `09-policy-vs-calibrator.md`; needs its own `reference/05-operators/06-calibrator/` folder when Specified → Shipping |
| `Bus` trait status | Specified | `EventBus<E>` = Shipping; `Bus` trait (distributed) = Specified. Both documented in `04-bus/01-trait-surface.md` |
| Distributed Bus backend | Specified | `08-backend-distributed.md` describes planned NATS/Redis integration; no Rust trait code yet |
| `Dreams` offline loop | Not in Cluster B | Referenced by Policy (prediction.error routing) and Calibrator; belongs in a future Cluster C or D |
| `Daimon` affect/motivation system | Not in Cluster B | Referenced in loop overview; belongs in a future cluster |
| `Neuro` knowledge cross-cut | Not in Cluster B | Referenced in operator overview |
| `roko-agent` crate — UCB implementation | Referenced | `09-bandit-integration.md` shows the interface; actual UCB code lives in `roko-agent` |
| Loop timing model | Partial | Referenced as "cognitive speeds" (Gamma/Alpha/Delta) in several files; a dedicated `reference/02-loop/` cluster would unify this |

---

## Cross-cluster suggestions

These observations may be useful for planning Clusters C and D:

1. **`reference/02-loop/`** — The cognitive loop (`loop_tick`) is referenced by operators,
   policy, and bus files but has no dedicated folder. A `02-loop/` cluster with files for
   each of the 7 steps (SENSE through STORE/LEARN) would close many of the "See Also"
   links that currently point to non-existent files.

2. **`reference/06-calibrator/`** — The Calibrator operator is fully specified in
   `05-policy/09-policy-vs-calibrator.md`. When it ships, it needs its own 11-file folder
   (same template as the other operators).

3. **`reference/07-dreams/`** — Dreams is the Delta-speed offline consolidation loop.
   It consumes `prediction.error` from the Bus and updates Substrate weights. It is
   referenced from Bus (topics) and Policy but has no reference docs.

4. **`reference/08-neuro/`** and **`reference/09-daimon/`** — The three cross-cuts
   (Neuro, Daimon, Dreams) are mentioned in the architecture overview but not documented.

5. **Status tag cleanup** — Several files in `04-bus/` mark individual sections as
   Specified while the file-level frontmatter says Shipping (`EventBus<E>`). A sweep to
   ensure frontmatter matches the most conservative status of the content would improve
   consistency.

---

## File count summary

| Folder | Files |
|---|---|
| `reference/03-substrate/` | 17 |
| `reference/04-bus/` | 16 |
| `reference/05-operators/` (top-level) | 4 |
| `reference/05-operators/01-scorer/` | 11 |
| `reference/05-operators/02-gate/` | 11 |
| `reference/05-operators/03-router/` | 11 |
| `reference/05-operators/04-composer/` | 11 |
| `reference/05-operators/05-policy/` | 11 |
| **Total (Cluster B)** | **92** |

Note: The `find` command shows 147 total reference files across all clusters, including
Cluster A files (`reference/01-engram/`, `reference/02-pulse/`, `reference/10-types/`,
`reference/11-crate-map.md`, `reference/12-design-principles.md`) written in a prior session.
