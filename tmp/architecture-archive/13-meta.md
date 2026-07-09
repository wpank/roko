# 13 -- Meta layer

Agents that create agents. Generators that produce arenas, gates, evals, and extensions. Lineage tracking across generations. Recursive safety enforcement. This document specifies the runtime types, coordination protocols, and safety model that make recursive agent creation tractable.

Dashboard surfaces consuming these APIs are specified in `16-meta-surfaces.md` (PRD).

---

## Design constraints

1. **Meta-agents are agents.** A meta-agent runs on the same `AgentRuntime` as any other agent. It has a domain, extensions, gates, model routing, a knowledge store, and a reputation. What distinguishes it is the tools available to it: agent creation, configuration, lifecycle management.
2. **Generators are agents with output schemas.** A generator is an agent whose output must conform to a typed schema for the object it produces. A gate validates the output against the schema before registration.
3. **Depth is bounded.** Recursive creation has a configurable maximum depth (default: 3). A meta-agent can create an agent; that agent can create another; that agent cannot create a fourth level without explicit override.
4. **Children cannot exceed parents.** Caveat inheritance is monotonically narrowing. A child agent's delegation caveats can only restrict, never expand, its parent's caveats.
5. **Lineage is permanent.** Parent-child relationships are recorded on-chain (ERC-8004 `parentPassport` field) and locally. Lineage cannot be erased or rewritten.
6. **Anomaly detection runs continuously.** The system monitors recursive creation for runaway patterns: excessive creation rates, quality degradation across generations, and circular dependencies.

---

## Meta-agents

A meta-agent creates and configures other agents. It takes a specification -- a goal, a domain, constraints -- and produces a fully configured agent ready to start.

### Runtime model

Meta-agents run on `AgentRuntime` with two specialized extensions:

```rust
/// Extension that gives an agent the ability to create other agents.
pub struct AgentCreatorExt {
    /// Maximum creation depth. Children inherit depth - 1.
    pub max_depth: u32,
    /// Current depth in the creation chain (0 = top-level meta-agent).
    pub current_depth: u32,
    /// Rate limit: maximum creations per hour.
    pub max_creations_per_hour: u32,
    /// Running creation count for the current hour window.
    pub creations_this_hour: u32,
    /// Quality gate: minimum eval score a created agent must achieve.
    pub min_child_quality: f64,
    /// Caveats that propagate to all children.
    pub inherited_caveats: Vec<DelegationCaveat>,
}

/// Extension that gives an agent the ability to optimize configurations.
pub struct ConfigOptimizerExt {
    /// Parameter ranges the optimizer can explore.
    pub tunable_params: Vec<TunableParam>,
    /// Optimization history (configs tried and their outcomes).
    pub history: Vec<ConfigTrialOutcome>,
    /// Strategy: grid search, random search, Bayesian, or bandit.
    pub strategy: OptimizationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunableParam {
    pub name: String,
    pub param_type: ParamType,
    pub range: ParamRange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    /// Floating-point parameter (e.g., temperature, top_p).
    Float,
    /// Integer parameter (e.g., max_tokens, retry_count).
    Int,
    /// Categorical parameter (e.g., model name, strategy type).
    Categorical { options: Vec<String> },
    /// Boolean toggle.
    Bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamRange {
    Float { min: f64, max: f64 },
    Int { min: i64, max: i64 },
    Categorical,
    Bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigTrialOutcome {
    pub config: serde_json::Value,
    pub eval_score: f64,
    pub cost_usd: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    GridSearch,
    RandomSearch,
    Bayesian,
    Bandit,
}
```

### Tools

Meta-agents have access to these tools through the standard tool dispatch system:

| Tool | Parameters | Description |
|------|-----------|-------------|
| `agent_create` | `config: AgentConfig` | Create a new agent from a configuration |
| `agent_configure` | `agent_id: String, patch: ConfigPatch` | Update a running agent's configuration |
| `agent_start` | `agent_id: String` | Start a stopped agent |
| `agent_stop` | `agent_id: String` | Gracefully stop a running agent |
| `agent_fork` | `source_id: String, overrides: ConfigPatch` | Fork an existing agent with modifications |
| `agent_eval` | `agent_id: String, eval_id: String` | Run an eval against an agent and return the score |
| `agent_list_children` | none | List all agents created by this meta-agent |

The `agent_create` tool enforces:
- Depth check: `current_depth + 1 <= max_depth`
- Rate limit: `creations_this_hour < max_creations_per_hour`
- Caveat inheritance: child caveats are the intersection of parent caveats and any additional child-specific caveats
- Quality gate: if `min_child_quality > 0`, the created agent runs through a quick eval before registration

### Configuration

