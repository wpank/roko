# SOURCE-INDEX — Code Anchors for 09-Daimon Parity

Verified code anchors for batch `09`, updated against the current tree.
This version corrects stale line offsets, removes outdated
`roko-golem`-as-runtime phrasing, and points at the live seams a docs
agent should trust first.

Generated: 2026-04-18

---

## Important Corrections First

Use these before trusting the Daimon docs literally:

- Doc 12 (`collective-emotional-contagion`) is frontier design material. Its `Implementation: Built` banner is stale.
- `roko-daimon` is live runtime code, not a scaffold.
- Shared affect primitives live in `crates/roko-core/src/affect.rs`; `roko-daimon` consumes them.
- `roko-golem` is dissolved. Treat any mention as historical provenance only.
- `roko-compose::ContextAssembler` is a thin re-export; the implementation lives in Neuro.
- `EmotionalTag` does not store a Plutchik/emotion string in the shipping struct.
- `BehavioralState::classify()` is memoryless; the separate hysteresis in `cascade_router.rs` is router hysteresis, not affect-state hysteresis.

---

## crates/roko-core/src/

### Shared affect primitives

| File | What | Section |
|------|------|---------|
| `affect.rs:7-14` | `PadVector { pleasure, arousal, dominance }` | A.04 |
| `affect.rs:16-82` | `new`, `neutral`, `clamped`, `apply_delta`, `decay_by_factor`, `cosine_similarity()` | A.04, A.06, A.07, D.03 |
| `affect.rs:87-101` | `BehavioralState` enum | A.02, B.04 |
| `affect.rs:103-131` | `BehavioralState::classify(pad, confidence)` | B.04, B.09 |
| `affect.rs:136-158` | `DaimonPolicy` | A.03, B.06 |
| `affect.rs:162-189` | `EmotionalTag { pad, intensity, trigger, mood_snapshot }` | D.01 |
| `affect.rs:192-249` | shared affect tests | A.06, B.04, D.01 |

---

## crates/roko-daimon/src/

### Live daimon engine

| File | What | Section |
|------|------|---------|
| `lib.rs:39-49` | somatic constants and `KdTree` alias, including `CONTRARIAN_FRACTION = 0.15` | C.05-C.07 |
| `lib.rs:57-130` | `AffectState`, decay/apply helpers, emotional-tag derivation | A.07, B.03, D.01 |
| `lib.rs:148-324` | `StrategyCoordinates` and `StrategySpaceDefinition` | C.01, C.02 |
| `lib.rs:335-979` | task/episode observations, role-aware projection, registered computers, `extract_strategy_point()` | C.03, C.04 |
| `lib.rs:983-1350` | `SomaticMarker`, `SomaticSignal`, `SomaticLandscape`, query/contrarian blend, dream depotentiation | C.05-C.08, D.07 |
| `lib.rs:1355-1406` | `DispatchStrategy`, `DispatchParams` | B.05 |
| `lib.rs:1411-1458` | `AffectEvent` enum | B.01 |
| `lib.rs:1462-1797` | `DaimonState`, `load_or_new`, persistence, `modulate_with_strategy`, `AffectEngine` impl | B.02, B.05, D.05 |
| `lib.rs:2140+` | appraisal / persistence / somatic tests | B.02, C.08 |

---

## crates/roko-neuro/src/

### Emotional provenance and retrieval

| File | What | Section |
|------|------|---------|
| `context.rs:200-239` | `ContextChunk` emotional fields plus `ContextAssembler` definition | D.02, D.03 |
| `context.rs:1466+` | assembler tests, including affect/somatic ranking coverage | D.02, D.03 |
| `knowledge_store.rs:255-307` | query path applying emotional retrieval boost during ranking | D.01-D.03 |
| `knowledge_store.rs:872-909` | emotional retrieval multiplier wiring | D.03 |
| `knowledge_store.rs:1118-1215` | emotional diversity / reinforced-entry retrieval tests | D.03, D.04 |
| `distiller.rs:227-248` | distilled knowledge entries inherit emotional metadata | D.04 |
| `distiller.rs:478-523` | emotional-tag / provenance aggregation for consolidation | D.04 |
| `distiller.rs:780-823` | distiller test transferring emotional provenance | D.04 |
| `lib.rs:320-374` | emotional consolidation / retrieval multipliers on `KnowledgeEntry` | D.04 |

---

## crates/roko-compose/src/

### Prompt composition and re-export boundary

