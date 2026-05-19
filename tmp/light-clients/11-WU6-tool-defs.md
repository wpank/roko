# WU-6: Verified Tool Definitions

**Layer**: 1
**Depends on**: WU-1 (core types — VerifiedState)
**Blocks**: WU-9
**Estimated effort**: 1 hour
**Crate**: `crates/roko-chain`

---

## Overview

Add 5 new tool definitions for verified chain operations to the existing `CHAIN_DOMAIN_TOOLS` array. These extend the 17 existing tools with light-client-verified variants.

**Key insight**: The 17 existing tools are already defined in `crates/roko-chain/src/tools.rs` and dispatched via `crates/roko-cli/src/chain_handler.rs`. We're adding NEW tool definitions, not modifying existing ones.

---

## Pre-read

- `crates/roko-chain/src/tools.rs` — existing 17 tool definitions, `CHAIN_TOOL_COUNT`, `CHAIN_TOOL_NAMES`
- `crates/roko-core/src/tool/` — `ToolDef` struct definition (search for `pub struct ToolDef`)
- `06-WU1-core-types.md` — `VerifiedState<T>` type (returned by verified tools)

---

## Tasks

### 6.1 Update `CHAIN_TOOL_COUNT` and `CHAIN_TOOL_NAMES`

**File**: `crates/roko-chain/src/tools.rs`

Change count from 17 to 22:
```rust
pub const CHAIN_TOOL_COUNT: usize = 22;
```

Add 5 new names to `CHAIN_TOOL_NAMES`:
```rust
pub const CHAIN_TOOL_NAMES: [&str; CHAIN_TOOL_COUNT] = [
    // ... existing 17 ...
    "chain.wallet_export_address",
    "chain.post_insight",
    "chain.search_insights",
    "chain.confirm_insight",
    // NEW — verified operations
    "chain.verified_balance",
    "chain.verified_storage",
    "chain.verify_transfer",
    "chain.head",
    "chain.backends",
];
```

### 6.2 Add 5 new tool definition functions

Add after the existing tool definition functions in `tools.rs`:

```rust
fn verified_balance_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.verified_balance".into(),
        description: "Get a light-client verified native token balance. Returns VerifiedState with trust level and consensus proof metadata.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "string",
                    "description": "Account address (0x-prefixed hex)"
                },
                "network": {
                    "type": "string",
                    "description": "Chain backend name (e.g. 'tempo-mainnet'). Defaults to configured default."
                }
            },
            "required": ["address"]
        }),
        category: "chain.verified".into(),
        permission: ToolPermission::Read,
        timeout_ms: 30_000,
        concurrency: 4,
        idempotent: true,
        source: "roko-chain".into(),
        metadata: serde_json::json!({"layer": "verified", "trust": "cryptographic"}),
    }
}

fn verified_storage_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.verified_storage".into(),
        description: "Get a light-client verified storage slot value. Verifies via MPT proof against consensus-verified state root.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "string",
                    "description": "Contract address (0x-prefixed hex)"
                },
                "slot": {
                    "type": "string",
                    "description": "Storage slot (0x-prefixed hex, 32 bytes)"
                },
                "network": {
                    "type": "string",
                    "description": "Chain backend name"
                }
            },
            "required": ["address", "slot"]
        }),
        category: "chain.verified".into(),
        permission: ToolPermission::Read,
        timeout_ms: 30_000,
        concurrency: 4,
        idempotent: true,
        source: "roko-chain".into(),
        metadata: serde_json::json!({"layer": "verified"}),
    }
}

fn verify_transfer_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.verify_transfer".into(),
        description: "Verify that a specific transfer occurred on-chain with light-client proof. Returns verified receipt with trust level.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "tx_hash": {
                    "type": "string",
                    "description": "Transaction hash to verify (0x-prefixed hex)"
                },
                "network": {
                    "type": "string",
                    "description": "Chain backend name"
                }
            },
            "required": ["tx_hash"]
        }),
        category: "chain.verified".into(),
        permission: ToolPermission::Read,
        timeout_ms: 30_000,
        concurrency: 4,
        idempotent: true,
        source: "roko-chain".into(),
        metadata: serde_json::json!({"layer": "verified"}),
    }
}

fn head_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.head".into(),
        description: "Get the latest verified block header from a chain backend. Shows block number, hash, timestamp, and trust level.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "network": {
                    "type": "string",
                    "description": "Chain backend name (e.g. 'tempo-mainnet')"
                }
            },
            "required": []
        }),
        category: "chain.verified".into(),
        permission: ToolPermission::Read,
        timeout_ms: 10_000,
        concurrency: 8,
        idempotent: true,
        source: "roko-chain".into(),
        metadata: serde_json::json!({"layer": "verified"}),
    }
}

fn backends_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.backends".into(),
        description: "List configured chain backends with their consensus mechanism, chain ID, trust level, and health status.".into(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
        category: "chain.info".into(),
        permission: ToolPermission::Read,
        timeout_ms: 5_000,
        concurrency: 8,
        idempotent: true,
        source: "roko-chain".into(),
        metadata: serde_json::json!({"layer": "info"}),
    }
}
```

### 6.3 Add new tools to `CHAIN_DOMAIN_TOOLS` array

In the `LazyLock` initializer, append the 5 new tool defs after the existing 17:
```rust
pub static CHAIN_DOMAIN_TOOLS: LazyLock<[ToolDef; CHAIN_TOOL_COUNT]> = LazyLock::new(|| [
    // ... existing 17 ...
    confirm_insight_tool_def(),
    // NEW
    verified_balance_tool_def(),
    verified_storage_tool_def(),
    verify_transfer_tool_def(),
    head_tool_def(),
    backends_tool_def(),
]);
```

### 6.4 Check `ToolPermission` and `ToolDef` imports

Ensure the imports at the top of `tools.rs` include whatever permission type is used. Search for how existing tools define `permission` — it may be `ToolPermission::Read` or a string or an enum. Match the existing pattern exactly.

---

## Verification Checklist

- [ ] `CHAIN_TOOL_COUNT` is 22
- [ ] `CHAIN_TOOL_NAMES` has 22 entries
- [ ] `CHAIN_DOMAIN_TOOLS` has 22 elements
- [ ] Each new tool has name, description, parameters JSON schema, category, permission, timeout
- [ ] New tools are in the `"chain.verified"` and `"chain.info"` categories
- [ ] `cargo test -p roko-chain` passes (tool array assertions if any)
- [ ] `cargo clippy -p roko-chain --no-deps -- -D warnings` passes