```rust
/// Configuration for a meta-agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaAgentConfig {
    /// Base agent configuration (domain, extensions, gates, model routing).
    pub agent: AgentConfig,
    /// What kinds of agents this meta-agent produces.
    pub target_spec: TargetSpec,
    /// Creator extension configuration.
    pub creator: AgentCreatorExt,
    /// Optional optimizer extension.
    pub optimizer: Option<ConfigOptimizerExt>,
}

/// Describes what a meta-agent is designed to produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpec {
    /// Human-readable description of the target agent type.
    pub description: String,
    /// Domain the produced agents should operate in.
    pub target_domain: String,
    /// Required capabilities the produced agents must have.
    pub required_capabilities: Vec<String>,
    /// Template to base new agents on (optional).
    pub template_id: Option<String>,
}
```

---

## Generators

A generator is an agent that produces non-agent objects: arenas, gates, evals, extensions, or domain profiles. Where a meta-agent's output is another agent, a generator's output is a registered first-class object.

### Output schema validation

Every generator declares the type of object it produces. Output is validated against the type's schema before registration. If validation fails, the object is not registered and the generation event records the failure.

```rust
/// Types of objects a generator can produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeneratorOutputType {
    Arena,
    Gate,
    Eval,
    Extension,
    DomainProfile,
}

/// A generator's output before validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorOutput {
    pub output_type: GeneratorOutputType,
    /// Serialized object matching the output type's schema.
    pub payload: serde_json::Value,
    /// Metadata about the generation process.
    pub metadata: GenerationMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    /// Generator agent ID.
    pub generator_id: String,
    /// Specification the generator was given.
    pub spec: serde_json::Value,
    /// Time spent generating.
    pub generation_duration_ms: u64,
    /// Model used.
    pub model: String,
    /// Cost of generation.
    pub cost_usd: f64,
}

/// Validates generator output against the expected schema for its type.
pub fn validate_generator_output(output: &GeneratorOutput) -> Result<(), ValidationError> {
    match output.output_type {
        GeneratorOutputType::Arena => validate_arena_schema(&output.payload),
        GeneratorOutputType::Gate => validate_gate_schema(&output.payload),
        GeneratorOutputType::Eval => validate_eval_schema(&output.payload),
        GeneratorOutputType::Extension => validate_extension_schema(&output.payload),
        GeneratorOutputType::DomainProfile => validate_domain_schema(&output.payload),
    }
}
```

### Generator configuration

```rust
/// Configuration for a generator agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Base agent configuration.
    pub agent: AgentConfig,
    /// What type of object this generator produces.
    pub output_type: GeneratorOutputType,
    /// JSON Schema for output validation (derived from output_type if not provided).
    pub output_schema: Option<serde_json::Value>,
    /// Whether generated objects are automatically registered or held for review.
    pub auto_register: bool,
    /// Quality threshold: minimum eval score before auto-registration.
    pub min_quality: f64,
}
```

---

## Lineage tracking

Every object in the system records its creation ancestry. This forms a directed acyclic graph of parent-child relationships traversable from any node.

### On-chain lineage

When an agent registers on-chain (ERC-8004), the `parentPassport` field records which agent created it. For non-agent objects (arenas, evals, etc.), the creating agent's passport ID is recorded in the object's metadata.

```rust
/// A lineage edge recording a parent-child creation relationship.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageEdge {
    /// Parent object identifier.
    pub parent_id: ObjectId,
    /// Child object identifier.
    pub child_id: ObjectId,
    /// Type of relationship.
    pub relationship: LineageRelationship,
    /// Block at which the relationship was recorded.
    pub recorded_at_block: u64,
    /// Timestamp.
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageRelationship {
    /// Parent created child from scratch.
    Generated,
    /// Child is a fork of an existing object with modifications.
    Forked,
    /// Child evolved from parent through an optimization process.
    Evolved,
}

/// A generic object identifier used in lineage tracking.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectId {
    /// Object type (agent, arena, eval, extension, gate, domain).
    pub object_type: ObjectType,
    /// Unique identifier within the type namespace.
    pub id: String,
    /// On-chain passport ID (for agents) or registration ID.
    pub chain_id: Option<u128>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    Agent,
    MetaAgent,
    Generator,
    Arena,
    Eval,
    Gate,
    Extension,
    DomainProfile,
}
```

### Lineage queries

```rust
/// Service for querying lineage relationships.
pub struct LineageService {
    /// Local lineage store (JSONL-backed).
    local_store: LineageStore,
    /// Chain reader for on-chain lineage data.
    chain_reader: Option<Box<dyn ChainReader>>,
}

impl LineageService {
    /// All ancestors of an object (parent, grandparent, ...).
    pub async fn ancestors(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// Direct children of an object.
    pub async fn children(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// All descendants recursively (children, grandchildren, ...).
    pub async fn descendants(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// Siblings: objects with the same parent.
    pub async fn siblings(&self, id: &ObjectId) -> Result<Vec<LineageEdge>> { ... }

    /// Full lineage graph for visualization.
    pub async fn graph(
        &self,
        root: &ObjectId,
        max_depth: u32,
    ) -> Result<LineageGraph> { ... }
}

/// Lineage graph for dashboard visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageGraph {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineageNode {
    pub id: ObjectId,
    /// Human-readable name.
    pub name: String,
    /// Total descendants count.
    pub descendant_count: u64,
    /// Aggregate success rate of descendants (for meta-agents).
    pub descendant_success_rate: Option<f64>,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
}
```

