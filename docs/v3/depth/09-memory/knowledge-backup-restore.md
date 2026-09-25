# Knowledge Backup and Restore

> **v3 depth -- 09-memory** | Source: v1/06-neuro/15

Users control knowledge lifecycle through explicit backup and restore with
genomic bottleneck selection. Restored entries start at Transient tier with
discounted confidence -- they must re-prove themselves in the new context.

---

## The Process

### Backup

Export selects the top entries by freshness score, preserving the most
valuable knowledge while shedding low-value entries. The selection mimics
a genomic bottleneck: only the fittest knowledge survives.

```bash
roko knowledge backup --output backup.json
```

**What gets exported:**
- All `KnowledgeEntry` objects (JSONL format)
- HDC vectors (binary, 1,280 bytes each)
- Tier metadata, provenance chain
- KnowledgeStats snapshot

**What does NOT get exported:**
- Episode logs (too large, too context-specific)
- Daimon PAD state (internal to the original agent)
- Active task state, credentials

### Restore

```bash
roko knowledge restore --input backup.json
```

On restore:
1. Each entry imported into new agent's NeuroStore
2. Confidence discounted by source channel discount (0.6x for ExternalApi)
3. Tier reset to **Transient** -- entries must re-prove themselves
4. Provenance updated with source agent ID and date
5. `created_at` preserved (original date, for decay computation)
6. HDC vectors preserved (same encoding)
7. Balance and tier are NOT preserved -- reset on restore

### Why entries start at Transient

1. **Context mismatch**: Knowledge from agent A may not work for agent B
2. **Staleness risk**: Backup may be days or months old
3. **Safety**: Prevents imported knowledge from immediately influencing
   critical decisions at full confidence

---

## Source Channel Discounting

| Channel | Discount | Rationale |
|---------|----------|-----------|
| `UserInput` | 1.00 | Fully trusted |
| `GateVerdict` | 0.95 | High but not perfect |
| `AgentOutput` | 0.80 | LLM outputs need verification |
| `ExternalApi` | 0.60 | External data may be unreliable |
| `DreamConsolidation` | 0.50 | Speculative knowledge |

---

## Mesh-Based Knowledge Sharing

### Collective sync

```toml
[neuro.mesh_sync]
enabled = true
sync_types = ["warning", "anti_knowledge"]
sync_interval = "1h"
min_confidence = 0.7
```

Mesh-synced entries enter with 0.80x discount at Transient tier.

---

## Academic Foundations

- Tulving, E. (1972). "Episodic and semantic memory." Academic Press.
- McClelland, J. L. et al. (1995). "Complementary learning systems."
  *Psychological Review*, 102(3).
- Nader, K. et al. (2000). "Reconsolidation." *Nature*, 406.

---

## Cross-References

- `four-validation-tiers.md` -- why Transient is the entry point
- `library-of-babel.md` -- five inflow channels and discounting
- `knowledge-query-api.md` -- NeuroStore ingest API
