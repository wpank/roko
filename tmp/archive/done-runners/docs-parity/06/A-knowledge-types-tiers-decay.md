# A — Knowledge Types, Tiers, Decay

Refresh of docs `00`, `01`, `02`, `03`, `07`, and `11` against current code.

## What Ships

- `KnowledgeKind` is real in `roko-neuro` with the six canonical variants:
  `Insight`, `Heuristic`, `AntiKnowledge`, `Warning`, `CausalLink`, and
  `StrategyFragment`.
- `KnowledgeTier` is real with the four shipping tiers:
  `Transient`, `Working`, `Consolidated`, and `Persistent`.
- tier progression is wired today through `TierProgression` and
  `TierProgressionDecision` in `crates/roko-neuro/src/tier_progression.rs`.
- `KnowledgeEntry` already carries the fields needed for durable neuro storage,
  including tier, anti-knowledge linkage, emotional metadata, and optional
  HDC bytes.
- the knowledge store already applies the tier multiplier through effective
  half-life and recency scoring.

## What This Means For The Docs

- docs should describe knowledge kinds and tiering in present tense
- docs should not imply that neuro still lacks a tier system
- docs should not present old `Fact`-style naming or 365-day fact constants as
  current reality

## What Is Still Deferred

### Demurrage

Demurrage remains **deferred**. The audit found **0 lines of code** for a
demurrage memory model. Current neuro behavior is still based on confidence,
half-life, tier multipliers, and anti-knowledge safeguards.

### Worldview

Worldview remains **target-state**. It should not be described as a current
organizing structure for neuro entries.

### Reactive AntiKnowledge Expansion

The basic anti-knowledge path exists, but the larger challenge framework,
parasite diagnostics, and other research-heavy extensions should stay labeled as
future work unless code lands.

## Highest-Value Follow-Up

The next bridge is not more decay theory. It is **putting an HDC fingerprint on
`Engram`** so the kernel-level memory object can participate in the same HDC
story that neuro and learning already use.

## Recommended Wording For Source Docs

- Use `shipping` for knowledge kinds, tier progression, and anti-knowledge
  basics.
- Use `target-state` for worldview and richer contradiction systems.
- Use `deferred` for demurrage and any economic freshness model.
