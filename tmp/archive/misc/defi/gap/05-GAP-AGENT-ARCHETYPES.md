# 05 -- Agent Archetypes: Work Batches

> **Scope**: Build the archetype registry, first DeFi archetypes, and delegation DAG.
> **Batches**: 3 | **Total effort**: L + L + L

---

## Batch 5.1: Archetype registry and manifest loader

> **Effort**: L | **Depends on**: none | **Crate**: roko-agent
> **Branch**: `defi/batch-5.1-archetype-registry`

### Context

Roko's agent system is general-purpose. The `AgentDefinition` struct in `roko-core/src/config/schema.rs:3669` has six flat fields: `name`, `domain`, `prompt`, `model`, `chain_rpc`, `enabled`. There are no tool profiles, no delegation targets, no per-archetype system prompt fragments.

The DeFi PRDs specify 42+ agent archetypes across 14 categories. Each archetype carries a structured manifest: tool categories, delegation targets, inference tier, safety constraints, and system prompt fragments. The archetype system is load-bearing -- later batches (5.2, 5.3, 6.2) need it for tool-profile resolution and delegation routing.

This batch builds the `ArchetypeManifest` type, a TOML-based registry that loads archetype definitions from disk at boot, and a resolver that maps agent names to their manifest. The resolver integrates with the existing `RoleToolProfile` system in `roko-std/src/roles.rs` (line 20) but extends it from 5 code-task roles to an open set of DeFi archetypes.

The existing `RoleToolProfileKind` enum (`roko-std/src/roles.rs:21`) has five variants: Implementer, Researcher, Reviewer, Strategist, Scribe. The archetype registry does not replace this -- it layers DeFi-specific tool filtering on top. The `DomainToolProfile` struct at line 178 already provides the extension point.

### Read First

| File | Why |
|------|-----|
| `crates/roko-core/src/config/schema.rs:3667-3690` | `AgentDefinition` struct -- the current agent config |
| `crates/roko-std/src/roles.rs:1-179` | `RoleToolProfile`, `RoleToolProfileKind`, `DomainToolProfile` -- existing tool filtering |
| `crates/roko-agent/src/lib.rs` | Module declarations -- where to add `archetype` module |
| `crates/roko-compose/src/system_prompt_builder.rs:49-96` | `SystemPromptBuilder` -- how system prompts are assembled |
| `crates/roko-agent/src/dispatcher/mod.rs:1-31` | `ToolDispatcher` pipeline -- where tool filtering hooks in |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work Items

**5.1.1 -- Define `ArchetypeManifest` struct**

Create `crates/roko-agent/src/archetype.rs`:

```rust
//! Agent archetype manifest and registry.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Tool category that an archetype can access.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Data,
    Trading,
    Lp,
    Vault,
    Safety,
    Intelligence,
    Memory,
    Identity,
    Fees,
    SelfImprovement,
    Streaming,
}

/// Archetype category for grouping.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchetypeCategory {
    Execution,
    Research,
    Strategy,
    Vault,
    Observer,
    Golem,
    Infrastructure,
    Capital,
    Monitor,
    Lending,
    Staking,
    Derivatives,
}

/// System prompt fragments embedded in an archetype manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPromptFragments {
    /// The agent's role identity sentence.
    pub role: String,
    /// Specific expertise areas.
    #[serde(default)]
    pub expertise: Vec<String>,
    /// Safety rules the agent must follow.
    #[serde(default)]
    pub safety_rules: Vec<String>,
    /// Workflow steps the agent follows per tick.
    #[serde(default)]
    pub workflow: Option<String>,
    /// Expected output format.
    #[serde(default)]
    pub output_format: Option<String>,
}

/// Agent lifecycle mode from the architecture redesign.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    /// Runs until stopped. Heartbeat loop. Use for chain monitoring, fleet supervision.
    Persistent,
    /// Spins up for one task, shuts down. Use for coding tasks, one-off queries.
    Ephemeral,
    /// Sleeps until triggered. Use for alerts, cron jobs, PR review.
    Reactive,
}

/// Isolation level controlling where the agent process runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolationLevel {
    /// Tokio tasks inside the roko control-plane process. Lightweight agents only.
    InProcess,
    /// Dedicated Fly Machine (microVM) with own volume and keys. For heavy agents: coding, trading.
    FlyMachine,
}

/// A single archetype manifest loaded from TOML.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArchetypeManifest {
    /// Unique kebab-case name (e.g. "trade-executor").
    pub name: String,
    /// One-sentence role description.
    pub description: String,
    /// Default inference tier: "mechanical", "focused", "integrative", "architectural".
    pub default_tier: String,
    /// Agent lifecycle mode (Persistent, Ephemeral, Reactive).
    pub mode: AgentMode,
    /// Where this agent runs (InProcess or FlyMachine).
    pub isolation: IsolationLevel,
    /// Tool categories this archetype can access.
    pub tool_categories: Vec<ToolCategory>,
    /// Archetype names this archetype can delegate to.
    #[serde(default)]
    pub delegates_to: Vec<String>,
    /// Whether this archetype is a leaf node (never delegates).
    #[serde(default)]
    pub terminal: bool,
    /// Grouping category.
    pub category: ArchetypeCategory,
    /// System prompt fragments.
    pub system_prompt: SystemPromptFragments,
}
```

