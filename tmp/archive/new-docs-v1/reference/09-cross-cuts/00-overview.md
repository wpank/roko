# Cognitive Cross-Cuts — Overview

> Neuro, Daimon, and Dreams are not stages in the loop — they are subsystems that
> participate in multiple stages simultaneously, injected as trait objects.

**Status**: Shipping
**Depends on**: [Cognitive Loop](../06-loop/README.md),
[Five-Layer Taxonomy](../08-layers/README.md), [L1 Framework](../08-layers/02-L1-framework.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

A "cross-cut" is a concern that spans multiple stages of the cognitive loop and
multiple layers of the stack. Roko has three: **Neuro** (knowledge management),
**Daimon** (affect and motivation), and **Dreams** (offline learning). Each is
implemented as a set of L1 traits with a default L2 implementation, injected into the
loop at L3.

---

## The Problem Cross-Cuts Solve

The eight-stage loop is cleanly decomposed, but some concerns cannot be localized to
one stage:

- **Knowledge indexing** needs to happen in QUERY (retrieval), SCORE (utility history),
  PERSIST (writing new Engrams), and the Delta consolidation pass. That is not one
  stage — it is "everywhere that touches memory."

- **Emotional salience** needs to affect SCORE (valence axis), ROUTE (urgency-driven
  routing), and COMPOSE (emphasis in prompts). That is not one stage — it is
  "everywhere that weights information."

- **Offline learning** needs to run during Delta ticks across QUERY + SCORE + PERSIST.
  But it also needs to *influence* future Gamma/Theta ticks (by updating Engram
  quality and routing priors). That is not one stage — it is "across time."

The cross-cut pattern solves this by giving each of these concerns its own
subsystem with its own traits, and injecting those traits into the `TickContext` at
the points where they participate.

---

## The Three Cross-Cuts

### Neuro — Knowledge Management

Neuro is the agent's long-term knowledge manager. It provides:
- The HDC index for semantic search
- Tier promotion/demotion logic for Engrams
- The "knowledge graph" view of related Engrams
- Integration with the Korai chain for shared knowledge (planned)

Neuro participates in: QUERY (search), SCORE (utility), PERSIST (indexing new Engrams),
Delta (consolidation and pruning).

Status: **Shipping**. See [Neuro](01-neuro.md) and
[`subsystems/neuro/`](../../subsystems/neuro/README.md).

### Daimon — Affect and Motivation

Daimon is the agent's affective system. It maintains a PAD (Pleasure-Arousal-Dominance)
emotional state vector and translates it into behavioral influences. It provides:
- The `valence` value for SCORE
- An `urgency` signal that shifts routing confidence thresholds
- A `somatic landscape` of marked Engrams (emotionally significant memories)
- Behavioral state labels (focused, exploratory, anxious, etc.)

Daimon participates in: SCORE (valence), ROUTE (urgency adjustment), COMPOSE
(emphasis modulation).

Status: **Built** (code exists, not wired into default runtime). See [Daimon](02-daimon.md).

### Dreams — Offline Learning

Dreams is the offline consolidation engine. It runs exclusively during Delta ticks.
It has two phases:
- **NREM-like replay**: surface and re-score recent Engrams; update routing priors
- **REM-like imagination**: generate novel `Kind::Imagined` Engrams by combining
  existing knowledge via HDC compositional algebra

Dreams participates in: Delta QUERY, Delta SCORE, Delta PERSIST.

Status: **Built** (code exists, not wired into default Delta). See [Dreams](03-dreams.md).

---

## Cross-Cut vs. Stage

The distinction is important for contributors:

| | Stage | Cross-Cut |
|---|---|---|
| Runs in | One position in loop_tick() | Multiple stages or outside the loop |
| Injected as | Stage trait in TickContext | Multiple traits in TickContext |
| Can add a Pulse? | Yes (via REACT) | Yes (directly via Bus ref in context) |
| Can read Substrate? | Yes (via TickContext.substrate) | Yes |
| Can write Substrate? | Only PERSIST stage | Only during PERSIST or Delta pass |

---

## See also

- [Injection Model](04-injection-model.md) — the mechanical injection mechanism
- [Composition](05-composition.md) — using multiple cross-cuts simultaneously
- [Boundaries](06-boundaries.md) — what cross-cuts may and may not do
- [L2 Scaffold](../08-layers/03-L2-scaffold.md) — where cross-cuts are implemented
- [L3 Harness](../08-layers/04-L3-harness.md) — where cross-cuts are injected
