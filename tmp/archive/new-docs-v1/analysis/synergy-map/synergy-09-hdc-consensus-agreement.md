# S9 — HDC × Consensus × Bus → Substantive agreement detection

> Agents emit agreement Pulses with HDC fingerprints of the ideas they endorse. Aggregators
> compare those fingerprints to proposal fingerprints rather than treating surface wording as
> ground truth. This is a semantic check, not a token-counting trick.

**Status**: Analysis — target-state synergy  
**Primitives involved**: P5 HDC fingerprint × Consensus subsystem × P3 Bus  
**Reality check**: P5 HDC is partially built; P3 Bus (generalized) is target-state. The
Consensus subsystem is Scaffold. Fingerprint-based agreement detection is target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| [P5 HDC fingerprint](../../reference/) | Encodes the semantic content of a proposal or an agent's endorsed idea; used to compare agreement semantically rather than by surface string match |
| Consensus subsystem | Aggregates agreement signals across agents; extended here to accept fingerprinted endorsements and compare them against proposal fingerprints |
| [P3 Bus / `EventBus<E>`](../../reference/04-bus/) | Carries agreement Pulses from endorsing agents to the consensus aggregator |

---

## What the Synergy Unlocks

### The surface-agreement problem

In a multi-agent system where agents produce and evaluate textual outputs, apparent agreement
is unreliable. Two agents may use different words to agree on the same underlying idea, or use
the same words while disagreeing on their meaning.

Standard voting-style consensus treats agreement as a string-level match or a binary
endorse/reject signal. This conflates:
- Genuine agreement (same idea, different words)
- Nominal agreement (same words, different meanings)
- Performative agreement (agent endorses because the idea comes from a high-confidence source,
  not because the agent evaluated the substance)

The synergy attacks all three failure modes.

### How it works

1. A proposal Engram is entered into Substrate and its HDC fingerprint is computed. The
   proposal is published on the Bus.
2. Each evaluating agent forms its own view of the proposal: it independently composes or
   retrieves the idea it endorses. The agent computes the HDC fingerprint of its endorsed idea.
3. The agent publishes an agreement Pulse on the Bus: `AgreesWith { proposal_id, endorsed_fingerprint }`.
4. The consensus aggregator subscribes to agreement Pulses. For each Pulse, it computes the
   cosine similarity between the `endorsed_fingerprint` and the `proposal_fingerprint`.
5. A high-similarity endorsement counts as substantive agreement. A low-similarity endorsement
   (the agent endorsed something semantically different) is flagged as nominal agreement and
   excluded from the consensus count, or counted separately.
6. Genuine consensus requires both a quorum of endorsements and a minimum similarity floor on
   those endorsements.

The result: the system can distinguish between genuine agreement and merely similar phrasing.
Consensus is a semantic check, not a token-counting trick.

### Why HDC is the right tool here

HDC vectors are cheap to compute, cheap to store, and cheap to compare. Cosine similarity on
HDC vectors is a constant-time operation regardless of the length of the ideas being compared.
This makes fingerprint-based agreement detection practical at the Pulse throughput rate of the
Bus — it does not require a heavyweight NLP comparison per endorsement.

---

## What Flows

```
Proposal phase:
  Proposal text → Engram(kind=Proposal, fingerprint=HDC(proposal_text)) → Substrate
  publish Pulse(topic=proposals.new, proposal_id=..., fingerprint=...)

Endorsement phase (per evaluating agent):
  Agent retrieves or composes endorsed_idea
  → endorsed_fingerprint = HDC(endorsed_idea)
  → publish Pulse(topic=consensus.agreement,
                  payload={proposal_id, endorsed_fingerprint, agent_id})

Aggregation phase:
  Consensus aggregator receives agreement Pulses
  → similarity = cosine(endorsed_fingerprint, proposal_fingerprint)
  → if similarity ≥ floor: count as substantive agreement
  → if similarity < floor: flag as nominal / divergent

Consensus decision:
  if substantive_agreement_count ≥ quorum_threshold:
    emit Pulse(topic=consensus.reached, proposal_id=..., strength=...)
  else:
    emit Pulse(topic=consensus.failed, proposal_id=..., divergence_summary=...)
```

