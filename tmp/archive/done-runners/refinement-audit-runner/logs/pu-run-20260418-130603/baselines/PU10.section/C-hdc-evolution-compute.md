# C — HDC Counterfactuals, Dream Evolution, Sleep-Time Compute (Docs 05, 06, 12)

Parity of three "deepening" chapters: dream evolution (memetic
selection, fourth phase), HDC counterfactual synthesis (10,240-bit
BSC, XOR binding, K-medoids clustering), and sleep-time compute
(Lin et al. 2025 budget allocation).

The HDC primitive (B.01 cross-ref from batch 08 and 09) is used by
`imagination.rs` and `cycle.rs` via `text_fingerprint`. Most of the
specific techniques Doc 05 / 06 / 12 reference (MAP-Elites, DreamerV3,
IRIS, Genie, Lin 2025 rethink_memory) are frontier work.

Generated 2026-04-16.

---

## C.01 — HDC text_fingerprint drives imagination similarity (Doc 06 §"HDC Counterfactual Synthesis")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 06 specifies 10,240-bit BSC vectors with XOR binding, majority bundling, K-medoids clustering. Used in the counterfactual synthesis pipeline.
**Reality**: `roko-primitives::hdc::text_fingerprint` (batch 09 B.01 canonical 10,240-bit primitive) is imported in `imagination.rs:13` and consumed at `imagination.rs:142` for similarity-based trust-region evaluation: `text_fingerprint(&current_value).similarity(&text_fingerprint(new_value))`. Also used in `cycle.rs` (cluster key hashing, `:129, :131`). The HDC canonical owner (`crates/roko-primitives/src/hdc.rs`) ships all three ops (bind, bundle, permute — see batch 09 B.01), and dreams uses at minimum the `similarity` (Hamming-based cosine).

---

## C.02 — K-medoids clustering over HDC episode vectors (Doc 06 §"K-Medoids Clustering", Doc 16 §"G6")

**Status**: DONE (infrastructure; cross-ref B.09 for wiring)
**Severity**: —
**Doc claim**: Doc 06 §"K-Medoids Clustering" describes k-medoids over HDC episode vectors for cross-episode pattern discovery.
**Reality**: `roko-learn/src/hdc_clustering.rs` (498 LOC per Doc 16) ships `k_medoids()`, `KMedoidsConfig { k, max_iterations }`, `HdcCluster { medoid_index, medoid, member indices }`, `ClusterResult { clusters, iterations, convergence }`. Greedy farthest-first seeding, assign-update loop. 9 tests including synthetic 3-cluster recovery (per Doc 16). The primitive ships; whether `cycle.rs` wires it into the NREM pass is the open question from B.09.

---

## C.03 — DreamClusterKey and DreamClusterReport show cluster outputs on cycle (Doc 06 §"Cross-Episode Patterns")

**Status**: DONE
**Severity**: —
**Doc claim**: Cross-episode patterns emerge from clustering + bundled representations.
**Reality**: `DreamClusterKey` at `cycle.rs:259` + `DreamClusterReport` at `cycle.rs:300` + `DreamOutcome` at `cycle.rs:282` exist as real cycle outputs. Stable HDC context vectors are hashed for cluster keys (`cycle.rs:129-131`). Cycle-level cluster reporting is wired.

---

## C.04 — XOR binding and majority bundling (Doc 06 §"HDC Operations")

**Status**: DONE (via canonical primitive)
**Severity**: —
**Doc claim**: Doc 06 §"HDC Operations" requires `BIND` (XOR), `BUNDLE` (majority vote), `PERMUTE` (cyclic shift) primitives.
**Reality**: All three ship in `roko-primitives/src/hdc.rs` at `:107-164` (bind, bundle, permute — see batch 09 B.01). Dreams currently uses `text_fingerprint` (seeded) and `similarity`; binding / bundling are available if the cycle ever needs to construct composite vectors (they are not currently used by dreams per grep).

---

## C.05 — Dream evolution / EVOLUTION fourth phase is pure design (Doc 05 §"Memetic Selection", §"Strategy Evolution")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 05 describes a fourth "EVOLUTION" dream phase: memetic selection, strategy evolution, HDC knowledge recombination, dream-prediction feedback. Cites MAP-Elites (Mouret & Clune 2015), DCRL-MAP-Elites (ACM TELO 2024), Rainbow Teaming (NeurIPS 2024).
**Reality**: `Grep 'MAP-?Elites|memetic|dream_evolution|quality_diversity' crates/roko-dreams --include=*.rs` returns zero matches. No evolution phase in the shipping `DreamCycle`. The three-phase NREM → REM → Integration pipeline (A.03) is the shipping cycle; Doc 05's fourth phase is frontier.
**Fix sketch**: Doc 05 should carry a `Design — Phase 2+` banner. The Doc 05 abstract currently leaves the phase count ambiguous (Doc 01 says three phases, Doc 05 talks about a fourth).

---

## C.06 — MAP-Elites quality-diversity archive absent (Doc 05 §"MAP-Elites Quality-Diversity")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 05's 2025-04 enhancement added MAP-Elites archive architecture, QD-score algorithm, behavioral descriptor dimensions. Cross-linked to Rainbow Teaming.
**Reality**: No MAP-Elites in dreams (see C.05 grep). The shipping "diversity" surface is the four `DreamReplayMode` variants + the three `ImaginationMode` variants — no archive of behavioral descriptors.

