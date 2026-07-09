# S4 — Replication ledger × Heuristics × paper Engram → Living research

> Papers live as Engrams. Claims extracted from them become heuristics. The replication ledger
> records which claims have held up under test and which have been falsified. Research becomes
> runtime material: a claim is not a static citation but a living object whose status updates as
> evidence arrives.

**Status**: Analysis — target-state synergy  
**Primitives involved**: P9 Replication ledger × P7 Heuristics × P1 Engram (paper variant)  
**Reality check**: P1 Engram storage is Shipping; P7 Heuristics are Scaffold; P9 Replication
ledger is Specified (no shipped implementation). The full synergy is target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| [P9 Replication ledger](../../research/) | Tracks claims, evidence, and falsification history; provides a machine-readable provenance chain from claim to evidence |
| [P7 Heuristics + falsifiers](../../subsystems/) | The runtime form of a claim: a predicate with confidence and a falsifier condition that links back to the ledger |
| [P1 Engram (paper variant)](../../reference/01-engram/) | The durable record that represents a research paper; holds the paper's content, metadata, and the HDC fingerprint used for similarity search |

---

## What the Synergy Unlocks

### The static-citation problem

Traditional AI systems treat research papers as static reference material: a paper is ingested,
its claims are used to initialize the system, and the system never updates its beliefs about
those claims unless a human explicitly revises the knowledge base.

This creates two failure modes:
1. Outdated claims persist and continue influencing decisions long after replication failures.
2. New evidence that confirms or refines a claim has no path back into the runtime.

### How it works

1. **Ingestion**: A paper enters the system as a paper Engram. The HDC fingerprint of its
   abstract is computed. The Engram is stored in Substrate.
2. **Claim extraction**: Key claims in the paper are extracted (manually or by a claim-extraction
   agent) and entered into the Replication ledger as claims with status `Unconfirmed`.
3. **Heuristic lift**: Each claim in the ledger whose confidence crosses a threshold is
   automatically "lifted" into the Heuristics store as an actionable rule. The heuristic carries
   a back-reference to the ledger claim that produced it.
4. **Evidence accumulation**: As the system runs, it gathers evidence about whether heuristic
   predictions hold. Outcome Pulses (from S2) feed the calibration loop. Calibration updates
   propagate back to the ledger: a consistently confirmed heuristic raises its ledger claim's
   replication score; a consistently falsified heuristic lowers it.
5. **Ledger update**: When a claim's replication score drops below a threshold, the ledger marks
   it `Weakened` or `Falsified`. This propagates to the heuristic store: confidence is reduced,
   and the heuristic may be quarantined for human review.

The result: a claim is not a static citation. It is a living object whose status changes as
evidence arrives. Research becomes runtime material.

### Why each primitive is necessary

- Without the paper Engram, there is no durable anchor for the provenance chain. Claims float
  without a source.
- Without the Replication ledger, calibration updates to heuristics are local and invisible to
  the research layer. There is no accumulated evidence record.
- Without Heuristics + falsifiers, claims have no runtime form — they sit in a ledger but do not
  influence decisions.

---

## What Flows

```
Paper ingestion:
  paper text → extract claims → enter each as Ledger.claim(status=Unconfirmed)
  paper → Engram(kind=Paper, fingerprint=HDC(abstract)) → Substrate

Heuristic lift (threshold-based):
  Ledger.claim.confidence > lift_threshold
  → Heuristics.store(predicate, confidence, falsifier=Ledger.claim.id)

Calibration loop (via S2):
  outcome Pulses → Calibration subscriber → update heuristic confidence
  → Ledger.claim.replication_score += delta

Ledger status propagation:
  Ledger.claim.replication_score < falsification_threshold
  → mark claim Weakened / Falsified
  → Heuristics.confidence(affected) → reduce and flag for review
```

---

## Invariants

