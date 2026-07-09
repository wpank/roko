# roko-daimon — Test Coverage

> Tests for the affect engine: PAD vectors, behavioral state transitions, and compute modulation.

**Status**: Built (affect engine is built; not yet called from the runtime)
**Crate**: `roko-daimon`
**Section**: 09 — Daimon
**Last reviewed**: 2026-04-19

---

## Test Count

Exact count not in the 2026-04-17 audit (listed as "Built" without a test count). The `roko-daimon` tests cover the PAD vector model and behavioral state machine.

---

## Key Test Focus Areas

### PAD Vectors (Pleasure-Arousal-Dominance)

The PAD model encodes the agent's affective state as a 3-axis vector:
- `Pleasure` in [-1, 1]: positive/negative valence.
- `Arousal` in [-1, 1]: activation/deactivation level.
- `Dominance` in [-1, 1]: sense of control.

Tests verify:
- PAD vector construction with valid axis ranges.
- Out-of-range values are clamped to [-1, 1].
- PAD vector arithmetic: weighted blending of two vectors produces a valid vector.
- Serialization round-trip.

Key property: [../by-property/pad-vector-bounds.md](../by-property/pad-vector-bounds.md).

### Behavioral States (6 states)

The Daimon cycles through 6 behavioral states:
1. `Engaged` — high pleasure, high arousal.
2. `Struggling` — low pleasure, high arousal.
3. `Coasting` — moderate pleasure, low arousal.
4. `Exploring` — moderate pleasure, high arousal + high dominance.
5. `Focused` — high pleasure, moderate arousal.
6. `Resting` — moderate pleasure, low arousal, low dominance.

Tests verify:
- Each state maps to the correct PAD region.
- State transitions are triggered by the correct PAD threshold crossings.
- No terminal state exists: from any state, there is a valid transition to at least one other state.
- State transition history is recorded in the Engram substrate.

Key property: [../by-property/daimon-no-terminal-state.md](../by-property/daimon-no-terminal-state.md).

### Compute Modulation

Daimon state influences:
- Model tier selection (higher arousal → prefer faster, cheaper models).
- Exploration rate in the CascadeRouter bandit (higher arousal → more exploration).
- Context retrieval depth (higher dominance → wider context window).
- Token budget (low pleasure → conservative token allocation).

Tests verify: modulation parameters are deterministic functions of the PAD vector.

---

## Property Tests

| Property | Test name |
|---|---|
| PAD vector bounds | `pad_vector_axes_in_range` |
| No terminal state | `daimon_always_has_outgoing_transition` |
| State determinism | `daimon_state_deterministic_from_pad` |

---

## Known Gaps

- `roko-daimon` has no integration tests with the agent (because the runtime wiring is incomplete).
- The somatic marker hypothesis implementation (fast pattern-matching) is not directly tested.
- Behavioral state transitions under adversarial PAD sequences are not tested.

## See also

- [../by-property/pad-vector-bounds.md](../by-property/pad-vector-bounds.md)
- [../by-property/daimon-no-terminal-state.md](../by-property/daimon-no-terminal-state.md)
