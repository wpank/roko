# C3: Implement ISFR Domain Tools

## Context

roko agents interact with ISFR via the tool system. 4 tools following the `ToolDef` +
`ToolHandler` pattern used by builtins in `crates/roko-std/src/tool/builtin/`.

Tools to implement:
1. `isfr.read_rates` — Get the current composite rate and per-class breakdown.
2. `isfr.read_rate_history` — Get historical rates from the 256-epoch ring buffer.
3. `isfr.oracle_status` — Get oracle epoch, voter count, bounty balance.
4. `isfr.source_status` — Check individual source liveness and last reading.

## Pre-Check

```bash
# See how builtin tools are structured (read_file is the canonical reference).
cat crates/roko-std/src/tool/builtin/read_file.rs

# Understand what mod.rs does and where to add new entries.
cat crates/roko-std/src/tool/builtin/mod.rs

# roko-std already depends on roko-chain (where ISFRKeeper lives).
grep "roko-chain" crates/roko-std/Cargo.toml
```

## Files to Create

- `crates/roko-std/src/tool/builtin/isfr.rs` (NEW)

## Files to Modify

- `crates/roko-std/src/tool/builtin/mod.rs` — add `pub mod isfr;` only.

**Important**: `mod.rs` maintains `ROKO_BUILTIN_TOOLS` and `BUILTIN_TOOL_NAMES` as
`LazyLock` statics built from individual `tool_def()` functions. For ISFR, we use the same
approach as chain tools: the 4 ISFR `ToolDef`s are published as a module-level static and
extended into `ROKO_BUILTIN_TOOLS`. No handler registration happens in `mod.rs` — handlers
are resolved by the dispatcher separately.

## Cargo.toml Changes

None. `roko-std/Cargo.toml` already lists `roko-chain = { path = "../roko-chain" }`.

## Implementation

### Create `crates/roko-std/src/tool/builtin/isfr.rs`

