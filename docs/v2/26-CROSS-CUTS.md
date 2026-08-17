# 26 — Cross-Cut Functors

> Cognitive cross-cuts (Memory, Daimon, Dreams) are endofunctors F: Signal -> Signal that transform the cognitive loop from the side. They do not occupy positions in the 7-step sequence — they modify it. Safety is a fourth endofunctor that operates at the capability level, outside VCG arbitration. The production abstraction is a generic signal-bundle enrichment adapter and does not depend on a concrete Cell type.

> **Implementation status:** E44 COMPLETE (8/8 tasks, 2026-08-15). `roko-compose` now provides `CrossCutFunctor`, `EnrichedCell`, Memory/Daimon/Dreams/Safety functors, all six natural transformations, priority/VCG arbitration, and the dream-output consumer. `roko-cli` launches the non-blocking gate-failure cascade from failed gate completions. Default and HDC `roko-compose` checks and the `roko-cli` check pass; focused cross-cut tests cover composition order, live-store mutation, affect gating, dream publication, commuting transformations, arbitration, and safety pre-filtering.

> The Rust snippets below explain the intended semantics. The authoritative production API is in `crates/roko-compose/src/{cross_cut,memory_functor,daimon_functor,dreams_functor,natural_transforms,safety_functor,auction}.rs`; conceptual names such as `MemoryCell` are not additional runtime types.

### Current implementation sources and reading contract

| Current surface | Authoritative source |
|---|---|
| `LoopStep`, `CrossCutContext`, `CrossCutFunctor`, `EnrichedCell` | `crates/roko-compose/src/cross_cut.rs` |
| `MemoryFunctor`, `DaimonFunctor`, `DreamsFunctor`, `SafetyFunctor` | `crates/roko-compose/src/{memory_functor,daimon_functor,dreams_functor,safety_functor}.rs` |
| Six transformations and gate-failure cascade | `crates/roko-compose/src/natural_transforms.rs` |
| `CrossCutArbitrator`, priority, and second-price resolution | `crates/roko-compose/src/auction.rs` |
| Runner launch of the failed-gate cascade | `crates/roko-cli/src/runner/event_loop.rs` |

The `CrossCutFunctor`/`EnrichedCell` snippet in section 2 mirrors the production
contract. Later `MemoryCell`, NREM/REM Cell, and cognitive-loop snippets are
conceptual explanations of that contract; the E44 status table and the source
paths above define the shipped boundary.

**Depends on**: [01-SIGNAL](01-SIGNAL.md) (Signal, Pulse, demurrage, HDC fingerprint), [02-CELL](02-CELL.md) (Cell, protocols, predict-publish-correct), [03-GRAPH](03-GRAPH.md) (Graph composition), [05-AGENT](05-AGENT.md) (Agent lifecycle, cognitive loop), [06-MEMORY](06-MEMORY.md) (Knowledge Store, tiers, distillation), [16-SECURITY](16-SECURITY.md) (CaMeL IFC, capability grants)

---

## 1. Cross-Cuts Are Not Loop Steps

The 7-step cognitive loop (SENSE, ASSESS, COMPOSE, ACT, VERIFY, PERSIST/BROADCAST, REACT) is a sequential pipeline. Memory (neuro), Daimon (affect), and Dreams (offline consolidation) do not occupy positions in this sequence. They operate orthogonally — each one modifies the loop's behavior from the side, touching multiple steps simultaneously.

The precise structure: each cross-cut is an **endofunctor F: Signal -> Signal** that transforms Signals passing through the loop. When you apply Memory enrichment to SENSE, you are not adding a step before SENSE. You are replacing SENSE with F_memory(SENSE) — a version of SENSE that includes knowledge retrieval.

This distinction matters because:
1. **Cross-cuts compose independently.** You can enable Memory without Daimon, or Daimon without Dreams.
2. **Cross-cuts do not change the operation's topology.** Production
   `EnrichedCell` wraps an inner operation with ordered pre/post hooks; the
   seven-node Graph notation is the architectural model of that composition.
3. **Cross-cuts can be tested independently.** Test Memory injection by running SENSE with and without F_memory.

---

## 2. The Functorial Structure

### 2.1 Category of Signals

Define a category **Sig** where:
- Objects are typed Signal bundles (e.g., `Vec<Signal>` with a particular schema)
- Morphisms are Cells (Signal -> Signal transformations)
- Composition is Graph sequencing (Cell A's output feeds Cell B's input)
- Identity is the pass-through Cell (output = input)

