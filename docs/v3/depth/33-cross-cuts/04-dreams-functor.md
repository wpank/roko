# 33-04 -- Dreams Functor (F_dreams)

> **Parent**: [33-CROSS-CUTS](../../33-CROSS-CUTS.md)
>
> `DreamsFunctor` is a per-tick identity. Dreams enrichment happens at delta speed
> through `DreamOutputConsumer`, which publishes completed dream cycle output to the
> live KnowledgeStore, DaimonState, and CascadeRouter. This depth file covers the
> two-speed design, the consumer protocol, and routing advice integration.

**Authority**: `crates/roko-compose/src/dreams_functor.rs`

---

## 1. Two-Speed Design

Dreams differs fundamentally from Memory and Daimon: it does not inject per-tick.
The `DreamsFunctor` struct is a zero-sized type that always short-circuits:

```rust
#[derive(Debug, Default, Clone, Copy)]
pub struct DreamsFunctor;

impl CrossCutFunctor for DreamsFunctor {
    fn name(&self) -> &str { "dreams" }

    async fn pre_enrich(&self, input: Vec<Signal>, _ctx: &CrossCutContext)
        -> CrossCutResult<Vec<Signal>> { Ok(input) }

    async fn post_enrich(&self, output: Vec<Signal>, _ctx: &CrossCutContext)
        -> CrossCutResult<Vec<Signal>> { Ok(output) }

    fn should_short_circuit(&self) -> bool { true }
}
```

The functor exists in the `EnrichedCell` wrapper to satisfy the compositional model
(four functors compose into `F_total`) but contributes no enrichment during live
execution. Dream output reaches the system through the `DreamOutputConsumer` after
the `roko-dreams` engine completes an offline consolidation cycle.

---

## 2. DreamOutputConsumer

### 2.1 Construction

```rust
pub struct DreamOutputConsumer {
    knowledge_store: Arc<KnowledgeStore>,
    daimon: Arc<RwLock<DaimonState>>,
    cascade_router: Arc<CascadeRouter>,
    latest_routing_advice: RwLock<Option<DreamRoutingAdvice>>,
}
```

The consumer binds the same live state targets as the other functors. It is
instantiated once per workspace and invoked by the dream engine after each
completed cycle.

### 2.2 Consumption Protocol

`consume(report, routing_advice)` applies two natural transformations:

1. **eta_DM (Dreams -> Memory)**: `eta_DM(report)` extracts knowledge entries from
   the report's clusters, regressions, and strategy hypotheses. These are ingested
   into the durable KnowledgeStore. Each entry's source defaults to
   `"natural_transform:eta_DM"` if not already set.

2. **eta_DN (Dreams -> Daimon)**: `eta_DN(report)` converts the report into an
   `AffectEvent::DreamOutcome` and a depotentiation flag. The live DaimonState is
   appraised with the event, and if `processed_episodes > 0`, dream depotentiation
   runs to reduce negative emotional charge.

3. **Routing advice** (optional): if present, the `DreamRoutingAdvice` is stored
   for subsequent `route_with_published_advice()` calls. The advice biases the
   CascadeRouter toward models that performed well during the dream analysis period.

### 2.3 Return Value

```rust
pub struct DreamConsumptionReport {
    pub knowledge_entries: usize,       // entries ingested
    pub affect_updated: bool,           // always true after successful consume
    pub routing_advice_published: bool, // true if advice was provided
}
```

---

## 3. Routing Advice Integration

`route_with_published_advice()` composes dream routing advice with the live
CascadeRouter:

1. If no advice is stored, the standard `cascade_router.route(context)` is used.
2. If advice is present, `dream_advice_to_routing_bias()` converts it into a
   `RoutingBias` for the given task category and complexity band.
3. The biased route is returned via `cascade_router.route_with_bias()`.

This closes the feedback loop from offline consolidation to live model selection:
dream analysis identifies which models succeed on which task categories, and the
routing bias ensures subsequent dispatches prefer those models.

---

## 4. What Dreams Produces

A `DreamCycleReport` contains:

| Field | Content |
|-------|---------|
| `clusters` | NREM replay clusters, each with `knowledge_entries` and `regression_entries` |
| `regressions_detected` | Entries identifying performance regressions |
| `strategy_hypotheses` | Speculative knowledge entries from REM imagination |
| `analysis` | Tier progression report with insights, heuristics, playbooks, falsifiers |
| `routing_recommendations` | Count of model-routing recommendations |
| `performance_notes` | Free-text observations |

The `eta_DM` transformation flattens clusters, regressions, and strategy hypotheses
into a deduplicated `Vec<KnowledgeEntry>` using a BTreeMap keyed by entry ID.

---

## 5. Key Tests

| Test | What it verifies |
|------|-----------------|
| `dreams_is_a_strict_per_tick_passthrough` | `pre_enrich` returns input unchanged; `should_short_circuit()` is true |
| `output_consumer_publishes_to_all_three_live_targets` | KnowledgeStore receives entry, Daimon PAD decreases, CascadeRouter routes biased |

---

## References

- See [10-DREAMS](../../10-DREAMS.md) for the full dream cycle specification.
- Mattar & Daw (2018). "Prioritized memory access." *Nature Neuroscience*.
- Walker & van der Helm (2009). "Overnight therapy?" *Psychological Bulletin*.
