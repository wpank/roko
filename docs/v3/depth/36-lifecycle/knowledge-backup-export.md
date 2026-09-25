# Knowledge Backup and Export

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/knowledge-backup-export.md`
> Canonical source: v1 `docs/v1/17-lifecycle/05-knowledge-backup-export.md`
> Status: **Current** (knowledge backup/restore with genomic bottleneck and
> 0.85^N confidence decay wired through `roko-neuro`; `roko knowledge backup`
> and `roko knowledge restore` CLI commands live)

---

## 1. Purpose

Knowledge backup replaces the legacy "death testament" system. Instead of an
agent producing a compressed knowledge artifact at death, the operator
initiates backups at any time via explicit CLI commands. The backup captures
the agent's entire knowledge store -- all Signals with their scores, decay
state, knowledge tier, provenance chains, and metadata -- in a portable format
that can be restored into a different agent.

This is the first step of the four-step knowledge transfer process:

```
BACKUP --> DELETE --> CREATE --> RESTORE
```

Each step is user-initiated, explicit, and reversible (except DELETE, which
is intentionally irreversible for the agent process -- the backup persists).

---

## 2. Backup Commands

```bash
# Full knowledge backup to default location
roko knowledge backup

# Backup to a specific path
roko knowledge backup --output ./backups/agent-V1St-2026-04-12.neuro

# Backup with compression
roko knowledge backup --compress

# Backup with encryption (operator's key)
roko knowledge backup --encrypt --key-file ~/.roko/backup.key

# Backup only specific knowledge types
roko knowledge backup --types insight,heuristic,causal_link

# Backup only entries above a confidence threshold
roko knowledge backup --min-confidence 0.3

# Dry-run: show what would be backed up
roko knowledge backup --dry-run
```

### Default Backup Location

```
.roko/backups/{agent_id}/{timestamp}.neuro
```

---

## 3. Archive Structure

The backup is a content-addressed archive. Each Signal is stored as a
self-contained file, content-addressed by BLAKE3 hash:

```
{agent_id}-{timestamp}.neuro
+-- manifest.toml          # Backup metadata
+-- signals/
|   +-- {hash1}.signal      # Individual Signal files (BLAKE3 addressed)
|   +-- {hash2}.signal
+-- scores/
|   +-- scores.jsonl        # All 7-axis scores per Signal
+-- tiers/
|   +-- tiers.jsonl         # Tier assignments (Transient/Working/Consolidated/Persistent)
+-- provenance/
|   +-- provenance.jsonl    # Lineage chains and source attribution
+-- decay/
|   +-- decay_state.jsonl   # Current decay state (Ebbinghaus parameters)
+-- playbook.md             # Machine-evolved heuristics snapshot
+-- checksum.blake3         # BLAKE3 hash of entire archive
```

### Manifest

```toml
[backup]
version = 1
agent_id = "agent-V1StGXR8_Z5j"
created_at = "2026-04-12T14:30:00Z"
roko_version = "0.1.0"

[stats]
total_signals = 12847
average_confidence = 0.63
median_confidence = 0.58

[provenance]
agent_lifetime_hours = 342.5
total_cognitive_loops = 48721
gate_pass_rate = 0.73
```

---

## 4. Backup Integrity

Every backup includes a BLAKE3 checksum of the entire archive. On restore,
the checksum is verified before any Signals are loaded. If verification fails,
the restore is aborted:

```rust
pub fn verify_backup(path: &Path) -> Result<BackupManifest, BackupError> {
    let archive = read_archive(path)?;
    let computed_hash = blake3::hash(&archive.raw_bytes);
    let expected_hash = read_checksum(path)?;
    if computed_hash != expected_hash {
        return Err(BackupError::IntegrityCheckFailed { expected, computed });
    }
    parse_manifest(&archive)
}
```

---

## 5. Automatic Backup Policies

Operators can configure automatic periodic backups:

```toml
[neuro.backup]
schedule = "0 */6 * * *"      # Every 6 hours (cron syntax)
max_backups = 10               # Retention limit
path = ".roko/backups/"
compress = true
```

Automatic backups are insurance: they ensure that even if the operator forgets
to back up before deleting, recent knowledge is available.

---

## 6. Genomic Bottleneck: Compressed Backups

For situations where the full backup is too large or the operator wants to
transfer only the most valuable knowledge, compressed backup applies the
genomic bottleneck principle (Shuvaev et al. 2024):

```bash
roko knowledge backup --compressed --max-signals 2048
```

The compression algorithm:

1. **25% reserved** for priority inclusions: all Warning-type Signals (safety
   knowledge) and all Persistent-tier Signals (repeatedly validated).
2. **50% allocated** to diversity-sampled top Signals across all knowledge
   types -- ensuring coverage of Insight, Heuristic, CausalLink,
   StrategyFragment, and AntiKnowledge.
3. **25% filled** with highest-scored Signals regardless of type -- raw
   quality selection.

This mirrors the biological genomic bottleneck: the human genome is ~1,000x
smaller than the information required to specify brain connectivity, yet
organisms are born with sophisticated innate behaviors. Critically, neural
networks compressed through a genomic-scale bottleneck exhibit enhanced
transfer learning to novel tasks (Shuvaev et al. 2024) -- the compression
is a regularizer that strips regime-specific overfitting while preserving
generalizable knowledge.

---

## 7. What Is NOT Backed Up

The backup captures knowledge (what the agent has learned), not state (what
the agent is doing):

- **Agent process state**: Running tasks, in-flight requests, temporaries.
- **Affect state**: Current PAD vector, behavioral state (transient by design).
- **Dream journal**: Current cycle state (dreams consolidate into Signals).
- **Coordination connections**: Peer topology, gossip state (re-established).
- **Configuration**: `roko.toml`, `STRATEGY.md` (operator-managed, backed up
  separately).
- **Session UI state**: Command-palette history, open views, checkpoints.

A new agent created from a backup starts with the predecessor's knowledge but
its own fresh operational state -- including a fresh affect state that adapts
to current conditions rather than carrying over stale emotional context.

---

## 8. Session Export Is Separate

Knowledge backup is distinct from session export:

| Artifact | Purpose | Command |
|----------|---------|---------|
| Knowledge backup | Durable knowledge transfer between agents | `roko knowledge backup` |
| Session export | Replay or share operator-visible run history | `roko history` |

Both are exportable; only the knowledge backup is the canonical preservation
artifact in lifecycle operations.

---

## 9. Implementation Sources

| Surface | File | What |
|---------|------|------|
| Backup types | `crates/roko-agent/src/lifecycle.rs` | `BackupManifest`, `BackupRecord` |
| Knowledge store | `crates/roko-neuro/src/` | Signal storage, BLAKE3 hashing |
| CLI commands | `crates/roko-cli/src/main.rs` | `roko knowledge backup/restore` |
| Genomic bottleneck | `crates/roko-neuro/src/` | Compressed backup selection |

---

## Cross-References

- [agent-deletion-8-step.md](agent-deletion-8-step.md) -- Back up before deleting
- [selective-restore.md](selective-restore.md) -- Restore into a new agent
- [new-agent-creation.md](new-agent-creation.md) -- Create successor agent
- [ebbinghaus-for-knowledge.md](ebbinghaus-for-knowledge.md) -- Decay on restored knowledge
