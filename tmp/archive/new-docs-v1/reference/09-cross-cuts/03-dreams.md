# Dreams — Offline Learning Cross-Cut

> Dreams consolidates long-term memory during Delta ticks using NREM-like replay and
> REM-like imagination.

**Status**: Built
**Crate**: `roko-dreams` (L2)
**Depends on**: [Substrate trait](../03-substrate/README.md), [Neuro cross-cut](01-neuro.md),
[HDC Fingerprint](../10-types/hdc-fingerprint.md), [Delta consolidation](../07-speeds/03-delta-consolidation.md)
**Used by**: [Delta QUERY, SCORE, PERSIST](../07-speeds/03-delta-consolidation.md)
**Last reviewed**: 2026-04-19

> **Full implementation**: [`subsystems/dreams/`](../../subsystems/dreams/README.md)
> (populated in a later refactor cluster). This page covers Dreams' role in the loop.

---

## TL;DR

Dreams runs exclusively during Delta ticks. It has two phases modeled after sleep
neuroscience: **replay** (NREM-like, consolidating recent experiences) and
**imagination** (REM-like, generating novel `Kind::Imagined` Engrams by composing
existing knowledge with HDC algebra). Dreams is the mechanism by which the agent
learns during rest rather than only during active processing.

---

## The Idea

The biological inspiration is well-established: during NREM slow-wave sleep, the
hippocampus replays recent experiences and transfers them to cortical long-term
storage. During REM sleep, the brain generates novel, loosely-associated content —
the material of dreams — which may reflect forward simulation, integration of
disparate memories, or emotion processing.

Roko's Dreams subsystem implements computational analogs of both:

- **NREM replay** — take recent Engrams (last 24 h), re-run them through a simplified
  SCORE pass with the current world model, and update routing priors based on what
  "would have happened" if the agent had encountered them today. This is the
  consolidation function: making recent experience available for efficient future
  retrieval.

- **REM imagination** — compose pairs or triplets of existing Engrams using HDC
  bind/bundle operations to generate new Engrams that encode relationships the agent
  has not explicitly seen. These are marked `Kind::Imagined` and have low initial
  trust. Over time, if imagined Engrams prove useful (high utility EMA), their trust
  is promoted.

---

## NREM Replay Phase

```
For each Engram E in last 24 h:
  Re-score E with current scoring weights
  Update routing prior: does E provide evidence for route R given stimulus S?
  If E.utility_ema changed significantly: update E in Substrate
  If E is near-duplicate of another Engram: merge or demote the lower-quality one
```

The re-scoring is cheap (no model call) — it uses the same `Scorer` trait objects
used during real-time ticks, but on historical data. The routing prior update is the
key consolidation operation: it ensures that lessons learned from recent ticks are
encoded in the CascadeRouter's Wilson CI table.

---

## REM Imagination Phase

```
Sample N pairs of Engrams (E_a, E_b) with high semantic similarity
For each pair:
  E_ab = HdcFingerprint::bind(E_a.fingerprint, E_b.fingerprint)
  If E_ab is not near-similar to any existing Engram:
    imagined_body = prompt_model("given these two concepts, what follows?", E_a, E_b)
    OR
    imagined_body = Body::Composite(E_a.key_concepts, E_b.key_concepts)  # no model
  Write Engram { kind: Imagined, fingerprint: E_ab, trust: 0.2, verified: false }
```

The imagination phase may or may not use a model. The no-model path (compositional
body) is cheaper and produces more structured output. The model path produces richer
text but costs compute.

---

## Hypnagogia

The transition between NREM and REM phases is the "hypnagogic" state — Roko's analog
of the vivid imagery that occurs at sleep onset. During hypnagogia, the agent:
- Generates a small number of high-quality cross-domain bindings
- Uses a slightly higher temperature (if model-assisted) to encourage divergent
  combinations

This is the "creativity" mode of Dreams. The resulting Engrams are more novel but also
less reliable — they need time (real-time usage) to demonstrate utility before their
trust is promoted.

---

## Loop Participation

Dreams participates only in Delta ticks:

| Delta stage | Dreams role |
|---|---|
| QUERY | Requests full 24 h lookback for replay candidates |
| SCORE | Re-scores candidates with current weights |
| PERSIST | Writes updated Engrams (replay) and new imagined Engrams (REM) |

Dreams does not participate in Gamma or Theta ticks. Its influence on real-time
processing comes through the Engrams it writes and the routing priors it updates.

---

## Configuration

```toml
[dreams]
enabled               = false     # Built but not default-enabled
replay_window         = "24h"     # lookback for NREM replay
imagination_pairs     = 20        # number of pairs for REM phase
imagination_model     = "none"    # "none" | model-id for model-assisted imagination
hypnagogia_enabled    = true
imagined_initial_trust = 0.20
```

---

## See also

- [`subsystems/dreams/`](../../subsystems/dreams/README.md) — full Dreams implementation
- [Delta consolidation](../07-speeds/03-delta-consolidation.md) — when Dreams runs
- [Neuro cross-cut](01-neuro.md) — provides the HDC index Dreams queries
- [Active Inference](../06-loop/11-active-inference.md) — free energy triggers Delta (and Dreams)