**5.1.2 -- Build `ArchetypeRegistry`**

Add to the same file:

```rust
/// Registry holding all loaded archetype manifests, keyed by name.
#[derive(Debug, Clone)]
pub struct ArchetypeRegistry {
    manifests: HashMap<String, ArchetypeManifest>,
}

impl ArchetypeRegistry {
    /// Load all `.toml` files from a directory into the registry.
    pub fn load_from_dir(dir: &Path) -> Result<Self, ArchetypeError> { /* ... */ }

    /// Look up a manifest by archetype name.
    pub fn get(&self, name: &str) -> Option<&ArchetypeManifest> { /* ... */ }

    /// All registered archetype names.
    pub fn names(&self) -> Vec<&str> { /* ... */ }

    /// Validate that all `delegates_to` references resolve and no cycles exist.
    pub fn validate(&self) -> Result<(), ArchetypeError> { /* ... */ }
}
```

Validation must enforce:
- All `delegates_to` entries reference existing archetypes
- Terminal archetypes have empty `delegates_to`
- No cycles (topological sort)
- Maximum delegation depth of 3

**5.1.3 -- Define `ArchetypeError`**

```rust
#[derive(Debug, thiserror::Error)]
pub enum ArchetypeError {
    #[error("archetype manifest not found: {path}")]
    NotFound { path: PathBuf },
    #[error("invalid manifest TOML in {path}: {source}")]
    InvalidToml { path: PathBuf, source: toml::de::Error },
    #[error("delegation target '{target}' not found (referenced by '{source}')")]
    MissingDelegationTarget { source: String, target: String },
    #[error("delegation cycle detected: {cycle:?}")]
    DelegationCycle { cycle: Vec<String> },
    #[error("delegation depth exceeds maximum of {max}: {chain:?}")]
    DelegationDepthExceeded { max: usize, chain: Vec<String> },
    #[error("terminal archetype '{name}' has non-empty delegates_to")]
    TerminalWithDelegates { name: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

**5.1.4 -- Wire module into `roko-agent/src/lib.rs`**

Add `pub mod archetype;` to `crates/roko-agent/src/lib.rs` (after line 75, alongside `safety`). Add re-exports:

```rust
pub use archetype::{ArchetypeManifest, ArchetypeRegistry, ArchetypeError, ToolCategory};
```

**5.1.5 -- Create initial manifest directory**

Create `crates/roko-agent/archetypes/` with a `_template.toml` showing the format. Actual archetype files ship in batch 5.2.

**Warning**: Do not extend `AgentDefinition` in `roko-core`. The archetype system is a layer above it. `AgentDefinition` stays flat for backward compatibility; `ArchetypeManifest` enriches it at runtime.

### Wiring

- `crates/roko-agent/src/lib.rs`: add `pub mod archetype;` and re-exports
- No config schema changes -- the registry loads from a directory, not from `roko.toml`

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_roundtrip() {
        // Serialize an ArchetypeManifest to TOML and back. Verify all fields survive.
    }

    #[test]
    fn test_registry_detects_missing_delegate() {
        // Create two manifests where one delegates to a non-existent name.
        // Assert validate() returns MissingDelegationTarget.
    }

    #[test]
    fn test_registry_detects_cycle() {
        // Create A -> B -> A cycle. Assert validate() returns DelegationCycle.
    }

    #[test]
    fn test_terminal_with_delegates_rejected() {
        // Create a manifest with terminal=true and non-empty delegates_to.
        // Assert validate() returns TerminalWithDelegates.
    }

    #[test]
    fn test_max_depth_exceeded() {
        // Create a chain of 4 delegations. Assert validate() returns DelegationDepthExceeded.
    }
}
```

