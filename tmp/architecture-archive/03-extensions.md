# Extensions

> Part of the [Roko Architecture Specification](00-INDEX.md). Extracted from the agent runtime section of the v2 redesign doc.
> Cross-reference: [Agent Runtime](02-agent-runtime.md) for the pipeline that invokes these hooks.

---

Agents are specialized by their extension chain, not code forks. Extensions implement hooks across eight layers:

```rust
#[async_trait]
pub trait Extension: Send + Sync {
    /// Human-readable name.
    fn name(&self) -> &str;

    /// Which layer this extension operates in.
    fn layer(&self) -> ExtensionLayer;

    // --- Foundation layer ---
    async fn on_init(&mut self, _ctx: &mut AgentContext) -> Result<()> { Ok(()) }
    async fn on_shutdown(&mut self, _ctx: &mut AgentContext) -> Result<()> { Ok(()) }

    // --- Perception layer ---
    async fn on_observe(&self, _obs: &mut Observations) -> Result<()> { Ok(()) }
    async fn filter_input(&self, _input: &mut AgentMessage) -> Result<FilterDecision> {
        Ok(FilterDecision::Pass)
    }

    // --- Memory layer ---
    async fn on_retrieve(&self, _query: &str, _results: &mut Vec<MemoryItem>) -> Result<()> {
        Ok(())
    }
    async fn on_store(&self, _item: &MemoryItem) -> Result<()> { Ok(()) }

    // --- Cognition layer ---
    async fn pre_inference(&self, _req: &mut InferenceRequest) -> Result<()> { Ok(()) }
    async fn post_inference(&self, _resp: &mut InferenceResponse) -> Result<()> { Ok(()) }
    async fn on_gate(&self, _decision: &mut GateDecision) -> Result<()> { Ok(()) }

    // --- Action layer ---
    async fn pre_action(&self, _action: &mut Action) -> Result<ActionDecision> {
        Ok(ActionDecision::Proceed)
    }
    async fn post_action(&self, _action: &Action, _result: &ActionResult) -> Result<()> {
        Ok(())
    }
    async fn on_tool_call(&self, _call: &mut ToolCall) -> Result<ToolDecision> {
        Ok(ToolDecision::Allow)
    }

    // --- Social layer ---
    async fn on_message_send(&self, _msg: &mut AgentMessage) -> Result<()> { Ok(()) }
    async fn on_message_receive(&self, _msg: &AgentMessage) -> Result<()> { Ok(()) }

    // --- Meta layer ---
    async fn on_reflect(&self, _state: &CorticalState) -> Result<Vec<Adjustment>> {
        Ok(vec![])
    }
    async fn on_cost_update(&self, _usage: &Usage) -> Result<()> { Ok(()) }

    // --- Recovery layer ---
    async fn on_error(&self, _error: &AgentError) -> Result<RecoveryAction> {
        Ok(RecoveryAction::Propagate)
    }
    async fn on_budget_exceeded(&self, _usage: &Usage) -> Result<BudgetAction> {
        Ok(BudgetAction::Sleepwalk)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ExtensionLayer {
    Foundation,
    Perception,
    Memory,
    Cognition,
    Action,
    Social,
    Meta,
    Recovery,
}
```

### Extension loading and discovery

Extensions are loaded from three sources, checked in order:

```
Source          Location                                    Format
──────          ────────                                    ──────
Built-in        Compiled into the roko binary               Rust code (static dispatch)
Local           .roko/extensions/{name}/                    Compiled .so (Linux), .dylib (macOS)
Registry        Fetched from relay extension registry       Downloaded to .roko/extensions/ on first use,
                on first use, cached locally                then loaded as local
```

**Load order**: Built-in extensions load first (always available). Then local extensions from disk. Registry extensions are fetched only if referenced in config but not found locally.

**Error handling**:

```toml
# roko.toml
[[agents]]
name = "coder-1"
extensions = [
  { name = "git",           optional = false },  # default: abort on load failure
  { name = "custom-linter", optional = true },    # skip with warning on load failure
]
```

- `optional = false` (the default): if the extension fails to load, agent startup aborts with an error. This is the default for profile defaults (e.g., `git` in the `coding` profile) because the agent cannot function without core extensions.
- `optional = true`: if the extension fails to load, log a warning and continue startup without it. The agent operates with a reduced extension chain.

**Registry fetch flow**:

```
Config references "vuln-scanner"
         │
         ▼
Check .roko/extensions/vuln-scanner/
         │
    found ──► Load .so/.dylib
         │
    not found
         │
         ▼
GET {relay_url}/registry/extensions/vuln-scanner
         │
         ▼
Download to .roko/extensions/vuln-scanner/
         │
         ▼
Verify SHA-256 checksum from registry manifest
         │
         ▼
Load .so/.dylib
```

