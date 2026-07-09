# Roko agent runtime: architecture specification

This document describes the complete architecture of the Roko agent runtime -- the system that turns LLM wrappers into long-lived autonomous processes with perception, memory, affect, learning, and dreams. It is the reference specification for implementation.

## Who this is for

Senior engineers building on or contributing to Roko. You may be deeply familiar with Rust but new to agent architectures, or deeply familiar with agent frameworks but new to systems programming. This document explains both dimensions.

---

## 1. Design principles

Seven axioms govern every design decision in the runtime.

### Single runtime, many profiles

One `AgentRuntime` trait serves all domains. A blockchain agent watching Ethereum blocks and a coding agent editing Rust files share the same tick pipeline, the same extension mechanism, the same lifecycle model. What differs is the `DomainProfile` -- a configuration struct that controls tick frequency, active extensions, event subscriptions, and verification gates.

This means you learn one runtime, and you can build agents for any domain by swapping the profile.

### Extension-based composition

Behavior comes from layered extensions, not monolithic code. Adding "react to new academic papers" to an agent means writing a `SourceWatcherExt` that implements the `Extension` trait's `on_observe` hook. You do not modify the runtime itself, the heartbeat pipeline, or any other extension.

Extensions declare their dependency layer (0-7). The runtime fires hooks in layer order. Extensions within the same layer fire in dependency-declared order.

### Heartbeat as clock

All agents tick. This is not optional. Even a "dormant" agent ticks at its gamma frequency -- it just gates everything at T0 (no LLM call, $0). The heartbeat is the universal scheduling primitive that replaces cron, event loops, and manual polling.

Three nested timescales:

| Scale | Default period | Purpose |
|-------|---------------|---------|
| Gamma | 5-60s | Perception. Read environment, triage events. |
| Theta | 30-300s | Decision. Full cognitive pipeline including LLM. |
| Delta | ~50 theta ticks | Consolidation. Dream cycle, knowledge promotion. |

### Event-driven reactivity

Agents subscribe to typed event streams via the EventFabric. A blockchain agent subscribes to `NewBlock` and `PriceFeed`. A coding agent subscribes to `FileChanged` and `TestResult`. Events can interrupt dreams (emergency wakeup) and contribute to prediction error (novel events escalate cognitive tier).

### Type-state lifecycle

The Rust type system enforces valid agent state transitions at compile time. You cannot call `.tick()` on a dead agent -- it is not a runtime error; the program will not compile. This eliminates an entire class of lifecycle bugs that plague other frameworks.

### Learnable context

Context assembly is a feedback loop, not a static template. Sections that correlate with successful outcomes get more allocation budget. Sections that don't contribute get squeezed. The `ContextPolicy` evolves over time through three cybernetic feedback loops operating at different timescales.

### Economic rationality

Cognitive gating makes agents cost-efficient by architecture, not by prompt engineering. When 80% of ticks are pure Rust pattern matching ($0) and only 5% require full LLM reasoning ($0.05), continuous operation becomes economically viable. A traditional agent framework running the same workload costs 35x more.

---

## 2. The heartbeat pipeline

The heartbeat is the core abstraction. Every agent, in every domain, on every tick, runs the same 9-step pipeline. Extensions hook into each step. Cognitive gating (step 4) decides whether expensive steps (5-8) execute at all.

### The nine steps

```
1. OBSERVE    — Extensions read their data sources
2. RETRIEVE   — Query knowledge stores for relevant context
3. ANALYZE    — Compute prediction error (expected vs. observed)
4. GATE       — Decide cognitive tier: T0 ($0) / T1 ($0.001) / T2 ($0.05)
5. SIMULATE   — (T1/T2 only) Sandbox candidate actions
6. VALIDATE   — (T1/T2 only) Check safety constraints
7. EXECUTE    — (T1/T2 only) Take action via tool calls
8. VERIFY     — (T1/T2 only) Confirm outcome matches expectation
9. REFLECT    — Record DecisionCycleRecord, update knowledge
```

Steps 5-8 are conditional. When the gate returns T0, the tick completes at step 4 -- no LLM call, no tool execution, no cost. The agent observed, determined nothing novel was happening, and went back to sleep.

### Implementation

```rust
pub struct HeartbeatPipeline {
    /// Current tick frequency (gamma, theta, or delta)
    frequency: Frequency,
    /// Monotonically increasing tick counter
    tick_count: u64,
    /// Current adaptive gating threshold
    adaptive_threshold: f64,
    /// Last computed prediction error (for trend analysis)
    last_prediction_error: f64,
    /// Per-tick arena allocator (freed after each tick)
    arena: bumpalo::Bump,
}

impl HeartbeatPipeline {
    pub async fn execute_tick(
        &mut self,
        extensions: &mut ExtensionChain,
        cortical: &CorticalState,
    ) -> Result<TickOutcome> {
        let arena = TickArena::new(&self.arena);
        let mut ctx = TickContext::new(self.tick_count, self.frequency, &arena);

        // Step 1: OBSERVE
        // Each extension reads its data source (chain blocks, file changes, etc.)
        extensions.fire_observe(&mut ObserveContext::from(&mut ctx)).await?;

        // Step 2: RETRIEVE
        // Extensions query knowledge stores (neuro, playbooks, chain HDC index)
        extensions.fire_tick_start(&mut ctx).await?;

        // Step 3: ANALYZE
        // Compute prediction error: how surprising is what we observed?
        let prediction_error = self.compute_prediction_error(&ctx);

        // Step 4: GATE
        // Extensions can force a tier (e.g., operator message forces T2).
        // Otherwise, the default gate uses prediction error + adaptive threshold.
        let tier = match extensions.fire_gate(&mut GateContext::from(&mut ctx)).await? {
            Some(forced) => forced,
            None => self.default_gate(prediction_error, cortical),
        };
        ctx.set_tier(tier);

        // Steps 5-8: conditional on tier
        let outcome = match tier {
            CognitiveTier::T0 => TickOutcome::Suppressed {
                prediction_error,
                reason: ctx.suppression_reason(),
            },
            CognitiveTier::T1 | CognitiveTier::T2 => {
                // Step 5-6: SIMULATE + VALIDATE (via context assembly)
                let mut workspace = CognitiveWorkspace::new(tier);
                extensions.fire_assemble_context(&mut workspace).await?;

                // Step 7: EXECUTE (inference + tool calls)
                let mut inf_ctx = InferenceContext::new(workspace, tier);
                extensions.fire_before_inference(&mut inf_ctx).await?;
                let response = self.run_inference(&inf_ctx).await?;
                inf_ctx.set_response(response);
                extensions.fire_after_inference(&mut inf_ctx).await?;

                let actions = self.execute_tool_calls(&inf_ctx, extensions).await?;

                // Step 8: VERIFY (gate pipeline for actions)
                let verified = self.verify_actions(&actions, extensions).await?;

                TickOutcome::Acted {
                    tier,
                    actions: verified,
                    cost: inf_ctx.cost(),
                    prediction_error,
                }
            }
        };

        // Step 9: REFLECT
        let record = DecisionCycleRecord::from_tick(&ctx, &outcome);
        extensions.fire_reflect(&record).await?;
        extensions.fire_tick_end(&mut ctx).await?;

        self.last_prediction_error = prediction_error;
        self.tick_count += 1;
        self.arena.reset(); // free per-tick allocations
        Ok(outcome)
    }
}
```

### Prediction error algorithm

Prediction error quantifies how surprising the current observation is relative to what the agent expected. It drives the gate decision -- high prediction error means "something novel happened, escalate to expensive reasoning."

Six weighted sources contribute:

```rust
pub fn compute_prediction_error(&self, ctx: &TickContext) -> f64 {
    let observations = ctx.observations();

    // Source 1: Environmental novelty (0.25 weight)
    // How different is the environment from the last observation?
    let env_novelty = observations.environmental_delta();

    // Source 2: Pattern match failure (0.20 weight)
    // Did known patterns fail to explain what happened?
    let pattern_miss = observations.unmatched_fraction();

    // Source 3: Value deviation (0.20 weight)
    // Did observed values deviate from statistical expectations?
    let value_dev = observations.value_deviation();

    // Source 4: Temporal surprise (0.15 weight)
    // Did something happen at an unexpected time?
    let temporal = observations.temporal_anomaly();

    // Source 5: Social signal (0.10 weight)
    // Did we receive an unusual pheromone or operator message?
    let social = observations.social_novelty();

    // Source 6: Somatic echo (0.10 weight)
    // Does this situation pattern-match against past negative outcomes?
    let somatic = observations.somatic_alarm();

    let raw = env_novelty * 0.25
        + pattern_miss * 0.20
        + value_dev * 0.20
        + temporal * 0.15
        + social * 0.10
        + somatic * 0.10;

    // Normalize to [0, 1] with sigmoid compression
    sigmoid(raw, self.normalization_scale)
}
```

What these sources mean in different domains:

| Source | Coding agent | Blockchain agent | Research agent |
|--------|-------------|-----------------|----------------|
| Environmental novelty | Files changed unexpectedly | Unusual block gas, new contract deployment | New paper in tracked topic |
| Pattern match failure | Test failure with no obvious cause | Transaction type never seen before | Citation contradicts existing knowledge |
| Value deviation | Build time 3x normal | Gas price 5 sigma above mean | Paper quality score anomalous |
| Temporal surprise | CI failure at 3am (no commits) | Block produced 2s after prior (MEV) | Paper published outside conference schedule |
| Social signal | Operator message or PR comment | Pheromone from another agent flagging risk | Colleague highlighted a paper |
| Somatic echo | This code pattern caused bugs before | This token pattern preceded a rug-pull | This source was unreliable in past |

