# F — Status And Frontier

Audit-aligned status summary for doc `16`.

## Shipping

- `KnowledgeEntry`, `KnowledgeKind`, `KnowledgeTier`, and `KnowledgeStore`
- `Distiller`
- `TierProgression`
- `ContextAssembler` as a real neuro primitive
- `HdcVector` in `roko-primitives`
- `RokoEvent` with exactly two event types:
  `PlanRevision` and `PrdPublished`

## Partial

- HDC is real, but it is not yet a universal `Engram` field
- neuro has HDC-backed query help internally, but `Substrate` still lacks
  `query_similar()`
- the current docs mix real neuro runtime with a much larger target-state memory
  architecture

## Deferred / Target-State

- demurrage
- worldview
- cross-domain resonance and analogy APIs
- Library of Babel exchange and mesh publication
- backup / restore / publish workflows
- any separate `roko-hdc` crate

## Frontier Reading

The frontier should now be described this way:

- **worth shipping next**: HDC fingerprint on `Engram`
- **worth documenting honestly**: `query_similar()` is not yet on `Substrate`
- **worth deferring**: the larger resonance, worldview, exchange, and economic
  systems

## Practical Outcome For Doc 16

Doc `16` should read as a status page for a substantial but still bounded neuro
subsystem, not as evidence that the full long-range memory architecture already
exists.