```rust
//! ISFR domain tools — read rates, check source status, query oracle state.
//!
//! Tools follow the same `ToolDef` + `ToolHandler` pattern as other builtins.
//! Handlers currently return mock/stub data; real data flows once the
//! ISFRKeeper is running and state-sharing is wired (post-C2).
//!
//! Registration: add `isfr::tool_def_*()` calls to `ROKO_BUILTIN_TOOLS` in
//! `builtin/mod.rs`, and add handler cases in the dispatcher's tool lookup.

use async_trait::async_trait;
use roko_core::tool::{
    ToolCall, ToolCategory, ToolConcurrency, ToolDef, ToolError, ToolHandler, ToolPermission,
    ToolResult, ToolSchema,
};

// ─── Tool names ───────────────────────────────────────────────────────────────

pub const ISFR_READ_RATES: &str = "isfr.read_rates";
pub const ISFR_READ_RATE_HISTORY: &str = "isfr.read_rate_history";
pub const ISFR_ORACLE_STATUS: &str = "isfr.oracle_status";
pub const ISFR_SOURCE_STATUS: &str = "isfr.source_status";

// ─── Tool definitions ─────────────────────────────────────────────────────────

/// `isfr.read_rates` — get the current ISFR composite rate.
pub fn tool_def_read_rates() -> ToolDef {
    ToolDef::new(
        ISFR_READ_RATES,
        "Read the current ISFR composite rate and per-class breakdown \
         (lending, structured, funding, staking). Returns the most recently \
         computed epoch result.",
        ToolCategory::Network,
        ToolPermission::networked(),
    )
    .with_parameters(ToolSchema::from_value(serde_json::json!({
        "type": "object",
        "properties": {
            "epoch": {
                "type": "integer",
                "description": "Epoch number to query. Omit for the latest."
            }
        },
        "additionalProperties": false
    })))
    .with_concurrency(ToolConcurrency::Parallel)
    .with_idempotent(true)
    .with_timeout_ms(5_000)
}

/// `isfr.read_rate_history` — get historical ISFR rates.
pub fn tool_def_read_rate_history() -> ToolDef {
    ToolDef::new(
        ISFR_READ_RATE_HISTORY,
        "Read historical ISFR composite rates. Returns a list of past epoch \
         results from the 256-epoch ring buffer.",
        ToolCategory::Network,
        ToolPermission::networked(),
    )
    .with_parameters(ToolSchema::from_value(serde_json::json!({
        "type": "object",
        "properties": {
            "from_epoch": {
                "type": "integer",
                "description": "Start epoch (inclusive)."
            },
            "to_epoch": {
                "type": "integer",
                "description": "End epoch (inclusive). Defaults to current."
            },
            "limit": {
                "type": "integer",
                "description": "Maximum entries to return. Default 10, max 256."
            }
        },
        "additionalProperties": false
    })))
    .with_concurrency(ToolConcurrency::Parallel)
    .with_idempotent(true)
    .with_timeout_ms(10_000)
}

/// `isfr.oracle_status` — get the oracle's current epoch and voter state.
pub fn tool_def_oracle_status() -> ToolDef {
    ToolDef::new(
        ISFR_ORACLE_STATUS,
        "Get ISFROracle status: current epoch, active voter count, \
         pending range submissions, and bounty pool balance.",
        ToolCategory::Network,
        ToolPermission::networked(),
    )
    .with_parameters(ToolSchema::from_value(serde_json::json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    })))
    .with_concurrency(ToolConcurrency::Parallel)
    .with_idempotent(true)
    .with_timeout_ms(5_000)
}

/// `isfr.source_status` — check individual rate source liveness.
pub fn tool_def_source_status() -> ToolDef {
    ToolDef::new(
        ISFR_SOURCE_STATUS,
        "Check ISFR rate source liveness and the last reported reading. \
         Returns status for all sources, or a single source if `source` is given.",
        ToolCategory::Network,
        ToolPermission::networked(),
    )
    .with_parameters(ToolSchema::from_value(serde_json::json!({
        "type": "object",
        "properties": {
            "source": {
                "type": "string",
                "description": "Source name to filter on. Omit to list all sources."
            }
        },
        "additionalProperties": false
    })))
    .with_concurrency(ToolConcurrency::Parallel)
    .with_idempotent(true)
    .with_timeout_ms(5_000)
}

/// All ISFR tool definitions in registration order.
pub fn all_tool_defs() -> Vec<ToolDef> {
    vec![
        tool_def_read_rates(),
        tool_def_read_rate_history(),
        tool_def_oracle_status(),
        tool_def_source_status(),
    ]
}

/// Canonical names of ISFR tools, in `all_tool_defs()` order.
pub const ISFR_TOOL_NAMES: [&str; 4] = [
    ISFR_READ_RATES,
    ISFR_READ_RATE_HISTORY,
    ISFR_ORACLE_STATUS,
    ISFR_SOURCE_STATUS,
];

// ─── Handler ──────────────────────────────────────────────────────────────────

/// Handler for all ISFR tools.
///
/// Currently returns stub/mock data for all tools. Real data flows once:
/// - The ISFRKeeper is running (provides live readings via `current_rate()`).
/// - State-sharing is wired (keeper instance accessible from handler context).
///
/// To wire live data: store `Arc<ISFRKeeper>` in this struct and call
/// `self.keeper.current_rate()` / `self.keeper.source_metas()` in `execute`.
#[derive(Debug, Clone, Default)]
pub struct ISFRHandler;

impl ISFRHandler {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ToolHandler for ISFRHandler {
    fn name(&self) -> &str {
        // This handler responds to multiple tool names.
        // The dispatcher routes by name; returning a sentinel here is fine.
        "isfr.*"
    }

    async fn execute(&self, call: roko_core::tool::ToolCall, _ctx: &roko_core::tool::ToolContext) -> ToolResult {
        match call.name.as_str() {
            ISFR_READ_RATES => {
                let epoch = call.arguments.get("epoch").and_then(|v| v.as_u64());
                ToolResult::structured(serde_json::json!({
                    "epoch": epoch.unwrap_or(1),
                    "composite_bps": 580,
                    "lending_bps": 620,
                    "structured_bps": 850,
                    "funding_bps": 0,
                    "staking_bps": 350,
                    "confidence_bps": 10_000,
                    "source_count": 4,
                    "note": "Stub — wire ISFRKeeper.current_rate() for live data."
                }).to_string())
            }

            ISFR_READ_RATE_HISTORY => {
                let _from = call.arguments.get("from_epoch").and_then(|v| v.as_u64()).unwrap_or(1);
                let _limit = call.arguments.get("limit").and_then(|v| v.as_u64()).unwrap_or(10);
                ToolResult::structured(serde_json::json!({
                    "rates": [],
                    "note": "Rate history not yet populated — keeper must run for at least one epoch."
                }).to_string())
            }

            ISFR_ORACLE_STATUS => {
                ToolResult::structured(serde_json::json!({
                    "current_epoch": 1,
                    "clearing_phase": false,
                    "voter_count": 4,
                    "pending_ranges": 0,
                    "bounty_balance": "10000.0",
                    "note": "Stub — wire ISFRRegistry for live oracle state."
                }).to_string())
            }

            ISFR_SOURCE_STATUS => {
                let filter = call.arguments.get("source").and_then(|v| v.as_str());
                let all_sources = serde_json::json!([
                    { "name": "mock-aave-v3", "class": "lending", "status": "live", "weight": 0.30, "rate_bps": 620 },
                    { "name": "mock-compound-v3", "class": "lending", "status": "live", "weight": 0.25, "rate_bps": 580 },
                    { "name": "mock-ethena-susde", "class": "structured", "status": "live", "weight": 0.20, "rate_bps": 850 },
                    { "name": "mock-eth-staking", "class": "staking", "status": "live", "weight": 0.25, "rate_bps": 350 }
                ]);
                let sources = if let Some(name) = filter {
                    let filtered: Vec<_> = all_sources
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|s| s.get("name").and_then(|n| n.as_str()) == Some(name))
                        .cloned()
                        .collect();
                    serde_json::Value::Array(filtered)
                } else {
                    all_sources
                };
                ToolResult::structured(serde_json::json!({
                    "sources": sources,
                    "note": "Stub — wire ISFRKeeper.source_metas() for live status."
                }).to_string())
            }

            _ => ToolResult::Err(ToolError::Other(
                format!("ISFR tool '{}' not found", call.name)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use roko_core::tool::{ToolCall, ToolContext};

    fn make_ctx() -> ToolContext {
        ToolContext::testing("/tmp/test-workdir")
    }

    fn make_call(name: &str, args: serde_json::Value) -> ToolCall {
        ToolCall::new("test-id", name, args)
    }

    #[test]
    fn tool_defs_have_correct_count() {
        assert_eq!(all_tool_defs().len(), ISFR_TOOL_NAMES.len());
    }

    #[test]
    fn tool_def_names_match_constants() {
        for (def, name) in all_tool_defs().iter().zip(ISFR_TOOL_NAMES.iter()) {
            assert_eq!(def.name, *name, "name mismatch at tool definition");
        }
    }

    #[test]
    fn read_tools_are_idempotent_and_parallel() {
        for def in all_tool_defs() {
            assert!(def.idempotent, "tool {} should be idempotent", def.name);
            assert_eq!(
                def.concurrency,
                ToolConcurrency::Parallel,
                "tool {} should be Parallel",
                def.name
            );
        }
    }

    #[tokio::test]
    async fn read_rates_returns_structured() {
        let handler = ISFRHandler::new();
        let ctx = make_ctx();
        let call = make_call(ISFR_READ_RATES, serde_json::json!({}));
        let result = handler.execute(call, &ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn source_status_filters_by_name() {
        let handler = ISFRHandler::new();
        let ctx = make_ctx();
        let call = make_call(
            ISFR_SOURCE_STATUS,
            serde_json::json!({ "source": "mock-aave-v3" }),
        );
        let result = handler.execute(call, &ctx).await;
        assert!(result.is_ok());
        if let roko_core::tool::ToolResult::Ok { content, .. } = result {
            let v: serde_json::Value = serde_json::from_str(&content).unwrap();
            let sources = v["sources"].as_array().unwrap();
            assert_eq!(sources.len(), 1);
            assert_eq!(sources[0]["name"], "mock-aave-v3");
        }
    }

    #[tokio::test]
    async fn unknown_tool_name_returns_err() {
        let handler = ISFRHandler::new();
        let ctx = make_ctx();
        let call = make_call("isfr.nonexistent", serde_json::json!({}));
        let result = handler.execute(call, &ctx).await;
        assert!(result.is_err());
    }
}
```

