# Synergy Map — Overview

> The architecture is best understood as a synergy graph. Some primitives are load-bearing in
> their own right; the real leverage comes from the way they combine. This page names the ten
> load-bearing primitives, presents the compact 10×10 interaction matrix, walks all ten named
> synergies in summary form, and explains what the moat claim means in practice.

**Status**: Analysis  
**Crate**: —  
**Last reviewed**: 2026-04-19

---

## 1. The Ten Load-Bearing Primitives

These are the nodes of the synergy graph. Each one has its own canonical specification
elsewhere; this folder treats them as the minimal set whose interactions explain the larger system.

> Shipped today: `Engram` (as `Signal` in code), `Substrate`, `EventBus<E>`, partial `HdcVector`,
> partial `c-factor`  
> Target state: `Pulse`, generalized `Bus` trait, `Demurrage`, replication ledger, Plugin SPI /
> domain profiles

| # | Primitive | Home doc | Status | Role in the weave |
|---|---|---|---|---|
| P1 | Engram | [`reference/01-engram/`](../../reference/01-engram/) | Shipping | Durable record, lineage anchor, substrate resident |
| P2 | Pulse | [`reference/02-pulse/`](../../reference/02-pulse/) | Scaffold | Ephemeral wire medium, live coordination unit |
| P3 | Bus / `EventBus<E>` | [`reference/04-bus/`](../../reference/04-bus/) | Built (EventBus); Scaffold (Bus trait) | Live transport today; generalized Bus trait + topic routing = target state |
| P4 | Substrate | [`reference/03-substrate/`](../../reference/03-substrate/) | Shipping | Storage fabric, durable persistence, query surface |
| P5 | HDC fingerprint | [`reference/`](../../reference/) | Built (partial) | Similarity, clustering, semantic indexing primitive |
| P6 | Demurrage | [`reference/`](../../reference/) | Specified | Attention economy, holding cost, self-trimming pressure |
| P7 | Heuristics + falsifiers | [`subsystems/`](../../subsystems/) | Scaffold | Learned rules of thumb with explicit calibration hooks |
| P8 | c-factor | [`reference/`](../../reference/) | Built (partial) | Collective intelligence signal, diversity pressure, policy target |
| P9 | Replication ledger | [`research/`](../../research/) | Specified | Claims, evidence, falsification history |
| P10 | Plugin SPI + domain profiles | [`subsystems/`](../../subsystems/) | Scaffold | Ecosystem growth, extension surface, domain-specific packaging |

The key claim is not that these primitives exist independently. It is that each becomes more
useful when it is allowed to constrain and enrich the others.

---

## 2. The Synergy Matrix

Each cell says what the **row** primitive gives to the **column** primitive. Empty cells are
intentional — the architecture stays clean by refusing to force a coupling. Cells marked
`[target-state]` depend on one or more planned primitives that are not yet shipped.

| gives ↓ \ to → | P1 Engram | P2 Pulse | P3 Bus | P4 Substrate | P5 HDC | P6 Demurrage | P7 Heuristics | P8 c-factor | P9 Ledger | P10 Plugins |
|---|---|---|---|---|---|---|---|---|---|---|
| **P1 Engram** | — | graduation source `[ts]` | publish target `[ts]` | store target | encode target | balance owner `[ts]` | lineage anchor | cohort artifact | paper body `[ts]` | plugin config target `[ts]` |
| **P2 Pulse** | graduation dest `[ts]` | — `[ts]` | payload `[ts]` | sub-event `[ts]` | live evidence `[ts]` | reinforcement signal `[ts]` | calibration trial `[ts]` | cohort event `[ts]` | ledger observation `[ts]` | plugin event `[ts]` |
| **P3 Bus** | `substrate.*` wakeups `[ts]` | delivery `[ts]` | — `[ts]` | notify `[ts]` | routing input `[ts]` | freshness pressure `[ts]` | falsifier watch `[ts]` | cohort floor `[ts]` | watchdog stream `[ts]` | lifecycle events `[ts]` |
| **P4 Substrate** | home | — `[ts]` | bridge `[ts]` | — | fingerprint store | balance home `[ts]` | heuristic store | metric source | ledger store `[ts]` | plugin state `[ts]` |
| **P5 HDC** | fingerprint field | — `[ts]` | — `[ts]` | index key | — | novelty score `[ts]` | similarity cluster | diversity signal | paper search `[ts]` | encoder slot `[ts]` |
| **P6 Demurrage** | weight `[ts]` | — `[ts]` | — `[ts]` | tier logic `[ts]` | — `[ts]` | — | freshness decay `[ts]` | minority support `[ts]` | anti-drift `[ts]` | plugin aging `[ts]` |
| **P7 Heuristics** | record variant | — `[ts]` | prediction / outcome `[ts]` | store | cluster | reinforcement signal `[ts]` | — | peer model | claim body `[ts]` | heuristic plugin `[ts]` |
| **P8 c-factor** | cohort record | — `[ts]` | metrics topic `[ts]` | metric store | diversity source | — `[ts]` | peer prediction | — | replication support `[ts]` | c-factor plugin `[ts]` |
| **P9 Ledger** | paper Engram `[ts]` | — `[ts]` | watchdog topic `[ts]` | ledger store `[ts]` | paper fingerprint `[ts]` | claim decay `[ts]` | lifted claim `[ts]` | ledger observation `[ts]` | — `[ts]` | claim plugin `[ts]` |
| **P10 Plugins** | plugin Engram `[ts]` | plugin events `[ts]` | plugin topics `[ts]` | plugin reads `[ts]` | plugin encoder `[ts]` | plugin budget `[ts]` | new heuristic source `[ts]` | new metric `[ts]` | new claim `[ts]` | — |

