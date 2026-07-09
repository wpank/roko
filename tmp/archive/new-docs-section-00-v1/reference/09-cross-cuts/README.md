# Cognitive Cross-Cuts

> Three subsystems that thread through every layer of the stack and participate in
> every tick without belonging to any single stage.

**Status**: Neuro = Shipping; Daimon = Built; Dreams = Built
**Depends on**: [Five-Layer Taxonomy](../08-layers/README.md),
[Cognitive Loop](../06-loop/README.md), [L1 Framework](../08-layers/02-L1-framework.md)

---

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](00-overview.md) | Why cross-cuts; trait-object injection across layers | Shipping |
| 01 | [Neuro](01-neuro.md) | Knowledge store; HDC index; pointer to subsystems/neuro/ | Shipping |
| 02 | [Daimon](02-daimon.md) | Affect and motivation; PAD vectors; behavioral states | Built |
| 03 | [Dreams](03-dreams.md) | Offline learning; consolidation; hypnagogia | Built |
| 04 | [Injection Model](04-injection-model.md) | How cross-cuts attach to the loop; lifecycle | Shipping |
| 05 | [Composition](05-composition.md) | Combining multiple cross-cuts; precedence rules | Shipping |
| 06 | [Boundaries](06-boundaries.md) | What a cross-cut may and may not do | Shipping |
| 07 | [Open Questions](07-open-questions.md) | Unresolved design decisions | — |

---

## Suggested reading order

**First-time reader**: 00 → 01 → 04 → 05

**Implementing a cross-cut**: 00 → 04 → 06 → 07

**Debugging cross-cut interaction**: 05 → 06 → the specific cross-cut's page

---

## See also

- [Five-Layer Taxonomy](../08-layers/README.md) — cross-cuts live at L2; injected by L3
- [Cognitive Loop](../06-loop/README.md) — where cross-cuts participate
- [`subsystems/neuro/`](../../subsystems/neuro/README.md) — full Neuro implementation
- [`subsystems/daimon/`](../../subsystems/daimon/README.md) — full Daimon implementation
- [`subsystems/dreams/`](../../subsystems/dreams/README.md) — full Dreams implementation
