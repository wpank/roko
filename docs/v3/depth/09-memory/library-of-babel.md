# Library of Babel: Cross-Collective Knowledge

> **v3 depth -- 09-memory** | Source: v1/06-neuro/14

The Library of Babel is the cross-collective knowledge exchange layer -- how
agents in different collectives (and on the public chain) share, discover,
and import knowledge with confidence discounting and publishing policies.

---

## Three-Level Knowledge Architecture

```
+----------------------------------------------------+
|            Chain (Global Public)                     |
|  On-chain HDC vectors, collective knowledge          |
+----------------------------------------------------+
|            Agent Mesh (Peer / Private)               |
|  WebSocket / P2P connections, permissioned subnets   |
+----------------------------------------------------+
|            Local Neuro Store (Private)               |
|  Per-agent knowledge, JSONL + HDC indexing            |
+----------------------------------------------------+
```

---

## Five Inflow Channels

### 1. Self-Distillation (highest trust)

Knowledge distilled from the agent's own episodes.
- **Entry tier**: Transient
- **Confidence discount**: None (1.0x)

### 2. Collective Mesh Sync

Knowledge shared by other agents in the same permissioned collective.
- **Entry tier**: Transient
- **Confidence discount**: 0.80x

### 3. Public Chain (Marketplace)

Knowledge published by agents outside the local collective.
- **Entry tier**: Transient
- **Confidence discount**: 0.60x

### 4. User-Directed Restore

Knowledge imported from a backup, directed by the user.
- **Entry tier**: Transient
- **Confidence discount**: 0.85x

### 5. Cross-Collective Exchange

Knowledge exchanged through negotiated sharing agreements.
- **Entry tier**: Transient
- **Confidence discount**: 0.50x

---

## Confidence Discounting

```
imported_confidence = original_confidence * source_discount_factor
```

### Inheritance discounting

When knowledge transfers through a lineage (A -> B -> C):
```
confidence_after_N_transfers = original_confidence * 0.85^N
```

After 5 transfers: `0.85^5 = 0.444` -- less than half.

---

## Publishing Policies

### What gets published

- Non-alpha Insights (general observations)
- General Heuristics (universally useful rules)
- Validated Warnings (safety as a public good)
- AntiKnowledge (false beliefs should be widely known)

### What stays private

- Proprietary strategies
- Private data (API keys, credentials)
- Alpha-generating signals

### Configuration

```toml
[neuro.publishing]
auto_publish = true
publish_to = "mesh"
publish_types = ["insight", "heuristic", "warning", "anti_knowledge"]
exclude_tags = ["proprietary", "internal", "alpha"]
min_confidence = 0.7
min_tier = "consolidated"
```

---

## Ingestion Safety

Four-stage pipeline:

1. **QUARANTINE**: Isolate, HDC check against known-bad patterns, discount
2. **CONSENSUS**: If from collective, verify multiple agents agree
3. **SKILL SANDBOX**: Test StrategyFragments/Heuristics in sandbox
4. **ADOPT**: Admit at Transient tier

**Immune memory**: LSH Bloom filter of previously rejected entries prevents
persistent re-injection of bad knowledge.

---

## Academic Foundations

- Borges, J. L. (1941). "La biblioteca de Babel."
- Grasse, P.-P. (1959). "Stigmergy." *Insectes Sociaux*, 6(1).
- Woolley, A. W. et al. (2010). "Collective Intelligence Factor." *Science*.

---

## Cross-References

- `four-validation-tiers.md` -- why imported entries start at Transient
- `antiknowledge-challenge.md` -- challenge mechanism on imported knowledge
- `knowledge-backup-restore.md` -- user-directed restore (channel 4)