---

## Recursive safety

Recursive agent creation is powerful and dangerous. Without bounds, a meta-agent could spawn thousands of agents that consume all available resources, create circular dependencies, or produce progressively worse outputs.

### Safety mechanisms

**Depth limit.** Every meta-agent has a `max_depth` (default: 3). The `AgentCreatorExt` tracks `current_depth` and refuses creation when the limit is reached. Children inherit `max_depth - 1` as their own maximum.

**Rate limit.** Each meta-agent is rate-limited to `max_creations_per_hour` (default: 10). The limit resets on a rolling window basis.

**Quality gate.** If `min_child_quality > 0`, every created agent runs through a quick eval before registration. Agents that score below the threshold are rejected and their creation events record the failure reason.

**Caveat inheritance.** Children can only narrow parent caveats, never widen them. The enforcement is structural:

```rust
/// Compute the effective caveats for a child agent.
/// The child's caveats are the intersection of the parent's inherited
/// caveats and any additional restrictions specified at creation time.
pub fn compute_child_caveats(
    parent_caveats: &[DelegationCaveat],
    additional_restrictions: &[DelegationCaveat],
) -> Vec<DelegationCaveat> {
    let mut child_caveats = parent_caveats.to_vec();
    for restriction in additional_restrictions {
        // Only add restrictions that narrow existing caveats.
        // Reject any attempt to widen (e.g., increasing a budget cap).
        if restriction.is_narrower_than_all(&child_caveats) {
            child_caveats.push(restriction.clone());
        }
    }
    child_caveats
}
```

**Anomaly detection.** The `RecursiveSafetyMonitor` runs continuously and watches for:

```rust
/// Monitors recursive creation patterns for anomalies.
pub struct RecursiveSafetyMonitor {
    /// Maximum creation rate across all meta-agents (global backstop).
    pub global_max_rate_per_hour: u32,
    /// Minimum quality trend slope before flagging degradation.
    pub min_quality_slope: f64,
    /// Window size for quality trend computation.
    pub quality_window: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SafetyAnomaly {
    /// A meta-agent is creating agents faster than its rate limit.
    RateLimitViolation {
        meta_agent_id: String,
        rate: u32,
        limit: u32,
    },
    /// Quality is degrading across generations.
    QualityDegradation {
        meta_agent_id: String,
        generation: u32,
        quality_trend: Vec<f64>,
        slope: f64,
    },
    /// Circular dependency detected (A created B, B created A).
    CircularDependency {
        agents: Vec<String>,
    },
    /// Global creation rate exceeded.
    GlobalRateExceeded {
        current_rate: u32,
        limit: u32,
    },
    /// A meta-agent attempted to widen parent caveats.
    CaveatEscalation {
        meta_agent_id: String,
        attempted_caveat: String,
    },
}

impl RecursiveSafetyMonitor {
    /// Check all active recursive processes for anomalies.
    pub fn scan(&self, active_processes: &[RecursiveProcess]) -> Vec<SafetyAnomaly> { ... }

    /// Recommended action for an anomaly.
    pub fn recommend_action(&self, anomaly: &SafetyAnomaly) -> SafetyAction { ... }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyAction {
    /// Log the anomaly but take no action.
    Log,
    /// Pause the offending meta-agent.
    Pause { agent_id: String },
    /// Quarantine: pause the agent and flag all its recent children for review.
    Quarantine { agent_id: String },
    /// Terminate: stop the agent and prevent restart without manual approval.
    Terminate { agent_id: String },
}
```

---

## Practical example

A market-regime meta-agent that creates trading agents optimized for different market conditions:

1. The meta-agent observes the current market regime (trending, ranging, volatile) via its knowledge store.
2. For each regime, it creates a specialized trading agent:
   - **Trending agent**: momentum-following strategy, wider stops, position sizing favors trends
   - **Ranging agent**: mean-reversion strategy, tight stops, reduced sizing in low-volatility conditions
   - **Volatile agent**: defensive strategy, minimal positions, hedging focus
3. Each child agent runs through a quick eval on historical data for its target regime.
4. Children that score above `min_child_quality` are registered and started.
5. The meta-agent monitors children's performance via `TradingReflect` events.
6. When a regime shift occurs, the meta-agent activates the appropriate child and pauses the others.
7. Over time, the `ConfigOptimizerExt` tunes each child's parameters based on accumulated P&L data.

