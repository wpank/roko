# 10-Dreams Parity Analysis

Gap analysis of `docs/10-dreams/` against the shipping dreams stack in
`crates/roko-dreams/`, its supporting learning/neuro integrations, and
the live CLI/daemon surfaces that invoke dream runs today.

Generated: 2026-04-16

---

## How To Use This Batch

Topic `10` should be treated as **mixed status regeneration + frontier
tagging**.

Unlike topic `09`, the issue here is not just small doc drift. The
dream subsystem has a lot of real code, but the top-level status doc
and several phase docs still describe an older runtime shape.

Distinguish four surfaces clearly:

1. **Shipping dreams runtime**
   - `DreamRunner`, `DreamCycle`, `DreamBudget`, `DreamSchedulePolicy`
   - idle / scheduled / manual triggers
   - heartbeat and daemon polling
   - `DreamReplayMode`, Mattar-Daw utility scoring
   - REM counterfactual imagination
   - `HypnagogiaEngine`
   - threat simulation
   - persisted `DreamCycleReport`s

2. **Shipping supporting infrastructure**
   - `PatternMiner`, `CrossEpisodeConsolidator`, `k_medoids`
   - `PlaybookStore`, `KnowledgeStore`, Daimon depotentiation
   - CLI commands and daemon loop integration

3. **Live-but-partial integrations**
   - cycle-level cluster outputs and strategy hypotheses
   - dream → playbook / learn / prompt / routing feedback seams
   - dream journal base via persisted reports, but not the richer journal system

4. **Honest Phase 2+ frontier**
   - evolution / MAP-Elites / world models
   - Sleepwalker mode and Lin et al. sleep-time compute
   - targeted dream incubation
   - alpha/divergence framework
   - hauntology metrics
   - rendering / oneirography
   - dream sharing / nightmare detection / lucid monitoring

Recommended single-agent serial order inside batch `10`:

`M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7 -> M8`

Reasoning:

- `M1-M3` settle the real runtime before the frontier/theory passes.
- `M4-M6` handle the large theory docs and mixed integration surfaces.
- `M7` regenerates the stale status doc from the settled findings.
- `M8` is the final top-level banner and housekeeping pass.

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-vision-and-cycle.md](A-vision-and-cycle.md) | 00, 01, 13 | A.01-A.11 | 8 DONE / 2 PARTIAL / 1 NOT DONE |
| [B-nrem-rem-consolidation.md](B-nrem-rem-consolidation.md) | 02, 03, 04 | B.01-B.13 | 7 DONE / 4 PARTIAL / 2 NOT DONE |
| [C-hdc-evolution-compute.md](C-hdc-evolution-compute.md) | 05, 06, 12 | C.01-C.11 | 4 DONE / 2 PARTIAL / 5 NOT DONE |
| [D-hypnagogia-divergence-threat.md](D-hypnagogia-divergence-threat.md) | 07, 08, 09 | D.01-D.10 | 2 DONE / 3 PARTIAL / 5 NOT DONE |
| [E-integration-status.md](E-integration-status.md) | 15, 16 | E.01-E.13 | 4 DONE / 8 PARTIAL / 1 NOT DONE |
| [F-frontier-concepts.md](F-frontier-concepts.md) | 10, 11, 14, 17 | F.01-F.12 | 0 DONE / 2 PARTIAL / 10 NOT DONE |
| [BATCHES.md](BATCHES.md) | — | 8 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |
| [run-docs-parity.sh](run-docs-parity.sh) | — | Batch runner | Launcher |

Context pack:

- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)
- [context-pack/dreams-summary.md](context-pack/dreams-summary.md)
- [context-pack/gaps-summary.md](context-pack/gaps-summary.md)
- [context-pack/repo-map.md](context-pack/repo-map.md)

---

## Overall Parity: 25/70 items DONE (36%)

Topic `10` is more shipped than its docs suggest.

The biggest issue is concentrated in Doc 16 and the docs that still
derive their status framing from it. A second issue is that several
theory/extension docs are written convincingly enough that later agents
could mistake them for near-runtime work.

### Tier 1 — Should Exist Now (runtime-critical)

None.

Dreams is useful and already integrated, but the self-hosting loop is
not blocked on the frontier ideas in topic `10`.

