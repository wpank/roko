# A — PAD Vector and Temporal Model (Docs 00, 01, 02)

Parity of the three foundational chapters in topic 09: vision (mortality
incompatibility), PAD vector math, ALMA three-layer temporal model.

The PAD primitive lives in `crates/roko-core/src/affect.rs:1-251`
(shared across the cognitive stack). The live daimon state uses a
**single-layer** temporal decay, not the full three-layer ALMA model
Doc 02 describes.

Generated 2026-04-16.

---

## A.01 — "Mortality framing removed" is honored across the stack (Doc 00 §"Vision")

**Status**: DONE
**Severity**: —
**Doc claim**: Topic 09 explicitly removes death / mortality framing. Doc 00 and Doc 13 both cite two legacy files (`bardo-backup/prd/03-daimon/04-mortality-daimon.md`, `05-death-daimon.md`) as skipped. Agents don't "die"; cyclical behavioral states replace terminal states (see B.04).
**Reality**: `Grep 'mortality\|death\|dying\|thanatopsis' crates/ apps/ --include=*.rs` returns zero matches in the active codebase (per user memory + MEMORY.md "Death concepts removed"). The shipping `BehavioralState` enum at `roko-core/src/affect.rs:87-101` has six **cyclical** variants (`Engaged`, `Struggling`, `Coasting`, `Exploring`, `Focused`, `Resting`) with no terminal state. Confirmed.

---

## A.02 — Cyclical-states framing is a hard architectural invariant (Doc 00 §"Behavioral States Are Cyclical")

**Status**: DONE
**Severity**: —
**Doc claim**: "The behavioral states are cyclical — Engaged, Struggling, Coasting, Exploring, Focused, Resting — with no terminal state." Doc 04 expands this.
**Reality**: Matches code exactly. `BehavioralState` variants at `roko-core/src/affect.rs:89-100` are the exact six names, in the same order as the doc table. `BehavioralState::classify(pad, confidence)` at `affect.rs:106-131` returns one of the six — never a "dead" or "exited" variant.

---

## A.03 — Daimon is a control signal, not cosmetic (Doc 00 §"What It Is / Isn't")

**Status**: DONE
**Severity**: —
**Doc claim**: "The Daimon is not cosmetic. It is a control signal that directly modulates how much compute the agent spends, which models it uses, and what context it retrieves."
**Reality**: `DaimonPolicy { affect_confidence, behavioral_state }` at `roko-core/src/affect.rs:134-158` is consumed by **ten non-daimon crates** (grep verification): `roko-serve/src/routes/providers.rs`, `roko-cli/src/orchestrate.rs`, `roko-cli/src/main.rs`, `roko-learn/src/runtime_feedback.rs`, `roko-learn/src/model_router.rs`, `roko-learn/src/cascade_router.rs`, `roko-core/src/lib.rs`, `roko-learn/tests/learning_loop.rs`, `roko-learn/src/model_experiment.rs`, `roko-core/src/affect.rs`. Additional consumers (`PadVector` / `AffectState` / `EmotionalTag` imports): `roko-compose/src/prompt.rs`, `roko-serve/src/dispatch.rs`, `roko-serve/src/dreams.rs`, `roko-learn/src/episode_logger.rs`, `roko-neuro/src/knowledge_store.rs`. The control-signal claim holds.

---

## A.04 — 10,240-bit (wait, that's HDC) — PAD is `(f64, f64, f64)` with `[-1, 1]` clamping (Doc 01 §"Three Dimensions")

**Status**: DONE
**Severity**: —
**Doc claim**: Three dimensions (Pleasure, Arousal, Dominance), each `[-1.0, 1.0]`. `PadVector { pleasure: f64, arousal: f64, dominance: f64 }`. Neutral is `(0, 0, 0)`.
**Reality**: `PadVector` at `roko-core/src/affect.rs:7-14` has exactly three `f64` fields with `[-1.0, 1.0]` doc comments. `neutral()` at `:29-31` returns `(0, 0, 0)`. `clamped()` at `:35-41` enforces the `[-1.0, 1.0]` invariant. `apply_delta` at `:44-51` clamps after mutation. `decay_by_factor` at `:54-61` scales all three dims then clamps.

