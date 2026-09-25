# Synergy & Integration Map

> **v3 depth file** -- `/docs/v3/depth/00-architecture/synergy-integration-map.md`
> Canonical source: v1 `docs/v1/00-architecture/34-synergy-integration-map.md`
> Status: **Reference** -- This is a mixed reality map, not a shipping-only inventory.
> Signal and Substrate are live primitives. `EventBus<E>` is the live transport surface.
> HDC fingerprinting, c-factor, and heuristics exist partially. Pulse (as distinct from
> Signal), the generalized `Bus` trait, demurrage, the replication ledger, and the full
> plugin domain-profile ecosystem remain target-state for some cells.

---

## 1. Purpose

This chapter is the cross-reference map for the architecture stack. The prior
architecture docs describe the primitives one by one; this doc shows how they reinforce
each other. The claim is simple: Roko's moat is not any single feature, but the
interaction density of the whole system. A competitor can copy one node or even a pair;
reproducing the weave means reproducing the same architectural choices in the same order.

---

## 2. The Ten Load-Bearing Primitives

These are the nodes of the synergy graph. Each one is established in the architecture
docs; this chapter treats them as the minimal set whose interactions explain the larger
system.

| # | Primitive | Role in the weave | Implementation status |
|---|---|---|---|
| P1 | Signal (Engram) | Durable record, lineage anchor, substrate resident | **Live** -- kernel type |
| P2 | Pulse | Ephemeral wire medium, live coordination unit | **Partial** -- `EventBus<E>` is live; generalized Pulse medium is target-state |
| P3 | Bus / EventBus | Live transport, topic routing, replay | **Live** -- `EventBus<E>` wired; `Bus` kernel trait defined |
| P4 | Substrate | Storage fabric, durable persistence, query surface | **Live** -- `FileSubstrate` with JSONL, GC |
| P5 | HDC fingerprint | Similarity, clustering, semantic indexing | **Partial** -- `HdcVector` live, per-episode fingerprints wired |
| P6 | Demurrage | Attention economy, holding cost, self-trimming | **Target-state** -- decay variants live, economic demurrage deferred |
| P7 | Heuristics + falsifiers | Learned rules with explicit calibration hooks | **Live** -- playbooks, when/then enrichment |
| P8 | c-factor | Collective intelligence signal, diversity pressure | **Live** -- CFactorSummary computed per runner dispatch |
| P9 | Replication ledger | Claims, evidence, falsification history | **Partial** -- knowledge entries with evidence; full ledger target-state |
| P10 | Plugin SPI + domain profiles | Ecosystem growth, extension surface | **Live** -- E32 8/8, signed deps, WASM hooks |

---

## 3. The Synergy Matrix

Each cell says what the row primitive gives to the column primitive. Empty cells indicate
no direct dependency or a coupling the architecture intentionally avoids. Cells marked
`[ts]` are target-state rather than currently shipping.

| gives \ to | P1 Signal | P2 Pulse | P3 Bus | P4 Substrate | P5 HDC | P6 Demurrage | P7 Heuristics | P8 c-factor | P9 Ledger | P10 Plugins |
|---|---|---|---|---|---|---|---|---|---|---|
| **P1 Signal** | -- | graduation source [ts] | publish target [ts] | store target | encode target | balance owner [ts] | lineage anchor | cohort artifact | paper body [ts] | plugin config |
| **P2 Pulse** | graduation dest [ts] | -- [ts] | payload [ts] | sub-event [ts] | live evidence [ts] | reinforcement [ts] | calibration trial [ts] | cohort event [ts] | observation [ts] | plugin event [ts] |
| **P3 Bus** | substrate wakeups [ts] | delivery [ts] | -- | notify [ts] | routing input [ts] | freshness [ts] | falsifier watch [ts] | cohort floor [ts] | watchdog [ts] | lifecycle events |
| **P4 Substrate** | home | -- [ts] | bridge [ts] | -- | fingerprint store | balance home [ts] | heuristic store | metric source | ledger store [ts] | plugin state |
| **P5 HDC** | fingerprint field | -- [ts] | -- [ts] | index key | -- | novelty score [ts] | similarity cluster | diversity signal | paper search [ts] | encoder [ts] |
| **P6 Demurrage** | weight [ts] | -- [ts] | -- [ts] | tier logic [ts] | -- [ts] | -- [ts] | freshness decay [ts] | minority support [ts] | anti-drift [ts] | plugin aging [ts] |
| **P7 Heuristics** | record variant | -- [ts] | prediction [ts] | store | cluster | reinforcement [ts] | -- | peer model | claim body [ts] | heuristic plugin |
| **P8 c-factor** | cohort record | -- [ts] | metrics topic [ts] | metric store | diversity source | -- [ts] | peer prediction | -- | replication [ts] | c-factor plugin |
| **P9 Ledger** | paper Signal [ts] | -- [ts] | watchdog [ts] | ledger store [ts] | paper fp [ts] | claim decay [ts] | lifted claim [ts] | observation [ts] | -- [ts] | claim plugin [ts] |
| **P10 Plugins** | plugin Signal | plugin events | plugin topics | plugin reads | plugin encoder [ts] | plugin budget [ts] | new heuristic | new metric | new claim [ts] | -- |