### Adaptive threshold

The gate threshold is not static. It adapts based on three modulating factors:

```rust
fn default_gate(&self, prediction_error: f64, cortical: &CorticalState) -> CognitiveTier {
    // Base threshold from domain profile (e.g., 0.3 for blockchain, 0.4 for coding)
    let base = self.adaptive_threshold;

    // Factor 1: Confidence
    // When the agent has low epistemic confidence, it should be more
    // willing to engage reasoning (lower threshold).
    let confidence = cortical.epistemic_confidence();
    let confidence_factor = 0.5 + (confidence * 0.5); // range: [0.5, 1.0]

    // Factor 2: Mortality pressure
    // When the agent is running low on budget, raise the threshold
    // (be more conservative, suppress more ticks).
    let vitality = cortical.economic_vitality();
    let mortality_factor = if vitality < 0.3 {
        1.5 // 50% harder to escalate when dying
    } else if vitality < 0.5 {
        1.2 // 20% harder in conservation mode
    } else {
        1.0 // no effect when healthy
    };

    // Factor 3: Arousal
    // High arousal (recently took important action) temporarily lowers
    // the threshold, keeping the agent alert.
    let arousal = cortical.arousal();
    let arousal_factor = 1.0 - (arousal * 0.3); // range: [0.7, 1.0]

    // Effective threshold, clamped to valid range
    let effective = (base * confidence_factor * mortality_factor * arousal_factor)
        .clamp(0.05, 0.80);

    if prediction_error < effective {
        CognitiveTier::T0
    } else if prediction_error < effective * 2.0 {
        CognitiveTier::T1
    } else {
        CognitiveTier::T2
    }
}
```

The clamp range `[0.05, 0.80]` prevents pathological behavior:
- Floor at 0.05: even a maximally conservative agent still escalates truly extreme events.
- Ceiling at 0.80: even a maximally reactive agent still suppresses some noise.

### Cost model

The economic argument for cognitive gating:

| Metric | With gating | Without gating |
|--------|-------------|---------------|
| T0 ticks (no LLM) | 80% | 0% |
| T1 ticks (cheap model) | 15% | 0% |
| T2 ticks (full model) | 5% | 100% |
| Cost per tick (avg) | $0.004 | $0.05 |
| Daily cost (1 tick/min) | $5.76 | $72.00 |
| Daily cost (1 tick/5s, blockchain) | $69 | $864 |

In calm market conditions (low prediction error, high pattern match):

- **With gating**: $6/day (95% T0, 4% T1, 1% T2)
- **Without gating**: $576/day (all T2)

In volatile conditions (high prediction error, novel events):

- **With gating**: $46/day (60% T0, 25% T1, 15% T2)
- **Without gating**: still $576/day

The gated agent adapts its spend to the environment's information density. When nothing is happening, it costs almost nothing. When everything is happening, it spends what's needed. Traditional frameworks have no mechanism for this.

### Tick outcome types

```rust
pub enum TickOutcome {
    /// T0: nothing novel, no action taken
    Suppressed {
        prediction_error: f64,
        reason: SuppressionReason,
    },
    /// T1/T2: the agent reasoned and possibly acted
    Acted {
        tier: CognitiveTier,
        actions: Vec<VerifiedAction>,
        cost: Cost,
        prediction_error: f64,
    },
    /// The agent transitioned to dreaming state
    DreamRequested {
        sleep_pressure: f64,
    },
    /// An error occurred during the tick (extension failure, LLM error, etc.)
    Error {
        phase: TickPhase,
        error: TickError,
    },
}
```

---

## 3. The extension system

Extensions are the unit of composition. Everything that was previously hardcoded in a monolithic orchestrator becomes an extension. Users add behavior by writing extensions (Rust traits), not by modifying the core runtime.

### The Extension trait

22 hooks across 7 categories. Every hook has a default no-op implementation -- extensions implement only the hooks they need.

```rust
#[async_trait]
pub trait Extension: Send + Sync + 'static {
    // === Identity ===

    /// Unique name for this extension (e.g., "chain-subscriber", "daimon")
    fn name(&self) -> &str;

    /// Which layer this extension lives in (determines firing order)
    fn layer(&self) -> ExtensionLayer;

    /// Other extensions this one depends on (must fire first within same layer)
    fn depends_on(&self) -> &[&str] { &[] }

    // === Lifecycle (4 hooks) ===

    /// Called once at agent boot. Initialize connections, load state.
    async fn on_boot(&mut self, ctx: &mut BootContext) -> Result<()> { Ok(()) }

    /// Called when resuming from a persisted snapshot.
    async fn on_resume(&mut self, ctx: &mut ResumeContext) -> Result<()> { Ok(()) }

    /// Called when the agent suspends (pauses between tasks).
    async fn on_suspend(&mut self, ctx: &mut SuspendContext) -> Result<()> { Ok(()) }

    /// Called during shutdown. Return a vote (approve, delay, or reject).
    async fn on_shutdown(&mut self, ctx: &ShutdownContext) -> Result<ShutdownVote> {
        Ok(ShutdownVote::Approve)
    }

    // === Heartbeat (3 hooks) ===

    /// Called at the start of every tick, before observation.
    async fn on_tick_start(&mut self, ctx: &mut TickContext) -> Result<()> { Ok(()) }

    /// Called during the OBSERVE step. Add observations to the context.
    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> { Ok(()) }

    /// Called at the end of every tick, after reflection.
    async fn on_tick_end(&mut self, ctx: &mut TickContext) -> Result<()> { Ok(()) }

    // === Cognition (4 hooks) ===

    /// Optionally override the cognitive tier decision.
    /// Return None to defer to the default gate.
    async fn on_gate(&mut self, ctx: &mut GateContext) -> Result<Option<CognitiveTier>> {
        Ok(None)
    }

    /// Contribute sections to the CognitiveWorkspace for this tick's LLM call.
    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> { Ok(()) }

    /// Inspect or modify the inference request before it goes to the LLM.
    async fn on_before_inference(&mut self, ctx: &mut InferenceContext) -> Result<()> { Ok(()) }

    /// Process the LLM response before tool execution.
    async fn on_after_inference(&mut self, ctx: &mut InferenceContext) -> Result<()> { Ok(()) }

    // === Action (2 hooks) ===

    /// Intercept a tool call before execution. Can allow, deny, or modify.
    async fn before_tool_call(&mut self, call: &mut ToolCall) -> Result<ToolDecision> {
        Ok(ToolDecision::Allow)
    }

    /// Observe the result of a tool call after execution.
    async fn after_tool_call(&mut self, call: &ToolCall, result: &ToolResult) -> Result<()> {
        Ok(())
    }

    // === Learning (2 hooks) ===

    /// Called when a tick produces a measurable outcome.
    async fn on_outcome(&mut self, ctx: &mut OutcomeContext) -> Result<()> { Ok(()) }

    /// Called with the complete DecisionCycleRecord for this tick.
    async fn on_reflect(&mut self, record: &DecisionCycleRecord) -> Result<()> { Ok(()) }

    // === Events (2 hooks) ===

    /// Called when a relevant event arrives from the EventFabric.
    async fn on_event(&mut self, event: &RuntimeEvent, ctx: &mut EventContext) -> Result<()> {
        Ok(())
    }

    /// Called when the operator sends a message.
    async fn on_message(&mut self, msg: &OperatorMessage, ctx: &mut MessageContext) -> Result<()> {
        Ok(())
    }

    // === Dreams (3 hooks) ===

    /// Called when the agent enters the dreaming state.
    async fn on_dream_start(&mut self, ctx: &mut DreamContext) -> Result<()> { Ok(()) }

    /// Called during each phase of the dream cycle.
    async fn on_dream_phase(&mut self, phase: DreamPhase, ctx: &mut DreamContext) -> Result<()> {
        Ok(())
    }

    /// Called when the dream cycle completes and the agent wakes.
    async fn on_dream_end(&mut self, outcome: &DreamOutcome) -> Result<()> { Ok(()) }

    // === Persistence (2 hooks) ===

    /// Serialize extension state for snapshot persistence.
    async fn save_state(&self) -> Result<serde_json::Value> { Ok(serde_json::Value::Null) }

    /// Restore extension state from a persisted snapshot.
    async fn load_state(&mut self, state: serde_json::Value) -> Result<()> { Ok(()) }
}
```

### Extension layers

