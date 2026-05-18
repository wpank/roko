# 04: Chain-Specific ISFR Tools

ISFR tools following the existing `ToolDef` + `LazyLock` pattern from `crates/roko-chain/src/tools.rs`. These tools let agents interact with the ISFR oracle through the standard tool system.

## Existing Pattern

From `tools.rs`:

```rust
pub static CHAIN_DOMAIN_TOOLS: LazyLock<[ToolDef; CHAIN_TOOL_COUNT]> = LazyLock::new(|| {
    [
        balance_tool_def(),
        transfer_tool_def(),
        // ...
    ]
});

fn balance_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.balance".into(),
        description: "Get the balance...".into(),
        parameters: ToolSchema::from_value(serde_json::json!({ /* JSON Schema */ })),
        category: ToolCategory::Network,
        permission: ToolPermission::networked(),
        timeout_ms: 30_000,
        source: ToolSource::Builtin,
        concurrency: ToolConcurrency::Parallel,
    }
}
```

## ISFR Tool Definitions

**File:** `crates/roko-chain/src/isfr_tools.rs` (new module)

```rust
//! ISFR domain tool definitions for oracle interaction.
//!
//! These [`ToolDef`] registrations define the ISFR-specific tools that let
//! agents query rates, submit observations, check reputation, and monitor
//! the oracle state. They follow the same pattern as the chain domain tools
//! in `tools.rs`.

use roko_core::tool::{
    ToolCategory, ToolConcurrency, ToolDef, ToolPermission, ToolSchema, ToolSource,
};
use std::sync::LazyLock;

/// Number of ISFR domain tools.
pub const ISFR_TOOL_COUNT: usize = 8;

/// All ISFR domain tool definitions.
pub static ISFR_DOMAIN_TOOLS: LazyLock<[ToolDef; ISFR_TOOL_COUNT]> = LazyLock::new(|| {
    [
        read_rates_tool_def(),
        read_rate_history_tool_def(),
        submit_rate_tool_def(),
        submit_range_rate_tool_def(),
        check_reputation_tool_def(),
        oracle_status_tool_def(),
        source_status_tool_def(),
        bounty_info_tool_def(),
    ]
});

/// Canonical names of the ISFR domain tools.
pub const ISFR_TOOL_NAMES: [&str; ISFR_TOOL_COUNT] = [
    "isfr.read_rates",
    "isfr.read_rate_history",
    "isfr.submit_rate",
    "isfr.submit_range_rate",
    "isfr.check_reputation",
    "isfr.oracle_status",
    "isfr.source_status",
    "isfr.bounty_info",
];
```

### Tool 1: `isfr.read_rates`

Read current ISFR composite and component rates.

```rust
fn read_rates_tool_def() -> ToolDef {
    ToolDef {
        name: "isfr.read_rates".into(),
        description: "Read the current ISFR composite rate and per-class component rates \
            (lending, structured, funding, staking). Returns rates in basis points, \
            confidence score, number of voters, and epoch number."
            .into(),
        parameters: ToolSchema::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "epoch": {
                    "type": "integer",
                    "description": "Epoch number to query. Omit for current epoch."
                }
            },
            "additionalProperties": false
        })),
        category: ToolCategory::Network,
        permission: ToolPermission::read_only(),
        timeout_ms: 15_000,
        source: ToolSource::Builtin,
        concurrency: ToolConcurrency::Parallel,
    }
}
```

**Handler returns:**
```json
{
    "epoch": 42,
    "composite_bps": 690,
    "lending_bps": 620,
    "structured_bps": 710,
    "funding_bps": 45,
    "staking_bps": 320,
    "confidence_bps": 8500,
    "voter_count": 3,
    "timestamp": 1713960000
}
```

### Tool 2: `isfr.read_rate_history`

Read historical ISFR rates from the oracle's ring buffer.

```rust
fn read_rate_history_tool_def() -> ToolDef {
    ToolDef {
        name: "isfr.read_rate_history".into(),
        description: "Read historical ISFR rates from the oracle's 256-epoch ring buffer. \
            Returns an array of rate snapshots, each with composite and component rates."
            .into(),
        parameters: ToolSchema::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "from_epoch": {
                    "type": "integer",
                    "description": "Start epoch (inclusive)."
                },
                "to_epoch": {
                    "type": "integer",
                    "description": "End epoch (inclusive). Omit for current."
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum entries to return. Default 10, max 256.",
                    "default": 10,
                    "maximum": 256
                }
            },
            "additionalProperties": false
        })),
        category: ToolCategory::Network,
        permission: ToolPermission::read_only(),
        timeout_ms: 15_000,
        source: ToolSource::Builtin,
        concurrency: ToolConcurrency::Parallel,
    }
}
```

### Tool 3: `isfr.submit_rate`