`[ts]` = target-state (depends on one or more unshipped primitives)

Read the matrix as a design test. If a feature connects to fewer than two nodes it is too thin to
matter. If it connects to too many nodes without a clear purpose it is too broad to land cleanly.

---

## 3. Ten Named Synergies — Summary

Each synergy has its own file with full analysis. Below are one-paragraph summaries; follow the
links for deep detail.

### S1 — Demurrage × HDC → Self-trimming semantic memory
[`synergy-01-demurrage-x-hdc.md`](synergy-01-demurrage-x-hdc.md)

Substrate stores each Engram with an HDC fingerprint. Demurrage charges holding cost over time.
HDC makes novelty measurable by comparing a record to its nearest neighbors. Together they make
memory economically selective: unique, frequently-used records survive; redundant or stale ones
decay away. Neither primitive produces this effect alone.

### S2 — Heuristics × Pulse × Bus → Continuous calibration
[`synergy-02-heuristics-pulse-bus.md`](synergy-02-heuristics-pulse-bus.md)

Heuristics carry explicit falsifiers. Pulses on the Bus deliver live evidence of whether each
heuristic helped, failed, or needs tightening. Calibration becomes a streaming process rather
than a periodic audit.

### S3 — c-factor × Bus × HDC → Diversity-aware routing
[`synergy-03-cfactor-bus-hdc.md`](synergy-03-cfactor-bus-hdc.md)

Bus statistics reveal how work is being distributed. HDC signals whether the system's
representations are converging too tightly. c-factor consumes both and can push policy toward
role diversity, model diversity, or pair rotation when the system becomes too homogeneous.
This is regulation, not just observability.

### S4 — Replication ledger × Heuristics × paper Engram → Living research
[`synergy-04-replication-living-research.md`](synergy-04-replication-living-research.md)

Papers live as Engrams. Claims extracted from them become heuristics. The replication ledger
records which claims have held up under test and which have been falsified. Research becomes
runtime material: a claim is not a static citation but a living object whose status updates as
evidence arrives.

### S5 — Plugin SPI × Substrate × Bus → Ecosystem growth path
[`synergy-05-plugin-spi-ecosystem.md`](synergy-05-plugin-spi-ecosystem.md)

Plugins declare what they can read from Substrate and what topics they subscribe to on the Bus.
The SPI constrains the extension boundary so new tools, gates, roles, and domain profiles land
without rewriting the core. Growth is structurally safe because it happens along declared seams.

### S6 — c-factor × Heuristics → Peer-model learning
[`synergy-06-cfactor-heuristics-peer-model.md`](synergy-06-cfactor-heuristics-peer-model.md)

The system can model other agents the same way it models the world — as a set of predictions
to calibrate. Peer-model accuracy becomes part of the collective intelligence signal, making
social perception measurable and coordination less wasteful.

### S7 — Dreams × Substrate × Pulse → Retroactive insight
[`synergy-07-dreams-retroactive.md`](synergy-07-dreams-retroactive.md)

Dreams read durable records from Substrate, reinterpret them under updated priors, and publish
new Pulses that refresh downstream caches and composers. Old episodes remain actionable, but only
after the system has grown enough to reinterpret them. The system does not merely remember — it
re-learns from memory.

### S8 — Demurrage × Heuristics × calibration → Graceful relearning
[`synergy-08-demurrage-heuristic-relearning.md`](synergy-08-demurrage-heuristic-relearning.md)

Confidence is not frozen. A heuristic that is not challenged has its confidence softened by
demurrage so that fresh contradictory evidence can move it without a manual reset. The
anti-stagnation mechanism prevents long-stable rules from dominating forever.

### S9 — HDC × Consensus × Bus → Substantive agreement detection
[`synergy-09-hdc-consensus-agreement.md`](synergy-09-hdc-consensus-agreement.md)

Agents emit agreement Pulses with HDC fingerprints of the ideas they endorse. Aggregators
compare those fingerprints to proposal fingerprints rather than treating surface wording as ground
truth. This is a semantic check, not a token-counting trick.

### S10 — TypedContext × domain profiles × Gate → Auditable domain safety
[`synergy-10-typed-context-domain-safety.md`](synergy-10-typed-context-domain-safety.md)