---

## A.05 — 8 octant states from PAD sign are partial (Doc 01 §"Eight Octant States")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 01 `:80-120` tables 8 octant states derived from the sign of each PAD dimension: `Excited (+P +A +D)`, `Surprised (+P +A -D)`, `Confident (+P -A +D)`, `Relaxed (+P -A -D)`, `Angry (-P +A +D)`, `Anxious (-P +A -D)`, `Bored (-P -A +D)`, `Depressed (-P -A -D)`. These were the Plutchik-adjacent categorical labels.
**Reality**: `Grep 'AffectOctant\|Excited\|Surprised\|Confident\|Relaxed\|Angry\|Anxious\|Bored\|Depressed' crates/roko-daimon crates/roko-core --include=*.rs` returns zero matches for the `AffectOctant` enum type. The shipping code has `BehavioralState` (6 variants; see A.02) but **not** the `AffectOctant` 8-variant enum from legacy `roko-golem`. Doc 13 §"Implemented Components" confirms: "`AffectOctant` enum — Done (roko-golem)". Since roko-golem has been dissolved (Tier 0C complete per Doc 13), the enum was not carried forward.
**Fix sketch**: Doc 01 should make the 8-octant table explicitly informational (a tertiary mapping from PAD sign-triples) rather than a type the code exposes. If downstream consumers ever need categorical labels, a helper `fn categorical_label(pad: PadVector) -> &'static str` can be added.

---

## A.06 — PAD cosine similarity maps to `[0.0, 1.0]` (Doc 01 §"PAD Cosine Similarity")

**Status**: DONE
**Severity**: —
**Doc claim**: `PadVector::cosine_similarity(a, b) -> f64` returns `(dot / (|a| × |b|) + 1.0) / 2.0`, bounded to `[0.0, 1.0]` with neutral fallback to `0.5` for zero-magnitude inputs.
**Reality**: `cosine_similarity` at `roko-core/src/affect.rs:70-81` matches exactly. Zero-magnitude fallback returns `0.5` (`:77-79`). Test `pad_similarity_uses_neutral_fallback` at `:223-229` pins the fallback.

---

## A.07 — PAD decay mechanics use exponential half-life (Doc 01 §"Decay Mechanics")

**Status**: DONE
**Severity**: —
**Doc claim**: Exponential decay: `factor = 0.5 ^ (elapsed_hours / half_life_hours)`. Default half-life is **4 hours** (Doc 01 §"Decay Mechanics"; Doc 13 §"F4 — Temporal decay" — Done).
**Reality**: `AffectState::decay` at `roko-daimon/src/lib.rs:71-85` computes `elapsed_hours = (now - updated_at).num_seconds() / 3600`, then applies `decay_factor(elapsed_hours, half_life_hours)` to PAD + confidence (pulling confidence toward 0.5 rather than 0). Refreshes behavioral state after. `default_half_life_hours()` (per Doc 13) matches. Confidence decay target is the neutral midpoint, not zero — a small design choice worth noting in Doc 01 §"Decay Mechanics".
**Fix sketch**: Doc 01 should clarify that confidence decays toward **0.5** (the midpoint) while PAD decays toward **0** (the neutral origin). These are not the same "decay target".

---

## A.08 — ALMA three-layer temporal (Emotion / Mood / Personality) is single-layer in shipping code (Doc 02 §"Three Temporal Layers")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Doc 02 `:40-120` specifies Gebhard 2005's ALMA layered model: **Emotion** (seconds, fast-responding), **Mood** (hours, medium persistence), **Personality** (lifetime, essentially static). Layer interactions are: emotions perturb mood; mood biases emotion appraisal; personality sets mood baseline.
**Reality**: `AffectState { pad, confidence, behavioral_state, updated_at }` at `roko-daimon/src/lib.rs:37-48` is a **single PAD vector** with one `half_life_hours` knob (default ~4 hours per F4). There is no separate Mood layer (hours-scale secondary state) or Personality layer (lifetime bias). `EmotionalTag.mood_snapshot` at `roko-core/src/affect.rs:170` is a per-event snapshot of the current PAD — **not** a persisting Mood layer. No personality struct exists (`Grep 'Personality\|PersonalityTrait' crates/ --include=*.rs` returns zero matches).