Submit a rate observation (fast path — single permissioned keeper).

```rust
fn submit_rate_tool_def() -> ToolDef {
    ToolDef {
        name: "isfr.submit_rate".into(),
        description: "Submit a rate observation to the ISFROracle contract. \
            Fast path: single permissioned keeper submits directly. \
            Requires KEEPER_ROLE on the oracle contract."
            .into(),
        parameters: ToolSchema::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "composite_bps": {
                    "type": "integer",
                    "description": "Composite rate in basis points."
                },
                "lending_bps": {
                    "type": "integer",
                    "description": "Lending class rate in basis points."
                },
                "structured_bps": {
                    "type": "integer",
                    "description": "Structured class rate in basis points."
                },
                "funding_bps": {
                    "type": "integer",
                    "description": "Funding class rate in basis points."
                },
                "staking_bps": {
                    "type": "integer",
                    "description": "Staking class rate in basis points."
                },
                "confidence_bps": {
                    "type": "integer",
                    "description": "Confidence score (0-10000)."
                }
            },
            "required": ["composite_bps", "lending_bps", "structured_bps",
                         "funding_bps", "staking_bps", "confidence_bps"],
            "additionalProperties": false
        })),
        category: ToolCategory::Network,
        permission: ToolPermission::networked(),
        timeout_ms: 60_000,
        source: ToolSource::Builtin,
        concurrency: ToolConcurrency::Sequential,
    }
}
```

### Tool 4: `isfr.submit_range_rate`

Submit a rate vote for a block range (coordinated path).

```rust
fn submit_range_rate_tool_def() -> ToolDef {
    ToolDef {
        name: "isfr.submit_range_rate".into(),
        description: "Submit a rate vote for a specific block range to the ISFROracle. \
            Part of the coordinated block-range voting path. Multiple keepers \
            vote, quorum triggers aggregation and on-chain submission."
            .into(),
        parameters: ToolSchema::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "range_start": {
                    "type": "integer",
                    "description": "Start block of the range."
                },
                "range_end": {
                    "type": "integer",
                    "description": "End block of the range."
                },
                "composite_bps": {
                    "type": "integer",
                    "description": "Composite rate in basis points."
                },
                "components": {
                    "type": "array",
                    "items": { "type": "integer" },
                    "description": "Component rates [lending, structured, funding, staking] in bps.",
                    "minItems": 4,
                    "maxItems": 4
                },
                "confidence_bps": {
                    "type": "integer",
                    "description": "Confidence score (0-10000)."
                }
            },
            "required": ["range_start", "range_end", "composite_bps",
                         "components", "confidence_bps"],
            "additionalProperties": false
        })),
        category: ToolCategory::Network,
        permission: ToolPermission::networked(),
        timeout_ms: 60_000,
        source: ToolSource::Builtin,
        concurrency: ToolConcurrency::Sequential,
    }
}
```

### Tool 5: `isfr.check_reputation`

Check a keeper's reputation and eligibility in the WorkerRegistry.

```rust
fn check_reputation_tool_def() -> ToolDef {
    ToolDef {
        name: "isfr.check_reputation".into(),
        description: "Check a keeper's reputation score, tier, and eligibility \
            in the WorkerRegistry contract. Returns EMA reputation, current tier \
            (1-4), probation status, and whether the keeper can submit rates."
            .into(),
        parameters: ToolSchema::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "string",
                    "description": "Keeper wallet address (0x-prefixed)."
                }
            },
            "required": ["address"],
            "additionalProperties": false
        })),
        category: ToolCategory::Network,
        permission: ToolPermission::read_only(),
        timeout_ms: 15_000,
        source: ToolSource::Builtin,
        concurrency: ToolConcurrency::Parallel,
    }
}
```

### Tool 6: `isfr.oracle_status`

Get overall oracle status — current epoch, phase, voter count.

```rust
fn oracle_status_tool_def() -> ToolDef {
    ToolDef {
        name: "isfr.oracle_status".into(),
        description: "Get the current status of the ISFROracle — current epoch, \
            clearing phase (commit/reveal/solve/certificate/verify/settle), \
            number of registered voters, pending ranges, and bounty pool balance."
            .into(),
        parameters: ToolSchema::from_value(serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })),
        category: ToolCategory::Network,
        permission: ToolPermission::read_only(),
        timeout_ms: 15_000,
        source: ToolSource::Builtin,
        concurrency: ToolConcurrency::Parallel,
    }
}
```

### Tool 7: `isfr.source_status`

Check liveness and last reading of individual rate sources.

