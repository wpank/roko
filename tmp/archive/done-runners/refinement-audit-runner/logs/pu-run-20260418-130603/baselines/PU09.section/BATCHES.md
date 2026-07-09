# Batch Execution Contract

7 batches ordered for unattended execution. Unlike the chain batch,
**topic 09 is already mostly shipped**, so the default work here is
calibration, status cleanup, and explicit deferral, not subsystem
construction.

---

## Batch Posture

- Default strategy: **trust the shipping daimon core, then fix the docs around it**.
- Treat `crates/roko-core/src/affect.rs` and `crates/roko-daimon/src/lib.rs`
  as the primary runtime contract.
- Treat `docs/09-daimon/13-current-status-and-gaps.md` as an unusually
  trustworthy status doc; polish it instead of regenerating it.
- Treat `docs/09-daimon/11-coding-agent-integration.md` and
  `docs/09-daimon/12-collective-emotional-contagion.md` as the primary
  frontier-banner hotspots.
- If a task starts requiring real implementation of ALMA layers,
  collective contagion, per-crate confidence, fatigue detection, or
  exact VCG settlement, record the seam and stop.
- Every completed batch should leave behind:
  - doc changes with explicit status/banner updates,
  - verification output,
  - explicit deferrals,
  - and a clearer boundary between shipping daimon code, partial
    integrations, stale legacy references, and frontier design.

Required reads for every batch:

- [00-INDEX.md](00-INDEX.md)
- the owning section file(s) named below
- [SOURCE-INDEX.md](SOURCE-INDEX.md)
- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)

---

## Recommended Serial Order

For a single long-running agent run, prefer:

`J1 -> J2 -> J3 -> J4 -> J5 -> J6 -> J7`

This settles the foundational narrative first, then the richer
integration docs, then the status-doc and housekeeping pass.

---

## Batch Overview

| Batch | Tasks | Purpose | Primary Write Scope | Verify Focus | Est. LOC |
|-------|-------|---------|---------------------|--------------|----------|
| J1 | A.05, A.08, A.09, C.12 | Calibrate Doc 01 / 02 around octants, Plutchik, and ALMA layering | `docs/09-daimon/01-*.md`, `02-*.md` | `rg -n "AffectOctant|Plutchik|ALMA|Personality layer|Design — Phase 2" docs/09-daimon/01-*.md docs/09-daimon/02-*.md` | 120 |
| J2 | B.07, B.09, B.10 | Separate shipped appraisal / behavioral-state logic from alternative control-law narratives | `docs/09-daimon/03-*.md`, `04-*.md`, `05-*.md` | `rg -n "Eight-Step Pipeline|prediction-error threshold|hysteresis|dwell time|bandit" docs/09-daimon/03-*.md docs/09-daimon/04-*.md docs/09-daimon/05-*.md` | 120 |
| J3 | C.03, C.09, C.10, C.11 | Clarify strategy-space fallback, latency claims, and frontier loop-breaking mechanisms | `docs/09-daimon/06-*.md`, `07-*.md`, `08-*.md` | `rg -n "role-aware|domain-native|mind wander|200-tick|Sub-1ms|kiddo" docs/09-daimon/06-*.md docs/09-daimon/07-*.md docs/09-daimon/08-*.md` | 110 |
| J4 | D.01-D.07, D.12 | Calibrate emotional-tag schema, retrieval scope, and integration-point depth | `docs/09-daimon/09-*.md`, `10-*.md` | `rg -n "Plutchik|discovery_emotion|ContextAssembler|four-factor|VCG|PromptComposer|externality|Spectre" docs/09-daimon/09-*.md docs/09-daimon/10-*.md` | 140 |
| J5 | D.08-D.10 plus legacy-source cleanup | Mark coding integration as frontier and remove active-path `roko-golem` drift where still misleading | `docs/09-daimon/11-*.md`, `docs/09-daimon/INDEX.md`, selected legacy-ref sections in `04`, `10`, `13` | `rg -n "roko-golem|per-crate confidence|fatigue|error pattern|Implementation\\*: Built" docs/09-daimon/04-*.md docs/09-daimon/10-*.md docs/09-daimon/11-*.md docs/09-daimon/13-*.md docs/09-daimon/INDEX.md` | 140 |
| J6 | E.01-E.08 | Banner Doc 12 as frontier and cross-link Doc 13 next steps to concrete parity entries | `docs/09-daimon/12-*.md`, `13-*.md` | `rg -n "Design — Phase 2\\+|Tier 2M|C\\.03|D\\.02|D\\.06|E\\.01" docs/09-daimon/12-*.md docs/09-daimon/13-*.md` | 100 |
| J7 | global banner/status housekeeping | Final top-level banner pass and parity/index housekeeping | `docs/09-daimon/*.md`, `tmp/docs-parity/09/*` | `rg -n "^> \\*\\*Implementation\\*\\*:" docs/09-daimon/*.md` | 80 |

