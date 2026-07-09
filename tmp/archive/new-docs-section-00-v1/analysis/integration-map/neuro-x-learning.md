---
title: "Neuro × Learning"
section: analysis
subsection: integration-map
id: im-neuro-x-learning
source: 24-cross-section-integration-map.md (§3.2, §3.3, §4.1)
tags: [neuro, learning, episodes, knowledge-promotion, partial-wired]
---

# Neuro × Learning

**Direction**: 05-Learning → 06-Neuro (episodes and skills promoted to knowledge); 06-Neuro → 05-Learning (knowledge patterns inform learning)  
**Status**: **Partial** — Episode flow to NeuroStore is partially wired; HDC fingerprinting of episodes exists; full knowledge promotion pipeline incomplete  
**Interface**: `roko-learn::EpisodeLog` ↔ `roko-neuro::NeuroStore`

## What Flows

| Signal | From | To | Status |
|---|---|---|---|
| `Kind::Episode` (HDC fingerprinted) | `EpisodeLog` | `NeuroStore` | **Partial** |
| `Kind::Skill` (promoted from skill library) | `SkillLibrary` | `NeuroStore` (as CausalLink or StrategyFragment) | **Missing** |
| `Kind::Heuristic` (pattern-mined from episodes) | `PatternMiner` | `NeuroStore` | **Partial** |
| HDC fingerprint index | `NeuroStore` | Episode retrieval for Dreams | **Partial** |

## Current Wiring

Episodes are written to `episodes.jsonl` with HDC fingerprints (via `bardo-primitives`). The NeuroStore can query by fingerprint similarity. However:
- PatternMiner runs periodically but does not always promote findings to NeuroStore
- Skills from the skill library are not promoted to NeuroStore knowledge types
- The tier field on `KnowledgeEntry` is missing (Readiness Audit G10)

## Enhancement Opportunities

- [dreams-x-neuro.md](./dreams-x-neuro.md) — M7: Dream consolidation should produce NeuroStore entries from episodes
- [learning-x-composition.md](./learning-x-composition.md) — M4: Skills should also be HDC-indexed in NeuroStore

## Cross-References

- Readiness audit: [RA-05: Learning](../readiness-audit/subsystem-learning.md), [RA-06: Neuro](../readiness-audit/subsystem-neuro.md)
