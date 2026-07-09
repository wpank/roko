# D — Hypnagogia, Divergence, Threat Simulation (Docs 07, 08, 09)

Parity of three specialized-phase chapters: hypnagogia engine
(Thalamic Gate / Executive Loosener / Dali Interrupt / Homuncular
Observer), divergence and alpha problem, and threat simulation
(Revonsuo Threat Simulation Theory, FMEA/FTA/ATLAS).

**Two major Doc 16 drift findings**: (a) hypnagogia's four-layer
engine ships as `roko-dreams/src/hypnagogia.rs:1-538`, not as the
legacy `roko-golem` placeholder Doc 16 describes; (b) threat simulation
ships as `roko-dreams/src/threat.rs:1-312` with real FMEA-style
severity scoring, contradicting Doc 16's "Not started" label.

Generated 2026-04-16.

---

## D.01 — HypnagogiaEngine with 4 layers ships (Doc 07 §"Architecture", Doc 16 §"roko-golem placeholder")

**Status**: DONE (Doc 16 drift — says PLACEHOLDER IN roko-golem)
**Severity**: HIGH (for doc honesty)
**Doc claim**: Doc 07 specifies the four-layer hypnagogia engine: Thalamic Gate (stochastic resonance gate), Executive Loosener (constraint relaxation), Dali Interrupt (random break), Homuncular Observer (top-K retention). Doc 16 §"Hypnagogia placeholder (43 lines)" says it "is a pure placeholder... The Hypnagogia engine is fully designed (see 07-hypnagogia-engine.md) but not yet implemented."
**Reality**: Doc 16 is **badly stale**. `crates/roko-dreams/src/hypnagogia.rs:1-538` ships all four layers:
- `ThalamicGate { relevance_floor: 0.45, noise_floor: 0.20 }` at `hypnagogia.rs:17-22`
- `ExecutiveLoosener { neighborhood: 4, looseness: 0.35 }` at `:35-40`
- `DaliInterrupt { stride: 3, intensity: 0.55 }` at `:53-58`
- `HomuncularObserver { retention_floor: 0.40, max_candidates: 6 }` at `:71-76`
- `HypnagogiaEngine { gate, loosener, interrupt, observer }` at `:89-98`

All four layers are configured with defaults that match the design doc's legacy temperature/threshold values (per Doc 16 §"Open Questions" "T=1.3 / T=0.4"). Imports: `roko_learn::episode_logger::Episode`, `roko_neuro::{KnowledgeEntry, KnowledgeKind, KnowledgeTier}`, `roko_primitives::hdc::text_fingerprint`.
**Fix sketch**: Update Doc 16 §"Hypnagogia placeholder" entirely — the 4-layer engine ships at `roko-dreams/src/hypnagogia.rs`. Remove the `roko-golem` reference (golem is dissolved per batch 09 E.06).

---

## D.02 — Stochastic resonance / novelty filtering (Doc 07 §"Stochastic Resonance", §"Novelty Filtering")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 07 cites Gammaitoni et al. 1998 (stochastic resonance) as the foundation for `ThalamicGate.noise_floor` and Lehman-Stanley 2011 (novelty search) as the basis for Executive Loosener + Dali Interrupt.
**Reality**: `ThalamicGate.noise_floor: 0.20` at `hypnagogia.rs:21` is the shipping stochastic-resonance parameter — below-floor low-confidence signals get a probability of leaking through as noise. `ExecutiveLoosener.looseness: 0.35` at `hypnagogia.rs:39` + `DaliInterrupt.intensity: 0.55` at `:57` are the shipping novelty-search knobs. The mechanism is present; whether the exact stochastic resonance math (signal/noise balance curves) is implemented is a deeper check.
**Fix sketch**: Doc 07 should cite the shipping thresholds.

---

