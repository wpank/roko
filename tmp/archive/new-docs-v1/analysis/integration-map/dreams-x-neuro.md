---
title: "Dreams × Neuro"
section: analysis
subsection: integration-map
id: im-dreams-x-neuro
source: 24-cross-section-integration-map.md (§6.1 M7, §3.2, §7.3, §8.3)
missing-integration: M7
tier: 2
tags: [dreams, neuro, consolidation, NREM-replay, knowledge-promotion, event-log-projection]
---

# Dreams × Neuro

**Direction**: 10-Dreams → 06-Neuro (consolidated knowledge from NREM replay); 06-Neuro → 10-Dreams (episodes for replay input, partial)  
**Status**: **Missing (M7)** — Tier 2, ~200 LOC. Highest-complexity Tier 2 item.  
**Interface**: `roko-dreams::DreamRunner` → `roko-neuro::NeuroStore` (knowledge promotion)

## What Flows

Dreams processes accumulated episodes during idle periods (Delta speed) and extracts consolidated knowledge — patterns, causal links, strategy fragments — that should be promoted into NeuroStore for future use. Currently, DreamRunner runs but its output is not consumed by NeuroStore.

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::Episode` (input) | `roko-learn::EpisodeLog` | `DreamRunner` | **Partial** — files exist; no cursor mechanism |
| `Kind::DreamInsight` (consolidated knowledge) | `DreamRunner` | `NeuroStore` | **Missing** (M7) |
| `Kind::CausalLink` (extracted from replay) | `DreamRunner` | `NeuroStore` | **Missing** |
| `Kind::StrategyFragment` (from REM) | `DreamRunner` | `NeuroStore` | **Missing** (requires REM impl) |
| Depotentiation signal | `DreamRunner` | `roko-daimon` | **Missing** (M18) |

## The Core Design: Dreams as Event-Log Projection

**Proposal** (source file 24, §7.3): Treat Dreams as an event log projection — a background consumer that maintains a cursor into `episodes.jsonl` and processes episodes in order, emitting consolidated knowledge back to NeuroStore.

```rust
pub struct DreamProjection {
    /// Last processed episode offset (persisted to .roko/learn/dream-cursor.json)
    cursor: u64,
    /// Accumulated episodes since last consolidation
    pending_episodes: Vec<Episode>,
    /// Consolidation trigger threshold
    min_episodes_for_nrem: usize,  // default: 20
}

impl DreamProjection {
    /// Process new episodes from the log
    pub fn catch_up(&mut self, episode_log: &EpisodeLog) -> Vec<ConsolidatedKnowledge> {
        let new_episodes = episode_log.read_from(self.cursor);
        self.pending_episodes.extend(new_episodes);
        self.cursor = episode_log.latest_offset();

        if self.pending_episodes.len() >= self.min_episodes_for_nrem {
            let consolidated = self.run_nrem_cycle();
            self.pending_episodes.clear();
            consolidated
        } else {
            vec![]
        }
    }
}
```

**Integration points**:
- `orchestrate.rs` calls `dream_projection.catch_up()` when an agent idle period begins.
- Consolidated knowledge feeds into NeuroStore (closing the Dreams→Neuro gap).
- The cursor file enables crash recovery — Dreams picks up where it left off.

The cursor-based design is inspired by the MemOS lifecycle model (arXiv:2507.03724).

## Invariants of the Interaction

1. Dreams only reads from `EpisodeLog` — it does not modify the episode log.
2. Consolidated knowledge is added to NeuroStore with appropriate decay (knowledge lives longer than episodes).
3. The DreamProjection cursor must be persisted atomically — cursor advances only after successful NeuroStore write.
4. If NREM cycle fails, cursor does not advance — episodes are reprocessed on next run.
5. Dream consolidation runs at Delta speed (hours); it must not block Gamma or Theta execution.

## Failure Modes

| Failure | Consequence | Detection |
|---|---|---|
| Cursor lost | Dreams reprocesses all episodes from start | Detect duplicate knowledge entries; deduplicate by content hash |
| NREM cycle produces no insights | No NeuroStore update; cursor advances | This is valid; log dry-run rate |
| Episode log unavailable | Dreams cannot catch up | Retry on next idle period |
| Mattar-Daw utility scoring not implemented | Replay not prioritized by utility | Current fallback: recency-based selection |
| NeuroStore write fails | Consolidated knowledge lost; cursor held | Retry loop with exponential backoff |

## Observed Metrics

Expected after implementation:
- Knowledge entries created per dream cycle
- Dream cycle frequency and duration
- Knowledge promotion rate (new entries per episode batch)
- Episodes pending processing (backlog depth)

## Open Questions

1. Should the Mattar-Daw prioritization (replay by expected utility) be implemented as part of M7 or as a follow-up (Readiness Audit G15)?
2. Should DreamProjection run in the same process as the orchestrator, or as a separate daemon?
3. How does M7 interact with M18 (Dreams→Daimon depotentiation)? Should they be implemented together?

## Cross-References

- Depotentiation: [dreams-x-daimon.md](./dreams-x-daimon.md) — M18 (second Dreams cross-cut output)
- Cross-cut triangle: [00-overview.md](./00-overview.md) §8.3 — the 6 natural transformations between cross-cuts
- Architectural finding: [AA-06: Cross-Cut Isolation](../architectural-analysis/06-finding-crosscut-isolation.md) — Dreams direct import of Neuro
- Readiness audit: [RA-10: Dreams](../readiness-audit/subsystem-dreams.md), [RA-06: Neuro](../readiness-audit/subsystem-neuro.md)
- Synergy: [../synergy-map/synergy-07-dreams-retroactive.md](../synergy-map/synergy-07-dreams-retroactive.md) — Dreams × Substrate × Pulse = retroactive insight
