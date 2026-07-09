# D - Hypnagogia, Divergence, Threat Simulation (Docs 07, 08, 09)

This section contains two more major doc undercounts. Both
`HypnagogiaEngine` and threat simulation already ship in
`roko-dreams`; the larger TDI/alpha/divergence theories do not.

Generated: 2026-04-18

---

## Shipping Now

### D.01 - HypnagogiaEngine ships in `roko-dreams`

**Status**: DONE

The current runtime ships the full four-layer structure in
`hypnagogia.rs`:

- `ThalamicGate` in `hypnagogia.rs:15-31`
- `ExecutiveLoosener` in `hypnagogia.rs:33-49`
- `DaliInterrupt` in `hypnagogia.rs:51-67`
- `HomuncularObserver` in `hypnagogia.rs:69-85`
- `HypnagogiaEngine` in `hypnagogia.rs:87-160`

Any doc that still says "planned, moved from `roko-golem`" is stale.

### D.02 - Threat simulation ships in `roko-dreams`

**Status**: DONE

`threat.rs` is a live runtime module:

- `ThreatScenario` in `threat.rs:14-36`
- `enumerate_threats()` in `threat.rs:39-81`
- `threat_warning_entries()` in `threat.rs:83-120`

The runtime already computes severity and turns high-severity repeated
failures into warning knowledge entries.

### D.03 - The current threat model is narrower than the docs

**Status**: PARTIAL

The shipping threat path is operational failure analysis. It is not the
full three-tier taxonomy, classifier stack, or adversarial red-team
framework some doc sections describe.

---

## Target-State Only

### D.04 - Targeted dream incubation

**Status**: TARGET-STATE

No TDI or cue-injection mechanism ships today.

### D.05 - N1 / N2 stage distinctions

**Status**: TARGET-STATE

The current hypnagogia runtime is a single liminal pipeline, not a
stage-specific sleep architecture.

### D.06 - Alpha / divergence frameworks

**Status**: TARGET-STATE

Alpha convergence, divergence taxonomies, and experiential-wisdom
theory do not have runtime support in `roko-dreams`.

### D.07 - Constitutional classifiers / quality-diversity red teaming

**Status**: TARGET-STATE

Those subsections are research extensions, not current dreams runtime.

---

## What To Carry Into The Live Docs

- Doc 07 should stop describing hypnagogia as unimplemented or as a pending move from `roko-golem`.
- Doc 09 should present the current threat simulation as shipped, while keeping broader classifier/red-team work future-facing.
- Doc 08 should be treated as theory/target-state, not as current system behavior.
