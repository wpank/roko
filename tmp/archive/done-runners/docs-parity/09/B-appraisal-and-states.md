# B — Appraisal and States (Docs 03, 04, 05)

This is one of the strongest parity sections in topic `09`.

The shipping runtime already has:

- live `AffectEvent` appraisal,
- explicit `BehavioralState::classify()` thresholds,
- `DispatchStrategy` / `DispatchParams` modulation,
- `DaimonPolicy` flowing into routing.

What needs correction is mostly narrative drift around theory framing,
control-law wording, and hysteresis.

Generated: 2026-04-18

---

## Current Read

| Area | Status | Parity note |
|------|--------|-------------|
| appraisal events and deltas | DONE | the affect path is live from CLI, serve, and learning surfaces |
| behavioral-state classification | DONE | thresholds are explicit and tested |
| DaimonPolicy -> routing | DONE | the routing feed is real |
| tier-routing prose in Doc 05 | PARTIAL | the live path is bandit-based, not the exact control law the prose sketches |
| hysteresis language | PARTIAL | router hysteresis exists; classifier hysteresis does not |
| eight-step appraisal pipeline wording | PARTIAL | useful rationale, not literal staged runtime code |

---

## B.01-B.06 — The runtime loop already ships

The core loop here is not speculative:

`AffectEvent -> appraise() -> PAD/confidence update -> BehavioralState -> Dispatch / Router bias`

That path is already part of the active system. Parity should keep that
visible instead of over-focusing on what is still frontier elsewhere in
the topic.

Key shipping anchors for the docs:

- `AffectEvent` is real and stable
- appraisal rules are real and use asymmetric valence
- `BehavioralState::classify()` is real and exact
- behavioral modulation is real
- `DaimonPolicy` is a live routing input

---

## B.07 — Routing is live, but the doc should describe the right mechanism

**Status**: PARTIAL

Doc 05 currently risks sounding like the shipping router is controlled by
a prediction-error-threshold scheme. That is too literal.

Parity stance:

- keep the three-tier intuition because it explains the compute posture,
- say clearly that the live routing path is bandit-based,
- present threshold-control prose as design rationale or alternative framing,
- do not describe it as the exact runtime law.

This correction strengthens the "mostly shipping" story by aligning the
docs to the mechanism that actually runs.

---

## B.08-B.09 — Separate classifier behavior from router behavior

The section needs one crisp split:

- behavioral-state classification is a direct function of current PAD and confidence
- router hysteresis is a separate model-selection concern

What does not ship:

- explicit classifier dwell time
- classifier hysteresis
- a daimon-state transition cooldown

What does ship:

- live routing decisions that already consume `DaimonPolicy`
- smoothing from the affect system's decay behavior
- router-side hysteresis logic in the learning stack

That distinction is the main doc fix for this section.

---

## B.10 — Keep the academic pipeline, but label it as rationale

**Status**: PARTIAL

Doc 03's OCC/Scherer pipeline is still useful, but it should read as
the conceptual basis for the chosen deltas, not as an internal staged
runtime pipeline with named intermediate signals.

The shipping implementation is simpler:

- event variant arrives
- direct PAD/confidence deltas are applied
- state is recomputed

That is good enough for the live control loop and already materially
integrated.

---

## Section Outcome

| Status | Count |
|--------|-------|
| DONE | 7 |
| PARTIAL | 3 |
| FRONTIER | 0 |

This section should land with a strong message:

- BehavioralState ships.
- DaimonPolicy ships.
- the appraisal and modulation loop ships.
- the remaining work is doc precision, not subsystem construction.

---

## Edit Guidance

- preserve the runtime confidence in Docs 03-05
- describe routing as live but bandit-based
- separate classifier behavior from router hysteresis
- keep theory framing as theory framing, not literal staged implementation