### Verification

```bash
cargo test -p roko-agent -- archetype
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-agent
```

### Acceptance Criteria

- [ ] `ArchetypeManifest` deserializes from TOML with all fields
- [ ] `ArchetypeRegistry::load_from_dir` loads all `.toml` files from a directory
- [ ] `ArchetypeRegistry::validate` detects missing delegation targets
- [ ] `ArchetypeRegistry::validate` detects cycles via topological sort
- [ ] `ArchetypeRegistry::validate` enforces max depth of 3
- [ ] `ArchetypeRegistry::validate` rejects terminal archetypes with delegates
- [ ] Module wired into `roko-agent/src/lib.rs` with re-exports
- [ ] All tests pass, clippy clean, fmt clean

### Commit Message

```
feat(roko-agent): add archetype manifest registry with delegation validation
```

---

## Batch 5.2: First five DeFi archetypes

> **Effort**: L | **Depends on**: 5.1, 2.1 (VenueAdapter) | **Crate**: roko-agent
> **Branch**: `defi/batch-5.2-core-archetypes`

### Context

With the archetype registry built in 5.1, this batch creates the first five concrete DeFi archetypes as TOML manifest files and wires their tool categories into the existing `ToolDispatcher` pipeline.

The five archetypes cover the minimum viable trading loop: a trade executor, a risk assessor, a safety guardian, a pool researcher, and an LP strategist. These five appear in the most PRD composition patterns (patterns 1, 2, 4, 7 from the PRD's 18-pattern list). Together they enable: research a pool, assess risk, get safety approval, execute a trade.

The `ToolDispatcher` in `roko-agent/src/dispatcher/mod.rs` runs tool calls through a six-step pipeline. Step 3 authorizes via `def.permission.satisfied_by(&role_perms)`. This batch adds a new filtering step between steps 2 and 3: resolve the agent's archetype, check whether the tool's category is in the archetype's `tool_categories`, and reject the call if not.

The system prompt integration uses `SystemPromptBuilder` from `roko-compose/src/system_prompt_builder.rs:61`. The builder already has a `role_identity` field (layer 1) and `domain` field (layer 3). The archetype's `SystemPromptFragments` map directly into these layers.

### Read First

| File | Why |
|------|-----|
| `crates/roko-agent/src/archetype.rs` | From batch 5.1 -- the types being populated |
| `crates/roko-agent/src/dispatcher/mod.rs:1-50` | ToolDispatcher pipeline -- where category filtering hooks in |
| `crates/roko-compose/src/system_prompt_builder.rs:49-96` | SystemPromptBuilder fields -- where fragments inject |
| `crates/roko-std/src/roles.rs:171-179` | `DomainToolProfile` -- extension point for domain tool filtering |
| `crates/roko-compose/src/templates/` | Existing role templates (implementer, researcher, etc.) |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work Items

**5.2.1 -- Create `trade-executor.toml`**

Create `crates/roko-agent/archetypes/trade-executor.toml`:

```toml
name = "trade-executor"
description = "Full swap pipeline from quote through on-chain confirmation"
default_tier = "focused"
mode = "persistent"
isolation = "fly_machine"
tool_categories = ["data", "trading", "safety"]
delegates_to = ["safety-guardian", "risk-assessor"]
terminal = false
category = "execution"

[system_prompt]
role = "You execute token swaps on supported chains."
expertise = [
    "Multi-hop swap routing via Uniswap Universal Router",
    "Slippage management and MEV protection",
    "Permit2 approval workflows",
    "Cross-chain bridge execution",
]
safety_rules = [
    "ALWAYS simulate before broadcast",
    "NEVER skip safety delegation for swaps above $1000",
    "NEVER execute without a valid quote less than 30 seconds old",
]
workflow = """
1. Parse swap intent (token pair, amount, max slippage)
2. Get quote from venue adapter
3. Simulate transaction in fork
4. Delegate to safety-guardian for approval
5. If approved: sign and broadcast
6. Wait for confirmation and record outcome
"""
```

**5.2.2 -- Create `risk-assessor.toml`**

Create `crates/roko-agent/archetypes/risk-assessor.toml`:

```toml
name = "risk-assessor"
description = "Independent risk evaluation returning APPROVE or VETO"
default_tier = "integrative"
mode = "reactive"
isolation = "in_process"
tool_categories = ["data", "intelligence", "safety"]
delegates_to = []
terminal = true
category = "strategy"

[system_prompt]
role = "You evaluate proposed trades and positions for risk."
expertise = [
    "Position sizing and portfolio exposure analysis",
    "Impermanent loss estimation",
    "Liquidity depth assessment",
    "Correlation and concentration risk",
]
safety_rules = [
    "ALWAYS return an explicit APPROVE or VETO with reasoning",
    "NEVER approve positions that exceed the portfolio concentration limit",
    "VETO any trade where simulation shows revert probability above 5%",
]
```

**5.2.3 -- Create `safety-guardian.toml`**

Create `crates/roko-agent/archetypes/safety-guardian.toml`:

```toml
name = "safety-guardian"
description = "Centralized safety oversight with no write-capable tools"
default_tier = "focused"
mode = "persistent"
isolation = "in_process"
tool_categories = ["safety"]
delegates_to = []
terminal = true
category = "infrastructure"

[system_prompt]
role = "You enforce safety constraints on proposed chain actions."
expertise = [
    "PolicyCage constraint validation",
    "Gas price anomaly detection",
    "Approved asset list enforcement",
    "Position limit enforcement",
]
safety_rules = [
    "NEVER approve a transaction that violates any PolicyCage constraint",
    "NEVER hold write-capable tool access",
    "ALWAYS log the full constraint check with pass/fail per rule",
]
```

**5.2.4 -- Create `pool-researcher.toml`**

Create `crates/roko-agent/archetypes/pool-researcher.toml`:

```toml
name = "pool-researcher"
description = "Pool analysis covering TVL, volume, fees, and VPIN"
default_tier = "focused"
mode = "reactive"
isolation = "in_process"
tool_categories = ["data", "intelligence"]
delegates_to = []
terminal = true
category = "research"

[system_prompt]
role = "You analyze liquidity pool characteristics and opportunities."
expertise = [
    "TVL and volume trend analysis",
    "Fee tier comparison across venues",
    "VPIN order flow toxicity measurement",
    "Tick-level liquidity distribution analysis",
]
safety_rules = [
    "NEVER recommend pools with TVL below $100k without explicit warning",
    "ALWAYS include VPIN score when available",
]
```

**5.2.5 -- Create `lp-strategist.toml`**

Create `crates/roko-agent/archetypes/lp-strategist.toml`:

```toml
name = "lp-strategist"
description = "Optimal LP range selection, fee tier choice, and rebalance timing"
default_tier = "integrative"
mode = "persistent"
isolation = "fly_machine"
tool_categories = ["data", "intelligence", "safety"]
delegates_to = ["pool-researcher", "risk-assessor"]
terminal = false
category = "strategy"

[system_prompt]
role = "You design and optimize liquidity provision strategies."
expertise = [
    "Concentrated liquidity range optimization",
    "Fee tier selection based on volume and volatility",
    "Rebalance timing using IL vs fee accrual tradeoff",
    "Multi-pool capital allocation",
]
safety_rules = [
    "ALWAYS delegate to risk-assessor before recommending position changes",
    "NEVER recommend ranges narrower than 2x current volatility",
]
```

**5.2.6 -- Add tool-category filtering to ToolDispatcher**

Extend `crates/roko-agent/src/dispatcher/mod.rs` to accept an optional `ArchetypeManifest` reference. Add a filtering step between resolve (step 2) and authorize (step 3):

```rust
/// Check whether a tool's domain category is allowed by the agent's archetype.
fn archetype_allows_tool(
    tool_name: &str,
    archetype: Option<&ArchetypeManifest>,
    category_map: &HashMap<String, ToolCategory>,
) -> bool {
    let Some(archetype) = archetype else { return true };
    let Some(category) = category_map.get(tool_name) else { return true };
    archetype.tool_categories.contains(category)
}
```

The `category_map` maps tool canonical names to their `ToolCategory`. This map is built once at boot from the tool registry.

**5.2.7 -- Wire archetype fragments into SystemPromptBuilder**

Add a method to `SystemPromptBuilder`:

```rust
/// Inject archetype-specific prompt fragments into layers 1 and 3.
pub fn with_archetype(mut self, manifest: &ArchetypeManifest) -> Self {
    // Layer 1: Prepend archetype role to role_identity
    // Layer 3: Inject expertise as domain context
    // Layer 7: Inject safety_rules as anti-patterns
    self
}
```

This is a builder method, not a breaking change.

**Warning**: The `ToolCategory` enum defined in 5.1 must match the category values used in the manifest TOML files. The serde `rename_all = "snake_case"` handles this. Do not introduce a separate string-based category system.

### Wiring

- `crates/roko-agent/archetypes/`: five new `.toml` files
- `crates/roko-agent/src/dispatcher/mod.rs`: add optional archetype-aware filtering
- `crates/roko-compose/src/system_prompt_builder.rs`: add `with_archetype` method

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_five_archetypes() {
        // Load the archetypes directory. Assert 5 manifests present.
        // Assert the registry validates without errors.
    }

    #[test]
    fn test_trade_executor_delegation_chain() {
        // Load registry. Assert trade-executor delegates to safety-guardian and risk-assessor.
        // Assert both targets are terminal. Assert depth is 2 (within limit of 3).
    }

    #[test]
    fn test_safety_guardian_has_no_write_tools() {
        // Load safety-guardian manifest.
        // Assert tool_categories contains only ToolCategory::Safety.
        // Assert delegates_to is empty and terminal is true.
    }

    #[test]
    fn test_archetype_tool_filtering() {
        // Create a mock tool with category "trading".
        // Assert archetype_allows_tool returns true for trade-executor.
        // Assert archetype_allows_tool returns false for pool-researcher.
    }

    #[test]
    fn test_system_prompt_fragments_injected() {
        // Build a SystemPromptBuilder with archetype fragments.
        // Assert the built prompt contains the archetype role text.
        // Assert the built prompt contains the safety rules.
    }
}
```

### Verification

```bash
cargo test -p roko-agent -- archetype
cargo test -p roko-compose -- system_prompt
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo clippy -p roko-compose --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-agent
cargo +nightly fmt --check -p roko-compose
```

### Acceptance Criteria

- [ ] Five archetype TOML files exist in `crates/roko-agent/archetypes/`
- [ ] Registry loads all five and validates without errors
- [ ] Delegation chain: trade-executor -> safety-guardian, risk-assessor (both terminal)
- [ ] Delegation chain: lp-strategist -> pool-researcher, risk-assessor
- [ ] `ToolDispatcher` rejects tool calls outside the archetype's tool categories
- [ ] `SystemPromptBuilder::with_archetype` injects role, expertise, and safety rules
- [ ] All tests pass, clippy clean, fmt clean

### Commit Message

```
feat(roko-agent): add first five DeFi archetypes with tool-category filtering
```

---

## Batch 5.3: Delegation DAG and tool profile resolver

> **Effort**: L | **Depends on**: 5.1, 5.2 | **Crate**: roko-agent
> **Branch**: `defi/batch-5.3-delegation-dag`

### Context

Batches 5.1 and 5.2 built the archetype manifest, registry, and first five archetypes. Each archetype declares `delegates_to` targets, and the registry validates that the graph is acyclic with max depth 3. But validation is not execution -- roko has no runtime mechanism to delegate work from one agent archetype to another.

The PRD's delegation DAG has three levels. Level 0 is the autonomous `golem-instance`. Level 1 archetypes (trade-executor, liquidity-manager, vault-manager) are invoked from the heartbeat loop. Level 2 archetypes (safety-guardian, risk-assessor, pool-researcher) are invoked by level 1 agents during their tool-call sequence. Level 3 is always terminal.

Roko's orchestrator in `crates/roko-cli/src/orchestrate.rs` dispatches agents based on task roles using a `RoleSystemPromptSpec`. It does not support inter-agent delegation within a single tick. The orchestrator treats each agent as independent.

This batch builds the `DelegationRouter` that resolves delegation at runtime: when an agent's tool call produces a `DelegateToArchetype` action, the router looks up the target archetype, assembles its prompt using the manifest's `SystemPromptFragments`, and dispatches the delegation as a sub-agent call. The router enforces the depth limit and prevents delegation cycles at runtime (not just at boot validation).

The tool profile resolver is the second piece. Each archetype has `tool_categories`, but the dispatcher needs a concrete list of tool names. The resolver maps `ToolCategory` values to canonical tool names from the registry, filtered by the archetype's categories. This replaces the static `RoleToolProfile` lookup for DeFi agents while preserving it for code-task agents.

### Read First

| File | Why |
|------|-----|
| `crates/roko-agent/src/archetype.rs` | From batches 5.1/5.2 -- manifest and registry |
| `crates/roko-agent/src/dispatcher/mod.rs` | ToolDispatcher -- where delegation actions originate |
| `crates/roko-cli/src/orchestrate.rs` (top 100 lines) | Orchestrator -- where delegation will be called from |
| `crates/roko-std/src/roles.rs:171-179` | `DomainToolProfile` -- existing tool resolution |
| `crates/roko-agent/src/composition.rs` | `AgentComposition`, `CompositeAgent` -- existing multi-agent pattern |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work Items

**5.3.1 -- Build `DelegationRouter`**

Create `crates/roko-agent/src/delegation.rs`:

```rust
//! Runtime delegation routing between agent archetypes.