The practical effect: the shipping Daimon blends Emotion and Mood into one PAD signal with one decay rate. This is simpler than ALMA and forfeits the explicit separation of "acute reaction" from "background climate". It works fine for the current dispatch-modulation use case but loses the ALMA property that sustained good experiences slowly shift baseline personality.
**Fix sketch**: Doc 02 should add an `Implementation: Design — Phase 2+` banner for the three-layer separation. Alternatively, reframe the shipping single-layer PAD as "Emotion layer + implicit Mood via the 4-hour half-life", and mark true Mood and Personality layers as future extensions.

---

## A.09 — Layer interactions (hot-cognition carry-over, mood biasing appraisal) are unimplemented (Doc 02 §"Layer Interactions")

**Status**: NOT DONE
**Severity**: LOW
**Doc claim**: Emotion perturbs mood (small leakage over many events); mood biases next appraisal (negative mood amplifies failure impact); personality sets mood baseline. Sustained positive experiences slowly shift personality.
**Reality**: Absent — follows directly from A.08. The appraisal rules at `roko-daimon/src/lib.rs` use **absolute** PAD deltas (e.g., `gate_pass: pleasure += 0.05 × rung_scale`) without scaling by current mood or personality state. No "mood amplifies impact" term in the appraisal math.

---

## A.10 — Domain-agnostic PAD (coding vs chain vs research) is structurally honored (Doc 02 §"Comparison with Alternatives")

**Status**: DONE
**Severity**: —
**Doc claim**: The PAD dimensions (Pleasure / Arousal / Dominance) are domain-independent; only the appraisal rules change per domain.
**Reality**: The PAD primitive at `roko-core/src/affect.rs:7-14` is in the kernel crate with no coding / chain / research specialization. Domain-specific projection lives in `roko-daimon/src/lib.rs:548-951` (`StrategySpaceComputer` trait + `CodingStrategySpace` built-in + `RegisteredStrategySpaceComputer` for non-coding domains). The 8D strategy-space axes are domain-configurable via `StrategySpaceDefinition` at `:226-314` (validated unique non-empty labels). PAD stays the same across domains; projection into strategy-space adapts. Cross-cuts Doc 08 (see C.01-C.05).

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 7 (A.01 mortality removed, A.02 cyclical states, A.03 control signal, A.04 PAD dimensions, A.06 cosine similarity, A.07 exponential decay, A.10 domain-agnostic PAD) |
| PARTIAL | 1 (A.05 8-octant enum removed with golem dissolution) |
| NOT DONE | 2 (A.08 ALMA three-layer, A.09 layer interactions) |

Section A is cleanly mostly-DONE. The PAD primitive is real, clamped,
decayable, tested, and consumed across the stack. The main design gap
is the **ALMA three-layer temporal model** (A.08 / A.09) — Doc 02
describes a Gebhard 2005 layered architecture (Emotion / Mood /
Personality) but the shipping code has a single `AffectState` with one
half-life. This is a deliberate simplification that works for the
current dispatch-modulation use case.

## Agent Execution Notes

### A.05 — AffectOctant enum was retired

Doc 01 §"Eight Octant States" implies an `AffectOctant` enum exists.
With `roko-golem` dissolved, the enum did not migrate. Doc 01 should
mark the 8-octant table as "categorical labels derivable from PAD
sign-triples" rather than a first-class type.

Several active topic-09 docs and `docs/09-daimon/INDEX.md` still cite
`roko-golem` in ways that read like live implementation references.
When cleaning those up, keep historical provenance only where it is
explicitly labeled legacy.

### A.08 / A.09 — ALMA layers are frontier work

If the single-PAD design stops serving the dispatch modulation well,
the next step is to split `AffectState` into `AffectState { emotion:
PadVector, mood: PadVector, personality: PadVector }` with three
independent half-lives. Today's code does not need it.

Acceptance criteria:

- Doc 01 clarifies that confidence decays toward 0.5, PAD toward 0,
- Doc 02 carries a "Design — Phase 2+" banner on the three-layer architecture,
- the 8-octant table in Doc 01 is marked informational.
