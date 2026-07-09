---
title: "Lifecycle × Neuro"
section: analysis
subsection: integration-map
id: im-lifecycle-x-neuro
source: 24-cross-section-integration-map.md (§6.1 M20)
missing-integration: M20
tier: 3
tags: [lifecycle, neuro, backup, restore, knowledge-restore, session-continuity]
---

# Lifecycle × Neuro

**Direction**: 17-Lifecycle → 06-Neuro (knowledge restore from backup on agent initialization)  
**Status**: **Missing (M20)** — Tier 3, ~100 LOC. Backup format is defined; restore path for NeuroStore does not exist.  
**Interface**: `roko-lifecycle::BackupRestore` → `roko-neuro::NeuroStore` initialization

## What Flows

When a Roko agent is restored from backup (e.g., moved to new machine, restored from disaster recovery), the knowledge accumulated in NeuroStore should be restored along with configuration and episode history.

| Signal | From | To | Status |
|---|---|---|---|
| Backup archive (`.roko/backup/`) | Lifecycle restore command | `roko-neuro::NeuroStore` population | **Missing** (M20) |
| Knowledge entry format | NeuroStore serialization | Backup archive | **Specified but not wired** |
| Restore validation | Lifecycle | NeuroStore integrity check | **Missing** |

## Wiring Recipe

```rust
// In roko-lifecycle restore operation:
pub async fn restore_knowledge_store(
    backup_path: &Path,
    neuro_store: &mut NeuroStore,
) -> Result<RestoreReport> {
    let knowledge_backup = backup_path.join("neuro/knowledge.jsonl");
    
    if knowledge_backup.exists() {
        let entries = KnowledgeEntry::deserialize_jsonl(&knowledge_backup)?;
        let mut restored = 0;
        
        for entry in entries {
            // Skip expired entries
            if !entry.is_expired() {
                neuro_store.put(entry).await?;
                restored += 1;
            }
        }
        
        Ok(RestoreReport { restored, skipped_expired: entries.len() - restored })
    } else {
        Ok(RestoreReport::empty())  // Fresh start
    }
}
```

Estimated LOC: ~100.

## Invariants of the Interaction

1. Restore is additive — it does not overwrite existing knowledge entries; it merges.
2. Expired entries (past their decay deadline) are not restored.
3. Restore validates content hashes before loading entries.
4. A restore report is logged so operators know what was recovered.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Corrupt backup | Partial knowledge restore | Validate each entry before inserting; skip corrupt entries |
| Version mismatch | Old format entries incompatible | Migration layer for format upgrades |
| No backup exists | Fresh start (expected) | Log info; not an error |

## Open Questions

1. Should PAD state (Daimon) also be restored alongside knowledge? Current scope is NeuroStore only.
2. Should episode history be restored? Episodes are large; selective restore (last N episodes) may be preferable.

## Cross-References

- Dreams connectivity: [dreams-x-neuro.md](./dreams-x-neuro.md) — M7 (Dreams also writes to NeuroStore; restore must not duplicate dream-consolidated entries)
- Readiness audit: [RA-17: Lifecycle](../readiness-audit/subsystem-lifecycle.md), [RA-06: Neuro](../readiness-audit/subsystem-neuro.md)