### 2.2 Cross-Cuts as Endofunctors

An endofunctor F: **Sig** -> **Sig** maps:
- Each Signal to an enriched Signal: F(s) has additional metadata or content.
- Each Cell to an enriched Cell: F(cell) wraps the original Cell with pre/post hooks.

```rust
/// A cross-cut endofunctor. Wraps a Cell with pre/post enrichment.
///
/// F(cell).execute(input) =
///   pre_enrich(input)
///     -> cell.execute(enriched_input)
///       -> post_enrich(output)
#[async_trait]
trait CrossCutFunctor<C = CrossCutContext>: Send + Sync + 'static {
    /// Identity: which cross-cut this is.
    fn name(&self) -> &str;

    /// Pre-enrichment: transform input Signals before the Cell runs.
    async fn pre_enrich(
        &self,
        input: Vec<Signal>,
        ctx: &C,
    ) -> CrossCutResult<Vec<Signal>>;

    /// Post-enrichment: transform output Signals after the Cell runs.
    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        ctx: &C,
    ) -> CrossCutResult<Vec<Signal>>;

    /// Optimization hint; safety always returns false.
    fn should_short_circuit(&self) -> bool;
}
```

`EnrichedCell` applies pre-hooks in declaration order, runs the inner operation once, and unwinds post-hooks in reverse order. The first functor is therefore the outermost wrapper.

### 2.3 The Three Functors

| Cross-Cut | Functor | F(Signal) | Injection Points |
|---|---|---|---|
| **Memory** | F_memory | Signal enriched with knowledge entries, HDC similarity scores, tier metadata | SENSE (knowledge retrieval), COMPOSE (context enrichment via VCG bids), VERIFY/REACT (consolidation feedback: reinforcement/weakening) |
| **Daimon** | F_daimon | Signal annotated with PAD bias, somatic markers, behavioral state | ASSESS (score bias via PAD + somatic markers, tier selection), ACT (action gating via prospect value, risk tolerance) |
| **Dreams** | F_dreams | Signal augmented with consolidated patterns, hypotheses, depotentiated affect | Delta speed (runs as its own loop); NREM replay + REM imagination + integration results feed into Memory and Daimon |

### 2.4 Implemented Surface

| E44 task | Production surface | Status |
|---|---|---|
| T01 | `CrossCutFunctor<C>`, `CrossCutContext`, `LoopStep`, `EnrichedCell` | Done |
| T02 | `MemoryFunctor` over `Arc<KnowledgeStore>`; keyword retrieval plus HDC similarity under the `hdc` feature; Neuro recommendations; REACT reinforcement/weakening | Done |
| T03 | `DaimonFunctor` over live `DaimonState`; PAD/somatic enrichment, tier escalation, risk deferral, prospect valuation | Done |
| T04 | Per-tick identity `DreamsFunctor` and `DreamOutputConsumer` for KnowledgeStore, Daimon, and CascadeRouter publication | Done |
| T05 | `eta_MN`, `eta_NM`, `eta_MD`, `eta_DM`, `eta_ND`, `eta_DN`, and `run_gate_failure_cascade` | Done |
| T06 | `CrossCutArbitrator`, fixed-priority resolution, and same-level conflicting-bid second-price VCG | Done |
| T07 | Always-active `SafetyFunctor`, capability default-deny, contract checks, and outer-wrapper composition | Done |
| T08 | Failed gate completions spawn the Memory -> Daimon -> Dreams cascade without blocking the runner event loop | Done |

---

## 3. Memory as Endofunctor (F_memory)

### 3.1 F_memory on SENSE

Memory enriches SENSE by injecting durable knowledge into the perception phase. The endofunctor wraps the SENSE Cell:

```rust
struct MemoryEnrichSense {
    memory: Arc<MemoryCell>,
    max_entries: usize,
    similarity_threshold: f32,
}

impl CrossCutFunctor for MemoryEnrichSense {
    fn name(&self) -> &str { "memory.sense" }

    async fn pre_enrich(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        // Query Memory for knowledge relevant to the current task
        let task_context = TaskContext::from_signals(&input)?;
        let hdc_query = task_context.to_hdc_vector();

        let knowledge = self.memory.query_by_similarity(
            &hdc_query,
            self.max_entries,
            self.similarity_threshold,
        ).await?;

        // Inject knowledge entries into the input Signal bundle
        let mut enriched = input;
        for entry in knowledge {
            enriched.push(entry.to_signal_with_metadata(SignalMetadata {
                source: Source::Memory,
                tier: entry.tier,
                similarity: entry.similarity_score,
                demurrage_balance: entry.balance,
            }));
        }

        Ok(enriched)
    }

    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        _ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        // No post-enrichment for SENSE
        Ok(output)
    }
}
```