use std::collections::HashSet;

use crate::archetype::{ArchetypeManifest, ArchetypeRegistry};

/// Maximum delegation depth enforced at runtime.
pub const MAX_DELEGATION_DEPTH: usize = 3;

/// A delegation request from one archetype to another.
#[derive(Debug, Clone)]
pub struct DelegationRequest {
    /// Source archetype name.
    pub from: String,
    /// Target archetype name.
    pub to: String,
    /// The payload to delegate (prompt or structured input).
    pub payload: String,
    /// Current depth in the delegation chain.
    pub depth: usize,
    /// Archetypes already visited in this chain (cycle detection).
    pub visited: HashSet<String>,
}

/// Result of resolving a delegation.
#[derive(Debug, Clone)]
pub enum DelegationResult {
    /// Delegation is valid. Contains the target's manifest for prompt assembly.
    Resolved {
        manifest: ArchetypeManifest,
        depth: usize,
    },
    /// Delegation was rejected.
    Rejected {
        reason: DelegationRejection,
    },
}

/// Why a delegation was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelegationRejection {
    /// Target archetype not found in registry.
    TargetNotFound(String),
    /// Source archetype is not allowed to delegate to target.
    NotAllowed { from: String, to: String },
    /// Delegation would exceed max depth.
    DepthExceeded { depth: usize, max: usize },
    /// Delegation would create a cycle.
    CycleDetected { chain: Vec<String> },
}

