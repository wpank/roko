# S1 — Demurrage × HDC → Self-trimming semantic memory

> Demurrage charges holding cost over time. HDC makes novelty measurable by comparing a record
> to its nearest neighbors. Together they make Substrate memory economically selective: unique
> and frequently-used records survive; redundant or stale ones decay away.

**Status**: Analysis — target-state synergy  
**Primitives involved**: P6 Demurrage × P5 HDC (with P1 Engram and P4 Substrate as substrate)  
**Reality check**: P5 HDC fingerprinting is partially built; P6 Demurrage is Specified (no
shipped implementation). The full synergy is target-state.  
**Last reviewed**: 2026-04-19

---

## Primitives Involved

| Primitive | Role in this synergy |
|---|---|
| [P6 Demurrage](../../reference/) | Imposes a holding cost on each Engram over time, creating economic pressure to evict low-value records |
| [P5 HDC fingerprint](../../reference/) | Encodes each Engram as a high-dimensional vector; nearest-neighbor distance measures semantic novelty |
| [P1 Engram](../../reference/01-engram/) | The unit of memory that carries both a demurrage balance and an HDC fingerprint |
| [P4 Substrate](../../reference/03-substrate/) | The store that evaluates eviction pressure and executes pruning |

---

## What the Synergy Unlocks

Without HDC, demurrage becomes a blunt tax: it evicts by age or access count, with no concept
of uniqueness. A cluster of nearly-identical records would survive at the same rate as a single
unique record — because the holding cost is per-record, not per-semantic-unit.

Without demurrage, HDC becomes an expensive search primitive with no pruning force. The system
can identify which records are semantically similar, but has no mechanism to act on that
similarity by evicting duplicates.

Together the two primitives create a **"unique-and-used" pressure**:

1. Each Engram carries an HDC fingerprint computed at write time.
2. Demurrage evaluates holding cost as a function of both time-since-access and semantic
   redundancy (distance to nearest neighbors).
3. Records that are semantically close to many other records pay higher effective holding cost —
   the system is already covered.
4. Records that are semantically distant (unique) but also frequently accessed carry negative or
   near-zero effective holding cost — the system needs them.
5. Pruning runs continuously. The Substrate does not wait for an explicit cleanup pass.

The practical result: memory gradually favors uniquely useful records rather than raw
accumulation. The longer the system runs, the more semantically diverse its Substrate becomes —
not because diversity was enforced as a rule, but because redundancy became expensive.

### The economic intuition

Demurrage without HDC = a tax on age.  
HDC without demurrage = a similarity index with no teeth.  
Demurrage + HDC = a tax on **semantic redundancy** — which is exactly what makes memory
economically selective.

---

## What Flows

```
Write path:
  new Engram → compute HDC fingerprint → store (Engram + fingerprint) → Substrate

Holding-cost evaluation (continuous):
  Substrate.query_similar(fingerprint, k) → nearest-neighbor distances
  → effective_holding_cost = base_demurrage_rate × (1 / avg_distance_to_k_nearest)
  → balance(Engram) -= effective_holding_cost × elapsed_time

Pruning path:
  balance(Engram) falls below threshold → evict Engram from Substrate
```

Note: this data-flow is target-state. Today the Substrate prunes by age and access count via
the `pruning` module; neither demurrage balances nor HDC-weighted eviction are shipped.
See [`reference/03-substrate/06-pruning.md`](../../reference/03-substrate/06-pruning.md).

---

## Invariants

1. Every Engram that enters Substrate carries an HDC fingerprint. There is no "unfingerprinted"
   record in the target-state design.
2. Holding cost is always non-negative. Demurrage cannot create artificial credits.
3. Pruning can only decrease the total record count, never increase it. The self-trimming
   pressure is one-directional.
4. A record with zero nearest neighbors (fully unique) pays only the base demurrage rate.
5. The novelty score that informs demurrage is recomputed lazily — it is not recalculated on
   every write, only when a pruning pass runs or when the neighborhood changes significantly.

---

## Failure Modes

| Failure | Mechanism | Mitigation |
|---|---|---|
| Over-pruning unique records | HDC distance metric poorly calibrated; semantically distinct records appear close | Tune distance threshold; add a minimum survival time before demurrage applies |
| Under-pruning redundant records | Demurrage rate too low relative to write rate | Configure rate as a function of Substrate capacity; raise rate when near capacity limit |
| HDC fingerprint drift | Encoding model updated mid-session; old fingerprints incomparable to new ones | Version fingerprints; re-fingerprint on encoding model change before next pruning pass |
| Pruning hot paths first | Frequently-accessed records with high semantic overlap still evicted | Add access-recency bonus that temporarily lowers effective holding cost |
| Substrate fragmentation | Rapid pruning leaves the index sparse in ways that degrade query performance | Compact the fingerprint index after major pruning passes |

---

## Today vs. Planned

**Today**: Substrate provides age- and access-count-based pruning
([`reference/03-substrate/06-pruning.md`](../../reference/03-substrate/06-pruning.md)).
HDC fingerprints exist as a data field but are not yet used as a pruning signal.

**Planned**: Demurrage (P6) ships as a trait / balance mechanism. Substrate's pruning module is
extended to query the fingerprint index for nearest-neighbor distances and feed them into the
holding-cost calculation. The two primitives become co-dependent at the pruning boundary.

---

## Cross-References

- [`analysis/architectural-analysis/08-novel-proposals.md`](../architectural-analysis/08-novel-proposals.md) — Demurrage + HDC appears as a novel proposal in the coherence analysis
- [`analysis/readiness-audit/subsystem-neuro.md`](../readiness-audit/subsystem-neuro.md) — HDC readiness status
- [`analysis/synergy-map/synergy-08-demurrage-heuristic-relearning.md`](synergy-08-demurrage-heuristic-relearning.md) — S8 also uses demurrage; compare scope
- [`analysis/synergy-map/99-master-synergy-table.md`](99-master-synergy-table.md) — synergy index

---

## Open Questions

- What is the right distance metric for "semantic redundancy"? Cosine distance on raw HDC
  vectors, or something normalised for vector density?
- Should access-recency be a first-class factor in the holding-cost formula, or a separate
  policy layered on top?
- How should the system handle records written before HDC fingerprinting was enabled — assign
  a default fingerprint, or exempt them from novelty-weighted demurrage?
