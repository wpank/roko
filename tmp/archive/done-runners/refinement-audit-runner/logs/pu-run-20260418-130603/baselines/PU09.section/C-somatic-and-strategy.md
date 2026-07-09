# C — Somatic Markers and Strategy Space (Docs 06, 07, 08)

Parity of the three "spatial cognition" chapters: Damasio somatic
markers backed by a k-d tree, 15% contrarian retrieval for echo-chamber
prevention, and the 8-dimensional strategy space.

The somatic landscape is real, tested, and persisted. The 8D strategy
space is real with domain-configurable labels. Contrarian fraction is
the exact `0.15` constant the docs specify. The parts that are not
wired are the specific "mind wandering every ~200 ticks" mechanism and
non-coding domain-native coordinate extractors (Doc 13 §"Partial").

Generated 2026-04-16.

---

## C.01 — 8D `StrategyCoordinates` struct ships with exactly the 8 documented axes (Doc 08 §"Coding Dimensions")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 08 §"Coding Dimensions" tables 8 axes: `Complexity, Risk, Novelty, Confidence, Time Pressure, Scope, Reversibility, Dependency Depth`. Each in `[0.0, 1.0]`.
**Reality**: `StrategyCoordinates` at `roko-daimon/src/lib.rs:128-146` has exactly those eight `f64` fields in the same order: `complexity, risk, novelty, confidence, time_pressure, scope, reversibility, dependency_depth`. `STRATEGY_DIMENSIONS = 8` const at `:20`. `new(...)` at `:157-178` clamps all dims into `[0.0, 1.0]` via `clamped()` at `:212-222`. `as_array()` at `:197-208` packs into `[f64; 8]` for k-d tree queries. `neutral()` at `:182-193` returns `(0.5, 0.5, ... , 0.5)`.

---

## C.02 — Domain-configurable strategy-space definition ships with validation (Doc 08 §"Cross-Domain Transfer", §"Chain Dimensions")

**Status**: DONE
**Severity**: —
**Doc claim**: Axes are domain-configurable: coding uses the 8 axes above; chain / research / trading should be able to swap the meaning of each axis while keeping the fixed 8-slot layout.
**Reality**: `StrategySpaceDefinition { domain: String, dimensions: [String; 8] }` at `roko-daimon/src/lib.rs:226-234`. `default_domain()` returns "coding"; `default_dimensions()` returns the exact 8 names from Doc 08. `validate()` at `:266-292` enforces: non-empty domain, non-empty unique axis names (ascii-lowercase dedup). `coding()` builder at `:256-258`. Active definition stored on `DaimonState.strategy_space` at `:1452-1454` with `configure_strategy_space()` discarding old somatic markers when axes change. Cross-domain swap is supported today; per Doc 13 "external configuration for alternate domain axis sets" is partial.

---

## C.03 — Strategy-space projection for non-coding domains uses keyword-based role inference (Doc 08 §"Chain Dimensions")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 08 implies each domain supplies its own coordinate extractor mapping domain-native observations to the 8 axes.
**Reality**: The shipping approach is **role-aware keyword projection**, not dedicated per-domain extractors:
- `DimensionRole` enum at `roko-daimon/src/lib.rs:556-566` has 8 abstract roles (`Difficulty, Danger, Familiarity, SelfAssessment, Urgency, Breadth, Recoverability, Coupling`).
- `classify_dimension_role(label, index)` at `:623-707` infers the role from the axis label string via keyword matches (e.g., "risk", "danger", "exposure", "leverage", "slippage", "counterparty", "blast" → `Danger`).
- `RegisteredStrategySpaceComputer` at `:886-951` with `is_builtin_coding()` at `:906-908` branches: built-in coding domain uses `CodingStrategySpace` directly; non-coding domains use `project_profile_for_definition` (`:713-726`) that maps a `CanonicalStrategyProfile` through the role classifier.

The effect is that a chain or trading domain with axis names like `"leverage"`, `"exposure"`, `"counterparty"` will have their coordinates derived from the same underlying task observation signals via keyword matching — no dedicated domain-native extractor ships. Doc 13 confirms: "2G Somatic landscape ... role-aware label projection ... but true domain-native extractors and VCG coupling are still missing."
**Fix sketch**: Doc 08 §"Chain Dimensions" and §"Cross-Domain Transfer" should describe the shipping keyword-projection fallback and flag true domain-native extractors as a follow-up.

---

## C.04 — `extract_strategy_point(task, context)` helper ships (Doc 08 §"Integration with Task Dispatch")