### 3.2 F_memory on COMPOSE

Memory enriches COMPOSE by providing knowledge entries to the VCG auction. This is where Memory competes for token budget.

```rust
struct MemoryEnrichCompose {
    memory: Arc<MemoryCell>,
}

impl CrossCutFunctor for MemoryEnrichCompose {
    fn name(&self) -> &str { "memory.compose" }

    async fn pre_enrich(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        // Memory contributes via NeuroBidder and HeuristicBidder
        // in the VCG auction. Pre-enrichment loads the relevant entries.
        let task = TaskContext::from_signals(&input)?;

        let knowledge_bids = self.memory.generate_bids(&task).await?;

        let mut enriched = input;
        for bid in knowledge_bids {
            enriched.push(bid.to_signal());
        }

        Ok(enriched)
    }

    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        _ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        Ok(output)
    }
}
```

### 3.3 F_memory on REACT

After verification, Memory consumes the outcome to reinforce or weaken knowledge entries. Gate pass reinforces; gate fail weakens. This is the feedback loop that makes knowledge self-trimming via demurrage.

```rust
struct MemoryReact {
    memory: Arc<MemoryCell>,
}

impl CrossCutFunctor for MemoryReact {
    fn name(&self) -> &str { "memory.react" }

    async fn pre_enrich(
        &self,
        input: Vec<Signal>,
        _ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        Ok(input)
    }

    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        let verdict = Verdict::from_signals(&output)?;

        // Gate pass: reinforce knowledge that was in context
        if verdict.passed() {
            let context_refs = ctx.get::<Vec<SignalRef>>("compose.included_refs")?;
            for r in context_refs {
                self.memory.reinforce(&r, ReinforcementKind::GatePass).await?;
            }
        }

        // Gate fail: weaken knowledge that was in context
        if verdict.failed() {
            let context_refs = ctx.get::<Vec<SignalRef>>("compose.included_refs")?;
            for r in context_refs {
                self.memory.weaken(&r, WeakeningKind::GateFail).await?;
            }
        }

        Ok(output)
    }
}
```

---

## 4. Daimon as Endofunctor (F_daimon)

The canonical `roko_core::BehavioralState` vocabulary is `Engaged`, `Struggling`, `Coasting`, `Exploring`, `Focused`, and `Resting`. There are no `Neutral`, `Cautious`, or `Anxious` enum variants. In this chapter, "neutral" means the PAD short-circuit region (`|P|`, `|A|`, and `|D|` all below `0.1`), while cautious/anxious behavior is implemented as `Struggling` and/or PAD arousal/dominance threshold checks.

### 4.1 F_daimon on ASSESS

The Daimon biases the ASSESS step by modulating Score weights and tier selection based on the PAD vector (Pleasure-Arousal-Dominance, Mehrabian 1996).

```rust
struct DaimonBiasAssess {
    daimon: Arc<DaimonState>,
}

impl CrossCutFunctor for DaimonBiasAssess {
    fn name(&self) -> &str { "daimon.assess" }

    async fn pre_enrich(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        let pad = self.daimon.current_pad();
        let behavioral_state = self.daimon.behavioral_state();

        // Inject PAD annotations into scored Signals
        let mut enriched = input;
        enriched.push(Signal::metadata("daimon.pad", pad.to_value()));
        enriched.push(Signal::metadata("daimon.state", behavioral_state.to_value()));

        // Somatic marker retrieval: recall how similar decisions felt
        let somatic_markers = self.daimon.retrieve_somatic_markers(
            &ctx.cortical().current_context_hash(),
            5,  // retrieve 5 nearest markers
        );

        // 15% mandatory contrarian retrieval
        let contrarian = self.daimon.retrieve_contrarian_markers(
            &pad,
            1,  // at least 1 contrarian marker
        );

        for marker in somatic_markers.iter().chain(contrarian.iter()) {
            enriched.push(marker.to_signal());
        }

        Ok(enriched)
    }

    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        _ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        // After ASSESS: check if Daimon wants to override tier selection
        let assessment = Assessment::from_signals(&output)?;
        let pad = self.daimon.current_pad();

        // Safety override: if PAD indicates high anxiety + low dominance,
        // escalate to higher tier regardless of EFE
        if pad.arousal > 0.5 && pad.dominance < -0.3 {
            let escalated = assessment.with_tier(
                assessment.tier.max(CognitiveTier::T2Reflective)
            );
            return Ok(escalated.into_signals());
        }

        Ok(output)
    }
}
```