---

## C.07 — Dream → strategy update feedback (Doc 05 §"Dream-Prediction Feedback")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Dream outcomes should feed back into: (a) gate threshold updates, (b) CascadeRouter updates, (c) playbook revisions.
**Reality**: `roko-dreams/Cargo.toml:17` declares `roko-learn` dep (which owns `cascade_router.rs`). `DreamCycle` at `cycle.rs:333` takes `PlaybookStore` per Doc 16 §"Consolidation pipeline". Whether specific dream → gate / dream → cascade / dream → playbook update paths are wired is the Doc 16 §"Phase 4 Integration and Feedback" open status: all four items marked "Not started" there (Gate / CascadeRouter / Playbook / Mesh). The crate has the dependencies; the feedback plumbing is unverified.
**Fix sketch**: Doc 05 §"Dream-Prediction Feedback" and Doc 16 §"Phase 4" should call out this as the open integration seam. See E-integration-status.md.

---

## C.08 — Sleep-time compute (Lin et al. 2025) is pure design (Doc 12 §"Lin et al. 2025 Sleep-Time Compute")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 describes Lin et al. 2025 sleep-time compute: `rethink_memory` function calling, query predictability metric `log P(q|c)`, 10× cost weighting, 2.5× multi-query amortization. 5× test-time compute reduction.
**Reality**: `Grep 'rethink_memory|sleep_time|query_predictability' crates/roko-dreams --include=*.rs` returns zero matches. The shipping `DreamBudget` (see A.08) is a budget envelope, not the Lin et al. rethink-memory-specific mechanism. No precomputed summaries to reduce test-time compute.
**Fix sketch**: Doc 12 should carry a `Design — Phase 2+` banner. Reference Lin et al. 2025 as a future optimization direction.

---

## C.09 — CascadeRouter-based model selection for dreams (Doc 12 §"CascadeRouter Model Selection")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 12 §"CascadeRouter Model Selection" describes using the CascadeRouter to pick a cheap model for routine dreams and an expensive model for intensive consolidation.
**Reality**: `DreamAgentConfig` at `runner.rs:43` carries `command`, `args`, `model`, `bare_mode`, `effort`, `timeout_ms`, `env` — the dream agent is configurable, and default is `claude-opus-4-6` (per Doc 16). But the **dynamic CascadeRouter selection per dream cycle** is not wired — the agent config is static. The CascadeRouter's `DaimonPolicy` consumption (batch 09 B.06) gives it affect awareness, but the dream cycle itself is not calling the router for model selection.
**Fix sketch**: Doc 12 §"CascadeRouter Model Selection" should flag this as a wiring gap.

---

## C.10 — Sleepwalker mode is absent (Doc 12 §"Sleepwalker Mode")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 12 §"Sleepwalker Mode" describes a lightweight overnight consolidation mode that only uses local (non-LLM) pattern discovery.
**Reality**: `Grep 'Sleepwalker|sleepwalker|local_only_dream' crates/roko-dreams --include=*.rs` returns zero matches. Not implemented.

---

## C.11 — World models (DreamerV3 / IRIS / Genie) are not integrated (Doc 03 §"World Models")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 03 references DreamerV3 (Hafner et al. 2025), IRIS (Micheli et al. 2023), Delta-IRIS (ICML 2024), Genie 2/3 (DeepMind 2024-2025) as advanced world-model variants.
**Reality**: `Grep 'DreamerV3|Dreamer|IRIS|Genie|world_model' crates/ --include=*.rs` returns zero matches in `roko-dreams`. The shipping counterfactual synthesis uses a simpler statistical `CausalModel` (B.04), not a neural world model.
**Fix sketch**: Doc 03 §"World Models" stays informational — the shipping Pearl-adjacent approach is functional without a neural world model.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 4 (C.01 HDC fingerprint driving similarity, C.02 k-medoids infrastructure, C.03 DreamClusterKey/Report on cycle, C.04 HDC primitives available) |
| PARTIAL | 2 (C.07 dream → strategy update feedback, C.09 CascadeRouter per-cycle selection) |
| NOT DONE | 5 (C.05 evolution fourth phase, C.06 MAP-Elites, C.08 Lin et al. sleep-time compute, C.10 Sleepwalker mode, C.11 world models) |

Section C is dominated by **academic-frontier** concepts (MAP-Elites,
Lin sleep-time compute, DreamerV3, IRIS, Genie) that the shipping
code hasn't and doesn't need to implement to do its job. The HDC
primitive is cleanly owned by `roko-primitives` and used by dreams
via `text_fingerprint`. K-medoids infrastructure is available but not
verified as wired into the cycle.

## Agent Execution Notes

### C.05 / C.06 / C.08 / C.10 / C.11 — Frontier banner pass

All five are unimplemented and require no wiring today.

### C.07 / C.09 — Wiring opportunity

Dream → gate / cascade / playbook feedback is the clearest
"deepening" target if topic 10 execution ever proceeds. Cross-refs to
Doc 16 §"Phase 4" and E-integration-status.

Acceptance criteria:

- Docs 05 / 12 carry Phase 2+ banners,
- Doc 06 §"K-Medoids" cites `roko-learn/src/hdc_clustering.rs`,
- Doc 03 §"World Models" remains informational.
