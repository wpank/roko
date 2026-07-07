# Three Cognitive Speeds

> Why Roko runs at three different tempos, and how they coordinate.

**Status**: Shipping
**Depends on**: [Cognitive Loop](../06-loop/README.md), [Pulse](../02-pulse/README.md)
**Used by**: [Cross-Cuts](../09-cross-cuts/README.md), [Orchestration layer](../08-layers/05-L4-orchestration.md)

---

## Contents

| # | Page | What it covers | Status |
|---|---|---|---|
| 00 | [Overview](00-overview.md) | Why three speeds; cognitive tempo motivation | Shipping |
| 01 | [Gamma (Reactive)](01-gamma-reactive.md) | 5–15 s reactive loop | Shipping |
| 02 | [Theta (Reflective)](02-theta-reflective.md) | ~75 s reflective loop | Shipping |
| 03 | [Delta (Consolidation)](03-delta-consolidation.md) | Hours-scale offline consolidation | Shipping |
| 04 | [Speed Coordination](04-speed-coordination.md) | How the three interact; adaptive clock | Shipping |
| 05 | [Triggers](05-triggers.md) | What advances each speed tier | Shipping |
| 06 | [Resource Budgets](06-resource-budgets.md) | Per-speed compute allocation | Shipping |
| 07 | [Examples](07-examples.md) | Worked scenarios across all three speeds | Shipping |
| 08 | [Open Questions](08-open-questions.md) | Unresolved design decisions | — |

---

## Suggested reading order

**First-time reader**: 00 → 01 → 02 → 03 → 04

**Implementer**: 00 → 04 → 05 → 06

**Debugger (agent running at wrong speed)**: 05 → 04 → 07

---

## See also

- [Cognitive Loop](../06-loop/README.md) — each speed tier drives loop_tick()
- [Dual-Process](../06-loop/10-dual-process.md) — routing confidence drives tier selection
- [Dreams cross-cut](../09-cross-cuts/03-dreams.md) — runs at Delta speed
- [Active Inference](../06-loop/11-active-inference.md) — free energy drives tier escalation