| File | What | Section |
|------|------|---------|
| `context_assembler.rs:1-4` | re-export of `roko_neuro::{ContextAssembler, ContextChunk, PadState}` | D.02 |
| `system_prompt_builder.rs:53-214` | `SystemPromptBuilder` and `with_affect_state(...)` | D.11 |
| `system_prompt_builder.rs:445-520` | affect-guidance section creation and wording | D.11 |
| `role_prompts.rs:41-120` | typed role-prompt task context | D.11 |
| `role_prompts.rs:353-377` | budget-aware composition through `PromptComposer` | D.06, D.11 |
| `role_prompts.rs:451-479` | scorer/context wiring for budgeted role prompts | D.11 |
| `prompt.rs:285-458` | `PromptComposer` and auction-tag emission | D.06, D.11 |
| `prompt.rs:587-619` | VCG-style payment summary | D.06 |
| `prompt.rs:1036-1106` | auction-affect and payment-diagnostics tests | D.06 |

---

## crates/roko-learn/src/

### Routing and learning consumers

| File | What | Section |
|------|------|---------|
| `cascade_router.rs:33-33` | `BehavioralState` / `DaimonPolicy` import boundary | B.06 |
| `cascade_router.rs:884-900` | `select_with_hysteresis(...)` | B.09 |
| `cascade_router.rs:994-1009` | `CascadeRouter` struct | B.06 |
| `cascade_router.rs:3293+` | routing tests using `DaimonPolicy` | B.06, B.09 |
| `cascade_router.rs:3718-3733` | hysteresis threshold tests | B.09 |
| `model_router.rs:49-49` | `DaimonPolicy` import | B.06 |
| `model_router.rs:140-159` | routing context carries first-class `daimon_policy` | B.06 |
| `model_router.rs:1343-1374` | model-router tests using `DaimonPolicy` | B.06 |
| `runtime_feedback.rs:323-410` | `LearningRuntime` opens persisted daimon + learning state | B.06 |
| `runtime_feedback.rs:430-449` | alternate model-list open path | B.06 |
| `runtime_feedback.rs:782-830` | `record_completed_run(...)` learning fan-out | B.06 |
| `runtime_feedback.rs:1733+` | runtime-feedback tests | B.06 |
| `model_experiment.rs:216-216` | `ExperimentStore::load_or_new(...)` | B.06 |

---

## crates/roko-cli/src/ and crates/roko-serve/src/

### Runtime call sites

| File | What | Section |
|------|------|---------|
| `crates/roko-cli/src/orchestrate.rs:2824-2850` | current affect snapshot -> `DaimonPolicy` routing context | A.03, B.06 |
| `crates/roko-cli/src/orchestrate.rs:6267-6267` | gate-result appraisal | B.02 |
| `crates/roko-cli/src/orchestrate.rs:6918-6992` | queue-wait and blocked appraisal paths | B.02 |
| `crates/roko-cli/src/orchestrate.rs:8378-8378` | time-pressure appraisal | B.02 |
| `crates/roko-cli/src/orchestrate.rs:8600-8604` | successful task outcome + somatic recording | B.02, C.08 |
| `crates/roko-cli/src/orchestrate.rs:10182-10186` | failure outcome + somatic recording | B.02, C.08 |
| `crates/roko-cli/src/orchestrate.rs:11483-11495` | live somatic query and dispatch modulation | C.08 |
| `crates/roko-cli/src/orchestrate.rs:11812-11927` | prompt assembly injects daimon section and PAD attrs | D.06, D.11 |
| `crates/roko-cli/src/orchestrate.rs:14378-14429` | `build_daimon_context_section(...)`, `current_daimon_policy()`, tool-policy shaping | B.06, D.11 |
| `crates/roko-serve/src/dispatch.rs:1757-1757` | HTTP path builds system prompt | D.11 |
| `crates/roko-serve/src/dispatch.rs:2218-2237` | HTTP dispatch appraisal path | B.02 |
| `crates/roko-serve/src/routes/providers.rs:150-157` | provider-routing context uses `DaimonPolicy::default()` | B.06 |
| `crates/roko-serve/src/dreams.rs:277-277` | dream-failure appraisal | D.12 |
| `crates/roko-serve/src/dreams.rs:592-627` | affect state exposed in dreams HTTP surface/tests | D.12 |

---

## crates/roko-conductor/src/

### Shipping cousin surfaces for Doc 11