---

## Dependency Graph

| Batch | Depends on |
|-------|------------|
| J1 | — |
| J2 | — |
| J3 | — |
| J4 | — |
| J5 | J1, J2, J4 |
| J6 | J1, J3, J4, J5 |
| J7 | J1, J2, J3, J4, J5, J6 |

Why `J5` depends on `J1/J2/J4`:

- the coding-integration cleanup is easier once the octant, appraisal,
  and integration-surface stories are settled.

Why `J6` depends on earlier passes:

- Doc 13 should point at the settled parity entries, not stale assumptions.

Why `J7` is last:

- the final banner pass should reflect all earlier doc calibrations.

Parallel-safe groups:

- `{J1, J2, J3, J4}` can start immediately.
- `J5` waits for `J1`, `J2`, `J4`.
- `J6` waits for `J1`, `J3`, `J4`, `J5`.
- `J7` should be last.

Conflict groups:

| Group | Files | Batches |
|-------|-------|---------|
| pad-doc | `docs/09-daimon/01-*.md`, `02-*.md` | J1 |
| appraisal-doc | `docs/09-daimon/03-*.md`, `04-*.md`, `05-*.md` | J2 |
| somatic-doc | `docs/09-daimon/06-*.md`, `07-*.md`, `08-*.md` | J3 |
| integration-doc | `docs/09-daimon/09-*.md`, `10-*.md` | J4 |
| coding-doc | `docs/09-daimon/11-*.md`, `INDEX.md`, parts of `04`, `10`, `13` | J5 |
| status-doc | `docs/09-daimon/12-*.md`, `13-*.md` | J6 |
| parity-09 | `tmp/docs-parity/09/*` | all batches |

---

## Batch Details

### J1 — PAD, Octant, And ALMA Calibration

**Owns**: A.05, A.08, A.09, C.12

**Read first**:

- [A-pad-and-temporal.md](A-pad-and-temporal.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md) section on `roko-core/src/affect.rs`

**Problem**: Docs 01 and 02 still imply live `AffectOctant` / Plutchik-backed runtime behavior and full ALMA layering.

**Scope**:

1. Reframe Doc 01 octants as informational PAD sign-triplet labels.
2. Make clear that Plutchik labels are optional human-readable interpretations, not active runtime types.
3. Mark Doc 02 three-layer ALMA architecture as `Design — Phase 2+`.
4. Document the shipping single-layer PAD + confidence decay behavior honestly.

**Out of scope**:

- adding `AffectOctant` or `PlutchikEmotion` enums,
- implementing mood/personality layers,
- code changes.

**Files**:

- `docs/09-daimon/01-pad-vector.md`
- `docs/09-daimon/02-alma-three-layer-temporal.md`
- `tmp/docs-parity/09/*`

**Verify**:

```bash
rg -n "AffectOctant|Plutchik|ALMA|Personality layer|Design — Phase 2" docs/09-daimon/01-*.md docs/09-daimon/02-*.md
```

**Acceptance criteria**:

- Doc 01 no longer implies a live octant enum,
- Doc 02 clearly marks true ALMA layering as future work,
- PAD/confidence decay behavior is described accurately.

---

### J2 — Appraisal, Classifier, And Router Narrative Split

**Owns**: B.07, B.09, B.10

**Read first**:

- [B-appraisal-and-states.md](B-appraisal-and-states.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md) sections on appraisal and cascade routing

**Problem**: The docs still blur shipped direct appraisal rules, behavioral-state classification, and alternative control-law ideas.