### Register in `crates/roko-std/src/tool/builtin/mod.rs`

Add the module declaration at the top of the `pub mod` block (alphabetical with others):

```rust
pub mod isfr;
```

Then extend `ROKO_BUILTIN_TOOLS` and `BUILTIN_TOOL_NAMES` to include the ISFR tools.

In `ROKO_BUILTIN_TOOLS` (after the `tools.extend(CHAIN_DOMAIN_TOOLS…)` line):
```rust
tools.extend(isfr::all_tool_defs());
```

In `BUILTIN_TOOL_NAMES` (after `names.extend_from_slice(&CHAIN_TOOL_NAMES)` line):
```rust
names.extend_from_slice(&isfr::ISFR_TOOL_NAMES);
```

Update the doc comment count from `16 std + 17 chain` to `16 std + 17 chain + 4 ISFR` and
update `TOOL_COUNT` from `33` to `37`.

**Note on handler registration**: `ISFRHandler` is a multi-tool handler (one struct handles
all 4 ISFR tools). The roko-agent dispatcher resolves handlers by name at dispatch time.
Register `ISFRHandler` in the dispatcher's handler map (in `roko-agent/src/dispatcher/mod.rs`
or wherever builtin handlers are registered) with one entry per tool name:

```rust
// Wherever builtin handlers are wired:
use roko_std::tool::builtin::isfr::{ISFRHandler, ISFR_TOOL_NAMES};
let isfr = Arc::new(ISFRHandler::new());
for name in ISFR_TOOL_NAMES {
    registry.register_handler(name, isfr.clone());
}
```

