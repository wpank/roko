# SOURCE-INDEX — Code Anchors for 09-Daimon Parity

Verified code references for batch `09`, organized around the actual
runtime seams an unattended docs agent will need.

Generated: 2026-04-16

---

## Important Corrections First

Use these before trusting the daimon docs literally:

- `roko-daimon` is not a scaffold. It is a live, heavily consumed affect subsystem.
- The shared affect primitives live in `crates/roko-core/src/affect.rs`, not in `roko-daimon` directly.
- `roko-golem` has been dissolved. Treat its mentions as historical provenance only.
- `roko-compose::ContextAssembler` is a re-export of `roko-neuro::ContextAssembler`; the implementation lives in Neuro.
- `EmotionalTag` does **not** carry a stored Plutchik/emotion string in the shipping struct.
- `BehavioralState::classify()` has no hysteresis; the separate hysteresis in `roko-learn/src/cascade_router.rs` is about model selection.

---

## crates/roko-core/src/

### Shared affect primitives

| File | What | Section |
|------|------|---------|
| `affect.rs:7-14` | `PadVector { pleasure, arousal, dominance }` | A.04 |
| `affect.rs:19-61` | `new`, `neutral`, `clamped`, `apply_delta`, `decay_by_factor` | A.04, A.07 |
| `affect.rs:70-81` | `cosine_similarity()` with neutral fallback | A.06, D.03 |
| `affect.rs:87-101` | `BehavioralState` enum | A.02, B.04 |
| `affect.rs:106-131` | `BehavioralState::classify(pad, confidence)` | B.04, B.09 |
| `affect.rs:134-158` | `DaimonPolicy` | A.03, B.06 |
| `affect.rs:162-189` | `EmotionalTag { pad, intensity, trigger, mood_snapshot }` | D.01 |
| `affect.rs:195-249` | classifier / PAD / tag tests | A.06, B.04, D.01 |

---

## crates/roko-daimon/src/

### Live daimon engine

| File | What | Section |
|------|------|---------|
| `lib.rs:20-30` | strategy/somatic constants including `CONTRARIAN_FRACTION = 0.15` and `kiddo` tree alias | C.05-C.07 |
| `lib.rs:37-125` | `AffectState` and decay/apply/tag helpers | A.07, B.03, D.01 |
| `lib.rs:128-222` | `StrategyCoordinates` | C.01 |
| `lib.rs:226-314` | `StrategySpaceDefinition` | C.02 |
| `lib.rs:318-545` | task / episode observations | C.04 |
| `lib.rs:548-960` | strategy computers, role-aware projection, `extract_strategy_point()` | C.03, C.04 |
| `lib.rs:963-1200` | `SomaticMarker`, `SomaticSignal`, `SomaticLandscape` | C.05-C.08, D.07 |
| `lib.rs:1336-1374` | `DispatchStrategy`, `DispatchParams` | B.05 |
| `lib.rs:1390-1439` | `AffectEvent` enum | B.01 |
| `lib.rs:1443-1760` | `DaimonState`, persistence, `modulate_with_strategy`, `AffectEngine` impl | B.02, D.05 |
| `lib.rs:2100-2600+` | appraisal / persistence / somatic tests | B.02, C.08 |

---

## crates/roko-neuro/src/

### Emotional provenance and retrieval

| File | What | Section |
|------|------|---------|
| `context.rs:204-239` | emotional provenance fields + `ContextAssembler` definition | D.02, D.03 |
| `context.rs:1839-1954` | assembler tests with affect state | D.02, D.03 |
| `knowledge_store.rs` | emotional-tag persistence / retrieval paths | D.01-D.04 |
| `distiller.rs:809-820` | emotional provenance preservation in consolidation | D.04 |
| `lib.rs:299` | consolidation multiplier from emotional provenance | D.04 |

---

## crates/roko-compose/src/

### Prompt composition and re-export boundary

| File | What | Section |
|------|------|---------|
| `context_assembler.rs:1-4` | re-export of `roko_neuro::ContextAssembler` | D.02 |
| `system_prompt_builder.rs` | live PAD / affect guidance in system prompt | D.11 |
| `role_prompts.rs` | role-specific affect framing | D.11 |
| `prompt.rs:285+` | `PromptComposer` and shared prompt auction | D.06, D.11 |
| `README.md` | `PromptComposer` usage and budget model | D.06 |

---

## crates/roko-learn/src/

### Routing and learning consumers

| File | What | Section |
|------|------|---------|
| `cascade_router.rs:33` | `DaimonPolicy` import | B.06 |
| `cascade_router.rs:884` | `select_with_hysteresis(...)` | B.09 |
| `cascade_router.rs:3693-3713` | routing hysteresis tests | B.09 |
| `model_router.rs` | `DaimonPolicy` consumption in model routing | B.06 |
| `runtime_feedback.rs` | `DaimonPolicy` in runtime feedback | B.06 |
| `model_experiment.rs`, `tests/learning_loop.rs` | learning-side daimon usage | B.06 |

---

## crates/roko-cli/src/ and crates/roko-serve/src/

### Runtime call sites

