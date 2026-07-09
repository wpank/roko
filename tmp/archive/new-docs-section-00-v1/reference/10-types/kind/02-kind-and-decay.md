# Kind — Kind and Decay

> How Kind determines the default decay model and parameters for a new Engram.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md), [Decay Overview](../decay/00-overview.md)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Each Kind has a default decay policy encoded in `Kind::default_decay()`. The method
returns a `Decay` variant configured with parameters appropriate for that Kind's expected
lifetime. Callers can override via the builder. The canonical table is in
[Decay Tier Matrix](../decay/08-tier-matrix.md); this page explains the reasoning behind
each choice.

---

## Why Kind Drives Default Decay

The creator of an Engram often knows only the Kind and Body. They should not be required
to choose a decay model from scratch. The Kind provides enough information for a sensible
default:

- A `ContextAssembly` is inherently session-scoped — it should die with the session.
- A `Reflection` is inherently long-lived — it should decay only if never accessed.
- A `Pheromone` is inherently ephemeral — it should be gone within hours.

Encoding these defaults in `Kind::default_decay()` means the builder produces a
well-configured Engram even when the caller specifies only `kind` and `body`.

---

## Default Choices by Kind

| Kind | Model | Key parameter | Rationale |
|---|---|---|---|
| `AgentOutput` | Demurrage | `idle_tax=0.02` | Used across tasks; moderate decay |
| `GateVerdict` | Demurrage | `idle_tax=0.03` | Reviewed in post-mortems; moderate life |
| `ToolTrace` | Demurrage | `idle_tax=0.05` | Ephemeral; fast decay |
| `KnowledgeEntry` | Demurrage | `idle_tax=0.005` | Durable knowledge; very slow decay |
| `Prediction` | Exponential | `half_life=7d` | Predictions age continuously; no reinforce |
| `Observation` | Exponential | `half_life=1d` | Raw data; fast aging |
| `Plan` | Step | `epoch=1w, mult=0.5` | Sprint-scoped; sharp step at epoch end |
| `Episode` | Demurrage | `idle_tax=0.01` | Episodic memory; moderate life |
| `Reflection` | Demurrage | `idle_tax=0.003` | Very slow decay; near-permanent |
| `Pheromone` | Exponential | `half_life=1h` | Trail signals expire within hours |
| `Metric` | Linear | `rate=1/86400` | 1-day hard deadline |
| `ContextAssembly` | Linear | `rate=1/1800` | 30-min session lifetime |
| `ModelSelection` | Demurrage | `idle_tax=0.01` | Decisions reviewed occasionally |
| `ErrorRecord` | Demurrage | `idle_tax=0.02` | Post-mortem review window |
| `Custom(_)` | Demurrage | defaults | Override always recommended |

---

## Overriding at Construction

```rust
<!-- source: crates/roko-core/src/builder.rs -->

// Use the default decay for KnowledgeEntry
let engram = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(body)
    .provenance(prov)
    .build()?;
// engram.decay = Decay::Demurrage({balance:1.0, idle_tax:0.005, reinforce:0.05})

// Override with faster decay
let short_lived = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(body)
    .provenance(prov)
    .decay(Decay::Linear(LinearDecayParams {
        balance: 1.0,
        rate_per_sec: 1.0 / 3_600.0,  // 1-hour life (for testing)
    }))
    .build()?;
```

---

## Open Questions

- Should `Pheromone` use Demurrage instead of Exponential to allow reinforcement when
  an agent successfully follows the trail?
- Should `Plan` use a calendar-aligned epoch (Monday 00:00 UTC) rather than
  creation-relative?

## See Also

- [`../decay/08-tier-matrix.md`](../decay/08-tier-matrix.md) — complete tier matrix with Rust code
- [`00-overview.md`](00-overview.md) — Kind overview
- [`01-variant-reference.md`](01-variant-reference.md) — per-Kind descriptions
