# WU-9: Chain Tool Dispatch Handlers

**Layer**: 2
**Depends on**: WU-6 (verified tool defs)
**Blocks**: WU-12, WU-13
**Estimated effort**: 2-3 hours
**Crates**: `crates/roko-cli` (chain_handler.rs, chain_registry.rs)

---

## Overview

Extend the EXISTING `ChainToolHandler` and `chain_handler_map()` to dispatch the 5 new verified chain tools. The handler infrastructure already exists — we're adding new match arms and the `VerifiedChainClient` as an optional field.

**Critical discovery**: `ChainToolHandler` already exists at `crates/roko-cli/src/chain_handler.rs` with 17 tool dispatch arms. We're extending it, not creating it.

---

## Pre-read

- `crates/roko-cli/src/chain_handler.rs` — existing `ChainToolHandler` struct and `execute()` method
- `crates/roko-cli/src/chain_registry.rs` — `chain_handler_map()`, `chain_handler_map_with_rpc()`, `chain_aware_resolver()`
- `crates/roko-chain/src/tools.rs` — `CHAIN_TOOL_NAMES` (now 22 entries from WU-6)

---

## Tasks

### 9.1 Add `VerifiedChainClient` field to `ChainToolHandler`

**File**: `crates/roko-cli/src/chain_handler.rs`

Add optional verified client field:
```rust
pub struct ChainToolHandler {
    pub client: Arc<dyn ChainClient>,
    pub wallet: Option<Arc<dyn ChainWallet>>,
    pub tool_name: String,
    pub rpc_url: Option<String>,
    // NEW
    pub verified_client: Option<Arc<VerifiedChainClient>>,
}
```

Add import at top:
```rust
use roko_chain::VerifiedChainClient;
```

### 9.2 Add 5 new match arms to `execute()`

In the `execute()` method's match block, add after existing arms:
```rust
"chain.verified_balance" => self.handle_verified_balance(args).await,
"chain.verified_storage" => self.handle_verified_storage(args).await,
"chain.verify_transfer" => self.handle_verify_transfer(args).await,
"chain.head" => self.handle_head(args).await,
"chain.backends" => self.handle_backends(args).await,
```

### 9.3 Implement handler methods

Add these methods to `impl ChainToolHandler`:

```rust
async fn handle_verified_balance(&self, args: &Value) -> ToolResult {
    let vc = self.verified_client.as_ref()
        .ok_or_else(|| ToolError::Other("no verified chain client configured".into()))?;
    let address = args["address"].as_str()
        .ok_or_else(|| ToolError::Other("missing 'address' parameter".into()))?;
    let network = args.get("network").and_then(|v| v.as_str());
    let _ = network; // TODO: multi-network resolution in WU-10

    let vs = vc.verified_balance(address, None).await
        .map_err(|e| ToolError::Other(format!("verified balance failed: {e}")))?;

    ToolResult::structured(serde_json::json!({
        "address": address,
        "balance_wei": vs.data.to_string(),
        "balance_eth": format!("{:.6}", vs.data as f64 / 1e18),
        "trust_level": format!("{:?}", vs.trust_level),
        "consensus_mechanism": vs.consensus_mechanism,
        "block_number": vs.block_number,
        "network": vs.network,
    }))
}

async fn handle_verified_storage(&self, args: &Value) -> ToolResult {
    let vc = self.verified_client.as_ref()
        .ok_or_else(|| ToolError::Other("no verified chain client configured".into()))?;
    let address = args["address"].as_str()
        .ok_or_else(|| ToolError::Other("missing 'address'".into()))?;
    let slot = args["slot"].as_str()
        .ok_or_else(|| ToolError::Other("missing 'slot'".into()))?;

    let vs = vc.verified_storage(address, slot, None).await
        .map_err(|e| ToolError::Other(format!("verified storage failed: {e}")))?;

    ToolResult::structured(serde_json::json!({
        "address": address,
        "slot": slot,
        "value": hex::encode(&vs.data),
        "trust_level": format!("{:?}", vs.trust_level),
        "block_number": vs.block_number,
    }))
}

async fn handle_verify_transfer(&self, args: &Value) -> ToolResult {
    let vc = self.verified_client.as_ref()
        .ok_or_else(|| ToolError::Other("no verified chain client configured".into()))?;
    let tx_hash_str = args["tx_hash"].as_str()
        .ok_or_else(|| ToolError::Other("missing 'tx_hash'".into()))?;
    let tx_hash = roko_chain::TxHash::new(tx_hash_str);

    let vs = vc.verify_transfer(&tx_hash).await
        .map_err(|e| ToolError::Other(format!("verify transfer failed: {e}")))?;

    ToolResult::structured(serde_json::json!({
        "tx_hash": tx_hash_str,
        "status": vs.data.status,
        "block_number": vs.data.block_number,
        "gas_used": vs.data.gas_used,
        "trust_level": format!("{:?}", vs.trust_level),
        "consensus_mechanism": vs.consensus_mechanism,
    }))
}

async fn handle_head(&self, _args: &Value) -> ToolResult {
    let vc = self.verified_client.as_ref()
        .ok_or_else(|| ToolError::Other("no verified chain client configured".into()))?;

    let header = vc.consensus().latest_finalized().await
        .map_err(|e| ToolError::Other(format!("head failed: {e}")))?;

    ToolResult::structured(serde_json::json!({
        "block_number": header.number,
        "block_hash": hex::encode(header.hash),
        "state_root": hex::encode(header.state_root),
        "timestamp": header.timestamp,
        "consensus_mechanism": vc.consensus().mechanism(),
        "trust_level": format!("{:?}", vc.consensus().trust_level()),
    }))
}

async fn handle_backends(&self, _args: &Value) -> ToolResult {
    // TODO: List all configured backends from config (WU-10)
    // For now, return info about the current client
    let name = self.client.name();
    let chain_id = self.client.chain_id().await.unwrap_or(0);
    let verified = self.verified_client.is_some();

    ToolResult::structured(serde_json::json!({
        "backends": [{
            "name": name,
            "chain_id": chain_id,
            "verified": verified,
            "consensus": self.verified_client.as_ref()
                .map(|vc| vc.consensus().mechanism().to_string())
                .unwrap_or_else(|| "none".into()),
        }]
    }))
}
```

