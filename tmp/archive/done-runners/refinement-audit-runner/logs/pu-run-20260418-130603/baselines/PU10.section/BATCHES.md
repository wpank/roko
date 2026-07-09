# Batch Execution Contract

8 batches ordered for unattended execution. Topic `10` needs more than
small note cleanup: it needs a proper execution contract for a dreams
subsystem whose runtime is ahead of its docs.

---

## Batch Posture

- Default strategy: **regenerate status docs from code, then frontier-tag the research halo**.
- Treat `crates/roko-dreams/` as the primary runtime contract.
- Treat `docs/10-dreams/16-implementation-status.md` as the main doc hotspot.
- Treat Docs 10, 11, 12, 14, and 17 as likely frontier docs unless code says otherwise.
- If a task starts requiring new dream runtime implementation, record the seam and stop.
- Every completed batch should leave behind:
  - doc changes with explicit status/banner updates,
  - verification output,
  - explicit deferrals,
  - and a clearer split between shipping runtime, supporting infrastructure, and future dream research.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) named below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

For a single long-running agent run, prefer:

`M1 -> M2 -> M3 -> M4 -> M5 -> M6 -> M7 -> M8`

This settles the shipping runtime surfaces first, then the mixed and
frontier docs, then regenerates the main status doc, then does the
final housekeeping pass.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus | Est. LOC |
|-------|-------|---------|---------------------|--------------|----------|
| M1 | A.01-A.11 | Lock in trigger/scheduling/runtime ownership and correct manual/scheduled trigger drift | `docs/10-dreams/00-*.md`, `01-*.md`, `13-*.md`, parity notes | `rg -n "scheduled|manual trigger|DreamTrigger|scheduled_cron|dream run|DreamHeartbeatPolicy" docs/10-dreams/00-*.md docs/10-dreams/01-*.md docs/10-dreams/13-*.md crates/roko-dreams crates/roko-cli` | 140 |
| M2 | B.01-B.13 | Reconcile NREM/REM/consolidation docs with replay/imagination reality and simpler staging path | `docs/10-dreams/02-*.md`, `03-*.md`, `04-*.md`, parity notes | `rg -n "Mattar-Daw|Counterfactual|Boden|SQLite staging|KnowledgeEntry|utility_score" docs/10-dreams/02-*.md docs/10-dreams/03-*.md docs/10-dreams/04-*.md crates/roko-dreams` | 160 |
| M3 | D.01-D.10 | Correct hypnagogia and threat-simulation ownership and strengthen frontier banners on divergence/TDI/red-team extensions | `docs/10-dreams/07-*.md`, `08-*.md`, `09-*.md`, parity notes | `rg -n "HypnagogiaEngine|ThreatScenario|roko-golem|Targeted Dream Incubation|alpha|Constitutional" docs/10-dreams/07-*.md docs/10-dreams/08-*.md docs/10-dreams/09-*.md crates/roko-dreams` | 140 |
| M4 | C.05-C.11 plus section F | Frontier pass for evolution, sleep-time compute, hauntology, rendering, oneirography, and advanced concepts | `docs/10-dreams/05-*.md`, `06-*.md`, `10-*.md`, `11-*.md`, `12-*.md`, `14-*.md`, `17-*.md` | `rg -n "Design — Phase 2\\+|MAP-Elites|Sleepwalker|rethink_memory|hauntology|Oneirography|world model|nightmare|lucid" docs/10-dreams/05-*.md docs/10-dreams/06-*.md docs/10-dreams/10-*.md docs/10-dreams/11-*.md docs/10-dreams/12-*.md docs/10-dreams/14-*.md docs/10-dreams/17-*.md` | 140 |
| M5 | section E plus F.10 fallout | Sharpen mixed integration/status docs and separate shipped report journaling from future sharing/nightmare systems | `docs/10-dreams/15-*.md`, `16-*.md`, `17-*.md`, parity notes | `rg -n "mesh|nightmare|dream journal|lucid|oneirography|DreamCycleReport|Design — Phase 2\\+|roko-golem" docs/10-dreams/15-*.md docs/10-dreams/16-*.md docs/10-dreams/17-*.md` | 140 |
| M6 | E.06-E.13 plus A/B/D fallout | Regenerate Doc 16 from current runtime and supporting infrastructure | `docs/10-dreams/16-*.md`, parity notes | `rg -n "roko-golem|Mattar-Daw|Counterfactual|Hypnagogia|Threat simulation|dream run|scheduled trigger" docs/10-dreams/16-*.md` | 180 |
| M7 | M1-M6 fallout | Rebuild top-level `INDEX.md` claims and stale generation notes to match current reality | `docs/10-dreams/INDEX.md`, parity notes | `rg -n "roko-golem|Sleepwalker|Oneirography|Hypnagogia|Threat simulation|Mattar-Daw" docs/10-dreams/INDEX.md` | 120 |
| M8 | global banner/status housekeeping | Final topic-10 consistency pass plus parity housekeeping | `docs/10-dreams/*.md`, `tmp/docs-parity/10/*` | `rg -n "^> \\*\\*Implementation\\*\\*:" docs/10-dreams/*.md` | 80 |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| M1 | — |
| M2 | — |
| M3 | — |
| M4 | — |
| M5 | M1, M2, M3, M4 |
| M6 | M1, M2, M3, M5 |
| M7 | M4, M6 |
| M8 | M1, M2, M3, M4, M5, M6, M7 |

