# B — Appraisal and Behavioral States (Docs 03, 04, 05)

Parity of the appraisal pipeline (OCC / Scherer), the six behavioral
states, and the behavioral-state → cascade-router tier-routing bias.

The appraisal rules and behavioral-state classification match the docs
almost exactly. Tier-routing bias is wired via `DaimonPolicy` but the
full Chen-FrugalGPT-style threshold scaling by behavioral state is
partial.

Generated 2026-04-16.

---

## B.01 — `AffectEvent` enum has 6 variants matching the doc (Doc 03 §"Event Types")

**Status**: DONE
**Severity**: —
**Doc claim**: Six event types feed the appraise() pipeline: `GateResult`, `TaskOutcome`, `Blocked`, `TimePressure`, `QueueWait`, `DreamFailure`.
**Reality**: `AffectEvent` at `roko-daimon/src/lib.rs:1390-1439` has exactly those six variants, tagged `#[serde(tag = "kind", rename_all = "snake_case")]`:
- `GateResult { plan_id, task_id, passed, rung: u32 }` (`:1394-1403`)
- `TaskOutcome { task_id, succeeded }` (`:1405-1410`)
- `Blocked { task_id, blocker_count }` (`:1412-1417`)
- `TimePressure { task_id, deadline_proximity }` (`:1419-1424`)
- `QueueWait { task_id, wait_hours }` (`:1426-1431`)
- `DreamFailure { task_type, failure_count }` (`:1433-1438`)

Matches Doc 13 §"F3 — AffectEvent enum and AffectEngine::appraise() — Done".

---

## B.02 — `AffectEngine::appraise` is wired from orchestrator + dispatch (Doc 03 §"Appraisal Pipeline")

**Status**: DONE
**Severity**: —
**Doc claim**: The appraisal pipeline runs on every event and updates PAD + confidence.
**Reality**: `AffectEngine` trait at `roko-daimon/src/lib.rs:1623-1632` with four methods: `appraise(&mut self, event) -> PadVector`, `query(&self) -> AffectState`, `modulate(&self, params: &mut DispatchParams)`, `persist(&self, path) -> Result<()>`. Implemented on `DaimonState` at `:1635+`. Live call sites (grep-verified):
- `crates/roko-cli/src/orchestrate.rs:5351` — `GateResult` after gate pass/fail
- `crates/roko-cli/src/orchestrate.rs:5895, 5920` — `Blocked` when dispatch refused
- `crates/roko-cli/src/orchestrate.rs:7237` — `TimePressure` on deadline proximity
- `crates/roko-cli/src/orchestrate.rs:7447, 8797` — `TaskOutcome` on task completion
- `crates/roko-serve/src/dispatch.rs:2218` — `GateResult` from HTTP dispatch path

The appraisal pipeline is live, not dark code.

---

## B.03 — Appraisal rules use `rung_scale` and asymmetric valence (Doc 03 §"Appraisal Rules", §"Prospect Theory")

**Status**: DONE
**Severity**: —
**Doc claim**: Rung scaling: `rung_scale = 1.0 + (rung.min(3) × 0.15)` — higher-rung gate results have stronger emotional impact. Asymmetric valence (prospect theory, Kahneman & Tversky 1979): failure has 2× the pleasure impact of success.
**Reality**: `rung_scale` at `roko-daimon/src/lib.rs:1646` is literally `let rung_scale = 1.0 + (rung.min(3) as f64 * 0.15);`. Appraisal deltas for `GateResult { passed: true }` at `:1649-1652` are `(pleasure: +0.05 × rung_scale, arousal: -0.01 × rung_scale, dominance: +0.03 × rung_scale, confidence: +0.03 × rung_scale)`. For `passed: false` at `:1657-1660`: `(pleasure: -0.10 × rung_scale, arousal: +0.04 × rung_scale, dominance: -0.08 × rung_scale, confidence: -0.08 × rung_scale)`. The pass→fail ratio on pleasure is `0.05 : 0.10` — exactly the 2× asymmetry the doc specifies. Dominance asymmetry is larger: `0.03 : 0.08` (~2.7×). Matches Doc 01 §"Appraisal Rules" and Doc 03 §"Asymmetric Valence".

---

## B.04 — Six behavioral states match the `classify()` thresholds exactly (Doc 04 §"PAD Thresholds")