## D.03 — Hypnagogia → insight pipeline (Doc 07 §"Hypnagogia-to-Insight Pipeline, Wallas/Collins-Loftus")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 07 §"Hypnagogia-to-Insight Pipeline" cites Wallas 1926 (preparation → incubation → illumination → verification) + Collins & Loftus 1975 (spreading activation) as the flow that hypnagogia uses to turn low-activation fragments into retained insights.
**Reality**: `HypnagogiaEngine { gate, loosener, interrupt, observer }` is structured as a pipeline (input → gate filter → loosener widen → interrupt inject → observer retain). Matches Wallas's 4-stage flow at the architectural level. `HomuncularObserver.retention_floor: 0.40, max_candidates: 6` at `:73-75` is the verification stage — only high-scoring candidates survive. The exact Wallas-stage labels are not code identifiers, but the shape is there.

---

## D.04 — Targeted Dream Incubation (TDI) is absent (Doc 07 §"Targeted Dream Incubation", Horowitz 2023)

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 07's 2025-04 enhancement added Horowitz et al. 2023 TDI: incubation cues prime the dream content, 43% creativity boost, 90% cue incorporation.
**Reality**: `Grep 'TDI|Targeted Dream|incubation_cue|incubation' crates/roko-dreams --include=*.rs` returns zero matches. TDI is frontier.

---

## D.05 — N1 / N2 creative sweet spots (Doc 07 §"N1 / N2 Sleep Stages")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 07's 2025-04 enhancement added PLOS Biology 2024 N2 aha-moment research and "Trends in Neurosciences 2024" alpha-theta transition. N1 associations vs N2 perceptual insight.
**Reality**: The shipping hypnagogia is a single-stage engine — no N1 / N2 stage distinction. The design essay is frontier.

---

## D.06 — Alpha convergence and three levels of divergence (Doc 08 §"Alpha Convergence Problem")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 §"Alpha Convergence Problem" describes three levels of divergence between multiple agents: strategy divergence, knowledge divergence, cultural divergence. Experiential wisdom thesis: divergence is necessary for robust collective cognition.
**Reality**: `Grep 'alpha_convergence|divergence|experiential_wisdom' crates/roko-dreams --include=*.rs` returns zero matches. No divergence-tracking surface in the dream cycle. The concept is design-only.
**Fix sketch**: Doc 08 stays `Design — Phase 2+`.

---

## D.07 — Alpha taxonomy (Doc 08 §"Alpha Taxonomy")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 08 tables a taxonomy of "alpha" types: local alpha, strategic alpha, cultural alpha.
**Reality**: No alpha-tracking primitives. Informational only.

---

## D.08 — Threat simulation (Revonsuo) with FMEA-style severity ships (Doc 09 §"Threat Simulation Theory", Doc 16 §"G3 — Not started, Phase 3 — Not started")

**Status**: DONE (Doc 16 drift — twice marked not-started)
**Severity**: HIGH (for doc honesty)
**Doc claim**: Doc 09 §"Threat Simulation Theory" cites Revonsuo 2000 and describes a three-tier threat taxonomy + systematic enumeration (FMEA, FTA, MITRE ATLAS) + severity assessment (CVSS, DREAD, Bayesian). Doc 16 §"G3 Mistake Identification" says "Not implemented" and §"Phase 3 REM and Creativity" row "Threat simulation (Revonsuo) | Not started | Low".
**Reality**: `crates/roko-dreams/src/threat.rs:1-312` ships real threat simulation:
- `ThreatScenario { id, description, likelihood, impact, detection_difficulty, mitigation }` at `threat.rs:15-29`.
- `ThreatScenario::severity() = likelihood × impact × (1 - detection_difficulty)` at `:34-36` — FMEA-/DREAD-adjacent severity score clamped `[0.0, 1.0]`.
- `enumerate_threats(episodes)` at `:41-81` groups failed episodes by threat key, computes likelihood / impact / detection_difficulty / mitigation, sorts by severity descending.
- `threat_warning_entries(episodes, created_at)` at `:85+` filters `severity >= 0.20` threats and emits `KnowledgeEntry` with tags `["dream", "threat", "warning", "fmea"]`.