| File | What | Section |
|------|------|---------|
| `diagnosis.rs:137-235` | diagnosis engine and confidence scoring | D.09 |
| `diagnosis.rs:534+` | diagnosis tests | D.09 |
| `circuit_breaker.rs:28-118` | circuit breaker implementation | D.10 |

These are useful cross-references for Doc 11, but they do **not**
constitute shipped daimon-side per-crate confidence, familiarity
tracking, or fatigue detection.

---

## Missing / Absent (code-search negatives)

These doc features still have no matching production code in the active
tree.

### ALMA layers and octant runtime

| Absent Feature | Search | Section |
|----------------|--------|---------|
| three-layer `emotion/mood/personality` affect state | `rg -n "PersonalityTrait|MoodLayer|Personality layer|struct Personality" crates -g '*.rs'` | A.08, A.09 |
| `AffectOctant` runtime enum | `rg -n "AffectOctant|Excited|Surprised|Relaxed|Depressed" crates -g '*.rs'` | A.05, C.12 |
| stored Plutchik field on `EmotionalTag` | `rg -n "discovery_emotion|pub emotion: String|Plutchik label" crates/roko-core crates/roko-neuro -g '*.rs'` | D.01 |

### Somatic and strategy frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| domain-native non-coding `StrategySpaceComputer`s | `rg -n "ChainStrategySpace|ResearchStrategySpace|TradingStrategySpace" crates -g '*.rs'` | C.03 |
| mind wandering | `rg -n "mind_wander|WANDER_INTERVAL|200_tick" crates -g '*.rs'` | C.09 |
| rolling-window contrarian tracker | `rg -n "rolling_window|contrarian_tracker" crates -g '*.rs'` | C.09, C.10 |

### Coding integration frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| per-crate confidence ledger | `rg -n "per_crate_confidence|CrateFamiliarity|crate_success_rate" crates -g '*.rs'` | D.08 |
| error-pattern familiarity tracker in daimon | `rg -n "error_pattern_tracker|PatternFamiliarity|familiarity_score" crates/roko-daimon crates/roko-agent -g '*.rs'` | D.09 |
| fatigue detection in daimon | `rg -n "fatigue|Fatigue|consecutive_failures" crates/roko-daimon crates/roko-learn -g '*.rs'` | D.10 |

### Collective contagion frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| inter-agent emotional contagion | `rg -n "contagion|Contagion|borrowed_emotion|emotional_sync|PAD_sync" crates apps -g '*.rs'` | E.02 |
| somatic field / C-Factor | `rg -n "c_factor|CFactor|somatic_field|collective_intelligence" crates apps -g '*.rs'` | E.03 |

---

## Runtime Negatives That Matter For Batch 09

These matter because code exists, but the docs can still mislead:

| Runtime-negative | Evidence | Section |
|------------------|----------|---------|
| Doc 12 still presents frontier contagion as “Built” | `docs/09-daimon/12-collective-emotional-contagion.md:6` conflicts with `:253-259` | E.01-E.04 |
| `roko-golem` can still be misread as a live dependency in topic-09 prose | active runtime lives in `roko-core` + `roko-daimon`; no `crates/roko-golem` exists | A.05, D.08-D.10, E.06 |
| `EmotionalTag` examples can overstate schema | shipping struct has no stored emotion string | D.01 |
| behavioral-state hysteresis is easy to misread | classifier is memoryless; router hysteresis is separate | B.09 |
| `ContextAssembler` ownership is easy to misread | implementation lives in Neuro; Compose re-exports | D.02 |

---

## Practical Search Priorities

Before editing, search these first:

```bash
rg -n "PadVector|BehavioralState|DaimonPolicy|EmotionalTag|AffectEvent" crates -g '*.rs'
rg -n "AffectOctant|Plutchik|roko-golem|discovery_emotion" docs/09-daimon tmp/docs-parity/09
rg -n "ContextAssembler|PromptComposer|externality|VCG" crates/roko-neuro crates/roko-compose docs/09-daimon -g '*.rs'
rg -n "hysteresis|bandit|prediction-error threshold" crates/roko-learn docs/09-daimon -g '*.rs'
rg -n "per-crate confidence|fatigue|contagion|C-Factor" docs/09-daimon crates -g '*.rs'
```

## Working Rule

If a Daimon task requires:

- new affect runtime code,
- new multi-agent contagion code,
- new per-crate confidence or fatigue implementation,
- or exact economic/VCG settlement logic,

then batch `09` should usually tighten the documentation/status
contract and defer the runtime build unless the user explicitly asks for
implementation work.