/// Routes delegation requests between archetypes at runtime.
pub struct DelegationRouter<'a> {
    registry: &'a ArchetypeRegistry,
}

impl<'a> DelegationRouter<'a> {
    pub fn new(registry: &'a ArchetypeRegistry) -> Self {
        Self { registry }
    }

    /// Resolve a delegation request. Returns the target manifest or a rejection.
    pub fn resolve(&self, request: &DelegationRequest) -> DelegationResult {
        // 1. Check target exists in registry
        // 2. Check source's delegates_to includes target
        // 3. Check depth < MAX_DELEGATION_DEPTH
        // 4. Check target not in visited set (cycle)
        // 5. Return Resolved with target manifest
        todo!()
    }
}
```

**5.3.2 -- Build `ToolProfileResolver`**

Add `crates/roko-agent/src/tool_profile.rs`:

```rust
//! Resolve archetype tool categories into concrete tool name lists.

use std::collections::{HashMap, HashSet};

use crate::archetype::{ArchetypeManifest, ToolCategory};

/// Maps tool categories to concrete tool names from the registry.
#[derive(Debug, Clone)]
pub struct ToolProfileResolver {
    /// Category -> set of tool canonical names.
    category_tools: HashMap<ToolCategory, HashSet<String>>,
}

impl ToolProfileResolver {
    /// Build a resolver from the tool registry.
    ///
    /// Each tool in the registry must be tagged with a `ToolCategory`.
    /// Tools without a category are placed in a default "data" bucket.
    pub fn from_tool_map(map: HashMap<ToolCategory, HashSet<String>>) -> Self {
        Self { category_tools: map }
    }