**Scope**:

1. Reframe Doc 03’s 8-step pipeline as rationale, not literal runtime stages.
2. Clarify that the shipping cascade path is bandit-based.
3. Clarify that `BehavioralState::classify()` itself is memoryless.
4. Distinguish that router hysteresis exists, but it is model-selection hysteresis, not daimon-state hysteresis.

**Out of scope**:

- changing the bandit router,
- adding classifier hysteresis or dwell time,
- new appraisal math.

**Files**:

- `docs/09-daimon/03-occ-scherer-appraisal.md`
- `docs/09-daimon/04-six-behavioral-states.md`
- `docs/09-daimon/05-behavioral-state-to-tier-routing.md`
- `tmp/docs-parity/09/*`

**Verify**:

```bash
rg -n "Eight-Step Pipeline|prediction-error threshold|hysteresis|dwell time|bandit" docs/09-daimon/03-*.md docs/09-daimon/04-*.md docs/09-daimon/05-*.md
```

**Acceptance criteria**:

- the docs separate shipped classifier behavior from router behavior,
- prediction-error-threshold math is clearly identified as non-shipping,
- explicit hysteresis claims on behavioral-state classification are gone or clearly future-marked.

---

### J3 — Somatic And Strategy Calibration

**Owns**: C.03, C.09, C.10, C.11

**Read first**:

- [C-somatic-and-strategy.md](C-somatic-and-strategy.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md) section on strategy/somatic surfaces

**Problem**: The docs still over-promise domain-native extractors, rolling-window tracking, and latency guarantees.

**Scope**:

1. Document role-aware keyword projection as the current non-coding fallback.
2. Mark mind wandering and rolling-window tracking as future work.
3. Reframe latency claims around `kiddo` expectations unless benchmark evidence is added.

**Out of scope**:

- implementing domain-native strategy computers,
- building mind-wandering scheduler logic,
- large benchmark harness work.

**Files**:

- `docs/09-daimon/06-somatic-markers-damasio.md`
- `docs/09-daimon/07-15-percent-contrarian-retrieval.md`
- `docs/09-daimon/08-8-dimensional-strategy-space.md`
- `tmp/docs-parity/09/*`

**Verify**:

```bash
rg -n "role-aware|domain-native|mind wander|200-tick|Sub-1ms|kiddo" docs/09-daimon/06-*.md docs/09-daimon/07-*.md docs/09-daimon/08-*.md
```

**Acceptance criteria**:

- Doc 08 points at the shipping projection fallback,
- Doc 07 clearly marks loop-breaking frontier work,
- latency claims are benchmarked or honestly softened.

---

### J4 — Retrieval And Integration Status Pass

**Owns**: D.01-D.07, D.12

**Read first**:

- [D-memory-and-integration.md](D-memory-and-integration.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md) sections on Neuro / Compose / CLI / Learn

**Problem**: Docs 09 and 10 still mix shipping emotional-tag plumbing with older or broader design claims.

**Scope**:

1. Correct `EmotionalTag` examples that still imply a stored Plutchik field.
2. Clarify that `ContextAssembler` lives in `roko-neuro`, with a `roko-compose` re-export.
3. Separate Neuro-internal retrieval weighting from broader cross-subsystem ambitions.
4. Keep VCG integration honest about approximation scope.
5. Keep TUI/CLI affect surfaces distinct from absent Spectre-like visualizations.

**Out of scope**:

- widening retrieval weighting into all subsystems,
- adding exact VCG settlement,
- adding new visualization code.

**Files**:

- `docs/09-daimon/09-mood-congruent-memory.md`
- `docs/09-daimon/10-integration-points.md`
- `tmp/docs-parity/09/*`

**Verify**:

```bash
rg -n "Plutchik|discovery_emotion|ContextAssembler|four-factor|VCG|PromptComposer|externality|Spectre" docs/09-daimon/09-*.md docs/09-daimon/10-*.md
```

**Acceptance criteria**:

- Doc 09 no longer implies a stored `emotion` field on `EmotionalTag`,
- retrieval scope is clearly separated into shipping vs future,
- Doc 10 reflects the actual approximation depth of the VCG path.

---

