# L3 — Harness Layer

> Wiring: assembles L2 implementations into a complete running agent.

**Status**: Shipping
**Crates**: `roko-orchestrator`, `roko-gate`
**Depends on**: [L2 Scaffold](03-L2-scaffold.md), [L1 Framework](02-L1-framework.md)
**Used by**: [L4 Orchestration](05-L4-orchestration.md)
**Last reviewed**: 2026-04-19

---

## TL;DR

L3 is the "assembly" layer. It takes the trait objects from L1, the implementations
from L2, and the configuration from the environment, and produces a fully configured
`TickContext` ready to run. `roko-orchestrator` manages agent lifecycle and
multi-agent coordination. `roko-gate` assembles and manages the VERIFY gate pipeline.

---

## `roko-orchestrator` — Agent Lifecycle

`roko-orchestrator` is responsible for:

- **Building `TickContext`** from configuration
- **Starting, stopping, and restarting** agents
- **Health monitoring** — tracking stuck detections, budget violations, persist failures
- **Multi-agent coordination** — routing Pulses between agents, managing shared substrates
- **Delta scheduling** — coordinating consolidation passes across agents
- **Graceful shutdown** — ensuring in-flight ticks complete before stopping

```rust
// source: crates/roko-orchestrator/src/orchestrator.rs
pub struct Orchestrator {
    agents:   HashMap<AgentId, AgentHandle>,
    bus:      Arc<dyn Bus>,
    config:   OrchestratorConfig,
}

impl Orchestrator {
    pub fn spawn_agent(&mut self, config: AgentConfig) -> AgentId {
        let ctx = TickContextBuilder::new()
            .with_config(&config)
            .with_bus(self.bus.clone())
            .build();
        let agent = Agent::new(ctx);
        let handle = tokio::spawn(agent.run_forever());
        let id = AgentId::new();
        self.agents.insert(id, AgentHandle { handle, config });
        id
    }
}
```

### `TickContextBuilder`

The `TickContextBuilder` is the primary user-facing API for creating agents. It
provides a builder pattern for wiring all trait objects:

```rust
let ctx = TickContextBuilder::new()
    .with_substrate(SledSubstrate::open("./agent.db")?)
    .with_scorer(WeightedScorer::default())
    .with_router(CascadeRouter::new())
    .with_composer(GreedyComposer::new())
    .with_gate_pipeline(GatePipeline::default())
    .with_policy(DenyListPolicy::from_config(&config.policy))
    .with_cross_cut(Neuro::new(&config.neuro))
    .with_cross_cut(Daimon::new(&config.daimon))
    .with_speed_tiers(config.speeds)
    .with_budget(config.budget)
    .build()?;
```

---

## `roko-gate` — Gate Pipeline Management

`roko-gate` provides:

- **`GatePipeline`** — ordered collection of `Gate` implementations
- **`GateRegistry`** — lookup by name; allows dynamic gate loading
- **`GateConfig`** — per-gate configuration (enabled, thresholds, soft vs. hard)
- **`GatePipelineBuilder`** — fluent API for assembling pipelines

```rust
// source: crates/roko-gate/src/pipeline.rs
pub struct GatePipeline {
    gates: Vec<Box<dyn Gate>>,
}

impl GatePipeline {
    pub fn verify(&self, output: &ActOutput, ctx: &GateContext) -> VerifyResult {
        let mut reports = vec![];
        for gate in &self.gates {
            let verdict = gate.check(output, ctx);
            reports.push(GateReport { gate_id: gate.id(), verdict: verdict.clone() });
            if matches!(verdict, Verdict::HardFail { .. }) {
                return VerifyResult { verdict, gate_reports: reports, .. };
            }
        }
        VerifyResult::from_reports(reports)
    }
}
```

---

## L3's Role in Dependency Injection

L3 is where all abstract trait objects become concrete implementations. Below L3,
the code talks about `&dyn Scorer`. At L3, it becomes `WeightedScorer` (or a custom
scorer). Above L3, the code talks about `Agent` — an assembled, running thing.

This is the inversion-of-control point. Dependency injection in Roko is not
a framework; it is a conventional pattern enforced by layer rules: L3 knows about
L2 implementations; L2 does not know about L3's wiring decisions.

---

## See also

- [L2 Scaffold](03-L2-scaffold.md) — the implementations L3 wires together
- [L4 Orchestration](05-L4-orchestration.md) — the layer that calls L3 from CLI / API
- [Cross-Cuts](../09-cross-cuts/README.md) — injected via `TickContextBuilder.with_cross_cut()`
- [Dependency Rules](06-dependency-rules.md) — L3 may not be imported from L2