    /// Resolve the concrete tool names allowed for an archetype.
    pub fn resolve(&self, archetype: &ArchetypeManifest) -> HashSet<String> {
        let mut allowed = HashSet::new();
        for cat in &archetype.tool_categories {
            if let Some(tools) = self.category_tools.get(cat) {
                allowed.extend(tools.iter().cloned());
            }
        }
        allowed
    }

    /// Check whether a specific tool is allowed for an archetype.
    pub fn is_allowed(&self, tool_name: &str, archetype: &ArchetypeManifest) -> bool {
        self.resolve(archetype).contains(tool_name)
    }
}
```

**5.3.3 -- Define the `DelegateToArchetype` tool action**

Add a tool definition that agents can call to delegate:

```rust
/// Tool call that an agent emits to delegate work to another archetype.
pub const DELEGATE_TOOL_NAME: &str = "delegate_to_archetype";

/// Input schema for the delegate tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegateInput {
    /// Target archetype name.
    pub target: String,
    /// What the delegate should do.
    pub instruction: String,
    /// Context to pass to the delegate.
    #[serde(default)]
    pub context: Option<String>,
}
```

Register this as a tool in the tool registry so agents can call it. The handler routes through `DelegationRouter::resolve` and dispatches the sub-agent.

**5.3.4 -- Wire `DelegationRouter` into the orchestrator**

In `crates/roko-cli/src/orchestrate.rs`, when a tool call result contains a `DelegateToArchetype` action:

1. Build a `DelegationRequest` with the current archetype, target, and depth
2. Call `DelegationRouter::resolve`
3. If resolved: assemble the target's system prompt via `SystemPromptBuilder::with_archetype`, dispatch the sub-agent, and return the result to the parent agent
4. If rejected: return the rejection reason as a tool error

**5.3.5 -- Wire modules into `roko-agent/src/lib.rs`**

Add:
```rust
pub mod delegation;
pub mod tool_profile;
```

And re-exports:
```rust
pub use delegation::{DelegationRouter, DelegationRequest, DelegationResult};
pub use tool_profile::ToolProfileResolver;
```

**Warning**: The delegation router is intentionally synchronous for resolution. The actual sub-agent dispatch is async and happens in the orchestrator. Do not make `DelegationRouter::resolve` async -- it only checks the manifest graph.

### Wiring

- `crates/roko-agent/src/lib.rs`: add `pub mod delegation;` and `pub mod tool_profile;`
- `crates/roko-agent/src/delegation.rs`: new file
- `crates/roko-agent/src/tool_profile.rs`: new file
- `crates/roko-cli/src/orchestrate.rs`: handle `DelegateToArchetype` tool calls

### Tests

```rust
// delegation.rs tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_delegation_resolves() {
        // trade-executor -> safety-guardian. Depth 1. Assert Resolved.
    }

    #[test]
    fn test_delegation_to_non_delegate_rejected() {
        // pool-researcher -> trade-executor (not in delegates_to).
        // Assert Rejected with NotAllowed.
    }

    #[test]
    fn test_depth_exceeded_rejected() {
        // Set depth to MAX_DELEGATION_DEPTH. Assert Rejected with DepthExceeded.
    }

    #[test]
    fn test_cycle_detected_rejected() {
        // Insert source into visited set, then delegate to source.
        // Assert Rejected with CycleDetected.
    }
}