### Tier 2 — Should Exist Soon (status and doc honesty)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.06 | Scheduled trigger ships; Doc 16 still says not implemented | DONE (doc drift) | HIGH |
| A.07 / E.05 | Manual trigger and CLI surface ship; older docs understate this | DONE / DONE | HIGH |
| B.02 | Mattar-Daw utility scoring ships; Doc 16 undercounts it | DONE (doc drift) | HIGH |
| B.04 | REM counterfactual simulation ships; Doc 16 undercounts it | DONE (doc drift) | HIGH |
| B.05 | Boden’s three creativity modes ship; Doc 16 undercounts them | DONE (doc drift) | HIGH |
| D.01 | Hypnagogia ships in `roko-dreams`; Doc 16 still talks about placeholder ownership | DONE (doc drift) | HIGH |
| D.08 | Threat simulation ships; Doc 16 still marks it not started | DONE (doc drift) | HIGH |
| E.06 / E.07 | Doc 16 still omits most shipping modules and still carries obsolete golem-dissolution status | PARTIAL / DONE | HIGH |
| B.11 | Doc 04 overstates SQLite staging vs the simpler shipping tag-based path | DONE / PARTIAL | MEDIUM |
| E.04 | Doc 15 mixes direct-dependency framing with the cleaner mediated-via-Neuro reality | PARTIAL | MEDIUM |

### Tier 3 — Future / Phase 2+ Frontier

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.10 | intensive consolidation mode | NOT DONE | LOW |
| B.07 / B.13 | advanced counterfactual diversity and DRL replay variants | NOT DONE | LOW |
| C.05-C.11 | evolution, MAP-Elites, Lin sleep-time compute, Sleepwalker, world models | NOT DONE | LOW |
| D.04-D.07, D.09 | TDI, N1/N2, divergence/alpha, red-team classifier expansions | NOT DONE | LOW |
| F.01-F.12 | hauntology, rendering, oneirography, dream sharing, nightmare detection, lucid monitoring, related frontier concepts | NOT DONE / PARTIAL | LOW |

### Already Shipped

| ID | Title | Status |
|----|-------|--------|
| A.01-A.09, A.11 | dream triggers, scheduling, heartbeat, budgets, persistence, core cycle framing | DONE |
| B.01-B.05, B.08, B.11 | replay modes, utility scoring, REM imagination, creativity modes, depotentiation, knowledge writing | DONE |
| C.01-C.04 | HDC usage, k-medoids infrastructure, cluster reports | DONE |
| D.01, D.08 | hypnagogia engine, threat simulation | DONE |
| E.01, E.02, E.05, E.13 | key cross-system integrations and idle scheduling claim | DONE |

---

## Execution Boundaries

These are valid findings, but they should usually be handled outside
batch `10`:

| Item | Better Home | Why |
|------|-------------|-----|
| actual evolution / MAP-Elites runtime | later dreams-deepening pass | not status-critical |
| world models / Dreamer / Genie / IRIS | later model-learning pass | no shipping owner today |
| oneirography blockchain/NFT work | later chain/domain pass | not dreams-core |
| mesh dream sharing / nightmare detection | later multi-agent safety pass | not live runtime |
| sleep-time compute / Sleepwalker | later compute optimization pass | theory-only today |
| targeted dream incubation | later hypnagogia-deepening pass | no code today |

Batch `10` should usually produce:

- an honest Doc 16,
- clearer banners on theory/extension docs,
- explicit separation between shipping dreams runtime and later dream research,
- and a stronger execution contract for unattended docs work.

---

## Critical Dreams Issues

1. **Doc 16 is materially stale and should be treated as the main regeneration target.**
2. **Several docs still describe `roko-golem` ownership that the live runtime no longer has.**
3. **Manual trigger and CLI/daemon integration are more complete than the current parity notes assumed.**
4. **Docs 10, 11, 14, 15, 16, and 17 needed to be reflected explicitly in the batch contract so agents don’t infer status from omission.**
5. **Theoretical extensions like oneirography, nightmare detection, and sleep-time compute need stronger frontier framing.**

---

## Key Insight

Topic `10` is not a “dream subsystem doesn’t exist” story.

It is a **shipping dream runtime plus stale status docs plus a large
research halo**. The best parity work here is to make those three
layers legible to later agents.

---

## Batch 10 Success Definition

Batch `10` is successful when:

- later agents can see from the first screen which dream features already ship,
- Doc 16 no longer describes an obsolete `roko-golem`-centric runtime,
- Docs 07 / 09 / 13 / 15 / 16 are aligned with the code on triggers, hypnagogia, and threat simulation,
- frontier docs like 10 / 11 / 12 / 14 / 17 are unmistakably future-facing,
- and `BATCHES.md` plus the context pack are strong enough for unattended overnight execution.