### 4.2 F_daimon on ACT

The Daimon gates risky actions and applies prospect-theoretic value computation (Kahneman-Tversky). In the production implementation, high-risk actions are deferred when the state is `Struggling` or dominance is below the configured `struggling_entry_dominance` threshold.

```rust
struct DaimonGateAct {
    daimon: Arc<DaimonState>,
    thresholds: BehavioralStateThresholds,
}

impl CrossCutFunctor for DaimonGateAct {
    fn name(&self) -> &str { "daimon.act" }

    async fn pre_enrich(
        &self,
        input: Vec<Signal>,
        _ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        let action_plan = ActionPlan::from_signals(&input)?;
        let behavioral_state = self.daimon.behavioral_state();

        let pad = self.daimon.current_pad();
        let cautious = behavioral_state == BehavioralState::Struggling
            || pad.dominance < self.thresholds.struggling_entry_dominance;
        if cautious && action_plan.risk_level() > RiskLevel::Medium {
            // Inject deferral signal: delay high-risk action
            let mut enriched = input;
            enriched.push(Signal::metadata(
                "daimon.gate",
                serde_json::json!({
                    "action": "defer",
                    "reason": "affect state does not support high-risk action",
                    "state": behavioral_state.as_str(),
                }),
            ));
            return Ok(enriched);
        }

        Ok(input)
    }

    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        // After ACT: update Daimon with outcome via prospect theory
        let result = ActionResult::from_signals(&output)?;

        // Prospect theory: asymmetric valuation (lambda = 2.25)
        let reference = ctx.get::<f64>("expected_reward").unwrap_or(0.5);
        let actual = result.reward().unwrap_or(0.5);
        let prospect_value = prospect_value(actual, reference);

        self.daimon.update_from_outcome(prospect_value);

        Ok(output)
    }
}

/// Kahneman-Tversky prospect value with lambda = 2.25.
/// Losses hurt 2.25x more than equivalent gains.
/// Diminishing sensitivity exponent = 0.88 (Tversky & Kahneman 1992).
fn prospect_value(outcome: f64, reference: f64) -> f64 {
    let delta = outcome - reference;
    if delta >= 0.0 {
        delta.powf(0.88)             // diminishing sensitivity to gains
    } else {
        -2.25 * (-delta).powf(0.88)  // loss aversion
    }
}
```

---

## 5. Dreams as Endofunctor (F_dreams)

Dreams differs from Memory and Daimon: it does not inject per-tick. Instead, it runs as its own delta-speed loop and publishes results that Memory and Daimon consume. The functorial structure is:

```
F_dreams: Signal -> Signal

F_dreams(episode) = consolidated_knowledge | hypothesis | depotentiated_affect
```

Dreams is an endofunctor that operates on a different timescale. Its output feeds into F_memory (consolidated knowledge entries) and F_daimon (depotentiated affect state).

### 5.1 The Three-Phase Dream Cycle as a Sub-Graph

```toml
[graph]
name = "dream-cycle"
version = "1.0.0"

[[graph.nodes]]
id = "nrem_replay"
cell = "roko.dreams.nrem_replay"
execution_class = "activity"

[[graph.nodes]]
id = "rem_imagination"
cell = "roko.dreams.rem_imagination"
execution_class = "activity"

[[graph.nodes]]
id = "integration"
cell = "roko.dreams.integration_staging"
execution_class = "activity"

[[graph.edges]]
from = "nrem_replay"
to = "rem_imagination"

[[graph.edges]]
from = "rem_imagination"
to = "integration"
```

### 5.2 NREM Replay Cell

Replays recent episodes ordered by prediction error magnitude (Mattar & Daw 2018: replay what is most useful for future decisions).

```rust
struct NremReplayCell {
    memory: Arc<MemoryCell>,
    episode_store: Arc<dyn Store>,
}

impl Cell for NremReplayCell {
    async fn execute(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        // Retrieve recent episodes, ordered by prediction error magnitude
        let episodes = self.episode_store
            .query(Query::recent_episodes(100))
            .await?;

        // Mattar & Daw (2018): replay what is most useful for future decisions
        let mut prioritized: Vec<(Signal, f64)> = episodes.iter()
            .map(|e| {
                let pe = e.get::<f64>("prediction_error").unwrap_or(0.0);
                (e.clone(), pe)
            })
            .collect();
        prioritized.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // Extract patterns from high-PE episodes
        let mut extracted = Vec::new();
        for (episode, pe) in prioritized.iter().take(20) {
            let patterns = extract_patterns(episode, *pe);
            extracted.extend(patterns);
        }

        Ok(extracted)
    }
}
```