// tool_profile.rs tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_trade_executor_tools() {
        // Build resolver with known category->tool map.
        // Resolve trade-executor. Assert tools from data + trading + safety categories.
    }

    #[test]
    fn test_safety_guardian_gets_safety_only() {
        // Resolve safety-guardian. Assert only safety-category tools returned.
    }

    #[test]
    fn test_unknown_category_ignored() {
        // Archetype requests a category not in the resolver map.
        // Assert no panic, empty set for that category.
    }
}
```

### Verification

```bash
cargo test -p roko-agent -- delegation
cargo test -p roko-agent -- tool_profile
cargo clippy -p roko-agent --no-deps -- -D warnings
cargo clippy -p roko-cli --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-agent
cargo +nightly fmt --check -p roko-cli
```

### Acceptance Criteria

- [ ] `DelegationRouter::resolve` validates depth, cycles, and `delegates_to` membership
- [ ] `ToolProfileResolver::resolve` maps archetype categories to concrete tool names
- [ ] `DelegateToArchetype` tool is registered and callable by agents
- [ ] Orchestrator handles delegation tool calls by dispatching sub-agents
- [ ] Runtime cycle detection prevents A -> B -> A chains even when registry allows both
- [ ] Delegation depth counter increments and enforces MAX_DELEGATION_DEPTH
- [ ] All tests pass, clippy clean, fmt clean

### Commit Message

```
feat(roko-agent): add delegation router and tool profile resolver
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives used

