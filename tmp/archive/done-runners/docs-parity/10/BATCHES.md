# Batch Execution Contract

Topic `10` is a status-regeneration and frontier-tagging pass for a
dreams subsystem whose runtime is ahead of its docs.

The current batch IDs stay in place for compatibility with the helper
script, but the work should be thought of as **four practical passes**:

1. shipping runtime evidence sweep (`M1`, `M2`, `M3`)
2. frontier tagging sweep (`M4`)
3. mixed integration plus Doc 16 regeneration (`M5`, `M6`)
4. top-level consistency and housekeeping (`M7`, `M8`)

If the work stops being doc calibration and starts requiring new dreams
runtime, record the seam and defer it.

---

## Required Reads

Every batch should read:

- [00-INDEX.md](00-INDEX.md)
- the owning section note(s) below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)
- [context-pack/repo-map.md](context-pack/repo-map.md)

---

## Practical Order

Recommended order for one agent:

`M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7 -> M8`

The important constraint is not the exact batch count. It is this:

- settle shipped evidence first,
- label frontier material before touching Doc 16,
- regenerate Doc 16 after the evidence pass,
- then clean up `INDEX.md` and final consistency.

---

## Batch Overview

| Batch | Purpose | Primary docs | Verify focus |
|-------|---------|--------------|--------------|
| M1 | confirm trigger, schedule, budget, and daemon reality | Docs 00, 01, 13 | `DreamTrigger|DreamSchedulePolicy|manual_enabled|scheduled_cron|DreamHeartbeatPolicy|dream run` |
| M2 | confirm replay/imagination/consolidation reality | Docs 02, 03, 04 | `DreamReplayMode|utility_score|CounterfactualQuery|ImaginationMode|KnowledgeEntry` |
| M3 | confirm shipped hypnagogia and threat simulation; leave expansions frontier | Docs 07, 08, 09 | `HypnagogiaEngine|ThreatScenario|roko-golem|Targeted Dream Incubation|alpha` |
| M4 | frontier-tag evolution, sleep-time compute, rendering, sharing, and related theory | Docs 05, 06, 10, 11, 12, 14, 17 | absence checks plus target-state banner checks |
| M5 | clarify mixed integration surfaces and partial seams | Docs 15, 16, 17 | `DreamCycleReport|KnowledgeStore|PlaybookStore|mesh|nightmare|lucid` |
| M6 | regenerate Doc 16 from code, not from legacy status text | Doc 16 | `DreamTrigger|scheduled_cron|utility_score|CounterfactualQuery|HypnagogiaEngine|ThreatScenario|DreamCycleReport` |
| M7 | rebuild top-level `INDEX.md` claims and generation notes | `docs/10-dreams/INDEX.md` | `roko-golem|Sleepwalker|Oneirography|Hypnagogia|Threat simulation|Mattar-Daw` |
| M8 | final consistency pass across topic 10 | all touched docs | frontier banners plus stale-ownership checks |

---

## Batch Details

### M1 - Vision, Cycle, Scheduling

Read first:

- [A-vision-and-cycle.md](A-vision-and-cycle.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

Scope:

1. make `DreamRunner`, `DreamCycle`, `DreamBudget`, `DreamSchedulePolicy`, and `DreamHeartbeatPolicy` visible as shipped runtime,
2. make idle / scheduled / manual triggers visible,
3. leave per-phase budgeting and intensive consolidation as target-state only.

Do not:

- add new scheduling logic,
- add new backlog policies,
- redesign the cycle model.

### M2 - Replay, Imagination, Consolidation

Read first:

- [B-nrem-rem-consolidation.md](B-nrem-rem-consolidation.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

Scope:

1. mark replay planning and Mattar-Daw utility as shipped,
2. mark REM counterfactual imagination and creativity modes as shipped,
3. note that shipping consolidation is simpler than the SQLite-heavy doc story,
4. leave advanced diversity / DRL ideas informational only.

### M3 - Hypnagogia, Divergence, Threat

Read first:

- [D-hypnagogia-divergence-threat.md](D-hypnagogia-divergence-threat.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

Scope:

1. replace stale `roko-golem` framing with `roko-dreams` ownership,
2. mark `HypnagogiaEngine` and threat simulation as shipped,
3. keep TDI, alpha, divergence, and red-team expansions as frontier.

### M4 - Frontier Halo

Read first:

- [C-hdc-evolution-compute.md](C-hdc-evolution-compute.md)
- [F-frontier-concepts.md](F-frontier-concepts.md)

Scope:

1. keep the small amount of shipping HDC evidence,
2. mark evolution, MAP-Elites, sleep-time compute, Sleepwalker, rendering, oneirography, sharing, nightmare systems, and lucid monitoring as target-state,
3. make citation-heavy sections unmistakably non-runtime.

### M5 - Mixed Integration

Read first:

- [E-integration-status.md](E-integration-status.md)
- [F-frontier-concepts.md](F-frontier-concepts.md)

Scope:

1. make the mediated-via-Neuro integration story explicit,
2. separate shipped reports from future journal/sharing/nightmare systems,
3. preserve partial seams without inflating them into implemented features.

### M6 - Doc 16 Regeneration

Read first:

- outputs of `M1` through `M5`
- [E-integration-status.md](E-integration-status.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

Scope:

1. rebuild Doc 16 from current `roko-dreams`, `roko-cli`, and `roko-learn` evidence,
2. remove stale `roko-golem` dependency framing,
3. split shipped runtime from support infra and from target-state work.

### M7 - INDEX Regeneration

Read first:

- [00-INDEX.md](00-INDEX.md)
- [E-integration-status.md](E-integration-status.md)
- [F-frontier-concepts.md](F-frontier-concepts.md)

Scope:

1. rebuild top-level dreams index claims,
2. remove stale generation-note claims that imply frontier work ships,
3. keep the current runtime vs target-state split visible from the top page.

### M8 - Final Consistency

Read first:

- all updated section notes
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

Scope:

1. catch stale ownership claims,
2. catch missing target-state banners,
3. ensure the parity pack and helper script tell the same story.

---

## Acceptance Standard

Good batch output for topic `10` looks like this:

- shipped runtime is explicitly named,
- support infrastructure is distinguished from fully wired dream-cycle behavior,
- target-state material stays target-state,
- Doc 16 is regenerated from code evidence,
- and no batch widens into runtime implementation.
