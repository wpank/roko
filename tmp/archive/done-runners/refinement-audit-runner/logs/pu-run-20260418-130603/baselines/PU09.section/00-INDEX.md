# 09-Daimon Parity Analysis

Gap analysis of `docs/09-daimon/` against the shipping affect stack in
`crates/roko-core/src/affect.rs`, `crates/roko-daimon/src/lib.rs`,
`crates/roko-neuro/src/context.rs`, `crates/roko-compose/`,
`crates/roko-learn/`, `crates/roko-cli/src/orchestrate.rs`, and the
other live consumers of `PadVector`, `BehavioralState`,
`DaimonPolicy`, and `EmotionalTag`.

Generated: 2026-04-16

---

## How To Use This Batch

**Topic 09 is mostly shipping code, not mostly missing code.**

This batch should therefore be treated as **doc calibration, frontier
tagging, and stale-reference cleanup**. The goal is not to invent new
daimon features overnight; it is to leave the daimon docs accurate
enough that later agents can tell which surfaces are real, which are
partial, and which still belong to a later deepening pass.

Distinguish four surfaces clearly:

1. **Shipping daimon primitives**
   - `PadVector`, `BehavioralState`, `DaimonPolicy`, `EmotionalTag`
   - `AffectState`, `AffectEvent`, `AffectEngine`, `DaimonState`
   - `DispatchStrategy`, `DispatchParams`
   - `SomaticLandscape`, `SomaticMarker`, `SomaticSignal`
   - `StrategyCoordinates`, `StrategySpaceDefinition`, `StrategySpaceComputer`

2. **Live partial integrations**
   - emotional congruence scoring in `roko-neuro::ContextAssembler`
   - PAD-fed `SystemPromptBuilder`
   - `DaimonPolicy` in cascade routing
   - VCG multipliers + prompt-auction selection
   - role-aware keyword projection for non-coding strategy spaces

3. **Doc drift / legacy-reference cleanup**
   - stale `roko-golem` references in active docs
   - `AffectOctant` / Plutchik labels described as live types
   - `EmotionalTag` examples that still imply a stored `emotion: String`
   - wording that conflates behavioral-state classification with router hysteresis

4. **Honest Phase 2+ frontier**
   - ALMA three-layer temporal model
   - mind wandering / rolling-window contrarian tracking
   - per-crate confidence aggregation
   - error-pattern familiarity tracker
   - fatigue detection
   - collective contagion / somatic field / C-Factor
   - fully faithful VCG externality accounting

Recommended single-agent serial order inside batch `09`:

`J1 -> J2 -> J3 -> J4 -> J5 -> J6 -> J7`

Reasoning:

- `J1-J5` can run mostly in parallel, but the default serial order keeps
  the foundational narrative (`J1-J2`) ahead of the richer integration
  docs (`J3-J5`).
- `J6` should polish Doc 13 only after the preceding sections settle.
- `J7` is the final banner/housekeeping pass and should be last.

---

## Document Index

| File | Docs Covered | Items | Status |
|------|--------------|-------|--------|
| [A-pad-and-temporal.md](A-pad-and-temporal.md) | 00, 01, 02 | A.01-A.10 | 7 DONE / 1 PARTIAL / 2 NOT DONE |
| [B-appraisal-and-states.md](B-appraisal-and-states.md) | 03, 04, 05 | B.01-B.10 | 7 DONE / 3 PARTIAL / 0 NOT DONE |
| [C-somatic-and-strategy.md](C-somatic-and-strategy.md) | 06, 07, 08 | C.01-C.12 | 7 DONE / 3 PARTIAL / 2 NOT DONE |
| [D-memory-and-integration.md](D-memory-and-integration.md) | 09, 10, 11 | D.01-D.12 | 4 DONE / 5 PARTIAL / 3 NOT DONE |
| [E-collective-and-status.md](E-collective-and-status.md) | 12, 13 | E.01-E.08 | 4 DONE / 0 PARTIAL / 4 NOT DONE |
| [BATCHES.md](BATCHES.md) | — | 7 batches | Execution contract |
| [SOURCE-INDEX.md](SOURCE-INDEX.md) | — | Verified code anchors | Reference |
| [run-docs-parity.sh](run-docs-parity.sh) | — | Batch runner | Launcher |

Context pack:

- [context-pack/agent-runbook.md](context-pack/agent-runbook.md)
- [context-pack/carry-forward-map.md](context-pack/carry-forward-map.md)
- [context-pack/daimon-summary.md](context-pack/daimon-summary.md)
- [context-pack/gaps-summary.md](context-pack/gaps-summary.md)
- [context-pack/repo-map.md](context-pack/repo-map.md)

Doc `INDEX.md` is absorbed into this file for parity purposes.

---

## Overall Parity: 29/52 items DONE (56%)

Topic `09` is still the strongest parity topic in the survey so far.
The main issue is not missing daimon infrastructure. The main issue is
that a handful of docs still describe **historical or designed surfaces**
as if they were the active runtime contract.

### Tier 1 — Should Exist Now (runtime-critical)

None.

The self-hosting loop already has a live affect path:

- appraisal from orchestrator and serve,
- explicit behavioral-state classification,
- dispatch modulation,
- cascade-router bias,
- emotional tagging,
- somatic modulation,
- live prompt affect guidance.