---

## Invariants

1. The proposal fingerprint is computed once at submission and stored in Substrate. All
   endorsements are compared against this fixed fingerprint — it cannot change after publication.
2. An agent can submit only one endorsement Pulse per proposal per evaluation round.
3. The similarity floor is configurable per proposal kind. Proposals that require strong
   semantic alignment (e.g., safety checks) use a higher floor than exploratory brainstorming.
4. Divergent endorsements are not discarded — they are stored in Substrate as a disagreement
   record, which feeds the replication ledger (S4) and heuristics calibration (S2) if relevant.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| HDC fingerprint instability | The same idea produces different fingerprints on different runs due to encoding non-determinism | Use a deterministic HDC encoding pipeline; hash-based initialization with fixed seed |
| Semantic collisions | Two genuinely different ideas produce similar fingerprints (HDC false positive) | Set the similarity floor conservatively; use ensemble fingerprinting for high-stakes proposals |
| Sycophantic endorsement | Agents endorse the proposal fingerprint directly (copy-paste) rather than independently composing their view | Require agents to produce independent formulations before fingerprinting; flag identical fingerprints |
| Aggregator lag | High Pulse volume from many agents overwhelms the aggregator; consensus deadline is missed | Aggregator uses a sliding time window; after the deadline, computes consensus on available endorsements |
| Low participation | Few agents endorse; quorum is not reached | Distinguish "no consensus" from "no participation"; surface low-participation separately |

---

## Relationship to Other Synergies

- **S3** (c-factor × Bus × HDC): S3 uses HDC to detect whether system **outputs** are
  converging (bad). S9 uses HDC to detect whether agent **endorsements** are converging on the
  same idea (good). The fingerprint primitive serves opposite directional purposes.
- **S1** (Demurrage × HDC): Proposal Engrams are subject to demurrage like any other Engram.
  Proposals that never reach consensus decay out of Substrate without special treatment.
- **S4** (Replication ledger × Heuristics × paper Engram): Divergent endorsements (low
  similarity to the proposal) can be stored as counter-claims in the replication ledger —
  feeding S4's living-research loop.

---

## Today vs. Planned

**Today**: HDC fingerprints are computed for Engrams. `EventBus<E>` routes typed events. The
Consensus subsystem has stub structures. No agreement-Pulse type exists. No similarity-based
aggregation is implemented.

**Planned**: A consensus aggregator subscribes to agreement Pulses. Pulse types for endorsement
(with fingerprint payload) are defined. The aggregator computes cosine similarity per Pulse and
emits consensus-reached or consensus-failed events.

---

## Cross-References

- [`analysis/integration-map/verification-x-orchestration.md`](../integration-map/verification-x-orchestration.md) — M3: verification-orchestration edge where consensus signals are consumed
- [`analysis/readiness-audit/subsystem-verification.md`](../readiness-audit/subsystem-verification.md) — verification subsystem gaps
- [`analysis/synergy-map/synergy-03-cfactor-bus-hdc.md`](synergy-03-cfactor-bus-hdc.md) — S3: opposite use of HDC (diversity detection vs. agreement detection)
- [`analysis/synergy-map/synergy-04-replication-living-research.md`](synergy-04-replication-living-research.md) — S4: divergent endorsements feed the replication ledger
- [`analysis/synergy-map/99-master-synergy-table.md`](99-master-synergy-table.md) — synergy index

---

## Open Questions

- What is the right similarity floor for substantive agreement? Is 0.8 cosine similarity
  a reasonable default, or does it need to be domain-calibrated?
- Can the consensus aggregator detect structured disagreement (multiple clusters of
  semantically similar but mutually divergent endorsements) rather than just pass/fail?
- Should the consensus result Pulse carry the centroid fingerprint of the endorsing cluster
  so that downstream components can find which Engrams are most semantically aligned with
  the consensus?