### Extension hook execution order

Per tick, extensions fire in layer order: L0 (Foundation) through L7 (Recovery). Within a layer, extensions fire in config order -- the order they appear in the `extensions = [...]` array in `roko.toml`.

```
Tick execution:

  L0 Foundation:   [git.on_init, compiler.on_init]         ← config order
  L1 Perception:   [git.on_observe, web-search.on_observe]
  L2 Memory:       [neuro-store.on_retrieve]
  L3 Cognition:    [safety.pre_inference, compiler.post_inference]
  L4 Action:       [git.pre_action, test-runner.post_action]
  L5 Social:       [slack.on_message_send]
  L6 Meta:         [cost-tracker.on_cost_update]
  L7 Recovery:     [circuit-breaker.on_error]
```

**Fault isolation**: If one extension's hook returns `Err`, the runtime logs the error with the extension name and hook name, then continues to the next extension in the chain. The agent does not abort on a single extension error. This prevents a buggy optional extension from taking down the entire agent.

```
[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s
       Continuing with remaining extensions.
```

The exception is `pre_action` hooks that return `ActionDecision::Block` -- these are not errors but intentional vetoes (e.g., safety extensions blocking dangerous tool calls). Blocks halt the action, not the agent.

### Extension dependency resolution

Extensions can declare dependencies on other extensions:

```toml
# .roko/extensions/report-writer/manifest.toml
[extension]
name = "report-writer"
layer = "action"
depends_on = ["citation", "summarizer"]
```

On load, the runtime performs a topological sort of extensions within each layer to resolve dependencies. If `report-writer` depends on `citation`, then `citation` hooks always fire before `report-writer` hooks within the same layer.

**Cyclic dependency** (e.g., A depends on B, B depends on A) is a startup error:

```
Error: Cyclic extension dependency detected: report-writer -> citation -> report-writer
       Remove the cycle or merge the extensions.
```

**Cross-layer dependencies** are not supported. Extensions in different layers already have a fixed execution order (L0 before L1 before L2, etc.). A Memory-layer extension that needs Foundation-layer setup gets it automatically through layer ordering.

---

## Connectors (universal primitive)

> Added 2026-04-24. Per dashboard PRD 23, Connector is a first-class primitive in the 12-primitive vocabulary.

A Connector wraps external system I/O behind a universal trait: `connect / query / execute / health / disconnect`. This generalizes what was previously hardcoded as `ChainClient`, `VenueAdapter`, MCP server configs, and database connections into a single composable abstraction.

### Why Connector is distinct from Extension

Extensions modify agent behavior through hooks -- they intercept, filter, and transform. Connectors provide bidirectional I/O with external systems. A Connector does not modify agent behavior; it provides a capability. The distinction matters for composition: an agent *loads* extensions but *uses* connectors.

### Connector trait shape

```rust
#[async_trait]
pub trait Connector: Send + Sync {
    /// Human-readable name (e.g., "hyperliquid", "github-mcp", "postgres").
    fn name(&self) -> &str;

    /// Connector kind for registry classification.
    fn kind(&self) -> ConnectorKind;

    /// Establish connection. Called once at agent startup.
    async fn connect(&mut self, config: &ConnectorConfig) -> Result<()>;

    /// One-shot query against the external system.
    async fn query(&self, request: QueryRequest) -> Result<QueryResponse>;

    /// Execute a mutating operation (order placement, tx submission, write).
    async fn execute(&self, request: ExecuteRequest) -> Result<ExecuteResponse>;

    /// Health check. Called periodically by the conductor's health watcher.
    async fn health(&self) -> Result<HealthStatus>;

    /// Graceful disconnect. Called on agent shutdown.
    async fn disconnect(&mut self) -> Result<()>;
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectorKind {
    ChainRpc,       // Ethereum, Solana, etc.
    Exchange,       // Hyperliquid, Binance, etc.
    McpServer,      // MCP tool servers
    Database,       // Postgres, SQLite, etc.
    Webhook,        // External HTTP endpoints
    Api,            // Generic REST/gRPC APIs
}
```

### Existing code that maps to Connector

| Existing construct | Crate | Becomes |
|--------------------|-------|---------|
| `ChainClient` / `AlloyChainClient` | `roko-chain` | `ChainRpcConnector` |
| `VenueAdapter` | `roko-chain` (gap doc 02) | `ExchangeConnector` |
| MCP server config in `roko.toml` | `roko-agent` | `McpConnector` (auto-registered from config) |
| Oracle endpoints | `roko-chain` (gap doc 01) | `ApiConnector` |

### Dashboard authoring surface

The Connector Manager is a 4-stage authoring surface (per PRD 23):

