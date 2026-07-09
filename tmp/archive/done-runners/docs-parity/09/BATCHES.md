# Batch Execution Contract

Seven batches for an unattended `docs/09-daimon/` parity pass.

The core rule for topic `09` is simple:

**calibrate and tag; do not invent or widen the subsystem.**

Generated: 2026-04-18

---

## Batch Posture

- Trust the shipping core in `roko-core/src/affect.rs` and `roko-daimon/src/lib.rs`.
- Treat `docs/09-daimon/13-current-status-and-gaps.md` as a strong status source.
- Keep the main story visible: Daimon is mostly shipping.
- Mark frontier material explicitly instead of describing it in present tense.
- If a task requires real ALMA layering, contagion, fatigue tracking, or exact VCG settlement, defer it.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Order

`J1 -> J2 -> J3 -> J4 -> J5 -> J6 -> J7`

This keeps the foundational calibration first, then the richer
integration docs, then the final status and housekeeping pass.

---

## Batch Overview

| Batch | Purpose | Primary docs | Verify focus | Scope note |
|------|---------|--------------|--------------|------------|
| J1 | Reframe PAD labels and ALMA | 01, 02 | `AffectOctant`, `Plutchik`, `ALMA` | calibration only |
| J2 | Separate shipping appraisal/state behavior from richer theory prose | 03, 04, 05 | `bandit`, `hysteresis`, `Eight-Step Pipeline` | keep runtime path explicit |
| J3 | Mark what somatic/strategy surfaces already ship and fence off frontier loop-breaking ideas | 06, 07, 08 | `role-aware`, `mind wander`, `Sub-1ms` | no new strategy machinery |
| J4 | Tighten emotional retrieval and integration-point status | 09, 10 | `EmotionalTag`, `ContextAssembler`, `VCG` | stress live partials |
| J5 | Mark coding-agent integration as frontier instead of built behavior | 11 | `per-crate confidence`, `fatigue`, `error pattern` | defer construction |
| J6 | Treat collective contagion as explicit frontier and polish the status chapter | 12, 13 | `Tier 2M`, `Not started`, `roko-golem` | Doc 13 gets polish, not rebuild |
| J7 | Final top-level cleanup across the topic | `INDEX.md` plus touched files | implementation banners and stale history | consistency pass |

---

## Dependency Graph

| Batch | Depends on |
|------|------------|
| J1 | — |
| J2 | — |
| J3 | — |
| J4 | — |
| J5 | J4 |
| J6 | J1, J3, J4, J5 |
| J7 | J1, J2, J3, J4, J5, J6 |

Parallel-safe start group:

- `{J1, J2, J3, J4}`

Later batches are lighter because they mostly consolidate the narrowed
status language from the earlier passes.

---

## Batch Details

### J1 — PAD Labels and ALMA Boundary

**Owns**: A.05, A.08, A.09, C.12

**Goal**:

- keep `PadVector` and the single-layer shipping affect path clear,
- downgrade octant/Plutchik language from runtime type to informational mapping,
- mark true ALMA layering as target-state/frontier.

**Do**:

1. Reframe Doc 01 octants as human-readable PAD sign labels.
2. Note that Plutchik mappings are explanatory, not active runtime types.
3. Mark Doc 02 three-layer ALMA separation as future work.
4. Describe the shipping single-layer PAD + decay behavior accurately.

**Do not**:

- add new affect enums,
- imply mood/personality layers already ship,
- widen into runtime design work.

**Verify**:

```bash
rg -n "AffectOctant|Plutchik|ALMA|Personality layer|Design — Phase 2|target-state" docs/09-daimon/01-*.md docs/09-daimon/02-*.md
```

### J2 — Appraisal, Behavioral State, and Router Narrative

**Owns**: B.07, B.09, B.10

**Goal**:

- keep the shipped appraisal and behavioral-state logic visible,
- separate it from richer academic/control-law framing.

**Do**:

1. Reframe the eight-step appraisal pipeline as rationale, not literal staged runtime code.
2. Make clear that shipping cascade routing is bandit-based.
3. State that `BehavioralState::classify()` is memoryless.
4. Keep router hysteresis separate from daimon-state hysteresis.

**Do not**:

- describe prediction-error-threshold routing as the live implementation,
- imply explicit classifier dwell-time exists.

**Verify**:

```bash
rg -n "Eight-Step Pipeline|prediction-error threshold|hysteresis|dwell time|bandit" docs/09-daimon/03-*.md docs/09-daimon/04-*.md docs/09-daimon/05-*.md
```

### J3 — Somatic and Strategy Calibration

**Owns**: C.03, C.09, C.10, C.11

**Goal**:

- show that somatic and strategy-space machinery already ships,
- fence off the still-frontier diversity and benchmarking claims.

**Do**:

1. Describe role-aware label projection as the current non-coding fallback.
2. Mark mind wandering and rolling-window tracking as frontier.
3. Soften latency claims unless benchmark evidence is present.

**Do not**:

- invent domain-native extractors,
- imply timed mind wandering already exists.

**Verify**:

```bash
rg -n "role-aware|domain-native|mind wander|200-tick|Sub-1ms|kiddo" docs/09-daimon/06-*.md docs/09-daimon/07-*.md docs/09-daimon/08-*.md
```

### J4 — Emotional Retrieval and Integration Depth

**Owns**: D.01-D.07, D.11-D.12

**Goal**:

- show the live emotional retrieval and prompt/routing integrations,
- avoid overstating the depth of the remaining layers.

**Do**:

1. Align `EmotionalTag` examples to the shipping struct.
2. Note that `ContextAssembler` lives in Neuro and is re-exported by Compose.
3. Mark four-factor retrieval as live but still mostly Neuro-scoped.
4. Mark prompt-auction affect bidding as live but approximate.
5. Keep `SystemPromptBuilder` integration clearly in the shipping bucket.

**Do not**:

- imply full cross-subsystem weighting is done,
- imply full VCG externality accounting is done.

**Verify**:

```bash
rg -n "Plutchik|discovery_emotion|ContextAssembler|four-factor|VCG|PromptComposer|externality|SystemPromptBuilder" docs/09-daimon/09-*.md docs/09-daimon/10-*.md docs/09-daimon/11-*.md
```

### J5 — Coding-Agent Integration Frontier Tagging

**Owns**: D.08-D.10

**Goal**:

- stop Doc 11 from reading like already-built runtime behavior.

**Do**:

1. Mark per-crate confidence aggregation as unimplemented.
2. Mark error-pattern familiarity scaling as unimplemented.
3. Mark fatigue detection as unimplemented.
4. Keep any adjacent live prompt-affect integration visible.

**Do not**:

- design new data structures,
- pull conductor concepts into Daimon as if already integrated.

**Verify**:

```bash
rg -n "per-crate confidence|fatigue|error pattern|Design — Phase 2|target-state" docs/09-daimon/11-*.md
```

### J6 — Collective Frontier and Status Polish

**Owns**: E.01-E.08

**Goal**:

- make Doc 12 unmistakably frontier,
- keep Doc 13 as the canonical, mostly-trustworthy status chapter.

**Do**:

1. Mark collective contagion, somatic field, and C-Factor as not started.
2. Keep Tier 2M language explicit.
3. Remove any parity wording that still treats `roko-golem` as an active dependency.
4. Cross-link Doc 13 next steps to concrete parity gaps.

**Do not**:

- regenerate Doc 13 wholesale,
- speculate about mesh-runtime details beyond the existing doc scope.

**Verify**:

```bash
rg -n "Design — Phase 2\\+|target-state|Tier 2M|Not started|roko-golem" docs/09-daimon/12-*.md docs/09-daimon/13-*.md
```

### J7 — Final Consistency Pass

**Owns**: top-level topic framing

**Goal**:

- make the whole topic read consistently as "mostly shipping, frontier at the edges."

**Do**:

1. Update `docs/09-daimon/INDEX.md` summaries that overstate frontier material.
2. Check implementation banners for consistency.
3. Remove leftover stale migration wording where it reads as present tense runtime ownership.

**Verify**:

```bash
rg -n "roko-golem|Implementation\\*: Built|ALMA temporal model|collective contagion|per-crate confidence" docs/09-daimon/*.md
```

---

## Deliverable Standard

Each batch should leave behind:

- explicit status corrections,
- clear shipping vs partial vs frontier wording,
- intentional deferrals where runtime work would otherwise start,
- and a short verification record.

If a batch starts turning into construction instead of documentation
calibration, stop and defer the excess work.