### 9.4 Update `chain_handler_map` functions in `chain_registry.rs`

**File**: `crates/roko-cli/src/chain_registry.rs`

Update `chain_handler_map_with_rpc()` to accept optional `VerifiedChainClient`:

```rust
pub fn chain_handler_map_with_rpc(
    client: Arc<dyn ChainClient>,
    wallet: Option<Arc<dyn ChainWallet>>,
    rpc_url: Option<String>,
    verified_client: Option<Arc<VerifiedChainClient>>,  // NEW
) -> HashMap<String, Arc<dyn ToolHandler>> {
    CHAIN_TOOL_NAMES
        .iter()
        .map(|&name| {
            let h: Arc<dyn ToolHandler> = Arc::new(ChainToolHandler {
                client: Arc::clone(&client),
                wallet: wallet.clone(),
                tool_name: name.to_string(),
                rpc_url: rpc_url.clone(),
                verified_client: verified_client.clone(),  // NEW
            });
            (name.to_string(), h)
        })
        .collect()
}
```

Update `chain_handler_map()` to pass `None` for verified_client (backward compatible):
```rust
pub fn chain_handler_map(
    client: Arc<dyn ChainClient>,
    wallet: Option<Arc<dyn ChainWallet>>,
) -> HashMap<String, Arc<dyn ToolHandler>> {
    chain_handler_map_with_rpc(client, wallet, None, None)
}
```

### 9.5 Update all call sites

Search for calls to `chain_handler_map` and `chain_handler_map_with_rpc` in `orchestrate.rs` and other files. Add the `verified_client` parameter. For now, pass `None` — WU-13 will wire the real `VerifiedChainClient`.

```bash
grep -rn 'chain_handler_map' crates/roko-cli/src/ --include='*.rs'
```

Each call site: add `None` as the last argument (or `verified_client.clone()` if the variable exists).

### 9.6 Add `hex` dependency to roko-cli if not present

Check `crates/roko-cli/Cargo.toml`. If `hex` isn't already a dependency, add:
```toml
hex = "0.4"
```

---

## Verification Checklist

- [ ] `ChainToolHandler` has `verified_client: Option<Arc<VerifiedChainClient>>` field
- [ ] 5 new match arms in `execute()` method
- [ ] Each handler checks for `verified_client` and returns clear error if missing
- [ ] `chain_handler_map()` still works with `None` verified client (backward compatible)
- [ ] `chain_handler_map_with_rpc()` accepts verified client parameter
- [ ] All existing call sites updated (grep shows no compiler errors)
- [ ] `cargo build -p roko-cli` compiles
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo test --workspace` — no breakage