The meta-agent itself has caveats: it can only create agents within the `trading` domain, with a maximum budget of $50/day per child, and a maximum of 3 concurrent children. These caveats propagate -- no child can spend more than $50/day, and no child can create further agents (depth = 1 for the children).

---

## Event types

```json
{
    "type": "meta.agent_created",
    "payload": {
        "meta_agent_id": "regime-meta-1",
        "child_agent_id": "trending-trader-v3",
        "depth": 1,
        "target_spec": "trending market trader",
        "eval_score": 0.82,
        "cost_usd": 0.45,
        "block_number": 19847300
    }
}
```

```json
{
    "type": "meta.generation_failed",
    "payload": {
        "meta_agent_id": "regime-meta-1",
        "reason": "quality_below_threshold",
        "eval_score": 0.31,
        "min_required": 0.60,
        "spec": "volatile market trader"
    }
}
```

```json
{
    "type": "meta.safety_anomaly",
    "payload": {
        "anomaly_type": "quality_degradation",
        "meta_agent_id": "regime-meta-1",
        "severity": "warning",
        "quality_trend": [0.82, 0.75, 0.68, 0.61],
        "recommended_action": "pause"
    }
}
```

```json
{
    "type": "generator.output_produced",
    "payload": {
        "generator_id": "arena-gen-1",
        "output_type": "arena",
        "output_id": "trading-arena-eth-momentum",
        "validation_passed": true,
        "auto_registered": true,
        "cost_usd": 0.23
    }
}
```

```json
{
    "type": "lineage.edge_recorded",
    "payload": {
        "parent_type": "meta_agent",
        "parent_id": "regime-meta-1",
        "child_type": "agent",
        "child_id": "trending-trader-v3",
        "relationship": "generated",
        "block_number": 19847300
    }
}
```

### Full event type list

| Event | Emitted by | Consumed by |
|-------|-----------|-------------|
| `meta.agent_created` | AgentCreatorExt | Dashboard, lineage service, reputation registry |
| `meta.agent_configured` | ConfigOptimizerExt | Dashboard, agent detail |
| `meta.generation_failed` | AgentCreatorExt | Dashboard (alert), safety monitor |
| `meta.safety_anomaly` | RecursiveSafetyMonitor | Dashboard (alert), auto-pause system |
| `generator.output_produced` | Generator agent | Dashboard, lineage service, output registry |
| `generator.validation_failed` | Output validation | Dashboard, generator detail |
| `lineage.edge_recorded` | Lineage service | Dashboard (lineage graph), chain indexer |

---

## API surface

### Meta-agent endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/agents` | List meta-agents (supports `?scope=fleet&owner={address}`) |
| `GET` | `/api/meta/agents/{id}` | Meta-agent detail with children summary |
| `POST` | `/api/meta/agents` | Create a new meta-agent |
| `GET` | `/api/meta/agents/{id}/children` | List agents created by this meta-agent |
| `GET` | `/api/meta/generations?limit=20` | Recent generation events across all meta-agents |

### Generator endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/generators` | List generators (supports `?type={arena,gate,...}`) |
| `GET` | `/api/meta/generators/{id}` | Generator detail with recent outputs |
| `POST` | `/api/meta/generators` | Create a new generator |
| `GET` | `/api/meta/generators/outputs?limit=20` | Recent generated objects |
| `GET` | `/api/meta/generators/featured` | Featured public generators |

### Lineage endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/lineage/{id}/ancestors` | Ancestor chain for an object |
| `GET` | `/api/meta/lineage/{id}/descendants` | Descendant tree |
| `GET` | `/api/meta/lineage/{id}/siblings` | Objects sharing the same parent |
| `GET` | `/api/meta/lineage/graph?root={id}&depth=3` | Full lineage graph for visualization |
| `GET` | `/api/meta/lineage/most-forked?type={type}` | Most-forked objects by type |

### Safety endpoints

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/meta/safety/active` | Currently active recursive processes |
| `GET` | `/api/meta/safety/anomalies?severity={warning,critical}` | Recent safety anomalies |
| `GET` | `/api/meta/safety/trace/{process_id}` | Full trace of a recursive process |

---

## Configuration

```toml
# roko.toml

[meta]
enabled = true

[meta.creation]
max_depth = 3
max_creations_per_hour = 10
global_max_rate_per_hour = 50
min_child_quality = 0.0  # 0 = no quality gate

[meta.safety]
quality_trend_window = 10
min_quality_slope = -0.05  # Flag if quality drops faster than 5% per generation
circular_detection = true
auto_pause_on_anomaly = false  # Manual response by default

[meta.lineage]
record_on_chain = true
local_store_path = ".roko/lineage/"
```