Read the matrix as a design test. If a feature does not connect to at least two nodes,
it is probably too thin to matter. If it connects to too many nodes without a clear
purpose, it is probably too broad to land cleanly.

---

## 4. Ten Named Synergies

### 4.1 Demurrage x HDC -> Self-Trimming Semantic Memory

Substrate stores each Signal with a fingerprint. Demurrage charges holding cost over
time. HDC makes novelty measurable by comparing a record to its nearest neighbors. The
result is memory that gradually favors uniquely useful records rather than raw
accumulation.

This is the core "unique-and-used" pressure. Without HDC, demurrage becomes a blunt
tax. Without demurrage, HDC becomes an expensive search primitive with no pruning force.
Together they make the memory layer economically selective.

### 4.2 Heuristics x Pulse x Bus -> Continuous Calibration

Heuristics carry explicit falsifiers. Pulses on the Bus provide live evidence of whether
the heuristic helped, failed, or needs tightening. That turns calibration into a
streaming process instead of a periodic audit.

The practical effect is continuous learning from lived outcomes. A heuristic is not
trusted because it exists; it is trusted because it has survived repeated contact with
relevant Pulses.

### 4.3 c-factor x Bus x HDC -> Diversity-Aware Routing

Bus statistics show how work is being distributed. HDC shows whether the system's
representations are converging too tightly. c-factor consumes both signals and can push
policy toward role diversity, model diversity, or pair rotation when the system becomes
too homogeneous.

This is not just observability. It is regulation. The system watches for monoculture
and actively corrects toward broader cognitive variety.

### 4.4 Replication Ledger x Heuristics x Paper Signal -> Living Research

Papers live as Signals. Claims extracted from them become heuristics. The replication
ledger records which claims have held up under test and which have been falsified.

This turns research into runtime material. A claim is not a static citation; it is a
living object whose status can change as evidence arrives.

### 4.5 Plugin SPI x Substrate x Bus -> Ecosystem Growth Path

Plugins declare what they can read from Substrate and what topics they subscribe to on
the Bus. The SPI constrains the extension boundary so new tools, gates, roles, and
domain profiles can land without rewriting the core.

The synergy matters because it makes growth structurally safe. The ecosystem expands
along declared seams instead of through ad hoc hooks.

### 4.6 c-factor x Heuristics -> Peer-Model Learning

The system can model other agents the same way it models the world: as a set of
predictions to calibrate. Peer-model accuracy becomes part of the collective
intelligence signal.

### 4.7 Dreams x Substrate x Pulse -> Retroactive Insight

Dreams read durable records from Substrate, reinterpret them under updated priors, and
publish new Pulses that refresh downstream caches and composers. Old episodes therefore
remain actionable, but only after the system has grown enough to reinterpret them.

The consequence: the system does not merely remember. It re-learns from memory.

### 4.8 Demurrage x Heuristic x Calibration -> Graceful Relearning

Confidence is not frozen. If a heuristic is not challenged, its confidence should soften
enough that fresh contradictory evidence can move it without a manual reset.

This is the anti-stagnation mechanism. A long-stable rule can become stale; demurrage
on confidence prevents it from dominating forever.

### 4.9 HDC x Consensus x Bus -> Substantive Agreement Detection

Agents can emit agreement Pulses with fingerprints of the ideas they endorse.
Aggregators compare those fingerprints to proposal fingerprints rather than treating
surface wording as the ground truth.

That lets the system tell the difference between genuine agreement and merely similar
phrasing. It is a semantic check, not a token-counting trick.

### 4.10 TypedContext x Domain Profiles x Gate -> Auditable Domain Safety

Domain profiles package the behavior of a specific operating domain. TypedContext carries
the structured situation. Gates evaluate typed predicates instead of free-text guesses,
and Custody records who acted, why, and with what evidence.

The synergy is auditability. Domain-sensitive actions remain inspectable after the fact.

---

## 5. The Seven-Step Loop Across the Matrix

The synergy matrix is not separate from the universal loop. It explains why the same
seven-step cycle compounds instead of resetting each turn. In current code, the
Signal/Substrate/EventBus parts of the loop are much more real than the Pulse,
demurrage, ledger, and custody portions.

1. `SENSE` draws from `Substrate` queries, `Bus` subscriptions, and external I/O, then
   anchors those reads in `Signal`, `Pulse`, and `TypedContext`.