Parallel-safe groups:

- `{M1, M2, M3, M4}` can start immediately.
- `M5` waits for the core runtime/status passes.
- `M6` should run after the status evidence is settled.
- `M7` should follow Doc 16 regeneration.
- `M8` should be last.

Conflict groups:

| Group | Files | Batches |
|-------|-------|---------|
| trigger-doc | `docs/10-dreams/00-*.md`, `01-*.md`, `13-*.md` | M1 |
| phase-doc | `docs/10-dreams/02-*.md`, `03-*.md`, `04-*.md` | M2 |
| hypo-threat-doc | `docs/10-dreams/07-*.md`, `08-*.md`, `09-*.md` | M3 |
| frontier-doc | `docs/10-dreams/05-*.md`, `06-*.md`, `10-*.md`, `11-*.md`, `12-*.md`, `14-*.md`, `17-*.md` | M4 |
| integration-doc | `docs/10-dreams/15-*.md`, `16-*.md`, `17-*.md` | M5 |
| status-doc | `docs/10-dreams/16-*.md` | M6 |
| index-doc | `docs/10-dreams/INDEX.md` | M7 |
| parity-10 | `tmp/docs-parity/10/*` | all batches |

---

## Batch Details

### M1 — Trigger And Scheduling Reality Pass

**Owns**: section A

**Read first**:

- [A-vision-and-cycle.md](A-vision-and-cycle.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

**Problem**: the docs still understate scheduled/manual triggers and daemon/runtime ownership.

**Scope**:

1. correct scheduled trigger claims,
2. correct manual trigger / CLI claims,
3. keep per-phase budget allocation and intensive mode explicitly future-marked.

**Out of scope**:

- adding new triggers,
- backlog-intensive implementation,
- scheduler refactors.

**Acceptance criteria**:

- later agents can see that idle, scheduled, and manual paths all exist,
- docs distinguish crate/runtime support from future scheduling policies.

---

### M2 — Replay, Imagination, And Consolidation Reality Pass

**Owns**: section B

**Read first**:

- [B-nrem-rem-consolidation.md](B-nrem-rem-consolidation.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

**Problem**: Docs 02-04 and Doc 16 undercount major replay/imagination surfaces and overstate the staging model.

**Scope**:

1. make replay/imagination features visible,
2. call out the simpler shipping integration/staging path,
3. keep advanced diversity / DRL references informational.

**Out of scope**:

- adding cross-episode wiring if absent,
- adding full GIRL or DRL replay machinery.

**Acceptance criteria**:

- Mattar-Daw, REM counterfactuals, and Boden modes are documented as shipping,
- Doc 04 reflects the actual simpler staging approach.

---

### M3 — Hypnagogia And Threat Reality Pass

**Owns**: section D

**Read first**:

- [D-hypnagogia-divergence-threat.md](D-hypnagogia-divergence-threat.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

**Problem**: Docs 07, 09, and 16 still understate two real shipped modules.

**Scope**:

1. correct hypnagogia ownership and status,
2. correct threat simulation status,
3. leave TDI/divergence/constitutional-red-team expansions as future work.

**Out of scope**:

- adding TDI,
- adding alpha/divergence runtime,
- nightmare detection implementation.

**Acceptance criteria**:

- docs clearly show `hypnagogia.rs` and `threat.rs` as shipping,
- the remaining theory extensions are explicitly future-facing.

---

### M4 — Frontier Research Halo Pass

**Owns**: section C frontier items plus section F

**Read first**:

- [C-hdc-evolution-compute.md](C-hdc-evolution-compute.md)
- [F-frontier-concepts.md](F-frontier-concepts.md)

**Problem**: multiple dreams docs are convincing but mostly theoretical.

**Scope**:

1. add or strengthen frontier banners,
2. make “informational citation” vs “shipping mechanism” clearer,
3. note when ownership better belongs to later domain batches.

**Out of scope**:

- oneirography runtime,
- sleep-time compute runtime,
- world-model integration.

**Acceptance criteria**:

- future-facing docs are unmistakably future-facing,
- later agents do not mistake research summaries for shipped surfaces.

---

### M5 — Mixed Integration And Status-Adjacency Pass

**Owns**: section E plus the dream-journal overlap with section F

**Read first**:

- [E-integration-status.md](E-integration-status.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md)

**Problem**: these docs mix real runtime integrations with stale status ownership and future mesh/nightmare/journal systems.

**Scope**:

1. sharpen per-integration status in Doc 15,
2. keep Doc 16 ownership/status cleanup aligned with those integrations,
3. split shipped dream-report journaling from future journal/nightmare/sharing systems in Doc 17.

**Out of scope**:

- implementing dream sharing,
- implementing nightmare detection,
- implementing lucid monitoring.

**Acceptance criteria**:

- Doc 15 has clear status per integration surface,
- Doc 17 distinguishes persisted reports from richer future systems,
- Doc 16’s mixed integration/dependency statements no longer contradict the section findings.

---

### M6 — Doc 16 Regeneration Pass

**Owns**: Doc 16

**Read first**:

- outputs of M1-M5
- [E-integration-status.md](E-integration-status.md)

**Problem**: Doc 16 is the main stale status artifact in topic 10.

**Scope**:

1. rebuild Doc 16 from current code ownership,
2. remove obsolete `roko-golem` re-export claims,
3. correct implemented / partial / not-started status entries.

**Out of scope**:

- runtime code changes,
- roadmap invention beyond what parity evidence supports.

**Acceptance criteria**:

- Doc 16 is usable as a canonical status summary,
- later agents no longer need to reverse-engineer dreams status from code.

---

### M7 — INDEX Regeneration Pass

**Owns**: `docs/10-dreams/INDEX.md`

**Read first**:

- outputs of M4 and M6

**Problem**: the top-level index still repeats several stale assumptions.

**Scope**:

1. update top-level descriptions,
2. update generation notes that still refer to older ownership,
3. add or strengthen parity-audit pointer if useful.

**Acceptance criteria**:

- INDEX.md reflects current runtime ownership and status posture.

---

### M8 — Global Banner And Housekeeping Pass

**Owns**: final topic-10 cleanup

**Read first**:

- outputs of M1-M7

**Problem**: topic 10 needs one last consistency sweep after the major status fixes.

**Scope**:

1. sweep implementation banners for consistency,
2. make sure the parity pack matches the final structure,
3. close the batch cleanly for unattended runs.

**Acceptance criteria**:

- topic 10 banners are internally consistent,
- the parity bundle is self-sufficient for later overnight runs.