- **Agent**: `ArchetypeManifest` is the definition record for an Agent primitive — it declares domain, tool profile, gate pipeline, model preferences, and behavioral constraints. The `ArchetypeRegistry` is the catalog of available Agent templates. `ToolProfileResolver` maps each archetype's capability categories to the concrete tool names an agent instance can invoke. The five named archetypes (trade-executor, risk-assessor, safety-guardian, pool-researcher, lp-strategist) are ready-made Agent definitions that users fork and configure rather than build from scratch.
- **Group**: `DelegationDag` defines a Group — a structured hierarchy of agents with declared delegation relationships, capability boundaries per link, and a depth limit. A trading desk is a Group: one coordinator agent with delegated sub-agents, each scoped to its archetype's tool profile.
- **Gate**: Each archetype manifest declares a required gate pipeline — the gates that must be present and passing before that agent type is allowed to take action. A trade-executor manifest requires `DeFiRiskGate` and `TxSimulatorGate`; a safety-guardian manifest requires `CircuitBreakerGate`.
- **Extension**: Each archetype manifest declares default extensions — the modifiers that run around tool calls via the `Extension` trait's hook points. These are the starting extensions for that role; users can add or override them in the Agent Composer. Extensions span all three Pi compatibility tiers (Tier 1 Pi-compatible JS tools, Tier 2 Roko-enhanced JS with heartbeat hooks, Tier 3 Roko-native Rust with 22 hooks).

### Authoring surfaces

- **Agent Composer Stage 2** — archetype selector replaces the plain domain picker; shows a rich card per archetype with tool profile preview, default gates, recommended model, and capability constraints; users pick an archetype to pre-populate the rest of the composer
- **Fleet → Templates** — browse, fork, and create archetype manifests; filter by domain (trading, risk, research, LP); see usage counts and community ratings
- **Agent Composer Stage 5** — delegation configuration panel appears when the selected archetype supports sub-agents; shows the delegation DAG with drag-and-drop assignment of archetypes to sub-agent slots

### Shareable artifacts

- Archetype manifests: complete agent templates with tool profiles, gates, extensions, and model preferences — the primary unit of sharing in the marketplace for DeFi agents
- Tool profiles: capability bundles per archetype, versioned independently so a new venue connector can be added to an existing profile without re-publishing the full manifest
- Delegation DAG templates: named desk and team configurations (e.g., "two-tier trading desk", "risk-gated research team") that users instantiate with their own agent selections

### Dashboard visibility

- **Fleet → Templates** — archetype gallery with domain badges, version history, and fork counts; searchable by capability
- **Fleet → Agents** — running agent list grouped by archetype with status badges; group view collapses delegation DAGs into expandable tree nodes
- **Agent Detail → Config** — full archetype manifest display with user overrides highlighted in a diff view