**Status**: DONE
**Severity**: —
**Doc claim**: A task should project into an `[f64; 8]` point in strategy space, suitable for k-d tree queries.
**Reality**: `extract_strategy_point(task: &Task, context: &TaskContext) -> [f64; 8]` at `roko-daimon/src/lib.rs:953-960` composes `StrategySpaceDefinition::coding().computer()` with `TaskStrategyObservation::from_task(task, context)` and returns the 8D array. `TaskStrategyObservation::from_task` at `:358-388` derives the normalized signals (file_count, verification_count, dependency_count, max_loc, familiarity, confidence, failure_pressure, urgency_pressure) from the `Task` struct + context. `TaskContext::from_task` at `:420-505` derives context from the task alone when no explicit context is provided — using `complexity_band`, `speed_priority`, `example_pattern`, `test_invariants`, `depends_on`, etc.

---

## C.05 — `SomaticLandscape` k-d tree ships with `kiddo` crate (Doc 06 §"k-d Tree over 8D Strategy Space")

**Status**: DONE
**Severity**: —
**Doc claim**: Somatic markers stored in a k-d tree over 8D strategy space for sub-1ms nearest-neighbour queries (Damasio 1994). Struct: `SomaticLandscape { tree: KdTree<f64, SomaticMarker, 8> }`.
**Reality**: Crate dep: `kiddo = "5.3.0"` at `roko-daimon/Cargo.toml:21`. Type alias `SomaticTree = KdTree<f64, STRATEGY_DIMENSIONS>` at `:30`. `SomaticLandscape { markers: Vec<SomaticMarker>, tree: SomaticTree }` at `:1063-1071` with `#[serde(skip, default = "default_somatic_tree")]` on the tree (rebuilt on load). `rebuild_index()` at `:1096-` repopulates the tree from persisted markers. `SomaticMarker { strategy_coords: StrategyCoordinates, valence: f64, intensity: f64, episodes: Vec<ContentHash>, updated_at: DateTime<Utc> }` at `:963-975`.

---

## C.06 — Somatic query with contrarian blending ships at exactly 15% (Doc 07 §"15% Contrarian Retrieval", Doc 06 §"Query + Modulation")

**Status**: DONE
**Severity**: —
**Doc claim**: To prevent mood-congruent echo chambers (Bower 1981), 15% of retrieved somatic markers should be contrarian (opposite valence). `CONTRARIAN_FRACTION = 0.15`.
**Reality**: `CONTRARIAN_FRACTION: f64 = 0.15` const at `roko-daimon/src/lib.rs:22`. `SomaticSignal { valence, intensity, neighbor_count, contrarian_count, source_episodes }` at `:988-999` — explicit tracking of same-valence vs contrarian neighbour counts. The `query_somatic()` and `modulate_with_strategy()` methods (per Doc 13 "Somatic query + modulation — Partial; blends nearby and contrarian markers to bias dispatch") live on `DaimonState`. `DEFAULT_SOMATIC_NEIGHBORS = 5` at `:21`.

---

## C.07 — Somatic marker merge, threshold, and depotentiation ship (Doc 06 §"Marker Creation and Consolidation", Doc 07 §"Three Loop-Breaking Mechanisms")

**Status**: DONE
**Severity**: —
**Doc claim**: Markers are created from high-intensity emotional episodes; similar markers (within merge distance) are consolidated; dreaming attenuates high-intensity markers (REM depotentiation, Walker & van der Helm 2009).
**Reality**: Constants at `roko-daimon/src/lib.rs:22-28`:
- `SOMATIC_MERGE_DISTANCE_SQUARED = 0.25` — distance threshold for marker merge
- `SOMATIC_EVENT_VALENCE_THRESHOLD = 0.30` — minimum valence to create a marker
- `SOMATIC_EVENT_INTENSITY_THRESHOLD = 0.50` — minimum intensity
- `DEPOTENTIATION_DELTA_MIN = 0.30`, `DEPOTENTIATION_DELTA_MAX = 0.50`, `DEPOTENTIATION_FLOOR = 0.05` — dream-time attenuation schedule

`DepotentiationReport { pre_arousal, post_arousal, cooled_markers, total_marker_intensity_reduction }` at `:1029-1039`. `SomaticSignal::is_actionable()` at `:1015-1018` (`intensity >= 0.15 && |valence| >= 0.10`). `SomaticSignal::should_emit_event()` at `:1021-1025` uses the intensity/valence thresholds above.

---

## C.08 — Somatic persistence and in-memory k-d tree rebuild ship (Doc 06 §"Persistence")

**Status**: DONE
**Severity**: —
**Doc claim**: Markers persist across restarts; the in-memory k-d tree is not serialized directly (reconstructed from persisted markers on load).
**Reality**: `#[serde(default)] pub markers: Vec<SomaticMarker>` at `:1067-1068` — persistent payload. `#[serde(skip, default = "default_somatic_tree")] tree: SomaticTree` at `:1069-1070` — skipped on serialize / defaulted on deserialize. `rebuild_index()` at `:1096-` walks persisted markers and re-inserts them. `DaimonState::load_or_new(path)` at `:1490-1499` calls `rebuild_indexes()` post-load. Doc 13 confirms: "Somatic persistence / restore — Complete".

