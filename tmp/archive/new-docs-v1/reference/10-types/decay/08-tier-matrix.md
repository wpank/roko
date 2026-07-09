# Decay — Tier Matrix

> A reference table mapping each Engram Kind to its default decay model, parameters, and cold-tier behaviour.

**Status**: Shipping  
**Crate**: `roko-core`  
**Depends on**: [Overview](00-overview.md), [Kind Enum](../../01-engram/04-kind-enum.md)  
**Used by**: [Substrate](../../../subsystems/substrate/)  
**Last reviewed**: 2026-04-19

---

## TL;DR

Different Engram kinds have different natural lifetimes. A tool trace from a finished task
should decay quickly. A consolidated knowledge entry should decay very slowly or not at all
under normal use. This page defines the canonical default decay parameters for every Kind,
derived from the source docs' decay tier matrix and supplemented by inference where gaps
existed.

---

## The Four Decay Tiers

Before listing per-Kind defaults, it is useful to name the four decay intensity tiers:

| Tier name | Characteristic | Typical half-life |
|---|---|---|
| **Transient** | Short-lived, single-session relevance | Hours to days |
| **Working** | Medium-lived, task-scoped relevance | Days to weeks |
| **Consolidated** | Durable, multi-session relevance | Weeks to months |
| **Persistent** | Long-term, near-immortal knowledge | Years |

These are informal categories, not a separate enum. They exist here to aid reasoning about
decay parameter choices.

---

## Default Decay by Kind

<!-- ADDED: tier matrix was referenced in the source docs but the precise per-Kind mapping
was inferred from the Kind descriptions and decay model semantics. -->

| Kind | Default model | Default params | Tier | Notes |
|---|---|---|---|---|
| `ToolTrace` | Demurrage | `idle_tax=0.05`, `reinforce=0.02` | Transient | Fast decay; tool runs are ephemeral |
| `GateVerdict` | Demurrage | `idle_tax=0.03`, `reinforce=0.03` | Working | Verdicts may be reviewed; moderate life |
| `AgentOutput` | Demurrage | `idle_tax=0.02`, `reinforce=0.05` | Working | Agent results are reused across tasks |
| `Observation` | Exponential | `half_life_secs=86_400` (1d) | Transient | Raw sensor data; value decays quickly |
| `Prediction` | Exponential | `half_life_secs=604_800` (7d) | Working | Predictions become stale after resolution |
| `Plan` | Step | `epoch_secs=604_800`, `step=0.5` | Working | Plans are sprint-scoped |
| `KnowledgeEntry` | Demurrage | `idle_tax=0.005`, `reinforce=0.05` | Consolidated | Core knowledge; decays slowly |
| `Episode` | Demurrage | `idle_tax=0.01`, `reinforce=0.04` | Consolidated | Episodic memory; reinforced by recall |
| `Reflection` | Demurrage | `idle_tax=0.003`, `reinforce=0.05` | Persistent | Introspective knowledge; very durable |
| `Pheromone` | Exponential | `half_life_secs=3_600` (1h) | Transient | Trail signals expire rapidly |
| `Metric` | Linear | `rate_per_sec=1/86_400` | Transient | Point-in-time metric; fixed 1-day life |
| `ContextAssembly` | Linear | `rate_per_sec=1/1_800` | Transient | Context windows are session-scoped |
| `ModelSelection` | Demurrage | `idle_tax=0.01`, `reinforce=0.03` | Working | Model choices remain valid for a while |
| `ErrorRecord` | Demurrage | `idle_tax=0.02`, `reinforce=0.02` | Working | Errors are reviewed in post-mortems |
| `Custom(s)` | Demurrage | defaults | Working | Creators should override at construction |

---

## Rust: Default Decay Constructor