### Tier 2 — Should Exist Soon (doc honesty / integration clarity)

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| A.05 / C.12 | `AffectOctant` / Plutchik mapping should be treated as informational legacy, not live runtime types | PARTIAL / NOT DONE | MEDIUM |
| A.08 / A.09 | Doc 02 still overstates ALMA three-layer behavior relative to shipping single-layer PAD state | NOT DONE | MEDIUM |
| B.07 | Doc 05 still frames a prediction-error threshold control law; shipping cascade routing uses a bandit | PARTIAL | MEDIUM |
| B.09 | No hysteresis on `BehavioralState::classify`; smoothing comes from PAD half-life, while router hysteresis is a separate surface | PARTIAL | MEDIUM |
| C.03 | Domain-native strategy extractors absent; shipping fallback is role-aware keyword projection | PARTIAL | LOW |
| D.01 | Doc 09 still implies a stored Plutchik field on `EmotionalTag`; shipping struct does not carry one | DONE (doc drift) | MEDIUM |
| D.02 / D.03 | Mood-congruent retrieval is live in Neuro, but still uneven cross-subsystem | PARTIAL | LOW |
| D.06 | VCG affect bidding ships as approximation, not full externality accounting | PARTIAL | LOW |
| D.08-D.10 | Doc 11 reads like built runtime behavior, but per-crate confidence / pattern familiarity / fatigue are still frontier | NOT DONE | MEDIUM |
| E.01-E.04 | Doc 12 is entirely frontier and should read that way immediately | NOT DONE | LOW |

### Tier 3 — Future / Phase 2+ Frontier

| ID | Title | Status | Severity |
|----|-------|--------|----------|
| C.09 / C.10 | mind wandering / rolling-window contrarian tracking | NOT DONE | LOW |
| D.08-D.10 | coding-agent affect deepening | NOT DONE | LOW |
| E.01-E.04 | collective contagion / somatic field / C-Factor | NOT DONE | LOW |
| true ALMA mood/personality layers | A.08 / A.09 | NOT DONE | LOW |
| domain-native non-coding strategy computers | C.03 | PARTIAL | LOW |

### Already Shipped

| ID | Title | Status |
|----|-------|--------|
| A.01-A.04, A.06-A.07, A.10 | PAD core, cyclical states, control-signal behavior, decay, similarity, domain-agnostic affect | DONE |
| B.01-B.06, B.08 | appraisal, classifier, dispatch modulation, cascade-router feed | DONE |
| C.01-C.08 | 8D strategy space, somatic landscape, k-d tree, persistence | DONE |
| D.01, D.04, D.07, D.11 | emotional-tag plumbing, emotional provenance, event emission, prompt integration | DONE |
| E.05-E.08 | Doc 13 accuracy, Tier 0C completion, tier path alignment, skipped legacy files | DONE |

---

## Execution Boundaries

These are valid findings, but they should usually be handled outside
batch `09`:

| Item | Better Home | Why |
|------|-------------|-----|
| true ALMA mood/personality layers | later affect-deepening pass | current single-layer PAD works today |
| collective contagion / somatic field | later multi-agent pass | no runtime mesh owner in topic 09 |
| per-crate confidence / fatigue / familiarity tracker | later coding-integration pass | all are design surfaces today |
| exact VCG settlement and fairness policy | later compose/econ pass | current approximation is already live |
| domain-native chain/research/trading strategy computers | later per-domain plugin passes | role-aware projection is the current fallback |
| new daimon primitives or large runtime refactors | out of scope for docs parity | this batch is a doc-contract pass |

Batch `09` should usually produce:

- explicit frontier banners where needed,
- removal or downgrading of stale `roko-golem` path language,
- clear separation of shipping vs partial vs future daimon behavior,
- one canonical source index and runbook for unattended agents,
- and no accidental widening into new affect code.

---

## Critical Daimon Issues

1. **Several active docs still talk about `roko-golem` as if it were part of the live runtime contract.**
2. **Doc 09 still describes an `EmotionalTag` shape that no longer matches `roko-core/src/affect.rs`.**
3. **Behavioral-state classification and router hysteresis are distinct surfaces, but the docs blur them.**
4. **Doc 11 is still the most misleading banner in topic 09: it reads like built runtime behavior for surfaces that are not implemented.**
5. **Doc 13 is already unusually accurate and should be polished, not regenerated from scratch.**

---

## Key Insight

Topic `09` does not need a rescue rewrite.

It needs a **truthful status contract** for a mature subsystem:

1. preserve the strong shipping story,
2. mark the remaining design-only surfaces as design-only,
3. remove stale historical references from active-path explanations,
4. leave later agents with one obvious place to start.

---

## Batch 09 Success Definition

Batch `09` is successful when:

- later agents can tell from the first screen that daimon is mostly shipping,
- Doc 01 and Doc 02 no longer imply `AffectOctant` or full ALMA layering are live runtime contracts,
- Doc 09 no longer implies a stored Plutchik field on `EmotionalTag`,
- Doc 11 and Doc 12 are unmistakably frontier where they need to be,
- Doc 13 keeps its accuracy while pointing to specific parity entries for follow-on work,
- and `BATCHES.md` plus the context pack are sufficient for an unattended overnight docs pass.