Eight layers determine global firing order. Within a layer, declared dependencies determine local order.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtensionLayer {
    /// Clock, CorticalState, lifecycle management.
    /// Must fire first -- everything else reads from the state it sets up.
    Foundation = 0,

    /// EventFabric consumers, probes, chain/file subscriptions.
    /// Reads environment into observations.
    Perception = 1,

    /// Neuro (knowledge store), episodic memory, working memory.
    /// Retrieves relevant knowledge for the current context.
    Memory = 2,

    /// Daimon (affect), attention salience, cognitive gating, habituation.
    /// Processes observations into decisions.
    Cognition = 3,

    /// ToolDispatch, safety policies, execution, gate verification.
    /// Takes actions in the world.
    Action = 4,

    /// Pheromones, A2A protocol, operator chat, delegation.
    /// Communicates with other agents and humans.
    Social = 5,

    /// Dreams, consolidation, playbook extraction, evolution.
    /// Offline processing between active periods.
    Meta = 6,

    /// Compensation, rollback, graceful shutdown, genome extraction.
    /// Handles failures and end-of-life.
    Recovery = 7,
}
```

Why 8 layers and not 3 or 20? The layer count matches the natural data flow of a cognitive pipeline: perceive -> remember -> decide -> act -> communicate -> learn -> recover. Each layer's outputs feed the next layer's inputs. If you put action (L4) before memory (L2), tool calls would execute without context. If you put recovery (L7) before action (L4), rollback logic would run before there's anything to roll back.

### ExtensionChain builder

The `ExtensionChain` is the runtime's compiled, validated registry of active extensions with pre-computed firing orders.

```rust
pub struct ExtensionChain {
    /// All registered extensions, stored contiguously.
    extensions: Vec<Box<dyn Extension>>,
    /// Pre-computed topological order per hook type.
    /// Key: which hook. Value: indices into `extensions` in firing order.
    firing_order: HashMap<HookId, Vec<usize>>,
}

impl ExtensionChain {
    pub fn builder() -> ExtensionChainBuilder {
        ExtensionChainBuilder { extensions: Vec::new() }
    }

    /// Fire a specific hook across all extensions that implement it.
    /// Extensions fire in layer order, then dependency order within layer.
    pub async fn fire_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
        let order = self.firing_order.get(&HookId::Observe)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        for &idx in order {
            self.extensions[idx].on_observe(ctx).await?;
        }
        Ok(())
    }

    /// Fire the gate hook. Returns the first non-None tier from any extension.
    pub async fn fire_gate(&mut self, ctx: &mut GateContext) -> Result<Option<CognitiveTier>> {
        let order = self.firing_order.get(&HookId::Gate)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        for &idx in order {
            if let Some(tier) = self.extensions[idx].on_gate(ctx).await? {
                return Ok(Some(tier));
            }
        }
        Ok(None)
    }

    // ... one firing method per hook ...
}

pub struct ExtensionChainBuilder {
    extensions: Vec<Box<dyn Extension>>,
}

impl ExtensionChainBuilder {
    /// Add an extension to the chain.
    pub fn add(mut self, ext: impl Extension) -> Self {
        self.extensions.push(Box::new(ext));
        self
    }

    /// Validate dependencies, detect cycles, compute firing orders, and build.
    pub fn build(self) -> Result<ExtensionChain> {
        // 1. Check that every declared dependency exists
        let names: HashSet<&str> = self.extensions.iter()
            .map(|e| e.name())
            .collect();
        for ext in &self.extensions {
            for dep in ext.depends_on() {
                if !names.contains(dep) {
                    return Err(Error::MissingDependency {
                        extension: ext.name().to_owned(),
                        missing: (*dep).to_owned(),
                    });
                }
            }
        }

        // 2. Topological sort (Kahn's algorithm) within each layer
        let mut firing_order = HashMap::new();
        for hook in HookId::all() {
            let order = topological_sort_for_hook(&self.extensions, hook)?;
            firing_order.insert(hook, order);
        }

        Ok(ExtensionChain {
            extensions: self.extensions,
            firing_order,
        })
    }
}
```

### How extensions communicate

Extensions never call each other directly. They communicate through three shared surfaces:

**1. CorticalState (real-time, atomic, lock-free)**

For signals that change every tick and must be read without blocking. One extension writes, any extension reads.

```rust
// DaimonExt writes affect after computing it
impl Extension for DaimonExt {
    async fn on_tick_end(&mut self, ctx: &mut TickContext) -> Result<()> {
        let pad = self.compute_affect(ctx.outcome());
        ctx.cortical().write_arousal(pad.arousal);
        ctx.cortical().write_pleasure(pad.pleasure);
        ctx.cortical().write_dominance(pad.dominance);
        Ok(())
    }
}

// ContextExt reads affect to modulate allocation
impl Extension for ContextExt {
    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> {
        let arousal = ws.cortical().read_arousal();
        // High arousal = more allocation to warnings and risk sections
        if arousal > 0.7 {
            ws.boost_category(ContextCategory::Warnings, 1.5);
        }
        Ok(())
    }
}
```

**2. CognitiveWorkspace (per-tick, structured, auditable)**

For context sections that the LLM will receive. Each extension adds its contribution during `assemble_context`. The workspace manages budgets, priorities, and allocation.

```rust
// NeuroExt adds knowledge sections
impl Extension for NeuroExt {
    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> {
        let relevant = self.store.query_relevant(ws.current_task(), limit: 5);
        for entry in relevant {
            ws.add_section(ContextSection {
                category: ContextCategory::Knowledge,
                priority: entry.confidence as u8,
                content: entry.render(),
                tokens: entry.estimated_tokens(),
                ..Default::default()
            });
        }
        Ok(())
    }
}
```

**3. EventFabric (asynchronous, broadcast, buffered)**

For inter-extension and inter-agent communication that happens outside the tick cycle.

```rust
// GateExt emits verdict events so other extensions (and the TUI) can react
impl Extension for GateExt {
    async fn after_tool_call(&mut self, call: &ToolCall, result: &ToolResult) -> Result<()> {
        if call.name == "compile" || call.name == "test" {
            self.fabric.emit(EventSource::Gate { rung: call.name.clone() }, EventPayload::GateVerdict {
                rung: call.name.clone(),
                passed: result.is_ok(),
                output: result.summary(),
            });
        }
        Ok(())
    }
}
```

---

## 4. Cognitive gating

This is the key architectural innovation. Cognitive gating is what makes continuous autonomous operation economically viable.

### The problem gating solves

Every existing agent framework has the same cost structure: every interaction requires a full LLM API call. Claude Code, Cursor, Aider, LangChain agents -- they all send the entire context to the model on every turn, regardless of whether the situation is novel or routine.

For a one-shot coding task, this is fine. For a long-lived autonomous agent that ticks every 5 seconds for days, it is ruinous. At $0.05 per tick, a blockchain agent costs $864/day. Nobody will run that.

Cognitive gating is the solution: classify each tick as T0 (handle with pure Rust, $0), T1 (handle with a cheap model, $0.001), or T2 (handle with a full model, $0.05). The classification is based on prediction error -- how surprising the current state is relative to expectations.

### The three tiers

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CognitiveTier {
    /// Deterministic response. Pure Rust pattern matching.
    /// No LLM call. Cost: $0.
    /// Used when: prediction error below threshold, known pattern matches,
    /// habituation mask says "you've seen this 100 times."
    T0,

    /// Lightweight reasoning. Cheap/fast model (Haiku-class).
    /// Cost: ~$0.001.
    /// Used when: prediction error above threshold but below 2x threshold,
    /// situation is somewhat novel but not complex.
    T1,

    /// Full reasoning. Expensive/capable model (Opus-class).
    /// Cost: ~$0.05.
    /// Used when: prediction error above 2x threshold, situation is
    /// genuinely novel and complex.
    T2,
}
```

### Domain-specific observation sources

What "observe" means depends on the domain:

**Coding agent (gamma = 30s, threshold = 0.4)**

