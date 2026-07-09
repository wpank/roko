# C - HDC, Evolution, Sleep-Time Compute (Docs 05, 06, 12)

This section needs a hard split: there is a small amount of real HDC
evidence in the runtime, but most of the big ideas here are still
frontier design.

Generated: 2026-04-18

---

## Shipping Evidence

### C.01 - HDC helpers are already used by dreams

**Status**: DONE

The runtime already uses `text_fingerprint`:

- trust-region similarity in `imagination.rs:136-148`
- cluster/vector construction in `cycle.rs:1100-1146` and `cycle.rs:1959-1978`

So Doc 06 should not read as if HDC use is purely aspirational.

### C.02 - K-medoids support infrastructure exists

**Status**: DONE

The supporting clustering stack is real:

- `k_medoids()` in `hdc_clustering.rs:54-120`
- `CrossEpisodeConsolidator` in `pattern_discovery.rs:291-390`
- cycle-level cluster/report types in `cycle.rs:259-324`

What stays mixed is direct invocation from the dream cycle, not the
existence of the infrastructure itself.

---

## Partial / Open Seams

### C.03 - Dream feedback into downstream learning/routing

**Status**: PARTIAL

Dreams already touch `KnowledgeStore` and `PlaybookStore`, but the
stronger "dream updates gate thresholds / CascadeRouter / mesh behavior"
story is still an integration seam, not a shipped loop.

### C.04 - Static dream-agent config, not dynamic sleep-time routing

**Status**: PARTIAL

`DreamAgentConfig` is configurable, but that is not the same thing as a
dynamic sleep-time compute policy or per-cycle router-driven model
selection.

---

## Target-State Only

### C.05 - Evolution / fourth dream phase

**Status**: TARGET-STATE

There is no fourth EVOLUTION phase in the shipping runtime. The current
cycle is still the three-stage runtime described in section A.

### C.06 - MAP-Elites and quality-diversity archives

**Status**: TARGET-STATE

No MAP-Elites archive, descriptor lattice, or quality-diversity search
exists in `roko-dreams`.

### C.07 - Sleep-time compute and `rethink_memory`

**Status**: TARGET-STATE

The current runtime has `DreamBudget`; it does not implement Lin-style
sleep-time compute or query-predictability machinery.

### C.08 - Sleepwalker mode

**Status**: TARGET-STATE

No dedicated Sleepwalker/local-only dream mode ships today.

### C.09 - World models

**Status**: TARGET-STATE

Dreamer, IRIS, Genie, and similar world-model integrations are not in
the current tree. The shipping counterfactual surface is the lighter
`CausalModel` path in `imagination.rs`.

---

## What To Carry Into The Live Docs

- Doc 06 can cite real HDC usage and real clustering support.
- Docs 05 and 12 should be labeled as target-state design, not as near-runtime architecture.
- Any world-model, MAP-Elites, or Sleepwalker language should move under explicit future-work banners.
