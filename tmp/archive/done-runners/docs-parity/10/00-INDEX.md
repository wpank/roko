# 10-Dreams Parity Refresh

Refresh target for `docs/10-dreams/` against the live runtime in
`crates/roko-dreams/`, the CLI/daemon surfaces that invoke it, and the
supporting learning/neuro infrastructure it already consumes.

Generated: 2026-04-18

---

## Batch Posture

Topic `10` is a status catch-up pass, not a redesign pass.

The audit correction is straightforward:

- the dreams runtime is ahead of the docs,
- Doc 16 is the main stale status artifact,
- several phase docs still describe old `roko-golem` ownership,
- and the research halo around dreams is much larger than the shipping surface.

Use this topic in three buckets only:

1. **Shipping runtime**
   - `DreamRunner`, `DreamCycle`, `DreamBudget`, `DreamSchedulePolicy`
   - idle / scheduled / manual triggers
   - `DreamHeartbeatPolicy` and daemon/orchestrator polling
   - replay planning with `DreamReplayMode` and `utility_score`
   - REM counterfactual imagination and creativity modes
   - `HypnagogiaEngine`
   - threat simulation
   - persisted `DreamCycleReport`s

2. **Shipping support, mixed wiring**
   - `PatternMiner`, `CrossEpisodeConsolidator`, `k_medoids`
   - `PlaybookStore`, `KnowledgeStore`, Daimon depotentiation
   - CLI commands plus daemon/orchestrator auto-dream entry points

3. **Target-state / frontier**
   - evolution / MAP-Elites / world models
   - sleep-time compute and Sleepwalker mode
   - targeted dream incubation
   - alpha / divergence theory
   - hauntology, rendering, oneirography
   - dream sharing, nightmare containment, lucid monitoring

---

## Section Map

| File | Scope | Refresh posture |
|------|-------|-----------------|
| [A-vision-and-cycle.md](A-vision-and-cycle.md) | Docs 00, 01, 13 | Runtime evidence sweep |
| [B-nrem-rem-consolidation.md](B-nrem-rem-consolidation.md) | Docs 02, 03, 04 | Replay/imagination reality check |
| [C-hdc-evolution-compute.md](C-hdc-evolution-compute.md) | Docs 05, 06, 12 | Keep HDC evidence; frontier-tag the rest |
| [D-hypnagogia-divergence-threat.md](D-hypnagogia-divergence-threat.md) | Docs 07, 08, 09 | Shipping hypnagogia/threat plus frontier split |
| [E-integration-status.md](E-integration-status.md) | Docs 15, 16 | Runtime-ahead-of-docs status regeneration |
| [F-frontier-concepts.md](F-frontier-concepts.md) | Docs 10, 11, 14, 17 | Uniform target-state register |
| [BATCHES.md](BATCHES.md) | execution contract | Narrowed for one-agent status work |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | code anchors | refreshed to current line ranges |
| [run-docs-parity.sh](run-docs-parity.sh) | helper script | prompt prep, not batch execution |

Context pack:

- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)
- [context-pack/dreams-summary.md](context-pack/dreams-summary.md)
- [context-pack/gaps-summary.md](context-pack/gaps-summary.md)
- [context-pack/repo-map.md](context-pack/repo-map.md)

---

## Priority Findings

1. `roko-dreams` is a 7-file, 5,964-LOC shipping runtime; the docs still read as if major pieces are pending.
2. Scheduled and manual triggers already ship, and the CLI/daemon/orchestrator surfaces already use them.
3. REM imagination, creativity modes, hypnagogia, and threat simulation already ship and should not be documented as speculative.
4. The support stack in `roko-learn` is real, but not every dreams-to-learning seam is fully wired into the cycle yet.
5. Docs 05, 08, 10, 11, 12, 14, and 17 must stay explicitly target-state so later agents do not infer nonexistent runtime.

---

## Success Definition

Batch `10` is successful when:

- the first screen makes it obvious that runtime is ahead of docs,
- Doc 16 is treated as the main regeneration target,
- docs around triggers, imagination, hypnagogia, and threat simulation stop understating shipped code,
- frontier material is clearly labeled as target-state rather than current architecture,
- and the parity pack is narrow enough for a single agent to execute without widening into runtime work.
