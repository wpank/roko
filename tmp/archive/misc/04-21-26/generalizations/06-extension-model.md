# Extension Model & Third-Party Extensibility

## Core Principle

Extensions are **the unit of composition** in roko. Everything that was previously hardcoded
in orchestrate.rs becomes an extension. Users add behavior by writing extensions (Rust traits),
not by modifying the core runtime.

## Extension Trait (Complete)

```rust
#[async_trait]
pub trait Extension: Send + Sync + 'static {
    // === Identity ===
    fn name(&self) -> &str;
    fn layer(&self) -> ExtensionLayer;
    fn depends_on(&self) -> &[&str] { &[] }

    // === Lifecycle (4 hooks) ===
    async fn on_boot(&mut self, ctx: &mut BootContext) -> Result<()> { Ok(()) }
    async fn on_resume(&mut self, ctx: &mut ResumeContext) -> Result<()> { Ok(()) }
    async fn on_suspend(&mut self, ctx: &mut SuspendContext) -> Result<()> { Ok(()) }
    async fn on_shutdown(&mut self, ctx: &ShutdownContext) -> Result<ShutdownVote> {
        Ok(ShutdownVote::Approve)
    }

    // === Heartbeat (3 hooks) ===
    async fn on_tick_start(&mut self, ctx: &mut TickContext) -> Result<()> { Ok(()) }
    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> { Ok(()) }
    async fn on_tick_end(&mut self, ctx: &mut TickContext) -> Result<()> { Ok(()) }

    // === Cognition (4 hooks) ===
    async fn on_gate(&mut self, ctx: &mut GateContext) -> Result<Option<CognitiveTier>> { Ok(None) }
    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> { Ok(()) }
    async fn on_before_inference(&mut self, ctx: &mut InferenceContext) -> Result<()> { Ok(()) }
    async fn on_after_inference(&mut self, ctx: &mut InferenceContext) -> Result<()> { Ok(()) }

    // === Action (2 hooks) ===
    async fn before_tool_call(&mut self, call: &mut ToolCall) -> Result<ToolDecision> {
        Ok(ToolDecision::Allow)
    }
    async fn after_tool_call(&mut self, call: &ToolCall, result: &ToolResult) -> Result<()> {
        Ok(())
    }

    // === Learning (2 hooks) ===
    async fn on_outcome(&mut self, ctx: &mut OutcomeContext) -> Result<()> { Ok(()) }
    async fn on_reflect(&mut self, record: &DecisionCycleRecord) -> Result<()> { Ok(()) }

    // === Events (2 hooks) ===
    async fn on_event(&mut self, event: &RuntimeEvent, ctx: &mut EventContext) -> Result<()> { Ok(()) }
    async fn on_message(&mut self, msg: &OperatorMessage, ctx: &mut MessageContext) -> Result<()> { Ok(()) }

    // === Dreams (3 hooks) ===
    async fn on_dream_start(&mut self, ctx: &mut DreamContext) -> Result<()> { Ok(()) }
    async fn on_dream_phase(&mut self, phase: DreamPhase, ctx: &mut DreamContext) -> Result<()> { Ok(()) }
    async fn on_dream_end(&mut self, outcome: &DreamOutcome) -> Result<()> { Ok(()) }

    // === State Persistence (2 hooks) ===
    async fn save_state(&self) -> Result<serde_json::Value> { Ok(serde_json::Value::Null) }
    async fn load_state(&mut self, state: serde_json::Value) -> Result<()> { Ok(()) }
}
```

Total: **22 hooks** across 7 categories.

## Extension Registration

### Static (Compile-Time)

```rust
// In your agent binary or library:
pub fn blockchain_extensions(config: &Config) -> Result<ExtensionChain> {
    ExtensionChain::builder()
        .add(HeartbeatExt::new(config.heartbeat))
        .add(ChainSubscriberExt::new(config.chain))
        .add(RiskExt::new(config.risk))
        .add(DaimonExt::new(config.daimon))
        .add(LearningExt::new(config.learning))
        .add(DreamsExt::new(config.dreams))
        .build()
}
```