2. `ASSESS` uses `HDC fingerprint`, `demurrage`, `heuristics`, and `c-factor` signals to
   decide what deserves attention and which policy lever should move next.
3. `COMPOSE` pulls the right durable records into scope, injects domain-profile
   structure, and uses the same matrix to decide which evidence belongs in the prompt.
4. `ACT` emits live `Pulse` traffic, produces tool and agent outcomes, and creates the
   next candidates for durable `Signal` storage.
5. `VERIFY` turns gates, falsifiers, and replication checks into evidence about whether
   the current behavior should be reinforced, revised, or quarantined.
6. `PERSIST` writes durable artifacts back to `Substrate`, assigns economic weight
   through demurrage, records `HDC fingerprint` values, and stores claim or custody
   evidence where lineage matters.
7. `BROADCAST` and `REACT` publish the new live state on the `Bus`, update observers
   such as `c-factor` and plugin consumers, and trigger follow-on policy, calibration,
   or consolidation.

That is the compounding claim in operational form: every step touches multiple
primitives, so improving one cell in the matrix tends to improve several loop steps.

---

## 6. What the Matrix Is, and Is Not

The matrix is a design map, not a priority queue. It tells you which primitives
reinforce one another and where the architecture has leverage, but it does not tell
you what to build first.

It is also not a completeness claim. More synergies exist. The matrix names the most
load-bearing ones because they explain the system's shape.

Finally, it is not a vendor pitch. The matrix is an internal coherence tool. Its value
is that it makes the architecture inspectable by composition rather than by feature list.

---

## 7. How To Design New Refinements

When proposing a new refinement, walk the matrix before writing the spec.

1. Identify which primitives the feature touches.
2. Decide whether it provides to the primitive or consumes from it.
3. Check whether the interaction is already covered by a named synergy.
4. If not, ask whether the missing coupling is a gap in the feature or a genuine
   opportunity for the architecture.

The practical test is simple: a strong refinement usually strengthens at least two
existing edges, or creates one edge that clearly unlocks several others.

Use the matrix to avoid dead-end features. If a proposal has no durable connection to
Signal, no live connection to Pulse or Bus, and no policy or calibration consequence,
it probably belongs in a narrower subsystem note instead of a chapter-level refinement.

---

## 8. Non-Synergies Worth Naming

Some pairs remain intentionally loose.

P5 HDC and P9 Replication ledger are not the same thing. Papers may be fingerprinted,
but the ledger's rigor comes from evidence and falsification, not from similarity search.

P10 Plugins and P8 c-factor are also distinct. Plugins can contribute telemetry that
informs c-factor, but c-factor is not a plugin-selection rule.

P2 Pulse and P9 Replication ledger are different levels of granularity. Pulses are live
stream material; the ledger consumes stabilized claims, not raw chatter.

These non-synergies matter because they keep the architecture honest. Not every pair
should couple just because the system is compositional.

---

## 9. The Moat Restated

The moat framing is target-state rather than a claim about the fully shipped system.
P1 through P4 are important, and today that mostly means Signal, Substrate, and the
live `EventBus<E>` transport. P5 through P7 are individually useful but have prior art.
P8 through P10 become strategically important only if the planned primitives around
them are implemented and integrated into a coherent runtime.

The competitive claim is architectural: a competitor can copy any node, and often a pair,
but not the full interaction lattice without committing to the same dependency order and
the same cross-cut discipline. That is what makes the system hard to clone.

---

## 10. Emergent Properties

Three properties are meant to emerge from the composition once the planned primitives
land. Today, only parts of this section are already true in shipping code.

The first is self-improvement without a separate training pipeline. The runtime predicts,
calibrates, and updates through its own Bus-mediated feedback.

The second is inspectability at every level. Pulse lineage, Signal lineage, heuristic
provenance, and ledger status together make decisions traceable instead of opaque.

The third is substrate neutrality. Because the key behaviors are driven by HDC,
demurrage, heuristics, and policy, the system can swap storage or transport
implementations without changing its core cognitive behavior.

These are not decorative benefits. They are the direct result of the matrix.

---

## Cross-References

- [Cross-section integration map](./cross-section-integration-map.md) -- Section-level dependency matrix
- [Cross-pollination innovations](./cross-pollination-innovations.md) -- Eight composition innovations
- [Design principles and frontier summary](./design-principles-frontier-summary.md) -- Moat framing
- [Naming and glossary](./naming-and-glossary.md) -- Vocabulary stability for the matrix
- [Vision and thesis](./vision-and-thesis.md) -- Signal/Pulse/Cell/Graph/Protocol primitives
- [Substrate trait](./substrate-trait.md) -- Storage fabric deep dive
- [Bus transport fabric](./bus-transport-fabric.md) -- Transport fabric deep dive
- [C-factor collective intelligence](./c-factor-collective-intelligence.md) -- Diversity and policy
