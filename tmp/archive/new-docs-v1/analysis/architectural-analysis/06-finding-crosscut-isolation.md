# Finding: Cross-Cut Isolation

> Trait object injection analysis, isolation gaps in Daimon and Dreams, and the formal
> requirement for functorial commutativity via an arbitration protocol.

**Status**: Analysis — fixes needed (F10, F11)
**Crate**: `roko-daimon`, `roko-dreams`, `roko-neuro`, `roko-core`
**Depends on**: [Cognitive Cross-Cuts](../../reference/09-cross-cuts/README.md)
**Last reviewed**: 2026-04-13

---

## TL;DR

The three cognitive cross-cuts (Neuro, Daimon, Dreams) use the correct Rust pattern for
cross-cutting concerns (trait object injection), but two of the three have isolation gaps.
Daimon is not injected via trait object, causing direct struct import in consumers. Dreams
imports `roko-neuro` and `roko-learn` directly, creating hidden coupling between cross-cuts.
The cross-cut arbitration protocol is not yet implemented, which means cross-cut conflicts
are resolved ad hoc rather than canonically.

---

## Trait Object Injection Analysis

| Cross-Cut | Injection Mechanism | Trait Used | Isolation Quality |
|---|---|---|---|
| **Neuro** | `&dyn Substrate` (NeuroStore implements Substrate) | Substrate | **Good**: consumers don't know they're accessing knowledge vs. generic storage |
| **Daimon** | PAD vector passed as context/config values | Custom structs | **Adequate**: not trait-object injected; uses direct struct access |
| **Dreams** | Delta-frequency timer triggers DreamRunner | Scheduled execution | **Adequate**: runs independently but reads/writes directly to Neuro |

---

## Isolation Gaps (F10)

| Gap | Impact | Recommendation |
|---|---|---|
| Daimon is not injected via trait object | L0/L1 code must import `roko-daimon` types directly | Define an `AffectModel` trait in `roko-core` with `fn pad(&self) -> PadVector` and `fn behavioral_state(&self) -> BehavioralState` |
| Dreams directly imports roko-neuro and roko-learn | Creates hidden coupling between cross-cuts | Dreams should receive `&dyn Substrate` and `&dyn EpisodeStore` trait objects |
| Arbitration protocol not yet implemented | Cross-cut conflicts resolved ad hoc | Implement the VCG arbitration described in `13-cognitive-cross-cuts.md` Section 6 |

**Fix for Daimon gap**:

```rust
// In roko-core/src/traits.rs
pub trait AffectModel: Send + Sync {
    fn pad(&self) -> PadVector;
    fn behavioral_state(&self) -> BehavioralState;
    fn update(&mut self, event: AffectEvent);
}
```

Then `roko-daimon` implements `AffectModel`, and all consumers receive `&dyn AffectModel`
instead of importing `roko-daimon` directly.

---

## Arbitration Protocol Gap (F11)

The cross-cut arbitration protocol is specified in `13-cognitive-cross-cuts.md` Section 6 but
not implemented. The current state is: when Neuro, Daimon, and Dreams all want to modify the
same pipeline step, the order of modification is determined by call-site ordering in
`orchestrate.rs`, not by a principled arbitration rule.

The specified arbitration uses a priority hierarchy (Daimon > Neuro > Dreams) with VCG
tiebreaking for conflicting cross-cut demands on the same pipeline resource.

---

## Functorial Commutativity Requirement

As analyzed in the categorical framework, cross-cuts form endofunctors on the Engram category.
For the functorial composition to be correct, the following diagram must commute:

```
                  Neuro
Pipeline ─────────────────→ Enriched Pipeline
    │                              │
    │ Daimon                       │ Daimon
    │                              │
    ▼                              ▼
Modulated Pipeline ───────→ Enriched + Modulated Pipeline
                  Neuro
```

That is: enriching with knowledge and then modulating with affect must produce the same
result as modulating first and then enriching. The arbitration protocol (priority hierarchy +
VCG tiebreaker) ensures this commutativity by defining a canonical resolution order.

Without the arbitration protocol, commutativity is violated whenever two cross-cuts compete
for the same pipeline resource. The priority hierarchy is the formal guarantee.

See [07-finding-category-theory.md](07-finding-category-theory.md) for the full categorical
treatment of cross-cuts as endofunctors.

---

## Related Findings

- [F3 — Layer Taxonomy](03-finding-layer-taxonomy.md): The unclassified crates (roko-neuro,
  roko-daimon, roko-dreams) are the same crates with isolation gaps.
- [Integration Map: daimon×composition](../integration-map/daimon-x-composition.md):
  The Daimon isolation gap directly blocks the M2 integration.
- [Integration Map: dreams×neuro](../integration-map/dreams-x-neuro.md): The Dreams
  isolation gap is the same as the M7 missing integration.

## Open Questions

- Should `AffectModel` be added to `roko-core` before the cross-cut arbitration protocol,
  or together with it?
- Is the VCG tiebreaker for cross-cut arbitration over-engineered for Phase 1?
