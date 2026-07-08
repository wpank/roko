# roko-dreams — Test Coverage

> Tests for the offline consolidation subsystem: NREM replay, REM imagination, and knowledge promotion.

**Status**: Scaffold (stubs exist; no meaningful implementation)
**Crate**: `roko-dreams`
**Section**: 10 — Dreams
**Last reviewed**: 2026-04-19

---

## Test Count

`roko-dreams` is at Scaffold status. No substantive tests exist beyond type compilation checks.

---

## Planned Test Areas

When implementation proceeds, tests will cover:

### NREM Replay (Mattar-Daw utility-based episode selection)

- Episode selection prioritizes high-utility episodes (high reward + high uncertainty).
- Replay does not modify the original episode record.
- Replay produces updated knowledge items in `roko-neuro`.
- The utility score used for selection is computed deterministically from the episode record.

Key property (planned): [../by-property/nrem-utility-selection.md](../by-property/nrem-utility-selection.md).

### REM Imagination

- REM produces synthetic episode variants by combining memory fragments.
- Imagined episodes are tagged with `synthetic: true` and cannot be promoted to Persistent knowledge.
- REM does not produce imagined episodes that contradict Persistent knowledge.

### Knowledge Promotion

- An observation validated by 3+ independent episodes is promoted from Working → Consolidated.
- A Consolidated item promoted during Dreams is tagged with the consolidation timestamp.
- Promotion is idempotent: promoting an already-Consolidated item has no effect.

Key property (planned): [../by-property/dreams-consolidation-idempotence.md](../by-property/dreams-consolidation-idempotence.md).

---

## Known Gaps

Everything. `roko-dreams` is a Scaffold. The planned consolidation architecture is specified but not implemented.

See [../gaps-and-roadmap.md](../gaps-and-roadmap.md) for the consolidation testing roadmap.

## See also

- [subsystem-neuro.md](subsystem-neuro.md) — knowledge types that Dreams promotes
- [subsystem-learn.md](subsystem-learn.md) — episodes that Dreams replays