### 5.3 REM Imagination Cell

Generates hypotheses via HDC recombination (cross-domain structural analogies), counterfactual generation (Pearl 2009), and emotional depotentiation (Walker & van der Helm 2009).

```rust
struct RemImaginationCell {
    memory: Arc<MemoryCell>,
}

impl Cell for RemImaginationCell {
    async fn execute(
        &self,
        input: Vec<Signal>,
        ctx: &CellContext,
    ) -> Result<Vec<Signal>, CellError> {
        let patterns = Vec::<Signal>::from_signals(&input)?;

        let mut hypotheses = Vec::new();

        // HDC recombination: combine knowledge from different domains
        let entries = self.memory.random_sample(50).await?;
        for pair in entries.windows(2) {
            let bundled = hdc_bundle(&pair[0].hdc_vector, &pair[1].hdc_vector);
            let similarity = self.memory.nearest_to(&bundled, 3).await?;

            if similarity.iter().any(|s| s.distance < 0.3) {
                // Structural analogy found across domains
                hypotheses.push(Signal::hypothesis(
                    "cross_domain_analogy",
                    pair,
                    &similarity,
                ));
            }
        }

        // Counterfactual generation (Pearl 2009)
        for pattern in &patterns {
            let counterfactual = generate_counterfactual(pattern, ctx).await?;
            if let Some(cf) = counterfactual {
                hypotheses.push(cf);
            }
        }

        // Emotional depotentiation (Walker & van der Helm 2009)
        // Reduce affective charge of negative experiences
        for pattern in &patterns {
            if let Some(pad) = pattern.get::<PADState>("affect") {
                if pad.pleasure < -0.3 {
                    hypotheses.push(Signal::depotentiated(
                        pattern,
                        pad.with_pleasure(pad.pleasure * 0.5),
                    ));
                }
            }
        }

        Ok(hypotheses)
    }
}
```

### 5.4 Integration Staging Cell

The integration cell writes consolidated knowledge to Store and publishes depotentiated affect to Bus for Daimon consumption. This is where Dreams outputs become inputs to the other two cross-cuts.

---

## 6. Natural Transformations Between Cross-Cuts

The cross-cuts interact with each other through **natural transformations** — structure-preserving maps between functors. There are 6 natural transformations forming a fully connected triangle.

```
eta_MN : Memory -> Daimon     (knowledge outcomes update PAD)
eta_NM : Daimon -> Memory     (PAD assessment stored as knowledge)
eta_MD : Memory -> Dreams     (episodes provided for replay)
eta_DM : Dreams -> Memory     (consolidated knowledge stored)
eta_ND : Daimon -> Dreams     (PAD triggers consolidation)
eta_DN : Dreams -> Daimon     (depotentiation updates PAD)
```

### 6.1 The Commuting Triangle

For the system to stay consistent, the composition of transformations must commute:

```
Daimon --eta_NM--> Memory --eta_MD--> Dreams
  |                                     ^
  +-------------eta_ND-----------------+
```

The path Daimon -> Memory -> Dreams (the assessment is stored, then offered for replay) produces the same episode IDs, consolidation priority, and delta-trigger decision as Daimon -> Dreams (PAD directly triggers consolidation). `eta_NM`, `eta_MD`, and `eta_ND` share this mapping, and focused tests assert that the triangle commutes; arbitration is not needed to repair a mismatch after the fact.

### 6.2 Gate Failure Cascade — Full 7-Step Example

When a gate fails, the natural transformations fire in sequence, demonstrating how all three cross-cuts interact:

```
1. VERIFY emits: gate_failure Verdict Signal
       |
       v
2. F_memory(REACT): Memory weakens knowledge entries that were in context
       |                          (eta_MN: knowledge outcome -> PAD update)
       v
3. eta_MN appraises the live Daimon immediately; the next ASSESS observes shifted PAD
       |                          (pleasure down, arousal up)
       v
4. F_daimon -> Dreams (eta_ND): If Daimon is Struggling, may trigger delta
       |
       v
5. Dreams NREM: Replays the failed episode with high priority
       |         (eta_MD: Memory provided the episode)
       v
6. Dreams -> Memory (eta_DM): New heuristic stored: "this approach fails for X"
       |
       v
7. Dreams -> Daimon (eta_DN): Depotentiation reduces negative affect from failure
```