The coding agent observes:
- File system changes (were files modified outside the agent's own edits?)
- Test results (did a previously-passing test start failing?)
- Git status (new commits on the branch? merge conflicts?)
- CI pipeline status (build passed? failed? pending?)
- Operator messages (new comment on PR? direct instruction?)

Most ticks: nothing changed since last tick. Prediction error near zero. Gate at T0.

When something changes: a test fails, or the operator posts a new instruction. Prediction error spikes. Gate at T1 or T2 depending on complexity.

**Blockchain agent (gamma = 5s, threshold = 0.2)**

The blockchain agent observes:
- Latest block (new transactions, gas prices, contract deployments)
- Mempool (pending transactions affecting watched addresses)
- Price feeds (asset prices, oracle updates)
- Position health (liquidation distance, impermanent loss)
- Pheromone signals (other agents flagging risk or opportunity)

Most ticks: routine blocks with no relevant transactions. Binary Fuse filter rejects 90% of transaction hashes in nanoseconds. Prediction error near zero. Gate at T0.

When something spikes: gas price 5-sigma above mean, or a large swap in a watched pool. Prediction error high. Gate at T1 or T2.

**Research agent (gamma = 60s, threshold = 0.35)**

The research agent observes:
- Source feeds (new papers on arXiv, new commits in tracked repos)
- Data updates (new dataset versions, benchmark results)
- Knowledge graph changes (contradictions, new connections)
- Hypothesis status (evidence for/against active hypotheses)

Most ticks: no new publications, no data changes. Gate at T0.

When a new paper drops in a tracked category: prediction error rises. If the paper contradicts existing knowledge (high PE), gate at T2 for deep analysis.

### The 80/15/5 distribution

The 80% T0 / 15% T1 / 5% T2 distribution is not a target -- it is an emergent property of real environments. Here is why:

Most of the time, nothing interesting happens. Files don't change between ticks. Blocks contain only routine transactions. No new papers appear. The information density of the environment is low. The agent correctly identifies "nothing to do" at near-zero cost.

When something does happen, it's usually simple: a test failed (run it again), a price moved slightly (check position health), a paper appeared (queue for later reading). T1 handles these with a cheap model.

Genuinely novel, complex situations are rare: a critical vulnerability discovered, a black-swan market event, a fundamental contradiction in the knowledge base. These are T2.

The distribution shifts with environmental volatility. During a market crash, a blockchain agent might run 40% T0 / 30% T1 / 30% T2. During a holiday weekend, it might run 98% T0 / 1.5% T1 / 0.5% T2. The adaptive threshold handles this automatically.

---

## 5. Context engineering

Context assembly is the most underrated component of an agent system. The quality of the LLM's reasoning is bounded by the quality of the context it receives. In Roko, context assembly is a learnable control system -- it improves autonomously based on outcome feedback.

### CognitiveWorkspace

Every T1/T2 tick assembles a `CognitiveWorkspace`: a typed, budgeted, auditable collection of context sections that will become the LLM's input.

```rust
pub struct CognitiveWorkspace {
    /// Which cognitive tier this workspace is assembled for.
    /// T1 gets a smaller budget than T2.
    pub tier: CognitiveTier,
    /// Ordered list of context sections (each contributed by an extension)
    pub sections: Vec<ContextSection>,
    /// Total token budget for this workspace (from domain profile)
    pub total_budget_tokens: u32,
    /// Currently allocated tokens
    pub used_tokens: u32,
    /// Audit log: why each section was included/excluded
    pub assembly_log: Vec<AssemblyDecision>,
    /// Reference to cortical state for affect-modulated assembly
    cortical: Arc<CorticalState>,
}

pub struct ContextSection {
    /// What category of information this section contains
    pub category: ContextCategory,
    /// Priority level (1-5). Priority 5 sections are always included.
    /// Priority 1 sections are dropped first when budget is tight.
    pub priority: u8,
    /// Current allocation fraction (learned over time)
    pub allocation: f64,
    /// The actual content (markdown text that the LLM will see)
    pub content: String,
    /// Estimated token count for this section
    pub tokens: u32,
    /// Source metadata (which extension contributed this, when, relevance score)
    pub metadata: SectionMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextCategory {
    /// System role, instructions, constraints
    Role,
    /// Workspace structure, file tree, cross-references
    Workspace,
    /// Current task brief, plan, acceptance criteria
    Task,
    /// Code intelligence (symbols, type signatures, file contents)
    Code,
    /// Knowledge from neuro store (insights, heuristics, anti-knowledge)
    Knowledge,
    /// Playbook rules from successful past episodes
    Playbook,
    /// Market/position state (blockchain domain)
    MarketState,
    /// Strategy parameters (blockchain domain)
    Strategy,
    /// Research sources and citations
    Sources,
    /// Active hypotheses (research domain)
    Hypotheses,
    /// Warnings, risk signals, somatic markers
    Warnings,
    /// Recent iteration memory (last N turns)
    Iteration,
    /// Custom (user-defined extensions)
    Custom(u16),
}
```

### ContextPolicy: learnable allocation

The `ContextPolicy` controls how much budget each category receives. It evolves through feedback -- categories that correlate with successful outcomes grow; categories that don't contribute shrink.

```rust
pub struct ContextPolicy {
    /// Revision counter (increments on every evolution step)
    pub revision: u32,
    /// Base allocations per category (sum to 1.0)
    pub allocations: HashMap<ContextCategory, f64>,
    /// Override allocations for specific behavioral phases
    pub phase_overrides: HashMap<BehavioralPhase, HashMap<ContextCategory, f64>>,
    /// Override allocations for specific task types
    pub task_overrides: HashMap<String, HashMap<ContextCategory, f64>>,
    /// Beta distribution tracking per category (for Thompson sampling)
    pub feedback: HashMap<ContextCategory, BetaDistribution>,
}

#[derive(Debug, Clone)]
pub struct BetaDistribution {
    pub alpha: f64, // successes + 1
    pub beta: f64,  // failures + 1
}

impl BetaDistribution {
    pub fn uniform() -> Self { Self { alpha: 1.0, beta: 1.0 } }
    pub fn mean(&self) -> f64 { self.alpha / (self.alpha + self.beta) }
}
```

### Three cybernetic feedback loops

**Loop 1: Per-tick outcome recalibration**

After every tick that produces a measurable outcome (gate pass/fail, task success/failure), the system records which context categories were included and whether the outcome was positive.

```rust
impl ContextPolicy {
    /// Called after every T1/T2 tick with an outcome.
    pub fn record_outcome(&mut self, included: &[ContextCategory], success: bool) {
        for cat in included {
            let dist = self.feedback.entry(*cat)
                .or_insert(BetaDistribution::uniform());
            if success {
                dist.alpha += 1.0;
            } else {
                dist.beta += 1.0;
            }
        }
    }
}
```

**Loop 2: Per-50-ticks curator cycle**

Every 50 ticks (configurable), the policy evolves its allocations based on accumulated feedback. Categories with high success rates grow. Categories with low success rates shrink.

```rust
impl ContextPolicy {
    /// Called every N ticks. Adjusts allocations based on accumulated feedback.
    pub fn evolve(&mut self, max_delta: f64) {
        for (cat, dist) in &self.feedback {
            let posterior_mean = dist.mean(); // alpha / (alpha + beta)
            // Move allocation toward posterior mean, capped by max_delta
            if let Some(alloc) = self.allocations.get_mut(cat) {
                let delta = (posterior_mean - 0.5) * max_delta * 2.0;
                *alloc = (*alloc + delta).clamp(0.01, 0.5);
            }
        }
        self.normalize_allocations();
        self.revision += 1;
    }

    fn normalize_allocations(&mut self) {
        let total: f64 = self.allocations.values().sum();
        if total > 0.0 {
            for v in self.allocations.values_mut() {
                *v /= total;
            }
        }
    }
}
```

**Loop 3: Per-regime structural policy changes**

When the agent's behavioral phase shifts (thriving -> conservation, stable -> declining), the entire allocation structure changes. This is not a gradual evolution -- it's a phase transition.

```rust
impl ContextPolicy {
    /// Called when behavioral phase changes.
    pub fn apply_phase_transition(&mut self, new_phase: BehavioralPhase) {
        if let Some(overrides) = self.phase_overrides.get(&new_phase) {
            for (cat, alloc) in overrides {
                self.allocations.insert(*cat, *alloc);
            }
            self.normalize_allocations();
        }
    }
}
```

Example phase effects:
- **Thriving** -> more allocation to `Strategy` and `Hypotheses` (explore)
- **Conservation** -> more allocation to `Warnings` and `Risk` (protect)
- **Declining** -> more allocation to `Knowledge` (consolidate before death)

### Affect-modulated allocation

The Daimon extension's affect state (Pleasure-Arousal-Dominance) modulates context assembly in real time:

| Affect state | Effect on context |
|-------------|-------------------|
| High arousal | Boost `Warnings` and `Risk` sections. Agent is alert to threats. |
| Low arousal | Boost `Hypotheses` and `Strategy`. Agent is reflective. |
| High pleasure | Slight boost to sections that drove the positive outcome. |
| Low pleasure | Boost `AntiKnowledge` and contrarian perspectives. |
| Low dominance | Boost `Playbook` (seek proven approaches when uncertain). |

```rust
pub fn apply_affect_modulation(cortical: &CorticalState, ws: &mut CognitiveWorkspace) {
    let arousal = cortical.read_arousal(); // [-1.0, 1.0]
    let pleasure = cortical.read_pleasure();

    if arousal > 0.5 {
        ws.boost_category(ContextCategory::Warnings, 1.0 + arousal * 0.5);
    }
    if pleasure < -0.3 {
        ws.boost_category(ContextCategory::Knowledge, 1.3); // seek anti-knowledge
    }
    if arousal < -0.3 {
        ws.boost_category(ContextCategory::Hypotheses, 1.2); // reflective mode
    }
}
```

### 4-tier caching

Context assembly is expensive. These four cache layers reduce redundant work:

**L3: Deterministic cache (hash-based)**

If the exact same context fingerprint was assembled before and produced the same response, skip the LLM call entirely. Content-addressed by a Blake3 hash of the full prompt.

Hit rate: ~5-10% (identical situations are rare but do occur in routine monitoring).

**L2: Semantic cache**

If a very similar prompt (cosine similarity > 0.95 in embedding space) produced a response recently, consider reusing it (with confidence decay).

Hit rate: ~15-25% (many prompts are near-duplicates with minor variations).

**L1: Prefix cache (provider-level)**

Structure the workspace so cacheable sections (role, workspace, task) appear first. These are identical across consecutive turns, enabling provider-side KV-cache reuse.

Hit rate: ~40-60% of input tokens (measured by reduced time-to-first-token).

**Aggregate savings**: 60-80% token cost reduction from caching alone, on top of the 35x reduction from cognitive gating. Combined, a gated + cached agent costs roughly 2-5% of an ungated, uncached one.

---

## 6. Event fabric

The EventFabric is the inter-component communication bus. It enables reactive behavior -- agents respond to events from the environment, from other agents, and from their own subsystems.

### Architecture

```rust
pub struct EventFabric {
    /// Broadcast channel for real-time event delivery
    tx: broadcast::Sender<RuntimeEvent>,
    /// Ring buffer for historical replay (last N events)
    ring: Arc<RwLock<VecDeque<RuntimeEvent>>>,
    /// Ring capacity (default: 10,000 events)
    ring_capacity: usize,
    /// Monotonically increasing sequence number
    seq: AtomicU64,
}

impl EventFabric {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            ring: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            ring_capacity: capacity,
            seq: AtomicU64::new(0),
        }
    }

    /// Emit an event to all subscribers and store in the ring buffer.
    pub fn emit(&self, source: EventSource, payload: EventPayload) {
        let event = RuntimeEvent {
            seq: self.seq.fetch_add(1, Ordering::Relaxed),
            timestamp: Instant::now(),
            source,
            payload,
        };
        // Store in ring (for replay)
        {
            let mut ring = self.ring.write();
            if ring.len() >= self.ring_capacity {
                ring.pop_front();
            }
            ring.push_back(event.clone());
        }
        // Broadcast (best effort -- lagging receivers miss events)
        let _ = self.tx.send(event);
    }

    /// Subscribe to all events (unfiltered).
    pub fn subscribe(&self) -> broadcast::Receiver<RuntimeEvent> {
        self.tx.subscribe()
    }

    /// Subscribe with a filter. Only matching events are delivered.
    pub fn subscribe_filtered(&self, filters: &[EventFilter]) -> FilteredReceiver {
        FilteredReceiver {
            inner: self.tx.subscribe(),
            filters: filters.to_vec(),
        }
    }

    /// Replay events from a given sequence number (for late joiners).
    pub fn replay_from(&self, since_seq: u64) -> Vec<RuntimeEvent> {
        let ring = self.ring.read();
        ring.iter()
            .filter(|e| e.seq >= since_seq)
            .cloned()
            .collect()
    }
}
```

### Event types

```rust
#[derive(Clone, Debug)]
pub struct RuntimeEvent {
    /// Unique sequence number (monotonically increasing)
    pub seq: u64,
    /// When this event was emitted
    pub timestamp: Instant,
    /// Who/what produced this event
    pub source: EventSource,
    /// The event data
    pub payload: EventPayload,
}

#[derive(Clone, Debug)]
pub enum EventSource {
    /// A blockchain data source
    Chain { chain_id: u64 },
    /// A file system watcher
    FileSystem { path: PathBuf },
    /// Another agent
    Agent { agent_id: AgentId },
    /// A gate pipeline step
    Gate { rung: String },
    /// A timer (heartbeat, scheduler)
    Timer { name: String },
    /// An external system (webhook, API)
    External { source: String },
}

#[derive(Clone, Debug)]
pub enum EventPayload {
    // === Chain events ===
    NewBlock { number: u64, timestamp: u64, tx_count: u32 },
    Transaction { hash: H256, from: Address, to: Option<Address>, value: U256, data: Bytes },
    PriceFeed { pair: String, price: f64, source: String },
    MempoolTx { hash: H256, gas_price: u64 },

    // === File system events ===
    FileChanged { path: PathBuf, kind: FileChangeKind },
    TestResult { suite: String, passed: u32, failed: u32, duration_ms: u64 },

    // === Agent events ===
    AgentStarted { agent_id: AgentId, domain: String },
    AgentCompleted { agent_id: AgentId, outcome: TaskOutcome },
    AgentDreamStarted { agent_id: AgentId },
    AgentDreamEnded { agent_id: AgentId, insights: u32 },
    PheromoneSignal { source: AgentId, kind: String, intensity: f64, decay_rate: f64 },

    // === Gate events ===
    GateVerdict { rung: String, passed: bool, output: String },

    // === Timer events ===
    HeartbeatTick { frequency: Frequency, tick: u64 },

    // === Generic ===
    Custom { kind: String, data: serde_json::Value },
}
```

### Event-driven wakeup

Events can interrupt the dream state. If a blockchain agent is dreaming and a large liquidation event occurs, the agent wakes immediately:

```rust
impl Agent<Dreaming> {
    /// Check for urgent events that should interrupt the dream.
    async fn check_wake_condition(&self, event: &RuntimeEvent) -> Option<WakeReason> {
        match &event.payload {
            EventPayload::PriceFeed { pair, price, .. } => {
                let deviation = self.expected_price_deviation(pair, *price);
                if deviation > 3.0 { // 3-sigma event
                    Some(WakeReason::PriceSpike { pair: pair.clone(), deviation })
                } else {
                    None
                }
            }
            EventPayload::PheromoneSignal { kind, intensity, .. } => {
                if kind == "danger" && *intensity > 0.8 {
                    Some(WakeReason::DangerPheromone)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Interrupt the dream and return to active state.
    pub fn emergency_wake(self, reason: WakeReason) -> Agent<Active> {
        // Dream state is preserved (can resume later)
        // Active state starts with elevated arousal from the wake event
        Agent {
            inner: AgentInner {
                wake_reason: Some(reason),
                arousal_boost: 0.5, // elevated alertness
                ..self.inner
            },
            _phase: PhantomData,
        }
    }
}
```

### Subscription patterns

Agents subscribe to events based on their domain profile:

```rust
impl Agent<Active> {
    /// Set up event subscriptions during boot.
    async fn setup_subscriptions(&mut self, fabric: &EventFabric) {
        let filters: Vec<EventFilter> = self.profile.event_subscriptions.clone();
        let mut rx = fabric.subscribe_filtered(&filters);

        // Spawn a task that feeds events into the agent's event queue
        let event_queue = self.event_queue.clone();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let _ = event_queue.send(event).await;
            }
        });
    }
}
```

---

## 7. Type-state lifecycle

The type-state pattern uses Rust's type system to make illegal state transitions unrepresentable. You cannot compile code that ticks a dead agent, dreams an active agent without transitioning, or extracts a genome from a living agent.

### Phase markers

```rust
/// Zero-sized type markers. They exist only at compile time.
/// The runtime representation is identical regardless of phase.
pub struct Provisioning;
pub struct Active;
pub struct Dreaming;
pub struct Suspended;
pub struct Terminal;

/// The agent struct, parameterized by its lifecycle phase.
pub struct Agent<Phase> {
    inner: AgentInner,
    _phase: PhantomData<Phase>,
}

/// Shared inner state (same representation in all phases).
struct AgentInner {
    id: AgentId,
    profile: DomainProfile,
    extensions: ExtensionChain,
    cortical: Arc<CorticalState>,
    heartbeat: HeartbeatPipeline,
    event_queue: mpsc::Receiver<RuntimeEvent>,
    fabric: Arc<EventFabric>,
    state_snapshot: Option<serde_json::Value>,
}
```

### Valid transitions

```rust
// Provisioning: the agent is being initialized
impl Agent<Provisioning> {
    /// Create a new agent in provisioning state.
    pub fn new(profile: DomainProfile, fabric: Arc<EventFabric>) -> Self { ... }

    /// Activate the agent with a built extension chain.
    /// Consumes the provisioning agent; returns an active one.
    pub fn activate(self, extensions: ExtensionChain) -> Agent<Active> {
        Agent {
            inner: AgentInner { extensions, ..self.inner },
            _phase: PhantomData,
        }
    }
}

// Active: the agent is ticking and processing events
impl Agent<Active> {
    /// Run a single tick of the heartbeat pipeline.
    pub async fn tick(&mut self) -> Result<TickOutcome> {
        self.inner.heartbeat.execute_tick(
            &mut self.inner.extensions,
            &self.inner.cortical,
        ).await
    }

    /// Process an incoming event from the fabric.
    pub async fn on_event(&mut self, event: RuntimeEvent) -> Result<EventResponse> {
        self.inner.extensions.fire_event(&event, &mut EventContext::new()).await?;
        Ok(EventResponse::Handled)
    }

    /// Receive an operator message.
    pub async fn on_message(&mut self, msg: OperatorMessage) -> Result<AgentResponse> {
        self.inner.extensions.fire_message(&msg, &mut MessageContext::new()).await?;
        Ok(AgentResponse::Acknowledged)
    }

    /// Inject a task (backwards compat with plan-based dispatch).
    /// This becomes a forced-T2 stimulus.
    pub async fn on_task(&mut self, task: TaskEnvelope) -> Result<TaskOutcome> {
        // Force T2 tier for the next tick
        self.inner.heartbeat.force_tier(CognitiveTier::T2);
        // Tick processes the task
        self.tick().await?;
        Ok(TaskOutcome::from_tick(self.inner.last_outcome()))
    }

    /// Transition to dreaming state (sleep pressure exceeded threshold).
    pub fn begin_dream(self) -> Agent<Dreaming> {
        Agent { inner: self.inner, _phase: PhantomData }
    }

    /// Suspend between tasks (for task-mode agents).
    pub fn suspend(self) -> Agent<Suspended> {
        Agent { inner: self.inner, _phase: PhantomData }
    }

    /// Begin shutdown (voluntary or forced).
    pub fn begin_shutdown(self) -> Agent<Terminal> {
        Agent { inner: self.inner, _phase: PhantomData }
    }
}

// Dreaming: the agent is running offline consolidation
impl Agent<Dreaming> {
    /// Run one dream cycle (NREM replay -> REM imagination -> integration).
    pub async fn dream_cycle(&mut self) -> Result<DreamOutcome> {
        self.inner.extensions.fire_dream_start(&mut DreamContext::new()).await?;

        for phase in [DreamPhase::NremReplay, DreamPhase::RemImagination, DreamPhase::Integration] {
            self.inner.extensions.fire_dream_phase(phase, &mut DreamContext::new()).await?;
        }

        let outcome = DreamOutcome::collect(&self.inner.extensions);
        self.inner.extensions.fire_dream_end(&outcome).await?;
        Ok(outcome)
    }

    /// Normal wake: dream completed naturally.
    pub fn wake(self) -> Agent<Active> {
        Agent { inner: self.inner, _phase: PhantomData }
    }

    /// Emergency wake: urgent event interrupted the dream.
    pub fn emergency_wake(self, reason: WakeReason) -> Agent<Active> {
        Agent { inner: self.inner, _phase: PhantomData }
    }
}

// Suspended: paused between tasks, waiting for assignment
impl Agent<Suspended> {
    /// Resume with a new task.
    pub fn resume(self, task: TaskEnvelope) -> Agent<Active> {
        Agent { inner: self.inner, _phase: PhantomData }
    }

    /// Shut down without resuming.
    pub fn begin_shutdown(self) -> Agent<Terminal> {
        Agent { inner: self.inner, _phase: PhantomData }
    }
}

// Terminal: the agent is shutting down
impl Agent<Terminal> {
    /// Execute the 10-phase shutdown protocol and extract genome.
    pub async fn shutdown(mut self) -> GenomeExtract {
        // Phase 1: Notify extensions
        self.inner.extensions.fire_shutdown(&ShutdownContext::new()).await.ok();
        // Phase 2: Flush pending writes
        // Phase 3: Save state snapshot
        // Phase 4: Compress knowledge (genomic bottleneck, <=2048 entries)
        // Phase 5: Record final episode
        // Phase 6: Emit terminal event
        // Phase 7: Close connections
        // Phase 8: Persist genome
        // Phase 9: Deregister from discovery
        // Phase 10: Drop self

        let genome = self.extract_genome().await;
        self.inner.fabric.emit(
            EventSource::Agent { agent_id: self.inner.id },
            EventPayload::AgentCompleted {
                agent_id: self.inner.id,
                outcome: TaskOutcome::Shutdown,
            },
        );
        genome
    }

    async fn extract_genome(&self) -> GenomeExtract {
        // Compress the knowledge store through a genomic bottleneck
        // Max 2048 entries, weighted by confidence and utility
        GenomeExtract {
            agent_id: self.inner.id,
            knowledge: self.inner.compress_knowledge(2048),
            somatic_markers: self.inner.export_somatic_markers(),
            context_policy: self.inner.export_context_policy(),
            total_ticks: self.inner.heartbeat.tick_count,
            total_cost: self.inner.heartbeat.total_cost(),
        }
    }
}
```

### Why this matters

Consider what happens without type-state enforcement:

```rust
// WITHOUT type-state: runtime panic at best, silent corruption at worst
agent.shutdown();
agent.tick(); // BUG: ticking a dead agent. Might panic, might corrupt state.
```

With type-state enforcement:

```rust
// WITH type-state: this code does not compile
let terminal = agent.begin_shutdown();
// terminal.tick(); // ERROR: no method `tick` found for `Agent<Terminal>`
let genome = terminal.shutdown().await;
// terminal.tick(); // ERROR: `terminal` was moved into shutdown()
```

The compiler catches the bug. Not at test time. Not in production. At compile time. The invalid program cannot exist.

### Valid transition diagram

```
                    ┌─────────────┐
                    │ Provisioning │
                    └──────┬──────┘
                           │ activate()
                           v
          resume()  ┌─────────────┐  begin_dream()
       ┌──────────> │   Active    │ ──────────────┐
       │            └──┬───┬───┬──┘               │
       │               │   │   │                  v
       │    suspend()  │   │   │          ┌─────────────┐
       │               v   │   │          │  Dreaming   │
       │     ┌───────────┐ │   │          └──┬───┬──────┘
       └─────│ Suspended │ │   │             │   │
             └─────┬─────┘ │   │   wake()    │   │ emergency_wake()
                   │       │   │  ┌──────────┘   │
                   │       │   │  │  ┌───────────┘
                   │       │   │  │  │
                   │  begin_shutdown()
                   │       │      │  │
                   v       v      v  v
              ┌─────────────────────────┐
              │       Terminal          │
              └────────────┬───────────┘
                           │ shutdown()
                           v
                    ┌─────────────┐
                    │   (dropped) │
                    └─────────────┘
```

---

## 8. CorticalState

The CorticalState is the agent's shared perception surface -- a collection of atomic values that extensions write and read without locks, without contention, without allocations.

### Why lock-free?

Extensions fire concurrently across hooks. The DaimonExt writes affect at the end of every tick. The ContextExt reads affect during context assembly. The GateExt reads prediction error during gating. If these operations required mutex locks, extensions would block each other. With atomics, reads and writes are independent.

### Structure

```rust
/// Lock-free atomic perception surface.
/// Each field occupies its own cache line to prevent false sharing.
#[repr(C, align(64))]
pub struct CorticalState {
    // === Affect (written by DaimonExt) ===

    /// Pleasure-Arousal-Dominance affect model.
    /// Fixed-point representation: actual_value * 1000.
    /// Range: [-1000, 1000] mapping to [-1.0, 1.0].
    pub pleasure: AtomicI32,
    pub arousal: AtomicI32,
    pub dominance: AtomicI32,

    /// Current behavioral phase (maps to BehavioralPhase enum).
    pub behavioral_phase: AtomicU8,

    _pad0: [u8; 64 - 13], // Cache line padding

    // === Vitality (written by MortalityExt or BudgetExt) ===

    /// Economic vitality: budget remaining / initial budget.
    /// Fixed-point: actual_value * 10000. Range: [0, 10000].
    pub economic_vitality: AtomicU16,

    /// Epistemic confidence: knowledge freshness / validation rate.
    /// Fixed-point: actual_value * 10000.
    pub epistemic_confidence: AtomicU16,

    /// Composite vitality: weighted combination of all vitality signals.
    pub composite_vitality: AtomicU16,

    _pad1: [u8; 64 - 6],

    // === Perception (written by HeartbeatExt + ObserverExt) ===

    /// Total tick count since boot.
    pub tick_count: AtomicU64,

    /// Timestamp of last observation (ms since boot).
    pub last_observation_ms: AtomicU64,

    /// Last computed prediction error.
    /// Fixed-point: actual_value * 10000. Range: [0, 10000].
    pub prediction_error: AtomicU32,

    /// Current cognitive tier (0=T0, 1=T1, 2=T2).
    pub cognitive_tier: AtomicU8,

    _pad2: [u8; 64 - 21],

    // === Attention (written by AttentionExt) ===

    /// Hash of the top-priority stimulus in the salience heap.
    pub attention_top_hash: AtomicU64,

    /// Current novelty score (how novel is the most salient item).
    pub novelty_score: AtomicU16,

    _pad3: [u8; 64 - 10],

    // === Communication (written by PheromoneExt) ===

    /// Encoded pheromone signal (packed: type(8) | intensity(8) | source_hash(48)).
    pub pheromone_signal: AtomicU64,

    _pad4: [u8; 64 - 8],
}
```

### Read/write patterns

```rust
impl CorticalState {
    // Typed read accessors (convert from fixed-point to f64)

    pub fn read_arousal(&self) -> f64 {
        self.arousal.load(Ordering::Relaxed) as f64 / 1000.0
    }

    pub fn read_pleasure(&self) -> f64 {
        self.pleasure.load(Ordering::Relaxed) as f64 / 1000.0
    }

    pub fn read_prediction_error(&self) -> f64 {
        self.prediction_error.load(Ordering::Relaxed) as f64 / 10000.0
    }

    pub fn read_economic_vitality(&self) -> f64 {
        self.economic_vitality.load(Ordering::Relaxed) as f64 / 10000.0
    }

    // Typed write accessors (convert from f64 to fixed-point)

    pub fn write_arousal(&self, value: f64) {
        let fixed = (value.clamp(-1.0, 1.0) * 1000.0) as i32;
        self.arousal.store(fixed, Ordering::Relaxed);
    }

    pub fn write_prediction_error(&self, value: f64) {
        let fixed = (value.clamp(0.0, 1.0) * 10000.0) as u32;
        self.prediction_error.store(fixed, Ordering::Relaxed);
    }
}
```

### Cache-line alignment

Each field group is padded to 64 bytes (one cache line on x86-64 and ARM). Without this, writing `arousal` on one core would invalidate the cache line containing `tick_count` on another core -- even though they are logically independent. This is false sharing, and it degrades performance under concurrent access.

The `#[repr(C, align(64))]` attribute plus explicit padding bytes guarantee each signal group lives on its own cache line.

---

## 9. Inference gateway

The inference gateway sits between the heartbeat pipeline and the LLM backends. It handles caching (to avoid redundant calls), routing (to pick the right model), and translation (to convert between Roko's internal format and each backend's API).

### Three-layer caching stack

```rust
pub struct InferenceGateway {
    /// L3: Exact-match cache (Blake3 hash of full prompt)
    l3_cache: ResponseCache,
    /// L2: Semantic cache (embedding similarity)
    l2_cache: SemanticCache,
    /// L1: Prefix alignment for provider KV-cache reuse
    l1_strategy: PrefixStrategy,
    /// Model router (picks backend based on intent)
    router: IntentRouter,
    /// Backend registry (Claude, OpenAI, Gemini, Ollama, etc.)
    backends: HashMap<String, Arc<dyn LlmBackend>>,
}

impl InferenceGateway {
    pub async fn infer(&self, workspace: &CognitiveWorkspace) -> Result<InferenceResult> {
        let prompt = workspace.render_prompt();

        // L3: Check exact hash
        let hash = blake3::hash(prompt.as_bytes());
        if let Some(cached) = self.l3_cache.get(&hash) {
            return Ok(InferenceResult::cached(cached));
        }

        // L2: Check semantic similarity
        if let Some(similar) = self.l2_cache.find_similar(&prompt, threshold: 0.95) {
            return Ok(InferenceResult::semantic_hit(similar));
        }

        // L1: Reorder sections for prefix alignment
        let optimized = self.l1_strategy.optimize_prefix(&prompt);

        // Route to backend
        let backend = self.router.select(&workspace)?;
        let response = backend.send_turn(&optimized.messages, &optimized.tools, &SessionState::default()).await?;

        // Store in caches
        self.l3_cache.insert(hash, response.clone());
        self.l2_cache.insert(&prompt, response.clone());

        Ok(InferenceResult::fresh(response))
    }
}
```

### Intent-based provider routing

Instead of hardcoding "use Claude for everything," the gateway routes based on declarative intent. The domain profile declares what the agent needs, and the router picks the first backend that satisfies those needs.

```rust
pub struct InferenceIntent {
    /// Minimum reasoning capability needed (1-5)
    pub reasoning_depth: u8,
    /// Maximum acceptable latency (for time-sensitive decisions)
    pub max_latency: Option<Duration>,
    /// Maximum acceptable cost per call
    pub max_cost: Option<f64>,
    /// Required capabilities (tool calling, vision, code, etc.)
    pub required_capabilities: Vec<Capability>,
    /// Preferred backend (soft preference, not hard requirement)
    pub preferred_backend: Option<String>,
}

pub struct IntentRouter {
    /// Ordered list of routing rules (first match wins)
    rules: Vec<RoutingRule>,
}

pub struct RoutingRule {
    /// When this rule matches
    pub condition: RoutingCondition,
    /// Which backend to use
    pub backend: String,
    /// Model within that backend
    pub model: String,
}

pub enum RoutingCondition {
    /// Always matches (catch-all)
    Always,
    /// Match on cognitive tier
    Tier(CognitiveTier),
    /// Match on required capability
    HasCapability(Capability),
    /// Match on cost constraint
    MaxCost(f64),
    /// Match on latency constraint
    MaxLatency(Duration),
}
```

Example routing configuration:

```rust
let router = IntentRouter::new(vec![
    // T1 ticks use Haiku (fast, cheap)
    RoutingRule {
        condition: RoutingCondition::Tier(CognitiveTier::T1),
        backend: "anthropic".into(),
        model: "claude-haiku".into(),
    },
    // T2 with vision requirement uses Opus
    RoutingRule {
        condition: RoutingCondition::HasCapability(Capability::Vision),
        backend: "anthropic".into(),
        model: "claude-opus".into(),
    },
    // T2 with tight latency uses Gemini Flash
    RoutingRule {
        condition: RoutingCondition::MaxLatency(Duration::from_secs(3)),
        backend: "google".into(),
        model: "gemini-flash".into(),
    },
    // Default: Claude Sonnet (good balance of cost/capability)
    RoutingRule {
        condition: RoutingCondition::Always,
        backend: "anthropic".into(),
        model: "claude-sonnet".into(),
    },
]);
```

### Mortality pressure on routing

When an agent's economic vitality drops, the router automatically becomes more cost-sensitive:

```rust
impl IntentRouter {
    pub fn select_with_vitality(
        &self,
        intent: &InferenceIntent,
        vitality: f64,
    ) -> &RoutingRule {
        let mut adjusted_intent = intent.clone();

        // Dying agents use cheaper models
        if vitality < 0.3 {
            adjusted_intent.max_cost = Some(0.005); // force Haiku-tier
        } else if vitality < 0.5 {
            adjusted_intent.max_cost = Some(0.02); // force Sonnet-tier
        }

        self.first_match(&adjusted_intent)
    }
}
```

### The Translator pattern

Each LLM backend has its own tool format, message structure, and response schema. The `Translator` trait converts between Roko's internal representation and the backend's API.

```rust
pub trait Translator: Send + Sync {
    /// Convert Roko's ToolDef list into the backend's tool specification format.
    fn render_tools(&self, tools: &[ToolDef]) -> RenderedTools;

    /// Convert a Roko message history into the backend's message format.
    fn render_messages(&self, messages: &[Message]) -> Vec<serde_json::Value>;

    /// Parse the backend's response into Roko's internal format.
    fn parse_response(&self, raw: &BackendResponse) -> ParsedResponse;

    /// Extract tool calls from the parsed response.
    fn extract_tool_calls(&self, response: &ParsedResponse) -> Vec<ToolCall>;
}
```

This pattern means adding a new backend (say, a local model via GGUF) requires implementing one trait with four methods. The rest of the system -- the heartbeat, the extensions, the caching, the routing -- works unchanged.

---

## 10. The native tool loop

Roko has two modes of tool execution:

1. **CLI-driven** (Claude CLI, Codex CLI): The external process drives its own tool loop. Roko spawns the process, passes a system prompt, and waits for completion. This is how current `plan run` works.

2. **Native** (Ollama, OpenAI-compat, Gemini, Anthropic API): Roko drives the tool loop itself. It sends messages to the LLM, parses tool calls from responses, executes them via the ToolDispatcher, and feeds results back.

The native tool loop integrates with the heartbeat at the EXECUTE step.

### ToolDispatcher pipeline

Every tool call (whether from CLI or native backends) passes through this pipeline:

```rust
impl ToolDispatcher {
    pub async fn dispatch(&self, call: ToolCall, ctx: ToolContext) -> ToolResult {
        // 1. VALIDATE: Check args against the tool's JSON schema
        if let Err(e) = validate(&call, &self.registry) {
            return ToolResult::error(format!("validation failed: {e}"));
        }

        // 2. AUTHORIZE: Check permissions against role
        let tool_def = self.registry.get(&call.name)
            .ok_or_else(|| ToolError::NotFound(call.name.clone()))?;

        if let Some(safety) = &self.safety {
            if let Err(violation) = safety.authorize(&call, &ctx) {
                return ToolResult::error(format!("denied: {violation}"));
            }
        }

        // 3. RESOLVE: Find the handler for this tool
        let handler = self.resolver.resolve(&call.name)
            .ok_or_else(|| ToolError::NoHandler(call.name.clone()))?;

        // 4. HOOKS: Run pre-dispatch safety hook chain
        if let Some(chain) = &self.hook_chain {
            if let Err(rejection) = chain.pre_dispatch(&call, &ctx).await {
                return ToolResult::error(format!("hook rejected: {rejection}"));
            }
        }

        // 5. RACE: Execute with timeout + cancellation
        let result = with_timeout(
            ctx.timeout.unwrap_or(Duration::from_secs(30)),
            handler.execute(call.clone(), ctx.clone()),
            wait_cancelled(ctx.cancel_token()),
        ).await;

        // 6. TRUNCATE: Cap oversized results
        let result = match result {
            Ok(r) => truncate_result(r, self.max_result_bytes),
            Err(e) => ToolResult::error(e.to_string()),
        };

        // 7. POST-HOOKS: Run post-dispatch hooks
        if let Some(chain) = &self.hook_chain {
            chain.post_dispatch(&call, &result, &ctx).await;
        }

        result
    }
}
```

### How it integrates with the heartbeat

During the EXECUTE step of a T1/T2 tick, tool calls from the LLM response are dispatched through extensions first:

```rust
impl HeartbeatPipeline {
    async fn execute_tool_calls(
        &self,
        inf_ctx: &InferenceContext,
        extensions: &mut ExtensionChain,
    ) -> Result<Vec<ToolCallResult>> {
        let calls = inf_ctx.tool_calls();
        let mut results = Vec::with_capacity(calls.len());

        for mut call in calls {
            // Extension hook: before_tool_call (can deny, modify, or allow)
            let decision = extensions.fire_before_tool_call(&mut call).await?;
            match decision {
                ToolDecision::Allow => {
                    let result = self.dispatcher.dispatch(call.clone(), self.tool_context()).await;
                    extensions.fire_after_tool_call(&call, &result).await?;
                    results.push(ToolCallResult { call, result });
                }
                ToolDecision::Deny(reason) => {
                    results.push(ToolCallResult {
                        call: call.clone(),
                        result: ToolResult::error(format!("denied by extension: {reason}")),
                    });
                }
                ToolDecision::Modify(modified_call) => {
                    let result = self.dispatcher.dispatch(modified_call.clone(), self.tool_context()).await;
                    extensions.fire_after_tool_call(&modified_call, &result).await?;
                    results.push(ToolCallResult { call: modified_call, result });
                }
            }
        }

        Ok(results)
    }
}
```

### Capability tokens

Tool capabilities are represented as unforgeable, single-use tokens. An agent cannot call `send_tx` unless it holds a `Capability::Transaction` token. Tokens are granted at boot time based on the domain profile and can be revoked by safety extensions.

```rust
/// An unforgeable capability token. Cannot be constructed outside the safety module.
/// Consumed on use (single-use by default; renewable tokens re-issue on success).
pub struct CapabilityToken {
    /// What this token authorizes
    capability: Capability,
    /// Who issued it (for audit)
    issuer: AgentId,
    /// When it was issued
    issued_at: Instant,
    /// When it expires (None = valid until used)
    expires_at: Option<Instant>,
    /// How many uses remain (decremented on dispatch)
    remaining_uses: AtomicU32,
    /// Cryptographic nonce (prevents replay)
    nonce: [u8; 32],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Read files from the workspace
    FileRead,
    /// Write files to the workspace
    FileWrite,
    /// Execute shell commands
    ShellExec,
    /// Make network requests
    NetworkOutbound,
    /// Send blockchain transactions
    Transaction,
    /// Modify git state (commit, branch, push)
    GitWrite,
    /// Access the knowledge store
    KnowledgeRead,
    /// Modify the knowledge store
    KnowledgeWrite,
    /// Spawn child agents
    AgentSpawn,
    /// Custom capability (for user extensions)
    Custom(u16),
}

impl CapabilityToken {
    /// Attempt to use this token. Returns Ok(()) if valid, Err if expired/exhausted.
    /// Decrements remaining_uses atomically.
    pub fn try_use(&self) -> Result<(), CapabilityError> {
        if let Some(expires) = self.expires_at {
            if Instant::now() > expires {
                return Err(CapabilityError::Expired);
            }
        }
        let prev = self.remaining_uses.fetch_sub(1, Ordering::AcqRel);
        if prev == 0 {
            return Err(CapabilityError::Exhausted);
        }
        Ok(())
    }
}
```

The safety extension validates tokens before tool execution:

```rust
impl Extension for SafetyExt {
    async fn before_tool_call(&mut self, call: &mut ToolCall) -> Result<ToolDecision> {
        let required = capability_for_tool(&call.name);
        match self.token_store.find_valid(required) {
            Some(token) => {
                token.try_use()?;
                Ok(ToolDecision::Allow)
            }
            None => Ok(ToolDecision::Deny(format!(
                "no valid {:?} capability token", required
            ))),
        }
    }
}
```

---

## Appendix A: Crate layout

The target crate layout for the fully realized runtime:

```
crates/
  roko-runtime/          # Core runtime (rewritten)
    src/
      lib.rs             # Public API
      runtime.rs         # AgentRuntime trait + Agent<Phase>
      heartbeat.rs       # HeartbeatPipeline (9-step tick)
      lifecycle.rs       # Type-state transitions, shutdown protocol
      event_fabric.rs    # EventFabric, RuntimeEvent, EventFilter
      cortical.rs        # CorticalState (lock-free atomics)
      arena.rs           # TickArena (bumpalo per-tick allocation)
      cognitive.rs       # CognitiveWorkspace, ContextPolicy
      extension.rs       # Extension trait, ExtensionChain, builder
      profile.rs         # DomainProfile definitions
      gateway.rs         # InferenceGateway, routing, caching

  roko-ext-core/         # Required extensions (all domains)
    src/
      heartbeat.rs       # HeartbeatExt (L0)
      context.rs         # ContextExt (L0)
      daimon.rs          # DaimonExt (L3)
      learning.rs        # LearningExt (L6)
      dreams.rs          # DreamsExt (L6)

  roko-ext-code/         # Coding domain extensions
    src/
      git.rs             # GitExt (L4)
      gate.rs            # GateExt (L4)
      conductor.rs       # ConductorExt (L3)

  roko-ext-chain/        # Blockchain domain extensions
    src/
      subscriber.rs      # ChainSubscriberExt (L1)
      risk.rs            # RiskExt (L3)
      mortality.rs       # MortalityExt (L2)
      pheromone.rs       # PheromoneExt (L5)

  roko-ext-research/     # Research domain extensions
    src/
      knowledge_graph.rs # KnowledgeGraphExt (L2)
      source_watcher.rs  # SourceWatcherExt (L1)
      synthesis.rs       # SynthesisExt (L6)

  roko-agent/            # LLM backends (kept, adapted)
  roko-core/             # Signal + 6 traits (kept)
  roko-compose/          # Prompt templates (used by ContextExt)
  roko-gate/             # Gate implementations (used by GateExt)
  roko-learn/            # Learning primitives (used by LearningExt)
  roko-neuro/            # Knowledge store (used by ContextExt/DreamsExt)
  roko-daimon/           # Affect engine (used by DaimonExt)
  roko-dreams/           # Dream cycle (used by DreamsExt)
  roko-conductor/        # Watchers (used by ConductorExt)
  roko-chain/            # Chain client (used by ChainSubscriberExt)
  roko-cli/              # CLI binary (thin coordinator)
```

## Appendix B: Domain profile examples

### Coding agent

```rust
DomainProfile {
    name: "coding",
    gamma_interval: Duration::from_secs(30),    // check files every 30s
    theta_interval: Duration::from_secs(120),   // full decision every 2 min
    delta_interval: Duration::from_secs(6000),  // dream every ~50 thetas
    base_gate_threshold: 0.4,                   // high: most ticks are idle
    extensions: vec![
        "heartbeat", "context", "neuro", "daimon", "conductor",
        "git", "gate", "tools", "safety", "learning", "dreams",
    ],
    event_subscriptions: vec![
        EventFilter::FileChange,
        EventFilter::TestResult,
        EventFilter::GateVerdict,
    ],
    context_categories: vec![
        ContextCategory::Task,
        ContextCategory::Code,
        ContextCategory::Knowledge,
        ContextCategory::Playbook,
    ],
    default_gates: vec!["compile", "test", "clippy"],
    uses_git: true,
    uses_worktrees: true,
}
```

### Blockchain agent

```rust
DomainProfile {
    name: "blockchain",
    gamma_interval: Duration::from_secs(5),     // fast: blocks come every 12s
    theta_interval: Duration::from_secs(30),    // decide every 30s
    delta_interval: Duration::from_secs(1500),  // dream every ~50 thetas
    base_gate_threshold: 0.2,                   // low: be reactive
    extensions: vec![
        "heartbeat", "context", "neuro", "daimon",
        "chain-subscriber", "risk", "mortality",
        "tools", "safety", "learning", "dreams", "pheromones",
    ],
    event_subscriptions: vec![
        EventFilter::NewBlock,
        EventFilter::MempoolTx,
        EventFilter::PriceFeed,
        EventFilter::Pheromone,
    ],
    context_categories: vec![
        ContextCategory::Strategy,
        ContextCategory::MarketState,
        ContextCategory::Knowledge,
        ContextCategory::Warnings,
    ],
    default_gates: vec!["simulation", "invariant-check", "risk-limit"],
    uses_git: false,
    uses_worktrees: false,
}
```

### Research agent

```rust
DomainProfile {
    name: "research",
    gamma_interval: Duration::from_secs(60),     // slow: research is deliberate
    theta_interval: Duration::from_secs(300),    // 5 min between decisions
    delta_interval: Duration::from_secs(15000),  // dream every ~50 thetas
    base_gate_threshold: 0.35,
    extensions: vec![
        "heartbeat", "context", "neuro", "daimon",
        "source-watcher", "knowledge-graph",
        "tools", "learning", "dreams", "synthesis",
    ],
    event_subscriptions: vec![
        EventFilter::NewPublication,
        EventFilter::DataUpdate,
        EventFilter::KnowledgeChange,
    ],
    context_categories: vec![
        ContextCategory::Sources,
        ContextCategory::Knowledge,
        ContextCategory::Hypotheses,
        ContextCategory::Task,
    ],
    default_gates: vec!["citation-check", "factual-consistency", "quality"],
    uses_git: false,
    uses_worktrees: false,
}
```

## Appendix C: Performance budget

All operations within the heartbeat are bounded:

| Operation | Budget | Actual (measured) |
|-----------|--------|-------------------|
| CorticalState read/write | <1 us | ~10 ns (atomic load/store) |
| HDC fingerprint (10,240 bits) | <10 us | ~5 us |
| HDC similarity (XOR + popcount) | <1 us | ~200 ns |
| Somatic marker lookup (k-d tree, 1K entries) | <200 us | ~80 us |
| Extension hook dispatch (per extension) | <1 us | ~100 ns (vtable call) |
| EventFabric emit | <5 us | ~2 us (broadcast + ring write) |
| VCG auction (8 bidders, 50 sections) | <2 ms | ~800 us |
| Context policy feedback update | <1 us | ~200 ns |
| Prediction error computation (6 sources) | <100 us | ~20 us |

The entire T0 tick (observe -> analyze -> gate -> suppress) completes in under 1 ms. Expensive operations (LLM calls, tool execution, network I/O) happen only in T1/T2 ticks -- and those are bounded by the LLM response time (seconds), not by the runtime overhead (microseconds).

For context: a single LLM API call takes 5-60 seconds. The runtime's overhead per tick is approximately 0.001% of the total tick duration for T1/T2 ticks.

---

*This document specifies the architecture. Implementation details (migration path, current state audit, concrete phases) are covered in a separate implementation document.*