Check the existing handler registration pattern in the dispatcher before adding this — match
the exact API used for other builtins.

## Verification

```bash
cargo build -p roko-std
cargo test -p roko-std isfr

# Verify the tool count is right and ISFR tools appear:
cargo test -p roko-std builtin -- --nocapture 2>&1 | grep isfr
```

## Dependencies

- C1 (SourceReading, CompositeRate types used in future handler wiring)
- C2 (ISFRKeeper — referenced in handler comments; not yet wired, stubs return mock data)

## Critical Implementation Notes

### ToolError variants

The `ToolError` enum in roko-core does NOT have a `NotFound` variant. Valid variants:
`SchemaInvalid`, `PermissionDenied`, `CommandNotAllowed`, `NetworkBlocked`, `Timeout`,
`Cancelled`, `PathOutsideWorktree`, `HandlerPanic`, `Other`. Use `ToolError::Other(msg)`
for the unknown tool name fallback.

### Dispatcher wiring is deferred

The handler registration snippet in "Register in mod.rs" is for reference only.
The actual dispatcher wiring (adding ISFRHandler to the tool registry) is NOT part
of this task — it happens when the full tool dispatch path is connected. This task
only creates the handler code and registers the tool definitions.

### ToolResult::is_ok() / is_err()

Verify these methods exist on `ToolResult` before using in tests. If `ToolResult` is
an enum `Ok{..}/Err(..)`, you can pattern-match directly:
```rust
assert!(matches!(result, ToolResult::Ok { .. }));
assert!(matches!(result, ToolResult::Err(_)));
```

## Note on Stubs

All handler responses are stubs. The stub responses match the final wire shape exactly
(same JSON keys, same types) so downstream consumers (TUI, HTTP routes) can be written
against the schema before the keeper is connected. Real data flows in a follow-up task
when `Arc<ISFRKeeper>` is stored in `ISFRHandler` and called at dispatch time.