1. **Type selection** -- pick connector type from a template gallery
2. **Configuration** -- connection string, auth, rate limits, retry policy (live health check)
3. **Tool registration** -- auto-discover operations; select which to expose as agent tools
4. **Test and deploy** -- execute test query, verify health endpoint, show latency and error rate

### Relationship to Extensions and Feeds

Connectors sit between Extensions and Feeds in the composition hierarchy:

- An **Extension** can *wrap* a Connector to add behavior (e.g., rate limiting, retry logic)
- A **Feed** is *sourced from* a Connector (e.g., a price feed subscribes to an exchange connector)
- An **Agent** *uses* Connectors for I/O but *loads* Extensions for behavior modification

---

## Spec clarifications (added 2026-04-25)

> Backported from `tmp/architecture-plans/06-architecture-implementation.md` Phase A.3.

### Decision enum variants

The Extension trait uses several decision types that were not fully specified. Complete definitions:

```rust
/// Perception layer: filter_input() return value
pub enum FilterDecision {
    Pass,                          // Message passes through unchanged
    Drop,                          // Message silently discarded
    Transform(AgentMessage),       // Replace message with transformed version
}

/// Action layer: pre_action() return value
pub enum ActionDecision {
    Proceed,                       // Action executes normally
    Block { reason: String },      // Action halted (not an error — intentional veto)
    Modify(Action),                // Execute modified action instead
}

/// Action layer: on_tool_call() return value
pub enum ToolDecision {
    Allow,                         // Tool call proceeds
    Block { reason: String },      // Tool call blocked (logged, agent notified)
    Substitute(ToolCall),          // Replace with different tool call
}

/// Recovery layer: on_error() return value
pub enum RecoveryAction {
    Propagate,                     // Error propagates up (default)
    Retry,                         // Retry the failed operation
    Ignore,                        // Suppress the error
    Escalate(String),              // Escalate with custom message
}

/// Recovery layer: on_budget_exceeded() return value
pub enum BudgetAction {
    Sleepwalk,                     // Enter sleepwalk mode (observe + reflect only)
    Stop,                          // Shut down the agent
    RequestMore(u64),              // Request additional budget (microdollars)
}

/// Meta layer: on_reflect() return value
pub enum Adjustment {
    SetGoal(Goal),                 // Replace or add a goal
    UpdateBelief(String, f64),     // Update belief key-value pair
    ShiftAttention(String),        // Change attention focus
}
```

**Behavioral consequences**:
- `FilterDecision::Drop` → message never reaches the agent's pipeline. Logged for debugging.
- `ActionDecision::Block` → action halted, agent continues (not crashed). Agent receives "action blocked by {extension_name}: {reason}" in its next turn.
- `ToolDecision::Substitute` → original tool call replaced transparently. The agent sees the substitute's result.

### Hook timeout

All extension hooks timeout after **5 seconds**. This is currently hardcoded (not configurable per hook).

```
[WARN] Extension "custom-linter" hook "post_action" failed: timeout after 5s
       Continuing with remaining extensions.
```

If timeout behavior becomes a problem, the first enhancement would be a per-extension `timeout_ms` field in `manifest.toml`. But keep it simple until proven needed.

### AgentContext (passed to extension hooks)

Extensions receive `&AgentContext` for read access to agent state:

```rust
pub struct AgentContext {
    pub agent_id: String,
    pub profile: DomainProfile,
    pub mode: AgentMode,
    pub regime: Regime,               // current adaptive clock regime
    pub budget_remaining: u64,        // microdollars
    pub episode_count: u64,
    pub config: Arc<AgentConfig>,     // full agent config (read-only)
}
```

This is **read-only**. Extensions that need to modify agent behavior do so through their return values (decision enums above), not by mutating context.

### Connector discovery

Connectors are discovered from three sources (matching extension discovery order):
1. **Config**: `[[agents]] connectors = ["postgres", "hyperliquid"]` in roko.toml
2. **MCP auto-register**: Any MCP server in `agent.mcp_config` auto-registers as `McpConnector`
3. **Extension-provided**: An extension can register connectors in its `on_init()` hook

There is no registry-based discovery for connectors (unlike extensions). Connectors are always explicitly declared in agent config or provided by extensions.

### Acceptance criteria

- [ ] Extension trait compiles with all 22 hooks and default no-op implementations
- [ ] `FilterDecision::Drop` silently discards message (logged)
- [ ] `ActionDecision::Block` halts action but agent continues
- [ ] `ToolDecision::Substitute` transparently replaces tool call
- [ ] Hook timeout at 5 seconds → warning logged, next extension continues
- [ ] Missing optional extension → warning logged, agent starts normally
- [ ] Missing required extension → agent startup aborts with clear error
- [ ] Cyclic dependency detected → startup error with cycle description
- [ ] Extensions sorted: by layer, then by dependency (topological), then by config order