**Status**: DONE
**Severity**: —
**Doc claim**: Doc 04 `:41-79` embeds the exact `BehavioralState::classify(pad, confidence)` function with the 6 state-ordering rules: neutral→Engaged; `c<0.30 || d<-0.25 || (p<-0.30 && a>0.30)`→Struggling; `p>0.35 && c>0.65`→Coasting; `d>0.30 && p>0.25`→Focused; `a<-0.20`→Resting; `d<0.10 && p>-0.20`→Exploring; else Engaged.
**Reality**: `BehavioralState::classify` at `roko-core/src/affect.rs:106-131` matches **byte-for-byte** the function embedded in Doc 04. Test `behavioral_state_classification_matches_thresholds` at `:195-221` pins all six state outcomes with concrete PAD values. This is one of the tightest doc-code alignments in topic 09.

---

## B.05 — Behavioral-modulation table is wired through `DispatchParams` (Doc 04 §"Behavioral Modulation Parameters")

**Status**: DONE
**Severity**: —
**Doc claim**: Each state maps to model tier bias + turn-limit adjustment + strategy preference + effort label. Doc 04 §"Behavioral Modulation Parameters" tables all six.
**Reality**: `DispatchStrategy` enum at `roko-daimon/src/lib.rs:1336-1347` has 5 variants (Conservative, Balanced, Exploratory, Escalating, Proactive) with `effort_label()` at `:1352-1360` returning `low / medium / high / high / medium`. `DispatchParams { model, turn_limit, strategy, effort }` at `:1364-1374`. The `modulate()` method on `DaimonState` applies behavioral-state-specific model promotion / demotion (haiku ↔ sonnet ↔ opus) + turn-limit adjustment (see Doc 13 §"Behavioral modulation — Complete"). Called from orchestration path during dispatch.

---

## B.06 — Behavioral-state → cascade-router tier routing is wired via `DaimonPolicy` (Doc 05 §"How Behavioral State Modulates CascadeRouter")

**Status**: DONE
**Severity**: —
**Doc claim**: Behavioral state + affect confidence modulates CascadeRouter's prediction-error thresholds. Struggling escalates; Coasting demotes; Exploring widens search; Focused narrows.
**Reality**: `DaimonPolicy { affect_confidence, behavioral_state }` at `roko-core/src/affect.rs:134-158` is the payload passed from Daimon into CascadeRouter. Consumed by `roko-learn/src/cascade_router.rs` and `roko-learn/src/model_router.rs` (grep-verified). CLAUDE.md "What exists" row "CascadeRouter (model routing) | Wired | Persists to `.roko/learn/cascade-router.json`, configurable models" — the persisted routing state already accepts affect feedback today. Doc 13 confirms: "F8 — Affect → CascadeRouter — Done; live Daimon behavioral state and confidence now arrive as a first-class `DaimonPolicy` in routing decisions."

---

## B.07 — Three-tier cognitive architecture (T0 / T1 / T2) is partial (Doc 05 §"Three-Tier Cognitive Architecture")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 05 §"Three-Tier Cognitive Architecture" describes a FrugalGPT-style cascade: T0 (cheap heuristic / cache-hit path), T1 (moderate-cost LLM), T2 (expensive / premium). Behavioral state modulates the switching thresholds between tiers.
**Reality**: CLAUDE.md "CascadeRouter (model routing) — Wired" confirms the cascade ships. The shipping model tiers are `haiku / sonnet / opus` (per Doc 13 "Behavioral modulation — Complete") which map T0 / T1 / T2 in the haiku→opus direction. But Doc 05's specific "prediction-error threshold" parameterization is a **design essay**: the shipping CascadeRouter uses `bandits.rs`-style arm selection (plus explicit reward calibration) rather than the prediction-error-threshold scheme Doc 05 sketches. The connection from affect → tier is real; the specific control law in Doc 05 is not how the shipping code routes.
**Fix sketch**: Doc 05 should align its "threshold modulation" math with the actual `cascade_router.rs` reward / bandit scheme, OR mark its specific control law as an alternative design the current implementation does not follow.

---

## B.08 — Cost implications table is informational (Doc 05 §"Cost Implications")

