# S7 — Dreams × Substrate × Pulse → Retroactive insight

> Dreams read durable records from Substrate, reinterpret them under updated priors, and publish
> new Pulses that refresh downstream caches and composers. Old episodes remain actionable, but
> only after the system has grown enough to reinterpret them. The system does not merely
> remember — it re-learns from memory.

**Status**: Analysis — target-state synergy  
**Primitives involved**: Dreams subsystem × P4 Substrate × P2 Pulse  
**Reality check**: P4 Substrate is Shipping. P2 Pulse and the generalized Bus trait are
target-state. The Dreams subsystem is Scaffold. Full retroactive reinterpretation is
target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| Dreams subsystem | The offline-processing layer that reads historical Engrams from Substrate and applies updated models and priors to reinterpret past episodes |
| [P4 Substrate](../../reference/03-substrate/) | The durable store that holds the historical Engrams Dreams reads from |
| [P2 Pulse](../../reference/02-pulse/) | The ephemeral wire medium through which Dreams publishes its reinterpretation outputs to refresh downstream caches and composers |

---

## What the Synergy Unlocks

### The static-memory problem

Most systems that record experiences treat memory as a read-only archive. Past episodes are
stored correctly but are never reinterpreted as the system's models improve. An episode that
was "unremarkable" when it occurred cannot become "highly significant" in light of later
knowledge — because nobody revisits it.

This is a fundamental ceiling on learning-from-experience. The system can learn from new
experiences but not from re-evaluating old ones.

### How it works

1. The Dreams subsystem runs during low-load periods (or on a scheduled cadence). It queries
   Substrate for Engrams that satisfy a reinterpretation criterion: episodes older than a
   minimum age, episodes not yet re-evaluated under the current model version, or episodes
   flagged by the heuristics calibration loop as potentially more informative than originally
   assessed.
2. Dreams applies updated priors, updated heuristics, and updated embedding models to each
   retrieved Engram. It produces a reinterpretation: a new assessment of what the episode means
   given what the system now knows.
3. If the reinterpretation materially changes the episode's significance, Dreams publishes a
   Pulse on the Bus: "episode E now has significance S under current priors — refresh downstream
   caches that depend on E."
4. Downstream components that subscribe to the reinterpretation topic — composers, routing
   layers, HDC index updaters — receive the Pulse and update their local state accordingly.

The practical consequence: old episodes remain actionable. They do not sit in Substrate as
inert archives — they are candidates for reinterpretation whenever the system's models have
grown enough to extract new value from them.

### Why each primitive is necessary

- Without Substrate, there is no durable archive to read. Dreams can only act on the current
  session.
- Without Pulse + Bus, Dreams can reinterpret but has no way to communicate that reinterpretation
  to the rest of the system in real time. Its findings would require a full cache rebuild instead
  of targeted updates.
- Without Dreams (the reinterpretation engine), Substrate holds a rich archive and Pulse carries
  live events, but no component bridges the two by applying current knowledge to old material.

---

## What Flows

```
Scheduled Dreams pass:
  Substrate.query(filter=ReinterpretationCandidate) → batch of Engrams

Per-Engram reinterpretation:
  Dreams.reinterpret(engram, current_priors, current_heuristics)
  → new_significance: Option<SignificanceScore>

If significant change:
  publish Pulse(topic=reinterpretation.updated,
               payload={engram_id, old_significance, new_significance, basis})

Downstream refresh (subscribers):
  Composer cache → update relevance weights for engram_id
  HDC index → recompute fingerprint if embedding model changed
  Routing layer → update priority for engrams in active context windows
```

---

## Invariants

1. Dreams never modifies the original Engram in Substrate. Reinterpretation produces a new
   record (a ReinterpretationEngram) that references the original; the original is immutable.
2. Reinterpretation Pulses are idempotent: receiving the same Pulse twice produces the same
   downstream state. This allows safe replay if downstream subscribers miss a Pulse.
3. Dreams runs at a lower resource priority than live-session processing. It backs off when
   the system is under active cognitive load.
4. Only Engrams that have passed a minimum age threshold are eligible for reinterpretation.
   Very recent episodes are not reinterpreted because the system's models have not had time
   to change meaningfully.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| Reinterpretation explosion | Every Engram is re-evaluated on every model update; compute explodes | Use model-version tagging on Engrams; only re-evaluate Engrams whose relevant model components changed |
| Cache invalidation storm | Reinterpretation Pulses flood the Bus faster than downstream subscribers can process | Rate-limit Pulse emission from Dreams; batch related reinterpretations into a single Pulse |
| Spurious reinterpretation | Small model changes produce noisy significance changes that create false update traffic | Only emit Pulse if significance delta exceeds a material threshold |
| Circular reinterpretation | A reinterpretation Engram is itself re-evaluated on the next Dreams pass | Exclude ReinterpretationEngrams from the reinterpretation candidate query |
| Model rollback | The system's models are rolled back; reinterpretation Engrams based on the old model become stale | Tag reinterpretation Engrams with model version; invalidate on rollback |

---

## Relationship to Other Synergies

- **S1** (Demurrage × HDC): Dreams may update HDC fingerprints if the embedding model changes.
  An Engram's reinterpreted fingerprint changes its novelty score and therefore its demurrage
  pressure. Retroactive reinterpretation can rescue records that were about to be evicted.
- **S4** (Replication ledger × Heuristics × paper Engram): Dreams can re-evaluate research
  papers under updated models. If a paper Engram is reinterpreted as more significant, its
  claims may be re-elevated in the ledger.
- **S2** (Heuristics × Pulse × Bus): Dreams publishes reinterpretation Pulses on the same Bus
  that S2's calibration loop uses. The two streams are separate topics but share the transport.

---

## Today vs. Planned

**Today**: Dreams is a Scaffold subsystem with stub structures. Substrate is Shipping and holds
historical Engrams. `EventBus<E>` is Built but there is no "reinterpretation" topic or
subscriber pattern.

**Planned**: Dreams gains a scheduled pass that reads Substrate via a ReinterpretationCandidate
query. Pulse types for reinterpretation output are defined. Downstream caches in the Composer
and Routing layers subscribe to the reinterpretation topic.

---

## Cross-References

- [`analysis/integration-map/dreams-x-neuro.md`](../integration-map/dreams-x-neuro.md) — M7: Dreams × Neuro integration edge
- [`analysis/integration-map/dreams-x-daimon.md`](../integration-map/dreams-x-daimon.md) — M18: Dreams × Daimon integration edge
- [`analysis/readiness-audit/subsystem-dreams.md`](../readiness-audit/subsystem-dreams.md) — Dreams readiness and gaps
- [`analysis/synergy-map/synergy-01-demurrage-x-hdc.md`](synergy-01-demurrage-x-hdc.md) — S1: demurrage interacts with reinterpreted fingerprints
- [`analysis/synergy-map/synergy-04-replication-living-research.md`](synergy-04-replication-living-research.md) — S4: paper reinterpretation pathway
- [`analysis/synergy-map/99-master-synergy-table.md`](99-master-synergy-table.md) — synergy index

---

## Open Questions

- What is the right trigger for a Dreams pass: model version bump, scheduled cadence, idle
  detection, or explicit request?
- How many levels deep can reinterpretation go? Can a reinterpretation Engram trigger a
  second reinterpretation of its own source Engram?
- Should Dreams have write access to Substrate for its ReinterpretationEngrams, or should it
  publish outputs exclusively through the Bus and let downstream components decide whether to
  persist them?