```rust
fn source_status_tool_def() -> ToolDef {
    ToolDef {
        name: "isfr.source_status".into(),
        description: "Check the status of individual ISFR rate sources — whether \
            each source (Aave, Compound, Ethena, staking) is live, stale, or offline, \
            with their last reading and liveness timeout."
            .into(),
        parameters: ToolSchema::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Source name to query. Omit for all sources."
                }
            },
            "additionalProperties": false
        })),
        category: ToolCategory::Network,
        permission: ToolPermission::read_only(),
        timeout_ms: 15_000,
        source: ToolSource::Builtin,
        concurrency: ToolConcurrency::Parallel,
    }
}
```

### Tool 8: `isfr.bounty_info`

Query bounty pool balance and reward distribution.

```rust
fn bounty_info_tool_def() -> ToolDef {
    ToolDef {
        name: "isfr.bounty_info".into(),
        description: "Query the ISFRBountyPool — total balance, per-range rewards, \
            claimable amounts for a specific keeper, and distribution history."
            .into(),
        parameters: ToolSchema::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "keeper_address": {
                    "type": "string",
                    "description": "Keeper address to check claimable rewards. Omit for pool overview."
                },
                "range_start": {
                    "type": "integer",
                    "description": "Start block of a specific range to query rewards for."
                },
                "range_end": {
                    "type": "integer",
                    "description": "End block of a specific range to query rewards for."
                }
            },
            "additionalProperties": false
        })),
        category: ToolCategory::Network,
        permission: ToolPermission::read_only(),
        timeout_ms: 15_000,
        source: ToolSource::Builtin,
        concurrency: ToolConcurrency::Parallel,
    }
}
```

## Tool Handlers

**File:** `crates/roko-chain/src/isfr_tool_handlers.rs` (new)

Tool handlers implement the actual logic behind each tool definition. They use the `ChainClient` trait for EVM calls.

```rust
use roko_core::tool::{ToolContext, ToolHandler, ToolResult};
use async_trait::async_trait;

/// Handler for all ISFR domain tools.
pub struct ISFRToolHandler {
    /// Chain client for contract calls.
    client: Arc<dyn ChainClient>,
    /// ISFROracle contract address.
    oracle_address: Address,
    /// ISFRBountyPool contract address.
    bounty_pool_address: Address,
    /// WorkerRegistry contract address.
    worker_registry_address: Address,
    /// Local keeper state (for source_status).
    keeper: Option<Arc<ISFRKeeper>>,
}

#[async_trait]
impl ToolHandler for ISFRToolHandler {
    async fn handle(&self, tool_name: &str, params: Value, ctx: &ToolContext) -> ToolResult {
        match tool_name {
            "isfr.read_rates" => self.handle_read_rates(params).await,
            "isfr.read_rate_history" => self.handle_read_rate_history(params).await,
            "isfr.submit_rate" => self.handle_submit_rate(params).await,
            "isfr.submit_range_rate" => self.handle_submit_range_rate(params).await,
            "isfr.check_reputation" => self.handle_check_reputation(params).await,
            "isfr.oracle_status" => self.handle_oracle_status(params).await,
            "isfr.source_status" => self.handle_source_status(params).await,
            "isfr.bounty_info" => self.handle_bounty_info(params).await,
            _ => ToolResult::error(format!("unknown ISFR tool: {tool_name}")),
        }
    }
}

impl ISFRToolHandler {
    async fn handle_read_rates(&self, params: Value) -> ToolResult {
        // Call ISFROracle.getCurrentRate() or ISFROracle.getRate(epoch)
        // Decode the return value into CompositeRate JSON
        // Return ToolResult::ok(json)
        todo!()
    }

    async fn handle_submit_rate(&self, params: Value) -> ToolResult {
        // Call ISFROracle.submitRate(composite, components, confidence)
        // Return tx hash on success
        todo!()
    }

    // ... other handlers follow the same pattern
}
```

## Registration with ToolRegistry

```rust
// In the agent startup or wherever tools are registered:
use roko_chain::isfr_tools::ISFR_DOMAIN_TOOLS;

// Register ISFR tools alongside chain domain tools
for tool_def in ISFR_DOMAIN_TOOLS.iter() {
    registry.register(tool_def.clone(), isfr_handler.clone());
}
```

This uses the same `DynamicToolRegistry` that MCP tools use — no special registration path.

## ABI Generation

Tool handlers need Solidity ABIs for contract calls. Generate from demo-ide contracts:

```bash
# From demo-ide/demo/contracts/
forge build
# ABIs at demo/contracts/out/ISFROracle.sol/ISFROracle.json
# Use alloy's sol! macro or generate Rust bindings
```

See `05-contracts-deployment.md` for full ABI generation and integration.

## Testing

```bash
# Unit tests for tool definitions (schema validation)
cargo test -p roko-chain -- isfr_tool_defs

# Handler tests against mock chain client
cargo test -p roko-chain -- isfr_tool_handlers

# Integration test against a live chain (mirage, daeji devnet, etc.)
cargo test -p roko-chain -- isfr_tools_chain --ignored
```