The synchronous portion is encoded once in `run_gate_failure_cascade`: it weakens affected knowledge, applies `eta_MN`, persists `eta_NM`, and produces equal `eta_MD`/`eta_ND` replay inputs. On a failed gate completion, `roko-cli` invokes that helper in a spawned blocking worker. If the transformed assessment is `Struggling`, the worker runs a delta dream and publishes its report through `eta_DM` and `eta_DN`; failure is logged without changing the gate result or blocking the event loop.

---

## 7. VCG Arbitration When Cross-Cuts Compete

When two or more cross-cuts produce conflicting recommendations for the same decision, the system resolves the conflict through a two-layer protocol.

### 7.1 Layer 1: Priority Hierarchy

Fixed priority ordering, applied first:

| Priority | Cross-cut | Rationale |
|---|---|---|
| 1 (highest) | Daimon | Safety constraints and behavioral gating override other concerns |
| 2 | Memory | Validated knowledge overrides speculative hypotheses |
| 3 (lowest) | Dreams | Dream-generated hypotheses are speculative |

```rust
fn resolve_by_priority(
    daimon: Option<Recommendation>,
    memory: Option<Recommendation>,
    dreams: Option<Recommendation>,
) -> Option<Recommendation> {
    // Daimon safety override: always wins if safety_critical
    if let Some(d) = &daimon {
        if d.safety_critical {
            return Some(d.clone());
        }
    }

    // Memory at Consolidated tier or higher overrides Dreams
    if let Some(m) = &memory {
        if m.knowledge_tier >= KnowledgeTier::Consolidated {
            if let Some(d) = &dreams {
                if d.conflicts_with(m) {
                    return Some(m.clone());
                }
            }
        }
    }

    // No clear priority resolution -> fall through to VCG
    None
}
```

### 7.2 Layer 2: VCG Auction (Tiebreaker)

When priority does not cleanly resolve the conflict, a VCG (Vickrey-Clarke-Groves) attention auction breaks the tie. Each cross-cut bids its confidence. The winner pays the second-highest bid (truthful reporting by mechanism design).

```rust
/// VCG auction for cross-cut arbitration.
///
/// Each cross-cut bids its confidence in its recommendation.
/// The winner pays the second-highest bid (truthful reporting).
struct VcgAuction;

impl VcgAuction {
    fn resolve(bids: &[(CrossCutId, f32, Recommendation)]) -> ArbitrationResult {
        if bids.is_empty() {
            return ArbitrationResult::NoConflict;
        }

        // Sort by bid value (confidence), descending
        let mut sorted = bids.to_vec();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        let winner = &sorted[0];
        let second_price = if sorted.len() > 1 { sorted[1].1 } else { 0.0 };

        ArbitrationResult::Resolved {
            winner: winner.0.clone(),
            recommendation: winner.2.clone(),
            attention_cost: second_price,
            runner_up: sorted.get(1).map(|b| b.0.clone()),
        }
    }
}
```

The VCG mechanism ensures truthful bidding: a cross-cut gains nothing by inflating its confidence because the price it "pays" (in attention cost) is determined by the second-highest bid.

### 7.3 When VCG Is Invoked

VCG tiebreaking activates **only** when:
1. Two cross-cuts are at the **same priority level** (both Memory and Dreams bidding on a COMPOSE slot).
2. Both have confidence **above 0.5** (low-confidence bids are ignored).
3. The conflict affects a **Route or Compose** decision (not safety decisions — those always go to Daimon).

### 7.4 Arbitration Adapter

The production `CrossCutArbitrator` is an async `roko-compose` adapter. It enriches with Memory, Daimon, and Dreams, applies the mandatory Safety pre-filter, and only then parses and resolves recommendations. Safety is held as a `CrossCutFunctor` trait object but is structurally absent from `CrossCutId`, so it cannot enter VCG:

```rust
struct CrossCutArbitrator {
    memory: Arc<MemoryFunctor>,
    daimon: Arc<DaimonFunctor>,
    dreams: Arc<DreamsFunctor>,
    safety_filter: Arc<dyn CrossCutFunctor<CrossCutContext>>,
}

impl CrossCutArbitrator {
    async fn arbitrate(
        &self,
        mut input: Vec<Signal>,
        ctx: &CrossCutContext,
    ) -> CrossCutResult<CrossCutArbitration> {
        input = self.memory.pre_enrich(input, ctx).await?;
        input = self.daimon.pre_enrich(input, ctx).await?;
        input = self.dreams.pre_enrich(input, ctx).await?;
        let signals = self.safety_filter.pre_enrich(input, ctx).await?;
        let recommendations = signals.iter()
            .filter_map(CrossCutRecommendation::from_signal)
            .collect::<Vec<_>>();
        let result = resolve_by_priority(&recommendations)
            .unwrap_or_else(|| resolve_by_vcg(&recommendations));
        Ok(CrossCutArbitration { signals, result })
    }
}
```

---

## 8. Safety as Fourth Endofunctor (F_safety)

The three named cross-cuts (Memory, Daimon, Dreams) are the architectural ones. Safety is a fourth endofunctor that operates at a fundamentally different level.

### 8.1 F_safety: Signal -> Signal

Safety is an endofunctor that operates at the **capability level**, not the behavioral level:

- **Pre-filter on every loop step**: remove Signals requiring capabilities outside the active grant set; unknown or malformed capability names are denied by default
- **Post-filter on every loop step**: remove Signals above the contract's taint ceiling or naming a tool outside its allowlist
- **Warn on violations**: filtering emits warnings rather than turning a permissive contract into an execution error
- **Wrap arbitration**: Safety runs before recommendation collection and never becomes a bidder

```rust
struct SafetyFunctor {
    contract: AgentContract,
    grants: CapabilitySet,
}

impl CrossCutFunctor for SafetyFunctor {
    fn name(&self) -> &str { "safety" }

    async fn pre_enrich(
        &self,
        input: Vec<Signal>,
        _ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<Signal>> {
        Ok(input.into_iter()
            .filter(|signal| self.capability_allowed(signal))
            .collect())
    }

    async fn post_enrich(
        &self,
        output: Vec<Signal>,
        _ctx: &CrossCutContext,
    ) -> CrossCutResult<Vec<Signal>> {
        Ok(output.into_iter()
            .filter(|signal| self.contract_allowed(signal))
            .collect())
    }

    fn should_short_circuit(&self) -> bool { false }
}
```

### 8.2 Safety vs Daimon: Different Levels

Safety and Daimon both claim high priority. The resolution: **Safety operates at the capability level** (what is permitted), **Daimon operates at the behavioral level** (what is advisable). Safety is a hard constraint; Daimon is a soft bias.

```
Safety:  "This tool call is not in the capability grant set. Blocked."
Daimon:  "This action is risky given current PAD state. Deferred."
```

**Safety does not participate in VCG arbitration.** It is a pre-filter that runs before the arbitrator, not a bidder within it. F_safety composes with the other functors as an outer wrapper:

```
F_total = F_safety . F_arbitrated(F_memory, F_daimon, F_dreams)
```

This means Safety never loses a "vote." It cannot be outbid. It is structurally prior to the cross-cut competition.

---

## 9. Feedback Loops

The architecture defines five feedback loops. E44 implements the first three integration paths; automatic arbitration calibration and safety-contract evolution remain policy follow-ups rather than claims of this tranche:

| Loop | What It Observes | What It Adjusts | Status |
|---|---|---|---|
| **Memory reinforcement** | Gate pass/fail with knowledge entries in context | Demurrage and prediction-utility state: pass uses gated reinforcement; fail records unsuccessful usage and utility | Implemented |
| **Daimon adaptation** | Gate and prospect-theory-weighted task outcomes (`lambda=2.25`, `alpha=0.88`) | Live PAD and behavioral state; somatic retrieval keeps the configured 15% contrarian fraction | Implemented |
| **Dream prioritization/publication** | Memory/Daimon replay inputs and completed `DreamCycleReport` values | Delta-dream input, consolidated KnowledgeStore entries, Daimon depotentiation, and routing advice | Implemented across `roko-dreams` and the E44 consumer |
| **Arbitration calibration** | VCG outcomes correlated with downstream gate results | Future confidence discount for consistently wrong bidders | Design follow-up |
| **Safety contract evolution** | Logged safety violations and reviewed false positives | Future contract refinement; any relaxation requires manual review | Design follow-up |

---

## 10. Composition Order and Overhead

### 10.1 Functor Application Order

F_memory and F_daimon both enrich ASSESS. The default application order:

```
ASSESS_enriched = F_daimon(F_memory(ASSESS_raw))
```