Domain profiles package the behavior of a specific operating domain. TypedContext carries the
structured situation. Gates evaluate typed predicates. Custody records who acted, why, and with
what evidence. Domain-sensitive actions remain inspectable after the fact without each team
inventing its own ad hoc logging stack.

---

## 4. The Seven-Step Loop Across the Matrix

The synergy matrix explains why the same seven-step cognitive cycle **compounds** instead of
resetting each turn. The Engram / Substrate / `EventBus<E>` parts of the loop are live today;
the Pulse, demurrage, ledger, and custody portions described below are target-state.

1. **SENSE** — draws from Substrate queries, Bus subscriptions, and external I/O; anchors those reads in Engram, Pulse, and TypedContext.
2. **ASSESS** — uses HDC fingerprint, demurrage, heuristics, and c-factor signals to decide what deserves attention and which policy lever should move next.
3. **COMPOSE** — pulls the right durable records into scope, injects domain-profile structure, and uses the same matrix to decide which evidence belongs in the prompt.
4. **ACT** — emits live Pulse traffic, produces tool and agent outcomes, and creates the next candidates for durable Engram storage.
5. **VERIFY** — turns gates, falsifiers, and replication checks into evidence about whether the current behavior should be reinforced, revised, or quarantined.
6. **PERSIST** — writes durable artifacts back to Substrate, assigns economic weight through demurrage, records HDC fingerprint values, and stores claim or custody evidence where lineage matters.
7. **BROADCAST / REACT** — publishes the new live state on the Bus, updates observers such as c-factor and plugin consumers, and triggers follow-on policy, calibration, or consolidation.

Every step touches multiple primitives, so improving one cell in the matrix tends to improve
several loop steps at once. That is the compounding claim in operational form.

---

## 5. The Moat Argument

P1–P4 are important, but today that mostly means Engram, Substrate, and the live `EventBus<E>`
transport. P5–P7 are individually useful but have prior art. P8–P10 become strategically
important only once the planned primitives around them are implemented and integrated.

The competitive claim is architectural: a competitor can copy any node, and often a pair, but not
the full interaction lattice without committing to the same dependency order and the same
cross-cut discipline. The moat is not a feature; it is an interaction density that is hard to
reproduce out of order.

---

## 6. Non-Synergies

Some pairs are intentionally loose. These non-synergies keep the architecture honest.

| Pair | Why they do NOT couple |
|---|---|
| P5 HDC × P9 Replication ledger | Papers may be fingerprinted, but the ledger's rigor comes from evidence and falsification, not from similarity search |
| P10 Plugins × P8 c-factor | Plugins can contribute telemetry that informs c-factor, but c-factor is not a plugin-selection rule |
| P2 Pulse × P9 Replication ledger | Different levels of granularity: Pulses are live stream material; the ledger consumes stabilized claims, not raw chatter |

---

## 7. Emergent Properties

Three properties are meant to emerge from the composition once the planned primitives land.
Today, only parts of these are already true in shipping code.

1. **Self-improvement without a separate training pipeline.** The runtime predicts, calibrates, and updates through its own Bus-mediated feedback.
2. **Inspectability at every level.** Pulse lineage, Engram lineage, heuristic provenance, and ledger status together make decisions traceable instead of opaque.
3. **Substrate neutrality.** Because the key behaviors are driven by HDC, demurrage, heuristics, and policy, the system can swap storage or transport implementations without changing its core cognitive behavior.

These are not decorative benefits. They are the direct result of the matrix.

---

## 8. How to Design New Refinements

When proposing a new refinement, walk the matrix before writing the spec:

1. Identify which primitives the feature touches.
2. Decide whether it provides to the primitive or consumes from it.
3. Check whether the interaction is already covered by a named synergy.
4. If not, ask whether the missing coupling is a gap in the feature or a genuine opportunity for the architecture.

A strong refinement usually strengthens at least two existing edges, or creates one edge that
clearly unlocks several others. If a proposal has no durable connection to Engram, no live
connection to Pulse or Bus, and no policy or calibration consequence, it probably belongs in a
narrower subsystem note rather than a chapter-level refinement.

---

## Open Questions

- Are there synergies not yet named? The matrix names the ten most load-bearing ones; the file
  is not exhaustive.
- What is the minimum working subset of primitives for a self-hosting demo that exercises at
  least three synergies?
- Should Dreams (S7) be treated as a primitive in its own right given how many synergies it
  participates in?

---

## See Also

- Individual synergy files: S1–S10 (see [README.md](README.md) for full list)
- [`99-master-synergy-table.md`](99-master-synergy-table.md) — searchable index
- [`analysis/integration-map/`](../integration-map/README.md) — point-to-point wiring
- [`analysis/readiness-audit/`](../readiness-audit/README.md) — implementation gaps
- [`analysis/architectural-analysis/08-novel-proposals.md`](../architectural-analysis/08-novel-proposals.md) — where several synergies appear as novel proposals