**Status**: DONE (narrative only)
**Severity**: —
**Doc claim**: Doc 05 §"Cost Implications" tables the expected cost delta per behavioral state (Struggling +30%, Coasting −40%, etc.). Not a code contract.
**Reality**: No enforcement mechanism needs to exist; the cost delta emerges from actual haiku→opus promotion. The table is an informational illustration of what the modulation path produces.

---

## B.09 — Feedback-loop stability (Doc 05 §"Feedback Loop Stability")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 05 §"Feedback Loop Stability" discusses the risk of oscillation (Struggling → escalate → success → Coasting → demote → failure → Struggling) and proposes hysteresis / dwell-time constraints.
**Reality**: `Grep 'hysteresis\|dwell_time\|state_transition_cooldown' crates/roko-daimon crates/roko-learn --include=*.rs` returns zero matches. The shipping `classify()` at `roko-core/src/affect.rs:106-131` is **memoryless** — each classification is a pure function of current PAD + confidence, with no minimum dwell time. In practice the exponential decay (~4h half-life; A.07) provides **implicit smoothing**, but no explicit hysteresis. If oscillation ever becomes a problem, the doc's hysteresis suggestion is the natural fix.
**Fix sketch**: Doc 05 §"Feedback Loop Stability" should note that current smoothing is via 4h PAD half-life only; explicit dwell-time or hysteresis is a future extension.

---

## B.10 — OCC + Scherer 8-step appraisal pipeline is compressed (Doc 03 §"Eight-Step Pipeline")

**Status**: PARTIAL
**Severity**: LOW
**Doc claim**: Doc 03 §"Eight-Step Pipeline" lists Scherer's Component Process Model stages: relevance → implication → goal-conduciveness → coping potential → normative significance → pleasure/arousal/dominance mapping. Plus OCC (Ortony-Clore-Collins) branches on event/agent/object.
**Reality**: The shipping `appraise()` implementation at `roko-daimon/src/lib.rs:1635+` is a **per-variant direct-mapping** from `AffectEvent` to PAD delta + confidence delta. It does not separately compute "relevance", "implication", "goal-conduciveness", etc. as intermediate signals. The academic framing in Doc 03 is a rationale for the chosen deltas, not a set of pipeline stages the code exposes.
**Fix sketch**: Doc 03 should reframe §"Eight-Step Pipeline" as "academic foundations for our direct-mapping deltas" rather than implying the stages are separately computed. The asymmetric valence (prospect theory) IS visible in code (B.03).

---

## Section Summary

| Status | Count |
|--------|-------|
| DONE | 7 (B.01 AffectEvent 6 variants, B.02 appraise wired, B.03 rung_scale + asymmetric, B.04 classify thresholds, B.05 modulation table, B.06 DaimonPolicy → cascade, B.08 cost table narrative) |
| PARTIAL | 3 (B.07 T0/T1/T2 control law drift, B.09 no explicit hysteresis, B.10 8-step pipeline compressed into direct deltas) |
| NOT DONE | 0 |

Section B is mostly DONE — the full appraisal → state → dispatch
modulation loop is live, wired from the orchestrator, tested, and
consumed by the cascade router. The three PARTIAL entries are
**doc-narrative drifts**: the three-tier control law (B.07), feedback
stability discussion (B.09), and 8-step appraisal pipeline (B.10) are
all areas where the doc describes a richer architectural detail than
the shipping direct-mapping code literally does.

## Agent Execution Notes

### B.03 / B.04 — Tight doc-code alignment

These entries are examples of what "shipping parity" looks like when
it's clean. The rung-scale math (B.03) is line-for-line identical
between Doc 03 and `lib.rs:1646`. The behavioral-state classifier
(B.04) is byte-for-byte identical between Doc 04 and
`affect.rs:106-131`.

### B.07 / B.09 / B.10 — Calibrate narrative, don't rewrite code

Acceptance criteria for this section:

- Doc 05 §"Three-Tier Cognitive Architecture" explicitly acknowledges that the shipping cascade uses a bandit scheme, not prediction-error-threshold modulation,
- Doc 05 §"Feedback Loop Stability" notes the 4h PAD half-life as implicit smoothing + flags explicit hysteresis as future extension,
- Doc 03 §"Eight-Step Pipeline" reframes as academic rationale rather than implying literal pipeline stages.
