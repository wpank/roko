# WU-4: RPC-Only Verifier

**Layer**: 1
**Depends on**: WU-1 (core types), WU-3 (ChainHeader state_root)
**Blocks**: WU-7, WU-10
**Estimated effort**: 1 hour
**Crate**: `crates/roko-chain`

---

## Overview

Implement `RpcOnlyVerifier` — a `ConsensusVerifier` that trusts the RPC provider. No consensus proof verification. This is the fallback for chains without a light-client protocol, local devnets, and the default when no consensus mechanism is configured.

Also wire the `PlaybackAdapter` and `RpcOnlyAdapter` in `adapter.rs` to use their real verifiers.

---

## Pre-read

- `06-WU1-core-types.md` — `ConsensusVerifier` trait, `TrustedHeader`, `TrustLevel`
- `07-WU2-playback.md` — `PlaybackVerifier` (similar pattern)
- `crates/roko-chain/src/client.rs` — `ChainClient` trait
- `crates/roko-chain/src/adapter.rs` — `RpcOnlyAdapter` stub (from WU-1)

---

## Tasks

### 4.1 Add `RpcOnlyVerifier` to `adapter.rs`

**File**: `crates/roko-chain/src/adapter.rs`

Add the verifier implementation:

```rust
use std::sync::Arc;
use async_trait::async_trait;
use crate::client::ChainClient;
use crate::consensus::{
    ConsensusError, ConsensusProof, ConsensusVerifier, TrustLevel, TrustedHeader,
};
use crate::types::{BlockNumber, ChainError};

/// Consensus verifier that trusts the RPC provider without independent verification.
/// Fetches block headers via the ChainClient and returns them as trusted.
pub struct RpcOnlyVerifier {
    client: Arc<dyn ChainClient>,
}

impl RpcOnlyVerifier {
    /// Create a new RPC-only verifier backed by the given chain client.
    pub fn new(client: Arc<dyn ChainClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ConsensusVerifier for RpcOnlyVerifier {
    async fn verify_finality(&self, block: BlockNumber) -> Result<TrustedHeader, ConsensusError> {
        let header = self.client.get_block_header(block).await?;

        // Parse hex strings to bytes — if parsing fails, use zeroed arrays
        let hash = parse_hex_to_bytes32(&header.hash).unwrap_or([0u8; 32]);
        let state_root = parse_hex_to_bytes32(&header.state_root).unwrap_or([0u8; 32]);

        Ok(TrustedHeader {
            number: header.number,
            hash,
            state_root,
            timestamp: header.timestamp,
            consensus_proof: ConsensusProof::RpcTrusted,
        })
    }

    async fn latest_finalized(&self) -> Result<TrustedHeader, ConsensusError> {
        let block = self.client.block_number().await?;
        self.verify_finality(block).await
    }

    fn mechanism(&self) -> &str { "rpc" }

    fn trust_level(&self) -> TrustLevel { TrustLevel::RpcTrusted }

    async fn is_healthy(&self) -> bool {
        self.client.block_number().await.is_ok()
    }
}

/// Parse a 0x-prefixed hex string into [u8; 32]. Returns None on failure.
fn parse_hex_to_bytes32(hex_str: &str) -> Option<[u8; 32]> {
    let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(stripped).ok()?;
    if bytes.len() != 32 { return None; }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}
```

### 4.2 Update `RpcOnlyAdapter::create_verifier()`

In the same file, update the adapter to use the real verifier:

```rust
impl ChainAdapter for RpcOnlyAdapter {
    fn consensus_type(&self) -> &str { "rpc" }

    fn create_verifier(
        &self,
        config: &ChainBackendConfig,
    ) -> Result<Arc<dyn ConsensusVerifier>, ChainError> {
        // RpcOnlyVerifier needs a ChainClient, but the adapter doesn't have one.
        // The factory function that calls this must provide the client separately.
        // For now, return an error — the actual construction happens in the backend factory (WU-10).
        Err(ChainError::Unsupported(
            "RpcOnlyAdapter requires a ChainClient — use create_rpc_verifier() instead".into()
        ))
    }
}

/// Create an RPC-only verifier with the given chain client.
/// This bypasses the adapter pattern since RpcOnlyVerifier needs the client at construction.
pub fn create_rpc_verifier(client: Arc<dyn ChainClient>) -> Arc<dyn ConsensusVerifier> {
    Arc::new(RpcOnlyVerifier::new(client))
}
```

### 4.3 Update `PlaybackAdapter::create_verifier()`

Wire the real `PlaybackVerifier`:

```rust
impl ChainAdapter for PlaybackAdapter {
    fn consensus_type(&self) -> &str { "playback" }

    fn create_verifier(
        &self,
        config: &ChainBackendConfig,
    ) -> Result<Arc<dyn ConsensusVerifier>, ChainError> {
        let path = config.playback_file.as_ref()
            .ok_or_else(|| ChainError::Rpc("playback_file required for playback adapter".into()))?;
        Ok(Arc::new(crate::playback::PlaybackVerifier::from_file(path)?))
    }
}
```

### 4.4 Export `create_rpc_verifier` from lib.rs

Add to pub use section:
```rust
pub use adapter::create_rpc_verifier;
```

### 4.5 Tests

Add to `adapter.rs` tests:
```rust
#[tokio::test(flavor = "current_thread")]
async fn rpc_only_verifier_returns_header() {
    let mock = crate::MockChainClient::local();
    mock.mine_empty_block();
    mock.mine_empty_block();
    let verifier = RpcOnlyVerifier::new(Arc::new(mock));

    let h = verifier.verify_finality(1).await.unwrap();
    assert_eq!(h.number, 1);
    assert!(matches!(h.consensus_proof, ConsensusProof::RpcTrusted));
    assert_eq!(verifier.trust_level(), TrustLevel::RpcTrusted);
    assert_eq!(verifier.mechanism(), "rpc");
}

#[tokio::test(flavor = "current_thread")]
async fn rpc_only_latest_finalized() {
    let mock = crate::MockChainClient::local();
    mock.mine_empty_block(); // block 1
    mock.mine_empty_block(); // block 2
    let verifier = RpcOnlyVerifier::new(Arc::new(mock));

    let h = verifier.latest_finalized().await.unwrap();
    assert_eq!(h.number, 2);
}

#[tokio::test(flavor = "current_thread")]
async fn rpc_only_is_healthy() {
    let mock = crate::MockChainClient::local();
    let verifier = RpcOnlyVerifier::new(Arc::new(mock));
    assert!(verifier.is_healthy().await);
}

#[test]
fn create_rpc_verifier_helper() {
    let mock = crate::MockChainClient::local();
    let verifier = create_rpc_verifier(Arc::new(mock));
    assert_eq!(verifier.mechanism(), "rpc");
}
```

---

## Verification Checklist

- [ ] `RpcOnlyVerifier` implements `ConsensusVerifier`
- [ ] `verify_finality()` fetches header from ChainClient and converts to TrustedHeader
- [ ] `latest_finalized()` calls `block_number()` then `verify_finality()`
- [ ] `is_healthy()` returns true when mock client is available
- [ ] `create_rpc_verifier()` convenience function exported
- [ ] `PlaybackAdapter::create_verifier()` uses real `PlaybackVerifier`
- [ ] `cargo test -p roko-chain` passes
- [ ] `cargo clippy -p roko-chain --no-deps -- -D warnings` passes