### J5 — Coding Integration Frontier And Legacy Cleanup

**Owns**: D.08-D.10 plus stale active-path `roko-golem` drift

**Read first**:

- [D-memory-and-integration.md](D-memory-and-integration.md)
- [SOURCE-INDEX.md](SOURCE-INDEX.md) sections on missing coding-integration features

**Problem**: Doc 11 still reads like built runtime behavior, and several docs still cite `roko-golem` as if it were an active source rather than historical provenance.

**Scope**:

1. Add or strengthen `Design — Phase 2+` banners in Doc 11.
2. Mark per-crate confidence, error-pattern familiarity, and fatigue as future work.
3. Remove or downgrade active-path `roko-golem` wording in Doc 11 and other directly adjacent docs where it misleads readers.
4. Keep historical provenance references only where they are explicitly labeled legacy.

**Out of scope**:

- implementing coding-integration features,
- reviving `roko-golem`,
- large historical-doc cleanup outside topic 09.

**Files**:

- `docs/09-daimon/11-coding-agent-integration.md`
- `docs/09-daimon/INDEX.md`
- selected stale-reference passages in `docs/09-daimon/04-*.md`, `10-*.md`, `13-*.md`
- `tmp/docs-parity/09/*`

**Verify**:

```bash
rg -n "roko-golem|per-crate confidence|fatigue|error pattern|Implementation\\*: Built" docs/09-daimon/04-*.md docs/09-daimon/10-*.md docs/09-daimon/11-*.md docs/09-daimon/13-*.md docs/09-daimon/INDEX.md
```

**Acceptance criteria**:

- Doc 11 reads as frontier where appropriate,
- historical `roko-golem` references are labeled historical, not active,
- later agents do not mistake Doc 11 for a shipped feature inventory.

---

### J6 — Collective Contagion Frontier And Doc 13 Cross-Links

**Owns**: E.01-E.08

**Read first**:

- [E-collective-and-status.md](E-collective-and-status.md)
- results of J1-J5

**Problem**: Doc 12 should be unmistakably frontier, and Doc 13 should point directly at the parity entries that describe the next deepening passes.

**Scope**:

1. Add a strong frontier banner to Doc 12.
2. Keep shipping precursors discoverable without overstating them.
3. Update Doc 13 “Recommended Next Steps” to cite specific parity entries:
   - C.03 for domain-native extractors,
   - D.02 / D.03 for retrieval widening,
   - D.06 for VCG deepening,
   - E.01-E.04 for collective contagion.

**Out of scope**:

- regenerating Doc 13 from scratch,
- designing the contagion protocol in more detail,
- new multi-agent code.

**Files**:

- `docs/09-daimon/12-collective-emotional-contagion.md`
- `docs/09-daimon/13-current-status-and-gaps.md`
- `tmp/docs-parity/09/*`

**Verify**:

```bash
rg -n "Design — Phase 2\\+|Tier 2M|C\\.03|D\\.02|D\\.06|E\\.01" docs/09-daimon/12-*.md docs/09-daimon/13-*.md
```

**Acceptance criteria**:

- Doc 12 is uniformly frontier,
- Doc 13 next steps point at concrete parity entries,
- Doc 13 stays concise and trustworthy.

---

### J7 — Global Banner And Housekeeping Pass

**Owns**: final topic-09 cleanup

**Read first**:

- outputs of J1-J6

**Problem**: after the narrow passes land, topic 09 still needs one last consistency sweep.

**Scope**:

1. Sweep `docs/09-daimon/*.md` for overstated or stale implementation banners.
2. Add a pointer from `docs/09-daimon/INDEX.md` to the parity audit if useful.
3. Make sure the parity pack itself reflects the settled structure.

**Out of scope**:

- changing runtime code,
- adding new parity items without evidence,
- rewriting Doc 13 wholesale.

**Files**:

- `docs/09-daimon/*.md`
- `tmp/docs-parity/09/*`

**Verify**:

```bash
rg -n "^> \\*\\*Implementation\\*\\*:" docs/09-daimon/*.md
```

**Acceptance criteria**:

- top-level banners across topic 09 are mutually consistent,
- the parity audit is discoverable,
- batch 09 is closed cleanly for unattended execution.