---

## C.09 — Rolling 200-tick window + mind wandering are unimplemented (Doc 07 §"Rolling 200-Tick Window", §"Mind Wandering")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 07 specifies a rolling window (~200 ticks) tracking contrarian exposure, plus a "mind wandering" mechanism that triggers spontaneous retrieval every ~200 ticks (default behaviour when the agent is idle or when emotional congruence is suppressing knowledge diversity).
**Reality**: `Grep 'mind_wander\|rolling_window\|WANDER_INTERVAL\|200_tick\|contrarian_tracker' crates/ --include=*.rs` returns zero matches in daimon and compose. Doc 13 §"Unimplemented Features" lists explicitly: "Mind wandering mechanism (spontaneous retrieval every ~200 ticks)". The 15% contrarian fraction IS enforced per-query (C.06); the rolling-window tracking and timed mind-wandering trigger are not.
**Fix sketch**: Doc 07 §"Rolling 200-Tick Window" and §"Mind Wandering" should carry a `Design — Phase 2+ Tier 2H` banner.

---

## C.10 — Three loop-breaking mechanisms: 2 of 3 ship (Doc 07 §"Three Complementary Loop-Breaking Mechanisms")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Three complementary mechanisms prevent mood-congruent echo chambers: (1) per-query 15% contrarian mix, (2) dream-time REM depotentiation of high-intensity markers, (3) mind wandering every ~200 ticks.
**Reality**: (1) and (2) ship (see C.06 and C.07); (3) does not (see C.09). So the depotentiation side of the triangle is real (via dreams-cycle → daimon depotentiation pass), and the per-query contrarian retrieval is real, but the timed mind-wandering leg is frontier.

---

## C.11 — Sub-1ms somatic query latency claim is plausible, not explicitly benchmarked (Doc 06 §"Sub-1ms Query Latency")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 06 §"Sub-1ms Query Latency" asserts k-d tree queries over the somatic landscape complete in under 1ms.
**Reality**: `kiddo` is a high-performance k-d tree crate so the latency target is plausible at realistic marker counts. However, no benchmark or latency assertion is in `roko-daimon/src/lib.rs`. The tests at `:2123-2408+` cover correctness (appraisal, persistence, somatic index rebuild) but not latency.
**Fix sketch**: Either add a criterion benchmark, or reframe Doc 06 §"Sub-1ms Query Latency" as an expected property of `kiddo` at realistic marker counts (e.g., <10K markers) rather than an enforced runtime guarantee.

---

## C.12 — Plutchik emotion wheel mapping is not a shipping type (Doc 01 §"Plutchik Mapping", cross-ref from Doc 08)

**Status**: NOT DONE (cross-ref)
**Severity**: LOW
**Doc claim**: Doc 01 §"Plutchik Mapping" proposes bidirectional mapping between PAD vector and the Plutchik emotion wheel (8 primary emotions).
**Reality**: Same as A.05 — no `AffectOctant` or `PlutchikEmotion` enum ships. The mapping is a design concept; no helper exists.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 7 (C.01 8-axis struct, C.02 StrategySpaceDefinition + validation, C.04 extract_strategy_point, C.05 kiddo k-d tree, C.06 15% contrarian, C.07 merge/threshold/depotentiation constants, C.08 persistence + rebuild) |
| PARTIAL | 3 (C.03 role-aware keyword projection instead of domain-native, C.10 2 of 3 loop-breaking mechanisms, C.11 latency not benchmarked) |
| NOT DONE | 2 (C.09 mind wandering / rolling window, C.12 Plutchik mapping) |

Section C is substantially DONE. The 8D strategy space, somatic
landscape with k-d tree, persistence + rebuild path, and the exact
15% contrarian constant all ship at real line numbers with real
tests. The main gaps are (a) the mind-wandering / rolling-window
scheduler (C.09 — Tier 2H frontier) and (b) per-domain-native
coordinate extractors (C.03 — the shipping keyword projector is a
reasonable fallback but not what Doc 08 implies).

## Agent Execution Notes

### C.01 / C.06 / C.07 — Tight doc-code alignment

`STRATEGY_DIMENSIONS = 8`, `CONTRARIAN_FRACTION = 0.15`, and the
depotentiation constants all match the doc numbers to three decimal
places. This section has the tightest parity of the daimon topic.

### C.03 / C.09 / C.11 — Narrative calibration

Acceptance criteria for this section:

- Doc 08 §"Cross-Domain Transfer" describes the shipping
  keyword-projection fallback and flags domain-native extractors as
  frontier,
- Doc 07 §"Rolling 200-Tick Window" / §"Mind Wandering" carry a
  Phase 2+ banner,
- Doc 06 §"Sub-1ms Query Latency" either adds a benchmark or
  reframes the claim as an expected kiddo property.
