# WU-13: Orchestrator Wiring

**Layer**: 4
**Depends on**: WU-10 (config/factory), WU-9 (tool handlers)
**Blocks**: none (leaf unit)
**Estimated effort**: 1-2 hours
**Crate**: `crates/roko-cli`

---

## Overview

Wire `VerifiedChainClient` and `BackendPool` into the `PlanRunner` in `orchestrate.rs`. This is the final connection point — it makes verified chain tools available to running agents. Without this, the handler code from WU-9 receives `None` for `verified_client` and always returns errors.

**Key principle**: "Wire, don't build." All pieces exist — we're connecting them.

---

## Pre-read

- `crates/roko-cli/src/orchestrate.rs` — Lines 2644-2647 (`chain_client`, `chain_wallet` fields), Lines 4364-4393 (AlloyChainClient construction), Lines 16059-16069 (resolver wiring with `chain_handler_map()`)
- `crates/roko-cli/src/chain_registry.rs` — `chain_handler_map()`, `chain_handler_map_with_rpc()` (now accepts `verified_client` from WU-9)
- `crates/roko-chain/src/backend_factory.rs` — `build_backend_pool()`, `BackendPool` (from WU-10)
- `crates/roko-core/src/config/chain.rs` — `ChainConfig::resolve_backends()` (from WU-10)

---

## Tasks

### 13.1 Add `BackendPool` field to `PlanRunner`

**File**: `crates/roko-cli/src/orchestrate.rs`

Add field near the existing chain fields (~line 2645):
```rust
/// Read-only chain client. `None` if `[chain] rpc_url` is not configured.
chain_client: Option<Arc<dyn ChainClient>>,
/// Signing wallet. `None` if `wallet_key` is not configured.
chain_wallet: Option<Arc<dyn ChainWallet>>,
/// Verified chain backend pool. Empty if no backends configured.
backend_pool: roko_chain::BackendPool,
```

Add import at top of file:
```rust
use roko_chain::{BackendPool, build_backend_pool};
```

### 13.2 Construct `BackendPool` in `PlanRunner::new()`

**File**: `crates/roko-cli/src/orchestrate.rs`

After the existing `chain_client` / `chain_wallet` construction (~line 4364-4393), add:

```rust
// Build verified chain backend pool from config
let backend_entries = roko_config.chain.resolve_backends();
let default_backend_name = roko_config.chain.default_backend_name().map(|s| s.to_string());
let backend_pool = build_backend_pool(
    &backend_entries,
    default_backend_name.as_deref(),
);
if !backend_pool.is_empty() {
    tracing::info!(
        backends = backend_pool.len(),
        "verified chain backends initialized"
    );
}
```

Wire into the `Self { ... }` struct initialization:
```rust
Ok(Self {
    // ... existing fields ...
    chain_client,
    chain_wallet,
    backend_pool,  // NEW
    // ... rest ...
})
```

### 13.3 Update resolver construction to pass `verified_client`

**File**: `crates/roko-cli/src/orchestrate.rs`

Find the resolver construction (~line 16059-16069). Currently:
```rust
let resolver: Arc<dyn HandlerResolver> = if self.chain_client.is_some() {
    let chain_map = chain_handler_map(
        Arc::clone(self.chain_client.as_ref().unwrap()),
        self.chain_wallet.clone(),
    );
    Arc::new(chain_aware_resolver(chain_map))
} else {
    Arc::new(|name: &str| -> Option<Arc<dyn ToolHandler>> {
        roko_std::tool::handlers::handler_for(name)
    })
};
```

Replace with:
```rust
let resolver: Arc<dyn HandlerResolver> = if self.chain_client.is_some() {
    let verified_client = self.backend_pool.default_verified_client();
    let rpc_url = roko_config.chain.rpc_url.clone();
    let chain_map = chain_handler_map_with_rpc(
        Arc::clone(self.chain_client.as_ref().unwrap()),
        self.chain_wallet.clone(),
        rpc_url,
        verified_client,
    );
    Arc::new(chain_aware_resolver(chain_map))
} else {
    Arc::new(|name: &str| -> Option<Arc<dyn ToolHandler>> {
        roko_std::tool::handlers::handler_for(name)
    })
};
```

Update the import to include `chain_handler_map_with_rpc`:
```rust
use crate::chain_registry::{chain_aware_resolver, chain_handler_map_with_rpc};
```

(Remove the old `chain_handler_map` import if it's no longer used elsewhere.)

### 13.4 Also wire `BackendPool` into sidecar construction

**Grep for sidecar construction** — the agent sidecar builder is typically called in the `agent serve` subcommand. Search:

```bash
grep -rn 'AgentServer::builder()' crates/roko-cli/src/ --include='*.rs'
```

At each call site, if a chain client is being passed, also consider wiring the backend pool's default client. The sidecar `AgentServerBuilder` already accepts `.chain_client()`. If a verified client exists, prefer passing it:

```rust
// Before (existing):
let builder = AgentServer::builder()
    .agent_id(&agent_id)
    .chain_client(Arc::clone(&chain_client));

// After (updated):
let chain_for_sidecar: Arc<dyn ChainClient> = match backend_pool.default_verified_client() {
    Some(vc) => vc as Arc<dyn ChainClient>,  // VerifiedChainClient implements ChainClient
    None => Arc::clone(&chain_client),
};
let builder = AgentServer::builder()
    .agent_id(&agent_id)
    .chain_client(chain_for_sidecar)
    .chain();  // enable chain feature routes
```

### 13.5 Handle `roko_config` access within resolver block

The resolver construction references `roko_config.chain.rpc_url` which may be in scope differently than `self.config`. Trace the variable:

1. In `PlanRunner::new()`, `roko_config` is a local `RokoConfig` loaded from disk
2. In `dispatch_agent_with()` (where the resolver is built), the config may be accessed differently

Search for the exact scope:
```bash
grep -n 'roko_config\|self.config' crates/roko-cli/src/orchestrate.rs | head -30
```

Ensure the `rpc_url` is available. If the resolver is built inside a method that doesn't have `roko_config`, store it as a field or pass it as a parameter. The simplest approach: store `rpc_url: Option<String>` on `PlanRunner` alongside the existing `chain_client` field.

Add to `PlanRunner` fields:
```rust
/// RPC URL for chain handler dispatch.
chain_rpc_url: Option<String>,
```

Initialize from config:
```rust
chain_rpc_url: roko_config.chain.rpc_url.clone(),
```

Then in the resolver construction:
```rust
let chain_map = chain_handler_map_with_rpc(
    Arc::clone(self.chain_client.as_ref().unwrap()),
    self.chain_wallet.clone(),
    self.chain_rpc_url.clone(),
    verified_client,
);
```

---

## Verification Checklist

- [ ] `PlanRunner` has `backend_pool: BackendPool` field
- [ ] `PlanRunner::new()` calls `build_backend_pool()` from config
- [ ] Resolver construction uses `chain_handler_map_with_rpc()` with `verified_client`
- [ ] Default verified client from pool is passed to handler map
- [ ] Import updated: `chain_handler_map` → `chain_handler_map_with_rpc`
- [ ] Sidecar construction uses `VerifiedChainClient` when available
- [ ] `rpc_url` is accessible in resolver scope (either via field or parameter)
- [ ] **Backward compatible**: When `[chain.backends]` is empty, behavior is identical to before
- [ ] `cargo build -p roko-cli` compiles
- [ ] `cargo test -p roko-cli` passes
- [ ] `cargo test --workspace` — no breakage
- [ ] Spot-check: `cargo run -p roko-cli -- doctor` still works (smoke test)