| File | What | Section |
|------|------|---------|
| `crates/roko-cli/src/orchestrate.rs:2364` | current `DaimonPolicy` assembly | A.03, B.06 |
| `crates/roko-cli/src/orchestrate.rs:5351, 5895, 5920, 7237, 7447, 8797` | live `appraise()` call sites | B.02 |
| `crates/roko-cli/src/orchestrate.rs:12875-12877` | current policy helper | B.06 |
| `crates/roko-cli/src/main.rs` | CLI wiring | A.03 |
| `crates/roko-serve/src/dispatch.rs:2218` | HTTP dispatch appraisal path | B.02 |
| `crates/roko-serve/src/routes/providers.rs` | provider routing feed from `DaimonPolicy` | B.06 |
| `crates/roko-serve/src/dreams.rs` | affect-related HTTP surface | D.12 |

---

## crates/roko-conductor/src/

### Shipping cousin surfaces for Doc 11

| File | What | Section |
|------|------|---------|
| `diagnosis.rs` | structured error categories/patterns in conductor | D.09 |
| `circuit_breaker.rs` | failure streak / breaker behavior | D.10 |

These are useful cross-references for Doc 11, but they do **not**
constitute shipped daimon-side per-crate confidence, familiarity
tracking, or fatigue detection.

---

## Missing / Absent (code-search negatives)

These doc features have no matching production code in the active tree:

### ALMA layers and octant runtime

| Absent Feature | Search | Section |
|----------------|--------|---------|
| three-layer `emotion/mood/personality` affect state | `rg -n "PersonalityTrait|MoodLayer|Personality layer|struct Personality" crates --include=*.rs` | A.08, A.09 |
| `AffectOctant` runtime enum | `rg -n "AffectOctant|Excited|Surprised|Relaxed|Depressed" crates --include=*.rs` | A.05, C.12 |
| stored Plutchik field on `EmotionalTag` | `rg -n "discovery_emotion|pub emotion: String|Plutchik label" crates/roko-core crates/roko-neuro --include=*.rs` | D.01 |

### Somatic and strategy frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| domain-native non-coding `StrategySpaceComputer`s | `rg -n "ChainStrategySpace|ResearchStrategySpace|TradingStrategySpace" crates --include=*.rs` | C.03 |
| mind wandering | `rg -n "mind_wander|WANDER_INTERVAL|200_tick" crates --include=*.rs` | C.09 |
| rolling-window contrarian tracker | `rg -n "rolling_window|contrarian_tracker" crates --include=*.rs` | C.09, C.10 |

### Coding integration frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| per-crate confidence ledger | `rg -n "per_crate_confidence|CrateFamiliarity|crate_success_rate" crates --include=*.rs` | D.08 |
| error-pattern familiarity tracker in daimon | `rg -n "error_pattern_tracker|PatternFamiliarity|familiarity_score" crates/roko-daimon crates/roko-agent --include=*.rs` | D.09 |
| fatigue detection in daimon | `rg -n "fatigue|Fatigue|consecutive_failures" crates/roko-daimon crates/roko-learn --include=*.rs` | D.10 |

### Collective contagion frontier

| Absent Feature | Search | Section |
|----------------|--------|---------|
| inter-agent emotional contagion | `rg -n "contagion|Contagion|borrowed_emotion|emotional_sync" crates apps --include=*.rs` | E.01 |
| somatic field / C-Factor | `rg -n "c_factor|CFactor|somatic_field|collective_intelligence" crates apps --include=*.rs` | E.02 |

---

## Runtime Negatives That Matter For Batch 09

These matter because the code exists, but the docs can still mislead:

| Runtime-negative | Evidence | Section |
|------------------|----------|---------|
| active docs still imply `roko-golem` runtime ownership | multiple topic-09 docs cite `roko-golem` code paths as if active | A.05, D.08-D.10, E.06 |
| `EmotionalTag` examples overstate schema | shipping struct has no `emotion` field | D.01 |
| behavioral-state hysteresis is easy to misread | classifier is memoryless; router hysteresis is separate | B.09 |
| `ContextAssembler` ownership is easy to misread | implementation in Neuro, Compose re-exports | D.02 |
| Doc 11 and Doc 12 overstate frontier behavior unless bannered | multiple “Built” or design-like descriptions | D.08-D.10, E.01-E.04 |

---

## Practical Search Priorities

Before editing, search these first:

```bash
rg -n "PadVector|BehavioralState|DaimonPolicy|EmotionalTag|AffectEvent" crates --include=*.rs
rg -n "AffectOctant|Plutchik|roko-golem|discovery_emotion" docs/09-daimon tmp/docs-parity/09
rg -n "ContextAssembler|PromptComposer|externality|VCG" crates/roko-neuro crates/roko-compose docs/09-daimon --include=*.rs
rg -n "hysteresis|bandit|prediction-error threshold" crates/roko-learn docs/09-daimon --include=*.rs
rg -n "per-crate confidence|fatigue|contagion|C-Factor" docs/09-daimon crates --include=*.rs
```

## Working Rule

If a daimon task requires:

- new affect runtime code,
- new multi-agent contagion code,
- new per-crate confidence or fatigue implementation,
- or exact economic/VCG settlement logic,

then batch `09` should normally implement the smallest honest
documentation/status contract and defer the runtime work.