This is exactly Doc 09's claimed behavior — Revonsuo threat rehearsal with FMEA-style severity — shipping in 312 LOC. Doc 16 badly undercounts.
**Fix sketch**: Update Doc 16 §"G3" and §"Phase 3 — Threat simulation" to Implemented with `threat.rs:1-312` anchor. Update Doc 09 to cite the shipping code.

---

## D.09 — Red teaming / Constitutional Classifiers are absent (Doc 09 §"Constitutional Classifiers", §"Quality-Diversity Red-Teaming")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 09's 2025-04 enhancement added Constitutional Classifiers (Anthropic 2025, 86%→4.4% jailbreak reduction), Rainbow Teaming (NeurIPS 2024), Quality-Diversity Red-Teaming (arXiv 2025). Perez et al. 2022, Ganguli et al. 2022, Mazeika 2024 HarmBench, MITRE ATLAS v5.1.0.
**Reality**: `Grep 'constitutional|jailbreak|rainbow_teaming|harmbench|ATLAS' crates/roko-dreams --include=*.rs` returns zero matches. The shipping threat simulation (D.08) covers operational failure threats, not LLM-adversarial jailbreak threats.
**Fix sketch**: Doc 09's 2025-04 enhancement subsections (Constitutional Classifiers, QD Red-Teaming, HarmBench) should carry `Design — Phase 2+` banners. They are a different class of threat (adversarial prompt vs operational failure) than the shipping code handles.

---

## D.10 — Three-tier threat taxonomy / gap analysis (Doc 09 §"Three-Tier Threat Taxonomy")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 09 §"Three-Tier Threat Taxonomy" and §"Gap Analysis" describe a structured taxonomy of threats by severity / class.
**Reality**: The shipping `enumerate_threats` groups by "threat key" (a hash of failure characteristics) without an explicit three-tier taxonomy. All scenarios feed through the same severity formula. Adequate for the current use case but not the full taxonomy.

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 2 (D.01 4-layer hypnagogia engine, D.08 threat simulation with FMEA severity) |
| PARTIAL | 3 (D.02 stochastic resonance noise floor, D.03 Wallas pipeline shape, D.10 three-tier threat taxonomy) |
| NOT DONE | 5 (D.04 TDI, D.05 N1/N2 stages, D.06 alpha convergence, D.07 alpha taxonomy, D.09 constitutional classifiers / red teaming) |

Section D has the **two biggest Doc 16 undercount findings in the
topic**: hypnagogia (D.01) and threat simulation (D.08) both ship at
real line numbers with real designs, while Doc 16 reports them as
unimplemented or placeholder. These are the clearest "Doc 16 is
significantly stale" signals.

## Agent Execution Notes

### D.01 / D.08 — Doc 16 regeneration triggers

These are the two strongest reasons to regenerate Doc 16. The
current state claims of "placeholder" and "not started" are
actively misleading for readers trying to understand what's in
`roko-dreams/`.

Doc 07 itself is also a hotspot: it still says the hypnagogia engine
is not implemented in `roko-dreams`, which is now directly contradicted
by `crates/roko-dreams/src/hypnagogia.rs`.

### D.04 / D.05 / D.06 / D.07 / D.09 — Frontier banner pass

All academic-frontier design extensions; no shipping implication.

Acceptance criteria:

- Doc 16 §"Hypnagogia placeholder" section is rewritten to cite `hypnagogia.rs:1-538` as shipping,
- Doc 16 §"Phase 3 — Threat simulation" moves from "Not started" to "Implemented" with `threat.rs` anchor,
- Doc 07 no longer claims hypnagogia is unimplemented,
- Docs 07 / 08 / 09 academic-frontier subsections carry Phase 2+ banners.