F_daimon runs after F_memory, so Daimon biases scores that already include knowledge context. This order is intentional: Daimon's somatic markers operate on the fully-contextualized assessment.

### 10.2 Short-Circuit Optimization

Each cross-cut functor exposes pre/post enrichment. With four functors and seven loop steps, a caller that invokes every hook has at most 56 hook calls per tick; irrelevant hooks are identity transforms, and `should_short_circuit` is an optimization hint to the caller rather than an implicit skip inside `EnrichedCell`:

- **F_memory** short-circuits when knowledge store is empty or query returns zero results
- **F_daimon** short-circuits when PAD is in the neutral region (|P|, |A|, |D| all < 0.1); `Neutral` is not a `BehavioralState` variant
- **F_dreams** short-circuits always (it does not inject per-tick; it runs on its own schedule)
- **F_safety** never short-circuits (safety is always active)

The number of non-identity enrichments depends on the current loop step, memory hits, and PAD state.

---

## 11. Acceptance Criteria

| Criterion | Current verification/status |
|---|---|
| `CrossCutFunctor` trait and generic wrapper ordering | Focused unit test verifies forward pre-hooks and reverse post-hooks |
| F_memory enriches SENSE from KnowledgeStore, including HDC retrieval when enabled | Real-store integration test covers retrieval metadata; the HDC path compiles under `--features hdc` |
| F_memory enriches COMPOSE with auction recommendations | Real-store integration test verifies `AttentionBidder::Neuro` recommendation tags |
| F_memory REACT reinforces on pass and weakens on fail | Real-store test verifies both mutations |
| F_daimon biases ASSESS with PAD and somatic markers | Implemented with live `DaimonState`, `SomaticRetrieval`, configured thresholds, and 15% contrarian retrieval |
| F_daimon escalates high-arousal/low-dominance ASSESS | Focused tier-escalation test |
| F_daimon gates high-risk ACT while `Struggling` or below the dominance threshold | Focused deferral test using the canonical state vocabulary |
| F_daimon applies prospect value (`lambda=2.25`, `alpha=0.88`) | Focused loss-asymmetry test |
| Dreams NREM/REM cycle behavior | Owned by the existing `roko-dreams` engine; E44 does not duplicate it |
| Dream output reaches Memory, Daimon, and routing | Real integration test verifies KnowledgeStore publication, Daimon cooling, and advice-biased CascadeRouter routing |
| Six natural transformations are wired | Structural exports plus focused transformation and live cascade tests |
| Commuting triangle: Daimon -> Memory -> Dreams = Daimon -> Dreams | Focused test asserts equal episode IDs, priority, and delta trigger |
| Priority hierarchy | Focused test verifies safety-critical Daimon and Consolidated/Persistent Memory overrides |
| VCG only considers conflicting Route/Compose recommendations at the same level with confidence > 0.5 | Focused eligibility/adversarial tests |
| VCG second-price mechanism | Focused test verifies that the winner pays the runner-up confidence |
| F_safety blocks capability violations before bid collection | Adversarial test filters a forbidden shell recommendation before arbitration |
| F_safety never participates in VCG | Structural: `CrossCutId` contains only Memory, Daimon, and Dreams |
| F_total = F_safety . F_arbitrated(F_memory, F_daimon, F_dreams) | Wrapper-order and full-arbitrator safety tests |
| Short-circuit: empty memory or empty query | Focused empty-store/query test |
| Short-circuit: neutral PAD region | Implemented predicate over all three PAD dimensions |
| Failed gates start the cross-cut cascade without blocking the runner | `roko-cli` event-loop wiring compiles and uses `tokio::spawn` plus `spawn_blocking` |
| Automatic arbitration-confidence calibration | Design follow-up; not part of E44 T01-T08 |

---

## Version History

| Version | Date | Changes |
|---|---|---|
| 3.1 | 2026-08-15 | E44 implementation complete: documented production functors, transformations, arbitration, safety ordering, runtime gate-failure cascade, canonical BehavioralState vocabulary, and verification boundaries. |
| 3.0 | 2026-04-26 | Unified spec: full functorial treatment with CrossCutFunctor trait, 6 natural transformations, commuting triangle, VCG arbitration protocol, Safety as 4th functor, 5 feedback loops, short-circuit optimization, acceptance criteria. |
| 2.0 | 2026-04-22 | Depth doc: cross-cut-functors.md with Rust code and category theory framing. |
| 1.0 | 2026-04-18 | Initial agent runtime cross-cut design. |
