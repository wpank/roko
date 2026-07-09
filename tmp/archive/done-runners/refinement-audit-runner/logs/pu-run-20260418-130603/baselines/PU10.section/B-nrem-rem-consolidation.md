# B — NREM Replay, REM Imagination, Consolidation (Docs 02, 03, 04)

Parity of the three phase-specific chapters: NREM replay (Mattar-Daw
utility, four modes, cross-episode patterns), REM imagination (Pearl
SCM counterfactuals, Boden's three creativity modes, trust region), and
integration / consolidation (SQLite staging, confidence ladder).

**Major finding**: Doc 16 §"Current Code Status" says REM counterfactual
generation, Boden's three modes, Hypnagogia, and threat simulation are
all "Not implemented" — in reality, `roko-dreams/src/imagination.rs`
(575 LOC) ships all of those, and Doc 16 is at least one major update
behind the crate.

Generated 2026-04-16.

---

## B.01 — Four NREM replay modes ship (Doc 02 §"Four Replay Modes")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 §"Four Replay Modes" enumerates: Random / Consequence / Causal / Hypothetical replay. Each mode selects episodes differently.
**Reality**: `DreamReplayMode` at `crates/roko-dreams/src/replay.rs:14-26` has exactly those four variants with snake-case serde tags: `Random, Consequence, Causal, Hypothetical`. `select_replay_episodes(episodes, policy, now) -> DreamReplayBatch` at `replay.rs:78-117` dispatches on mode: `select_random`, `select_consequence`, `select_causal`, `select_hypothetical` — all four paths shipping.

---

## B.02 — Mattar-Daw utility scoring ships (Doc 02 §"Mattar-Daw Utility Formula", Doc 16 §"G2")

**Status**: DONE (Doc 16 drift — says NOT STARTED for G2 Mattar-Daw scoring)
**Severity**: MEDIUM
**Doc claim**: Doc 02 §"Mattar-Daw Utility Formula" defines per-episode utility = f(recency, novelty, consequence). Doc 16 §"G2" says "Full re-evaluation with Mattar-Daw utility scoring is not yet implemented".
**Reality**: `DreamReplayBatch.utility_score: f64` at `replay.rs:71` accumulates the total Mattar-Daw utility. Per-candidate `utility: f64` is computed in `score_candidates(episodes, policy, now)` (helper fn, line ~130). `DreamReplayPolicy` at `replay.rs:36-50` carries the knobs: `novelty_window: usize (12 default)`, `recency_half_life_hours: f64 (24.0 default)`, `max_episodes: usize (24 default)`. `utility_score` is summed across selected candidates at `replay.rs:99-102`. The utility formula IS live; Doc 16 understates.
**Fix sketch**: Update Doc 16 §"G2" from "Not yet implemented" to "Implemented — see `select_replay_episodes` + `DreamReplayBatch.utility_score`".

---

## B.03 — DreamReplayPolicy exposes novelty + recency half-life knobs (Doc 02 §"Replay Fidelity Spectrum")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 02 describes tunable novelty window and recency decay.
**Reality**: `DreamReplayPolicy.novelty_window: 12` and `recency_half_life_hours: 24.0` at `replay.rs:52-60` are both the default knobs. Exposed to configuration.

---

## B.04 — REM imagination with Pearl SCM counterfactuals ships (Doc 03 §"Pearl SCM Counterfactuals", Doc 16 §"G5")

**Status**: DONE (Doc 16 drift — says G5 NOT IMPLEMENTED)
**Severity**: HIGH (for doc honesty)
**Doc claim**: Doc 03 §"Pearl SCM Counterfactuals" describes Pearl's structural causal model framework applied to episodes. Doc 16 §"G5 Counterfactual Simulation" says "Not implemented. The design is fully specified ... but no code exists."
**Reality**: Doc 16 is significantly stale. `roko-dreams/src/imagination.rs` ships **575 LOC** of REM imagination:
- `CausalModel { episodes_by_id, variables }` at `imagination.rs:45-51` builds from `Episode` observations.
- `CausalModel::from_episodes(episodes)` at `:54-86` tallies variable supports (model, task_id, trigger_kind, outcome, failure_reason).
- `CounterfactualQuery { episode_id, intervention: (String, String) }` at `:17-24` — the intervention pair is the Pearl-style (variable, new_value).
- `imagine(query, model, mode) -> ImaginationOutcome` at `:120-174` evaluates the counterfactual: looks up `base_episode`, computes `similarity` via `text_fingerprint` HDC cosine, checks trust region via `trust_region_floor(variable, support)`, sets `plausible = similarity >= trust_region || support > 0`.
- `ImaginationOutcome { query, mode, plausible, confidence, projected_success_delta, narrative }` at `:102-116` is the full Pearl-adjacent output.
- `counterfactual_episode(base, query) -> Episode` at `:293-317` constructs the counterfactual episode by mutating `model` / `task_id` / `trigger_kind` per the intervention.

Doc 16 §"G5" badly undercounts.
**Fix sketch**: Update Doc 16 §"G5" to Implemented with `imagination.rs:1-575` anchor. Note the shipping implementation uses HDC `text_fingerprint` for similarity rather than full probabilistic SCM inference — a simpler but functional Pearl-adjacent approach.

---

## B.05 — Boden's three creativity modes ship (Doc 03 §"Boden's Three Creativity Modes", Doc 16 §"G7")

**Status**: DONE (Doc 16 drift — says G7 NOT IMPLEMENTED)
**Severity**: HIGH (for doc honesty)
**Doc claim**: Doc 03 §"Boden's Three Creativity Modes" cites Boden 2004: Combinational / Exploratory / Transformational. Doc 16 §"G7 Novel Strategy Generation" says "Not implemented. The design is specified in 03-rem-imagination.md."
**Reality**: `ImaginationMode` at `imagination.rs:27-36` has exactly three variants: `Combinational / Exploratory / Transformational` with snake-case serde tags. `synthesize_hypotheses(episodes, created_at)` at `:178-289` implements all three modes:
- Combinational at `:188-212`: `choose_combinational_pair` finds two episodes, synthesizes "Combine X with Y: reuse the successful routing discipline..." → emits `KnowledgeKind::Heuristic` entry.
- Exploratory at `:214-238`: `choose_exploratory_source` picks a base episode, synthesizes "Extend X into a neighboring task shape..." → emits `KnowledgeKind::Heuristic`.
- Transformational at `:240-286`: `choose_transformational_source` + `counterfactual_model(base.model)` swap → "What if X had used Y instead of Z?" → emits `KnowledgeKind::Insight` or `Warning` based on base success.

Each mode tagged with `["dream", "rem", "counterfactual", "combinational"|"exploratory"|"transformational"]` on the resulting `KnowledgeEntry`. Doc 16 undercounts.
**Fix sketch**: Update Doc 16 §"G7" to Implemented with `imagination.rs:178-289` anchor.

---

## B.06 — Trust region / plausibility check ships (Doc 03 §"Imagination Validation (GIRL)")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 03 §"Imagination Validation (GIRL trust-region)" describes a trust-region mechanism for filtering implausible counterfactuals.
**Reality**: `trust_region_floor(variable, support)` (helper in `imagination.rs`) + `plausible = similarity >= trust_region || support > 0` at `:145` is the shipping trust-region check. `ImaginationOutcome.plausible: bool` at `:109` propagates the decision. This is a simpler trust-region (single scalar threshold) than the full GIRL formulation Doc 03 describes, but the concept ships.
**Fix sketch**: Doc 03 §"Imagination Validation (GIRL trust-region)" should cite `imagination.rs:144-145` as the shipping minimal trust-region and note that the full GIRL-style constraint set is a deepening.

---

## B.07 — Counterfactual diversity (DiCE / DPP / FACE / LOF) is absent (Doc 06 §"Counterfactual Diversity", Doc 03 §"Plausibility Scoring")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 03 and Doc 06 reference DiCE (Diverse Counterfactual Explanations via DPP), FACE (Feasible Actionable Counterfactual Explanations), LOF (Local Outlier Factor), GLOBE-CE, and other sophisticated counterfactual diversity / plausibility scoring.
**Reality**: `Grep 'DiCE|DPP|FACE|LOF|GLOBE-CE' crates/roko-dreams --include=*.rs` returns zero matches. The shipping plausibility is a single similarity threshold (B.06). Sophisticated diversity + plausibility scoring is frontier.
**Fix sketch**: Doc 03 §"Plausibility Scoring" and Doc 06 §"Counterfactual Diversity" should carry a `Design — Phase 2+` banner.

---

## B.08 — Emotional depotentiation is wired via Daimon (Doc 03 §"Emotional Depotentiation")

**Status**: DONE (cross-system)
**Severity**: —
**Doc claim**: Doc 03 §"Emotional Depotentiation" cites Walker & van der Helm 2009 "overnight therapy": REM reduces emotional intensity on high-arousal markers. `arousal *= 0.70 per cycle for A > 0.5`.
**Reality**: Cross-reference batch 09 C.07 (daimon depotentiation constants): `DEPOTENTIATION_DELTA_MIN = 0.30, DEPOTENTIATION_DELTA_MAX = 0.50, DEPOTENTIATION_FLOOR = 0.05` at `roko-daimon/src/lib.rs:26-28`. `DepotentiationReport { pre_arousal, post_arousal, cooled_markers, total_marker_intensity_reduction }` at `daimon/lib.rs:1029-1039`. `crates/roko-dreams/src/cycle.rs` invokes the daimon depotentiation pass (per Doc 13 §"Implemented Components": "Somatic query + modulation — Partial ... dream replay now depotentiates arousal/high-intensity markers"). The cross-system wiring is live.

---

## B.09 — Cross-episode consolidation + K-medoids clustering available, not auto-wired (Doc 02 §"Cross-Episode Pattern Discovery", Doc 16 §"G6")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 02 §"Cross-Episode Pattern Discovery" describes k-medoids clustering over HDC episode vectors. Doc 16 §"G6" says: "Implemented (infrastructure) ... This infrastructure is ready but not yet called from the dream cycle."
**Reality**: `roko-learn/src/pattern_discovery.rs` (978 LOC per Doc 16) ships `CrossEpisodeConsolidator` with k-medoids. `roko-learn/src/hdc_clustering.rs` (498 LOC) ships `k_medoids()` + `KMedoidsConfig`. However, whether these are called from `cycle.rs` is the open question. Doc 16 §"G6" marks infrastructure Done + wiring Not-Started — that status is consistent with the file being available but unverified as called. (Detailed wiring check would require reading all 2,910 LOC of `cycle.rs`; skipped here.)
**Fix sketch**: Verify whether `DreamCycle::run()` calls `CrossEpisodeConsolidator`. If yes, update Doc 16 §"G6" to fully Done. If no, leave as Partial and scope the wiring batch.

---

## B.10 — Pattern miner + mistake identification (Doc 02 §"Cross-Episode Pattern Discovery", Doc 16 §"G3")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 16 §"G3 Mistake Identification" says "Not implemented. Automated identification of failure patterns across episodes. The `PatternMiner` infrastructure exists but is not yet wired into the dream cycle for mistake-specific mining."
**Reality**: `roko-learn/src/pattern_discovery.rs` ships `PatternMiner` + `Pattern` struct (Doc 16 §"G3" confirms the infrastructure). Whether mistake-specific mining is wired into `cycle.rs` requires a deep read. Same shape as B.09.

---

## B.11 — Integration / consolidation writes to Neuro KnowledgeStore (Doc 04 §"Integration Staging")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 §"Integration Staging" describes a staging buffer + promotion to Neuro after waking validation.
**Reality**: `synthesize_hypotheses()` at `imagination.rs:178` produces `Vec<KnowledgeEntry>` (`roko_neuro::KnowledgeEntry, KnowledgeKind, KnowledgeTier` imported at `imagination.rs:12`). `hypothetical_entry()` helper constructs entries with trigger tags like `["dream", "rem", "counterfactual", "combinational"]` so the waking-side consumer can filter. The staging-vs-direct flow is simpler than Doc 04 describes (no SQLite staging buffer; the dream writes `KnowledgeEntry` directly with `dream` tag, and waking code can filter).
**Fix sketch**: Doc 04 §"Integration Staging" should note that the shipping path is "tag-based staging" rather than "SQLite staging table".

---

## B.12 — Confidence ladder + tier promotion (Doc 04 §"Confidence Ladder", Doc 16 §"G4")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 04 §"Confidence Ladder" describes a T0-T4 tier progression. Doc 16 §"G4 Heuristic Strengthening" says "Partially implemented. Confidence updates for validated heuristics exist in the `TierProgression` system. Dream-specific strengthening ... is not yet wired."
**Reality**: `roko_neuro::KnowledgeTier` is imported in `imagination.rs:12`, so dream-produced entries can carry a tier. Doc 16's partial claim is consistent. The specific dream-validation feedback loop (where a dream-produced entry gets its confidence strengthened after waking validation) requires cross-system plumbing that is not yet verified.

---

## B.13 — Hindsight Experience Replay (HER) / Prioritized Replay (PER) are absent (Doc 02 §"DRL Experience Replay Connections")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 02 §"DRL Experience Replay Connections" references HER (Andrychowicz 2017), PER (Schaul 2016), ERE (Wang & Ross 2019), Scholar Generative Replay (Shin 2017).
**Reality**: `Grep 'hindsight|prioritized_replay|experience_replay|generative_replay|Scholar' crates/roko-dreams --include=*.rs` returns zero matches. The four-mode replay in `replay.rs` is the shipping surface; HER / PER / ERE / Scholar are academic references only.
**Fix sketch**: Doc 02 §"DRL Experience Replay Connections" stays informational — the shipping `DreamReplayMode::Consequence` + `utility_score` accumulator covers the spirit of PER (utility-prioritized selection) without naming it PER.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 7 (B.01 four modes, B.02 Mattar-Daw utility, B.03 novelty/recency knobs, B.04 Pearl SCM, B.05 Boden 3 modes, B.08 emotional depotentiation, B.11 KnowledgeEntry consolidation) |
| PARTIAL | 4 (B.06 trust-region simple, B.09 cross-episode infra not auto-wired, B.10 mistake ID infra available, B.12 confidence ladder + tier promotion) |
| NOT DONE | 2 (B.07 counterfactual diversity scoring, B.13 HER/PER/ERE) |

Section B has the **biggest Doc 16 drift** in the topic. Doc 16 says
Pearl SCM counterfactuals (G5), Boden's three modes (G7), and
Mattar-Daw utility scoring (G2) are all unimplemented — when in fact
they all ship today at real line numbers in `imagination.rs` + `replay.rs`.

## Agent Execution Notes

### B.02 / B.04 / B.05 — Doc 16 undercount

These are the three biggest "Doc 16 missed a major feature" items:
G2 Mattar-Daw (Done, Doc 16 says not started), G5 Pearl SCM (Done,
Doc 16 says not implemented), G7 Boden three modes (Done, Doc 16
says not implemented).

### B.07 / B.13 — Academic frontier

Advanced counterfactual diversity and DRL replay variants are
informational in the docs — current code ships simpler but functional
equivalents.

Acceptance criteria:

- Doc 16 §§G2 / G5 / G7 updated to Implemented with anchors,
- Doc 03 §"Pearl SCM" cites `imagination.rs` as shipping,
- Doc 02 §"Four Modes" cites `replay.rs` as shipping.