### Config-Driven

```toml
# roko.toml
[agent.extensions]
# Core (always loaded)
heartbeat = { gamma_secs = 5, theta_secs = 30 }
context = { policy = "blockchain" }
daimon = { config = "blockchain" }
learning = {}
dreams = { min_episodes = 50 }

# Domain-specific
chain-subscriber = { rpc_url = "wss://...", chains = ["ethereum", "base"] }
risk = { max_position_pct = 0.05, kelly_fraction = 0.25 }
mortality = { budget_usdc = 1000.0 }

# Custom (user-provided, loaded from crate path)
[agent.extensions.custom]
my-strategy = { crate = "my-strategy-ext", config = { threshold = 0.5 } }
```

### Dynamic Discovery (Future)

```
.roko/extensions/
  my-strategy/
    Cargo.toml      # declares Extension impl
    src/lib.rs      # implements Extension trait
```

The runtime compiles and loads extensions from `.roko/extensions/` on boot.
(Phase 2 — requires dynamic linking or WASM compilation.)

## How Extensions Compose

### Data Flow Between Extensions

Extensions communicate through:

1. **CorticalState** (atomic, lock-free) — for real-time signals
2. **CognitiveWorkspace** (per-tick) — for context contribution
3. **EventFabric** (broadcast) — for async event notification
4. **Shared state** (via BootContext) — for initialization-time wiring

```rust
// DaimonExt writes affect to CorticalState
impl Extension for DaimonExt {
    async fn on_tick_end(&mut self, ctx: &mut TickContext) -> Result<()> {
        let pad = self.compute_pad(ctx.outcome());
        ctx.cortical().write_pad(&pad);
        Ok(())
    }
}

// ContextExt reads affect from CorticalState for affect-modulated allocation
impl Extension for ContextExt {
    async fn assemble_context(&mut self, ws: &mut CognitiveWorkspace) -> Result<()> {
        let pad = ws.cortical().read_pad();
        self.apply_affect_modulation(&pad, &mut ws.allocations);
        // ... assemble sections based on allocations
        Ok(())
    }
}

// DreamsExt reads from both
impl Extension for DreamsExt {
    async fn on_tick_end(&mut self, ctx: &mut TickContext) -> Result<()> {
        self.sleep_pressure += self.pressure_rate;
        if self.sleep_pressure > self.threshold {
            ctx.request_dream(); // will transition to Dreaming state
        }
        Ok(())
    }
}
```

### Dependency Ordering

Extensions declare dependencies; the chain validates no cycles and computes
topological firing order:

```rust
impl Extension for RiskExt {
    fn depends_on(&self) -> &[&str] { &["chain-subscriber", "daimon"] }
    // ...
}
```

If `chain-subscriber` hasn't fired `on_observe` yet, `RiskExt` won't see
the latest data. Layer ordering prevents this: L1 (Perception) always fires
before L3 (Cognition).

## Third-Party Extension Examples

### Example: Social Media Monitor

```rust
pub struct SocialMonitorExt {
    client: TwitterClient,
    keywords: Vec<String>,
    last_check: Instant,
    pending: VecDeque<Tweet>,
}

impl Extension for SocialMonitorExt {
    fn name(&self) -> &str { "social-monitor" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Perception }

    async fn on_observe(&mut self, ctx: &mut ObserveContext) -> Result<()> {
        if self.last_check.elapsed() > Duration::from_secs(60) {
            let tweets = self.client.search(&self.keywords, since: self.last_check).await?;
            for tweet in tweets {
                ctx.add_observation(Observation::Custom {
                    kind: "social_mention".into(),
                    data: serde_json::to_value(&tweet)?,
                    prediction_error_contribution: if tweet.engagement > 1000 { 0.3 } else { 0.05 },
                });
            }
            self.last_check = Instant::now();
        }
        Ok(())
    }
}
```

