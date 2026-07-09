# 09-Daimon Parity Analysis

Parity refresh for `docs/09-daimon/` against the live affect stack in
`crates/roko-core/src/affect.rs`, `crates/roko-daimon/src/lib.rs`,
`crates/roko-neuro/`, `crates/roko-compose/`, `crates/roko-learn/`,
`crates/roko-cli/src/orchestrate.rs`, and `crates/roko-serve/`.

Generated: 2026-04-18

---

## Operating Stance

**Topic 09 is mostly shipping.**

The parity problem here is not "build Daimon." The shipping core already
exists:

- shared `PadVector`, `BehavioralState`, `DaimonPolicy`, and `EmotionalTag`
- live `AffectState`, `AffectEvent`, `AffectEngine`, and `DaimonState`
- behavioral-state-driven dispatch modulation
- `DaimonPolicy` in cascade routing
- persisted somatic landscape with `kiddo`-backed nearest-neighbor queries
- emotional congruence scoring in Neuro retrieval
- PAD-fed prompt guidance and live prompt-auction affect multipliers

This batch is therefore a **doc-calibration pass**:

1. keep the shipping story visible,
2. tag frontier material honestly,
3. clean up stale legacy references,
4. leave later agents with a narrow carry-forward map.

---

## Reality Split

### Shipping core

- `PadVector` and PAD math ship in `roko-core`
- `BehavioralState::classify()` ships and is tested
- `DaimonPolicy` ships and is consumed by routing
- appraisal, decay, persistence, and dispatch modulation ship in `roko-daimon`
- `SomaticLandscape`, `SomaticMarker`, and `StrategySpaceComputer` ship
- `EmotionalTag` ships and is consumed across Neuro, Compose, CLI, Dreams, and Learn

### Live but still partial

- emotional congruence scoring is live, but broader four-factor weighting is still uneven across subsystems
- prompt-auction affect bidding is live, but full VCG externality accounting is still approximate
- non-coding strategy spaces use role-aware label projection, not true domain-native extractors
- somatic modulation is real, but deeper cross-subsystem use is still incomplete

### Frontier to tag clearly

- ALMA three-layer temporal model
- per-crate confidence aggregation
- error-pattern familiarity scaling
- fatigue detection
- mind wandering / rolling-window contrarian tracking
- collective contagion, somatic field, and C-Factor

### Cleanup hotspots

- stale `roko-golem` language that still reads like active runtime ownership
- `AffectOctant` / Plutchik labels described like live runtime types
- `EmotionalTag` examples that still imply a stored emotion string
- Doc 11 wording that reads as built runtime behavior instead of frontier design

---

## Document Index

| File | Docs Covered | Main posture |
|------|--------------|--------------|
| [A-pad-and-temporal.md](A-pad-and-temporal.md) | 00, 01, 02 | PAD is real; ALMA layering is frontier |
| [B-appraisal-and-states.md](B-appraisal-and-states.md) | 03, 04, 05 | appraisal, state classification, and routing feed ship |
| [C-somatic-and-strategy.md](C-somatic-and-strategy.md) | 06, 07, 08 | somatic landscape and strategy space ship; mind wandering does not |
| [D-memory-and-integration.md](D-memory-and-integration.md) | 09, 10, 11 | emotional retrieval and prompt affect wiring are live; coding-integration deepening is frontier |
| [E-collective-and-status.md](E-collective-and-status.md) | 12, 13 | Doc 12 is frontier; Doc 13 is largely trustworthy |
| [BATCHES.md](BATCHES.md) | — | narrowed execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | corrected code anchors |
| [run-docs-parity.sh](run-docs-parity.sh) | — | updated batch descriptions |

Context pack:

- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)
- [context-pack/daimon-summary.md](context-pack/daimon-summary.md)
- [context-pack/gaps-summary.md](context-pack/gaps-summary.md)
- [context-pack/repo-map.md](context-pack/repo-map.md)

---

## Overall Parity

Raw tally across the parity entries remains:

- `29/52` DONE
- `12/52` PARTIAL
- `11/52` NOT DONE

That raw score understates the runtime picture. The **critical path is
already present**:

`appraisal -> PAD update -> behavioral state -> routing/prompt/somatic bias`

What remains is mostly edge depth and frontier research surfaces, not
missing subsystem foundations.

---

## Priority Fixes

| Area | Why it matters | Parity stance |
|------|----------------|---------------|
| Doc 01 octants / Plutchik | readers can mistake legacy labels for runtime types | keep as informational mapping only |
| Doc 02 ALMA layering | strongest doc/runtime mismatch in topic 09 | mark as target-state / frontier |
| Doc 05 routing narrative | routing is live, but the shipping path is bandit-based | separate shipping behavior from alternate control-law prose |
| Doc 09 `EmotionalTag` schema | active examples drift from the struct | align to PAD + intensity + trigger + mood snapshot |
| Doc 10 integration points | all four points have some live wiring | present per-point status, not binary built/unbuilt |
| Doc 11 coding-agent integration | currently the most misleading "built" read in topic 09 | banner as frontier |
| Doc 12 collective contagion | no shipping code | mark as explicit Phase 2+/target-state material |
| Doc 13 status chapter | already unusually accurate | polish and cross-link, do not regenerate |

---

## Carry-Forward Boundaries

These are valid future tasks, but should not be expanded inside this
parity pass:

| Item | Better home | Why |
|------|-------------|-----|
| true ALMA emotion/mood/personality split | later affect-deepening pass | current single-layer PAD is already operational |
| domain-native non-coding strategy computers | later per-domain pass | role-aware projection is the current shipping fallback |
| full four-factor retrieval everywhere | later neuro/compose deepening | Neuro already carries the first live version |
| full VCG settlement / fairness policy | later compose/econ pass | current prompt auction already uses affect multipliers |
| per-crate confidence, familiarity, fatigue | later coding-integration pass | all remain design surfaces |
| collective contagion / somatic field / C-Factor | later multi-agent pass | no runtime owner exists yet |

---

## Key Insight

The right correction is not to downplay Daimon.

The right correction is to describe it as a **mature, already-integrated
affect subsystem with a smaller frontier perimeter**:

1. preserve the live core,
2. fence off the frontier,
3. stop describing migration history as active architecture,
4. keep follow-on work narrow and explicit.

---

## Success Definition

Batch `09` is successful when:

- the first screen says Daimon is mostly shipping,
- PadVector, BehavioralState, DaimonPolicy, SomaticLandscape, and prompt/retrieval integrations are all treated as live surfaces,
- ALMA layering, coding-agent deepening, and collective contagion are tagged as frontier,
- stale `roko-golem` runtime framing is cleaned up,
- and later agents can use this parity pack without mistaking research ideas for production contracts.