1. Every Heuristic lifted from a paper claim carries a ledger back-reference. No claim-derived
   heuristic is orphaned from its provenance.
2. Ledger status flows one-way to heuristics: a Weakened claim can only reduce the linked
   heuristic's confidence, not raise it. Confidence recovery requires new confirming evidence.
3. A paper Engram is never modified after ingestion — it is an immutable record. The ledger is
   the mutable view of the claims it contains.
4. The replication score is evidence-weighted: a single experiment cannot drive a claim from
   `Unconfirmed` to `Falsified` unless the evidence strength is explicitly high.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| Claim extraction errors | An automated claim extractor misidentifies correlations as causal claims | Require human review before a claim reaches `Unconfirmed` status; treat automated extraction as a draft |
| Ledger-heuristic divergence | Ledger marks a claim Falsified but the heuristic store is not notified promptly | Use a Bus watchdog topic that broadcasts ledger-status changes; Heuristics subscriber reacts within the same session |
| Confidence floor gaming | A heuristic's confidence is floored to avoid quarantine even after its backing claim is falsified | When a backing claim is Falsified, override the confidence floor for that specific heuristic |
| Research bias | Only papers from certain domains are ingested; the heuristic store reflects that bias | Surface ingestion coverage as a readiness-audit metric; flag underrepresented domains |
| Ledger explosion | Every minor outcome Pulse updates the ledger; ledger grows unboundedly | Batch ledger updates; only flush aggregated replication-score deltas on a configurable cadence |

---

## Relationship to Other Synergies

- **S2** (Heuristics × Pulse × Bus): S2 provides the streaming calibration loop that generates
  the evidence signals consumed by S4's ledger updates. S4 is the long-term memory of what S2
  has learned.
- **S8** (Demurrage × Heuristics × calibration): S8 softens confidence when evidence is sparse.
  S4 provides the mechanism for that absence of evidence to be connected back to the ledger's
  replication record.
- **S1** (Demurrage × HDC): If a paper Engram's claims are all falsified, demurrage will
  eventually evict the Engram from Substrate — but only after the ledger has marked the claims
  as falsified and the linked heuristics have been quarantined. The pruning is semantically
  informed, not just temporal.

---

## Today vs. Planned

**Today**: Papers can be stored as Engrams. Heuristics can be stored as Substrate records.
No Replication ledger exists. Calibration is manual. There is no claim-lift mechanism.

**Planned**: Replication ledger (P9) ships as a structured store. Claim-extraction tooling
(manual or automated) produces ledger entries. A lift-threshold policy periodically promotes
high-confidence claims to Heuristics. A Bus watchdog propagates ledger-status changes to the
Heuristics subscriber.

---

## Cross-References

- [`analysis/architectural-analysis/08-novel-proposals.md`](../architectural-analysis/08-novel-proposals.md) — living research named as a novel proposal
- [`analysis/readiness-audit/subsystem-learning.md`](../readiness-audit/subsystem-learning.md) — learning subsystem gaps relevant to heuristic lift
- [`analysis/synergy-map/synergy-02-heuristics-pulse-bus.md`](synergy-02-heuristics-pulse-bus.md) — S2: the streaming calibration that feeds S4
- [`analysis/synergy-map/synergy-08-demurrage-heuristic-relearning.md`](synergy-08-demurrage-heuristic-relearning.md) — S8: confidence softening for stale claims
- [`analysis/synergy-map/99-master-synergy-table.md`](99-master-synergy-table.md) — synergy index

---

## Open Questions

- What is the minimal claim schema? At minimum: claim text, source Engram ID, extraction
  method, initial confidence, replication score. Are there other required fields?
- Should the lift threshold be global or per-domain? Domain-specific lift thresholds would
  allow safety-critical domains to require higher evidence before a claim becomes a heuristic.
- Can the ledger support counter-claims (one paper claims X, another claims ¬X)? How are
  conflicting claims resolved when both are lifted to heuristics?