### Example: Notification Extension

```rust
pub struct SlackNotifyExt {
    webhook_url: String,
    notify_on: Vec<NotifyTrigger>,
}

impl Extension for SlackNotifyExt {
    fn name(&self) -> &str { "slack-notify" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Social }

    async fn on_outcome(&mut self, ctx: &mut OutcomeContext) -> Result<()> {
        if self.should_notify(ctx.outcome()) {
            self.send_slack(format!(
                "Agent {} completed task: {}",
                ctx.agent_name(), ctx.task_summary()
            )).await?;
        }
        Ok(())
    }
}
```

### Example: Custom Gate Extension

```rust
pub struct PropertyTestGateExt {
    proptest_binary: PathBuf,
    timeout: Duration,
}

impl Extension for PropertyTestGateExt {
    fn name(&self) -> &str { "property-test-gate" }
    fn layer(&self) -> ExtensionLayer { ExtensionLayer::Action }

    async fn on_outcome(&mut self, ctx: &mut OutcomeContext) -> Result<()> {
        // After a coding task succeeds normal gates, run property tests
        if ctx.domain() == "code" && ctx.passed_standard_gates() {
            let result = Command::new(&self.proptest_binary)
                .timeout(self.timeout)
                .output()
                .await?;
            if !result.status.success() {
                ctx.fail_gate("property-test", String::from_utf8_lossy(&result.stderr));
            }
        }
        Ok(())
    }
}
```

## Mapping from orchestrate.rs to Extensions

| orchestrate.rs Responsibility | Extension | Layer |
|---|---|---|
| Agent dispatch + MCP | Built into HeartbeatPipeline | Core |
| Gate execution | `GateExt` | Action (L4) |
| Worktree/git management | `GitExt` | Action (L4) |
| Learning/episodes | `LearningExt` | Meta (L6) |
| Conductor/watchers | `ConductorExt` | Cognition (L3) |
| Daimon/affect | `DaimonExt` | Cognition (L3) |
| Neuro/knowledge | `NeuroExt` | Memory (L2) |
| Dreams | `DreamsExt` | Meta (L6) |
| Event bus | `EventFabric` (core, not extension) | Core |
| Task tracking | Built into `Agent` state | Core |
| Skill extraction | `LearningExt` | Meta (L6) |
| Budget management | `MortalityExt` (generalized) | Memory (L2) |
| Signal emission | `ObservabilityExt` | Foundation (L0) |
| TUI/dashboard | External consumer of EventFabric | N/A |
| Config/setup | `DomainProfile` + config | Core |
| System prompt assembly | `ContextExt` | Foundation (L0) |
| Replan logic | `ConductorExt` (intervention policy) | Cognition (L3) |

## Compatibility with External Systems

### MCP Tools (Already Supported)

MCP servers are discovered and their tools added to the `ToolsExt` registry.
No change needed — MCP tools work through the existing `before_tool_call` /
`after_tool_call` hooks.

### A2A Protocol (JSON-RPC 2.0)

Agents expose their capabilities via A2A Agent Cards:

```json
{
  "name": "blockchain-agent-1",
  "description": "Ethereum DeFi agent",
  "version": "1.0.0",
  "capabilities": {
    "tools": ["swap", "simulate_tx", "balance_of"],
    "events": ["new_block", "price_feed"],
    "domains": ["blockchain"]
  },
  "endpoint": "ws://localhost:8080/a2a"
}
```

Other frameworks (ElizaOS, GOAT, custom) can discover and interact with
roko agents via standard JSON-RPC 2.0 over WebSocket.

### Tool Compatibility

The `ToolDef` pattern is compatible with:
- MCP tools (auto-converted via `mcp_to_tool_def()`)
- OpenAI function calling format (JSON schema)
- Custom shell commands (wrapped in handler)
- A2A tool advertisements (JSON-RPC discovery)