```rust
<!-- source: crates/roko-core/src/decay.rs -->

impl Kind {
    /// Return the default Decay for an Engram of this Kind.
    pub fn default_decay(&self) -> Decay {
        match self {
            Kind::ToolTrace => Decay::Demurrage(DemurrageParams {
                balance: 1.0,
                idle_tax_per_day: 0.05,
                reinforcement_per_use: 0.02,
            }),
            Kind::GateVerdict => Decay::Demurrage(DemurrageParams {
                balance: 1.0,
                idle_tax_per_day: 0.03,
                reinforcement_per_use: 0.03,
            }),
            Kind::AgentOutput => Decay::Demurrage(DemurrageParams {
                balance: 1.0,
                idle_tax_per_day: 0.02,
                reinforcement_per_use: 0.05,
            }),
            Kind::Observation => Decay::Exponential(ExponentialDecayParams {
                half_life_secs: 86_400,
            }),
            Kind::Prediction => Decay::Exponential(ExponentialDecayParams {
                half_life_secs: 604_800,
            }),
            Kind::Plan => Decay::Step(StepDecayParams {
                balance: 1.0,
                epoch_secs: 604_800,
                step_multiplier: 0.5,
            }),
            Kind::KnowledgeEntry => Decay::Demurrage(DemurrageParams {
                balance: 1.0,
                idle_tax_per_day: 0.005,
                reinforcement_per_use: 0.05,
            }),
            Kind::Episode => Decay::Demurrage(DemurrageParams {
                balance: 1.0,
                idle_tax_per_day: 0.01,
                reinforcement_per_use: 0.04,
            }),
            Kind::Reflection => Decay::Demurrage(DemurrageParams {
                balance: 1.0,
                idle_tax_per_day: 0.003,
                reinforcement_per_use: 0.05,
            }),
            Kind::Pheromone => Decay::Exponential(ExponentialDecayParams {
                half_life_secs: 3_600,
            }),
            Kind::Metric => Decay::Linear(LinearDecayParams {
                balance: 1.0,
                rate_per_sec: 1.0 / 86_400.0,
            }),
            Kind::ContextAssembly => Decay::Linear(LinearDecayParams {
                balance: 1.0,
                rate_per_sec: 1.0 / 1_800.0,
            }),
            Kind::ModelSelection => Decay::Demurrage(DemurrageParams {
                balance: 1.0,
                idle_tax_per_day: 0.01,
                reinforcement_per_use: 0.03,
            }),
            Kind::ErrorRecord => Decay::Demurrage(DemurrageParams {
                balance: 1.0,
                idle_tax_per_day: 0.02,
                reinforcement_per_use: 0.02,
            }),
            Kind::Custom(_) => Decay::default(),
        }
    }
}
```

---

## Cold-Tier Dwell Limits

Some Kinds warrant longer cold-tier dwell before GC. These override `MAX_COLD_DWELL_SECS`:

| Kind | Cold dwell override |
|---|---|
| `Reflection` | 5 years |
| `KnowledgeEntry` | 3 years |
| `Episode` | 2 years |
| `ModelSelection` | 1 year (default) |
| `Plan`, `AgentOutput`, `GateVerdict` | 6 months |
| `ToolTrace`, `Observation`, `Metric`, `ContextAssembly`, `Pheromone` | 30 days |

<!-- ADDED: cold dwell overrides — these were not in the source docs; inferred from the
relative durability values described for each Kind. -->

---

## Overriding Defaults

The [Engram Builder](../../01-engram/07-builder-pattern.md) allows overriding the default
decay at construction time:

```rust
<!-- source: crates/roko-core/src/builder.rs -->

let engram = Engram::builder()
    .kind(Kind::KnowledgeEntry)
    .body(body)
    // Override default: use faster decay for this specific entry
    .decay(Decay::Demurrage(DemurrageParams {
        balance: 1.0,
        idle_tax_per_day: 0.02,  // 2× faster than default
        reinforcement_per_use: 0.05,
    }))
    .build()?;
```

Overrides are always permitted. The tier matrix is a default, not a constraint.

---

## Invariants

1. Every Kind has a default decay defined in `default_decay()`.
2. Default decays are **suggestions** — callers may override via the builder.
3. A `Decay::Custom` default for `Kind::Custom` is intentional — custom Kinds must provide
   their own decay logic.
4. Cold dwell overrides are defined per-Kind in the Substrate configuration, not in
   `roko-core` itself.

---

## Open Questions

- Should the tier matrix be data-driven (config file) rather than code-driven? This would
  allow tuning without a recompile. Not yet implemented.
- Should `Pheromone` use Demurrage instead of Exponential to allow reinforcement on
  trail traversal? Currently using Exponential for simplicity.

## See Also

- [`00-overview.md`](00-overview.md) — all decay variants compared
- [`01-demurrage.md`](01-demurrage.md) — the dominant decay model
- [`07-cold-tier-freeze-thaw.md`](07-cold-tier-freeze-thaw.md) — cold storage details
- [`../../01-engram/04-kind-enum.md`](../../01-engram/04-kind-enum.md) — Kind variants
