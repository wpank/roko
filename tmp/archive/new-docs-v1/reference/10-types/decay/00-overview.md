# Decay — Overview

> Why Engrams decay; the demurrage framing; the five decay models at a glance.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Engram](../../01-engram/00-overview.md)  
**Used by**: Substrate GC, retrieval ranking  
**Last reviewed**: 2026-04-19

---

## TL;DR

Engrams decay because information has a shelf life. The decay system implements "use it
or lose it" at the substrate level. The primary model (Demurrage) rewards retrieval and
taxes idleness. Four other models (Exponential, Step, Linear, Custom) handle specialized
use cases. The decay value is not part of the identity hash — it can be changed without
creating a new Engram.

---

## The Idea

A system with no decay accumulates everything forever. Eventually, old, irrelevant
Engrams crowd out recent, relevant ones in retrieval results. The agent's "memory"
becomes stale and slow.

Decay is the solution: information that is not used fades away. When the substrate runs
garbage collection, the Engrams with the lowest effective weight (score × decay) are
removed first. The substrate stays trim and relevant.

The **demurrage** framing frames this as an economic phenomenon: idle information incurs
a holding cost (like demurrage in economics — a charge on currency to prevent hoarding).
This incentivizes the retrieval loop to "spend" its knowledge by using it, which
reinforces it, which keeps it alive.

---

## The Five Decay Models

### Demurrage (Primary)

Balance-based model: a `balance` that decreases each idle day and increases each
retrieval. The most nuanced model; recommended for most Engrams.

```
idle day:     balance *= (1 - idle_tax_per_day)
retrieval:    balance = min(1.0, balance + reinforcement_per_use)
weight(t) = balance
```

### Exponential

Classic half-life decay. Weight halves every `half_life_secs` seconds.

```
weight(t) = 0.5^(elapsed_secs / half_life_secs)
```

### Step

Weight drops by a fixed multiplier at each epoch boundary.

```
epoch_number = elapsed_secs / epoch_secs
weight(t) = step_multiplier^epoch_number
```

### Linear

Weight decreases linearly until it reaches 0.

```
weight(t) = max(0.0, 1.0 - rate_per_sec * elapsed_secs)
```

### Custom

A user-provided function `(elapsed_secs: f64, params: &CustomParams) -> f64`.

---

## When to Use Which Model

| Engram kind | Recommended decay |
|-------------|------------------|
| `KnowledgeEntry` | Demurrage (long-lived; rewarded on use) |
| `AgentOutput` | Exponential (stale quickly) |
| `GateVerdict` | Step (valid per epoch; then expires) |
| `Pheromone` | Exponential (short half-life for ACO evaporation) |
| `Metric` | Linear or Step (time-series; expires after retention window) |
| `Observation` | Exponential (recent observations matter; old ones don't) |
| `Episode` | Demurrage (valuable for learning; should persist if used) |

For the full cross-product of Kind × Decay, see [`08-tier-matrix.md`](08-tier-matrix.md).

---

## See Also

- [`01-demurrage.md`](01-demurrage.md) — the primary model
- [`08-tier-matrix.md`](08-tier-matrix.md) — which model for which kind
- [`reference/01-engram/09-decay-fields.md`](../../01-engram/09-decay-fields.md) — decay on the Engram struct
