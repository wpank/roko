# Selective Knowledge Restore

> **v3 depth file** -- `/docs/v3/depth/36-lifecycle/selective-restore.md`
> Canonical source: v1 `docs/v1/17-lifecycle/08-selective-restore.md`
> Status: **Current** (knowledge restore with 0.85^N confidence decay wired
> through `roko-neuro`; quarantine-validate-adopt pipeline live)

---

## 1. Purpose

Selective restore is the final step of the four-step knowledge transfer
process:

```
BACKUP --> DELETE --> CREATE --> RESTORE
```

It imports knowledge from a predecessor's backup into a new agent's knowledge
store, with configurable filtering, confidence decay, and validation. Restore
is always selective -- the operator controls which knowledge types, confidence
thresholds, and decay rates apply.

---

## 2. Restore Commands

```bash
# Restore with default confidence decay (0.85x per generation)
roko knowledge restore ./backups/agent-V1St-2026-04-12.neuro

# Restore with custom confidence decay
roko knowledge restore ./backups/agent-V1St-2026-04-12.neuro --confidence-decay 0.7

# Restore only specific knowledge types
roko knowledge restore ... --types insight,causal_link

# Restore only high-confidence entries
roko knowledge restore ... --min-confidence 0.5

# Restore with maximum entry count (genomic bottleneck)
roko knowledge restore ... --max-signals 2048

# Dry-run: show what would be restored
roko knowledge restore ... --dry-run

# Restore with validation against current context
roko knowledge restore ... --validate
```

---

## 3. Generational Confidence Decay

The core mechanism that prevents blind inheritance. Every restored Signal's
confidence is multiplied by a decay factor:

```
effective_confidence = original_confidence * decay_rate ^ generation
```

Default decay rate: **0.85 per generation** (configurable).

This rate was chosen based on the genomic bottleneck research (Shuvaev et
al. 2024) and transgenerational epigenetic inheritance studies (Heard &
Martienssen 2014):

| Generation | Multiplier | From 0.9 | Interpretation |
|-----------|-----------|---------|----------------|
| G0 (original) | 1.000 | 0.900 | Full confidence in self-generated knowledge |
| G1 (first restore) | 0.850 | 0.765 | Slight skepticism |
| G2 | 0.723 | 0.650 | Moderate skepticism -- must validate more |
| G3 | 0.614 | 0.553 | Significant -- inherited knowledge is suggestions |
| G5 | 0.444 | 0.399 | Most inherited knowledge below active threshold |
| G10 | 0.197 | 0.177 | Only the most robust knowledge survives |
| G15 | 0.087 | 0.079 | Effectively zero -- ancestral knowledge absorbed |

This implements "survival of the flattest" (Bull et al. 2005) -- over many
generations, only robust, widely applicable knowledge persists. Fragile,
regime-specific knowledge fades naturally.

```rust
pub fn restore_confidence(
    original_confidence: f64,
    generation: u32,
    decay_rate: f64,
) -> f64 {
    if generation == 0 {
        return original_confidence;
    }
    let decayed = original_confidence * decay_rate.powi(generation as i32);
    decayed.max(0.01) // Floor at 0.01 to preserve provenance tracking
}
```

---

## 4. Restore Pipeline: Quarantine, Validate, Adopt

### Stage 1: Quarantine

All Signals from the backup are loaded into a quarantine buffer -- not
immediately added to the knowledge store:

- Filter by configured type filter.
- Filter by minimum confidence threshold.
- Apply `max_signals` limit if specified.
- Compute decayed confidence for each Signal.
- Tag with restore provenance (source agent, source generation, timestamp).

### Stage 2: Validate (Optional)

If `--validate` is specified, each quarantined Signal is checked:

1. **Schema validation**: Signal format matches current roko version.
2. **Contradiction detection**: Restored Signal contradicts existing Signals.
3. **Provenance verification**: BLAKE3 hash matches content.
4. **Regime tagging** (domain-specific): Regime-matched Signals receive a
   +0.1 confidence bonus (capped). Non-matching Signals are deprioritized.

### Stage 3: Adopt

Validated Signals are adopted into the knowledge store:

- Apply confidence decay.
- Set tier based on decayed confidence (never above Consolidated -- the
  Persistent tier requires independent validation by the current agent).
