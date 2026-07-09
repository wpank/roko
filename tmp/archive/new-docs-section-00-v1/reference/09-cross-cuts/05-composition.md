# Cross-Cut Composition

> Combining multiple cross-cuts; interaction rules; precedence.

**Status**: Shipping
**Last reviewed**: 2026-04-19

---

## TL;DR

All three cross-cuts can be enabled simultaneously. They interact through shared
`TickContext` fields. When two cross-cuts both influence the same field, a precedence
rule determines the outcome. The default is additive composition: both contributions
are summed and clamped to the valid range.

---

## Interaction Map

| Field | Neuro contribution | Daimon contribution | Composition rule |
|---|---|---|---|
| SCORE: Valence axis | None | `pad.pleasure × valence_weight` | Daimon wins |
| SCORE: Utility axis | `utility_ema[id]` | None | Neuro wins |
| SCORE: Novelty weight | None | Adjusted by behavioral state | Daimon adjusts Scorer weight config |
| ROUTE: confidence threshold | None | `± urgency_signal × threshold_delta` | Additive |
| COMPOSE: system prompt | None | Behavioral state note appended | Additive (Daimon appends) |
| PERSIST: Engram.affect_charge | None | `pad.arousal × sign(pad.pleasure)` | Daimon writes |

---

## Neuro + Daimon

When both are active, the SCORE stage receives:
- Utility axis from Neuro (historical usage)
- Valence axis from Daimon (current emotional charge)

These are independent axes — they do not conflict. The composite score formula weights
both independently:
```
composite = w_utility × neuro.utility_ema + w_valence × |daimon.valence| + …
```

The only interaction is in the Novelty axis weight: Daimon's `Exploratory` behavioral
state increases `w_novelty`; `Focused` decreases it. This is implemented as a
multiplicative modifier applied to the Scorer's weight vector:

```rust
let weights = base_weights.apply_behavioral_modifier(&daimon.behavioral_state());
```

---

## Neuro + Dreams

Dreams depends on Neuro. During the Delta pass:
1. Dreams requests the full HDC index from Neuro.
2. Dreams uses the index to find semantically similar Engram pairs for imagination.
3. Dreams writes new `Kind::Imagined` Engrams via the Substrate.
4. Neuro indexes the new Engrams immediately.

This means Neuro must be active whenever Dreams is active. `TickContextBuilder`
enforces this:
```rust
if config.dreams.enabled && !config.neuro.enabled {
    return Err(ConfigError::DreamsRequiresNeuro);
}
```

---

## Daimon + Dreams

During the imagination phase, Daimon's current PAD vector influences which Engram
pairs Dreams selects for binding:

- High arousal → prefer Engrams with high recency (recent = salient)
- High pleasure → prefer Engrams with positive affect_charge
- Low dominance → prefer Engrams from trusted sources

This is the closest thing Roko has to "dreaming about what worries you" — in an
anxious state (low pleasure, high arousal, low dominance), the imagination phase
surfaces threat-relevant knowledge.

---

## Adding a Fourth Cross-Cut

If a future subsystem needs cross-cut treatment:
1. Define its L1 traits in `roko-core`.
2. Implement it at L2 in a new crate.
3. Add it to `TickContextBuilder` with appropriate injection fields.
4. Document interactions with Neuro, Daimon, and Dreams here.
5. Describe it in a new `0N-<name>.md` page in this folder.

---

## See also

- [Injection Model](04-injection-model.md) — how each cross-cut is injected
- [Boundaries](06-boundaries.md) — what cross-cuts may not do to each other
- [Neuro](01-neuro.md), [Daimon](02-daimon.md), [Dreams](03-dreams.md)