- Add restore provenance entry.
- Reset decay state (fresh Ebbinghaus curve from restore time).
- Insert into knowledge store.

**Tier assignment after decay:**

| Decayed confidence | Assigned tier |
|-------------------|---------------|
| >= 0.8 | Consolidated (max for restored knowledge) |
| >= 0.5 | Working |
| < 0.5 | Transient |

Restored Signals never start at Persistent. That tier is reserved for
knowledge repeatedly validated through the current agent's own experience.

---

## 5. PLAYBOOK.md Handling

The predecessor's `PLAYBOOK.md` (machine-evolved heuristics) receives special
treatment:

- **NOT automatically loaded** as active heuristics.
- Stored as a reference document accessible to the agent.
- Available for retrieval during planning but not automatically applied.
- The new agent develops its own `PLAYBOOK.md` through Dream consolidation.

This is the anti-proletarianization measure (Stiegler 2010): making inherited
heuristics available but not active forces the new agent to independently
validate and re-derive its own operational knowledge.

---

## 6. Knowledge Type Priorities

Different knowledge types have different restore priorities. When
`--max-signals` is specified and the backup exceeds the limit, compression
prioritizes in this order:

| Knowledge Type | Priority | Rationale |
|---------------|----------|-----------|
| Warning | Highest | Safety-critical: mistakes to avoid |
| AntiKnowledge | High | "What doesn't work" is often most valuable |
| CausalLink | High | Causal understanding transfers well across regimes |
| Insight | Medium | Useful but may be regime-specific |
| Heuristic | Medium | Practical but may be stale |
| StrategyFragment | Low | Most regime-specific, requires validation |

---

## 7. Restore Report

```
Restore Report
==============
Source agent:     agent-V1StGXR8_Z5j
Source generation: 2
This generation:   3
Confidence decay:  0.85 (effective: 0.614 = 0.85^3)

Signals processed:  12,847
  - Filtered (below threshold): 4,521
  - Quarantined:               8,326
  - Validated:                  8,201
  - Rejected (contradictions):    125
  - Adopted:                    8,201

By tier (after decay):
  - Transient:    5,421
  - Working:      2,102
  - Consolidated:   678
  - Persistent:       0 (requires independent validation)

PLAYBOOK.md: Stored as reference (not active)
```

---

## 8. Live Restore (Running Agent)

Knowledge can be restored into a running agent without deletion:

```bash
roko knowledge restore ./backups/other-agent.neuro --merge
roko knowledge restore ./backups/other-agent.neuro --merge --prefer-existing
roko knowledge restore ./backups/other-agent.neuro --merge --prefer-restored
```

Conflicts (same content hash, different scores) are resolved based on the
`--prefer` flag. Default: prefer existing (the running agent's knowledge is
assumed more current).

---

## 9. Cross-Agent and Cross-Domain Restore

Backups can be restored into agents with different configurations, domains,
and strategies. The Signal format is domain-agnostic.

Cross-domain knowledge transfer is possible through HDC (Hyperdimensional
Computing) structural analogy. Signals with HDC vectors can be matched across
domains based on structural similarity -- a causal pattern discovered in
financial markets might have an analogous pattern in code quality metrics.
Cross-domain transfer threshold: Hamming similarity >= 0.526 for 10,240-bit
BSC vectors.

---

## 10. Implementation Sources

| Surface | File | What |
|---------|------|------|
| `RestoreConfig` | `crates/roko-agent/src/lifecycle.rs` | Restore configuration |
| `restore_confidence()` | `crates/roko-agent/src/lifecycle.rs` | Generational decay |
| Knowledge store | `crates/roko-neuro/src/` | Quarantine, validation, adoption |
| CLI command | `crates/roko-cli/src/main.rs` | `roko knowledge restore` |

---

## Cross-References

- [knowledge-backup-export.md](knowledge-backup-export.md) -- Backup format
- [knowledge-transfer-via-mesh.md](knowledge-transfer-via-mesh.md) -- Live sharing
- [ebbinghaus-for-knowledge.md](ebbinghaus-for-knowledge.md) -- Decay on restored knowledge
- [new-agent-creation.md](new-agent-creation.md) -- Creating the successor agent
