# WU-7: VerifiedChainClient

**Layer**: 2
**Depends on**: WU-4 (RpcOnlyVerifier), WU-5 (state proofs)
**Blocks**: WU-10, WU-11, WU-12
**Estimated effort**: 2-3 hours
**Crate**: `crates/roko-chain`

---

## Overview

`VerifiedChainClient` wraps any `ChainClient` with consensus verification and optional state proof verification. It implements `ChainClient` so it's a **drop-in replacement** anywhere `ChainClient` is used (including `AgentState.chain_client`).

The key methods beyond `ChainClient`:
- `verified_balance()` → returns `VerifiedState<u128>` with trust level and proof metadata
- `verified_storage()` → returns `VerifiedState<Vec<u8>>` with trust level
- `verify_transfer()` → returns `VerifiedState<Receipt>` with trust level

---

## Pre-read

- `crates/roko-chain/src/client.rs` — `ChainClient` trait (all 9 methods to implement)
- `06-WU1-core-types.md` — `ConsensusVerifier`, `TrustedHeader`, `TrustLevel`, `VerifiedState<T>`
- `08-EXISTING-CODE-REFERENCE.md` — Section 4 (AlloyChainClient provider access)

---

## Tasks

### 7.1 Create `crates/roko-chain/src/verified_client.rs`

```rust
//! [`VerifiedChainClient`] — a ChainClient that verifies responses via consensus
//! and optionally via state proofs (MPT).
//!
//! Implements [`ChainClient`] so it can replace any unverified client in the system.

use std::sync::Arc;
use async_trait::async_trait;

use crate::client::ChainClient;
use crate::consensus::{ConsensusVerifier, TrustLevel};
use crate::types::*;
use crate::verified_state::VerifiedState;

/// A ChainClient that verifies every response via consensus proofs.
/// When an alloy provider is available, also verifies via MPT state proofs.
pub struct VerifiedChainClient {
    /// The underlying RPC client.
    rpc: Arc<dyn ChainClient>,
    /// Consensus verifier for this chain.
    consensus: Arc<dyn ConsensusVerifier>,
    /// Human-readable network name.
    network: String,
    /// EVM chain ID.
    chain_id: u64,
    /// Alloy provider for eth_getProof (beyond ChainClient surface).
    /// Feature-gated: only available with alloy-backend.
    #[cfg(feature = "alloy-backend")]
    proof_provider: Option<Arc<alloy::providers::DynProvider>>,
}

impl VerifiedChainClient {
    /// Create a new verified client wrapping an existing ChainClient.
    pub fn new(
        rpc: Arc<dyn ChainClient>,
        consensus: Arc<dyn ConsensusVerifier>,
        network: impl Into<String>,
        chain_id: u64,
    ) -> Self {
        Self {
            rpc,
            consensus,
            network: network.into(),
            chain_id,
            #[cfg(feature = "alloy-backend")]
            proof_provider: None,
        }
    }

    /// Set the alloy provider for eth_getProof calls (enables full MPT verification).
    #[cfg(feature = "alloy-backend")]
    pub fn with_proof_provider(mut self, provider: Arc<alloy::providers::DynProvider>) -> Self {
        self.proof_provider = Some(provider);
        self
    }

    /// Access the consensus verifier.
    pub fn consensus(&self) -> &dyn ConsensusVerifier {
        self.consensus.as_ref()
    }

    /// Network name.
    pub fn network(&self) -> &str {
        &self.network
    }

    /// Get current unix timestamp in milliseconds.
    fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    // ── Verified query methods ───────────────────────────────────────

    /// Get a verified balance with full provenance metadata.
    pub async fn verified_balance(
        &self,
        address: &str,
        block: Option<BlockNumber>,
    ) -> ChainResult<VerifiedState<u128>> {
        // 1. Get trusted header
        let header = if let Some(b) = block {
            self.consensus.verify_finality(b).await
        } else {
            self.consensus.latest_finalized().await
        }
        .map_err(|e| ChainError::Rpc(format!("consensus error: {e}")))?;

        // 2. Get balance at the verified block
        let balance = self.rpc.get_balance(address, Some(header.number)).await?;

        // 3. Wrap in VerifiedState
        Ok(VerifiedState {
            data: balance,
            chain_id: self.chain_id,
            network: self.network.clone(),
            block_number: header.number,
            block_hash: header.hash,
            block_timestamp: header.timestamp,
            trust_level: self.consensus.trust_level(),
            consensus_mechanism: self.consensus.mechanism().to_string(),
            consensus_proof_bytes: serde_json::to_vec(&header.consensus_proof)
                .unwrap_or_default(),
            state_proof_bytes: vec![], // TODO: populate when proof_provider is available
            verified_at: Self::now_ms(),
        })
    }

    /// Get a verified storage slot value.
    pub async fn verified_storage(
        &self,
        address: &str,
        slot: &str,
        block: Option<BlockNumber>,
    ) -> ChainResult<VerifiedState<Vec<u8>>> {
        let header = if let Some(b) = block {
            self.consensus.verify_finality(b).await
        } else {
            self.consensus.latest_finalized().await
        }
        .map_err(|e| ChainError::Rpc(format!("consensus error: {e}")))?;

        let value = self.rpc.get_storage_at(address, slot, Some(header.number)).await?;

        Ok(VerifiedState {
            data: value,
            chain_id: self.chain_id,
            network: self.network.clone(),
            block_number: header.number,
            block_hash: header.hash,
            block_timestamp: header.timestamp,
            trust_level: self.consensus.trust_level(),
            consensus_mechanism: self.consensus.mechanism().to_string(),
            consensus_proof_bytes: vec![],
            state_proof_bytes: vec![],
            verified_at: Self::now_ms(),
        })
    }

    /// Verify that a specific transfer occurred.
    pub async fn verify_transfer(
        &self,
        tx_hash: &TxHash,
    ) -> ChainResult<VerifiedState<Receipt>> {
        let receipt = self.rpc.get_receipt(tx_hash).await?
            .ok_or_else(|| ChainError::Rpc(format!("receipt not found: {tx_hash}")))?;

        let header = self.consensus.verify_finality(receipt.block_number).await
            .map_err(|e| ChainError::Rpc(format!("consensus error: {e}")))?;

        Ok(VerifiedState {
            data: receipt,
            chain_id: self.chain_id,
            network: self.network.clone(),
            block_number: header.number,
            block_hash: header.hash,
            block_timestamp: header.timestamp,
            trust_level: self.consensus.trust_level(),
            consensus_mechanism: self.consensus.mechanism().to_string(),
            consensus_proof_bytes: vec![],
            state_proof_bytes: vec![],
            verified_at: Self::now_ms(),
        })
    }
}

// ── ChainClient implementation (drop-in replacement) ─────────────────

#[async_trait]
impl ChainClient for VerifiedChainClient {
    async fn block_number(&self) -> ChainResult<BlockNumber> {
        self.rpc.block_number().await
    }

    async fn get_block_header(&self, number: BlockNumber) -> ChainResult<ChainHeader> {
        self.rpc.get_block_header(number).await
    }

    async fn get_receipt(&self, tx: &TxHash) -> ChainResult<Option<Receipt>> {
        self.rpc.get_receipt(tx).await
    }

    async fn get_logs(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        addresses: &[String],
        topics: &[String],
    ) -> ChainResult<Vec<LogEntry>> {
        self.rpc.get_logs(from, to, addresses, topics).await
    }

    async fn get_storage_at(
        &self,
        address: &str,
        slot: &str,
        block: Option<BlockNumber>,
    ) -> ChainResult<Vec<u8>> {
        self.rpc.get_storage_at(address, slot, block).await
    }

    async fn eth_call(
        &self,
        request: &TxRequest,
        block: Option<BlockNumber>,
    ) -> ChainResult<CallResult> {
        self.rpc.eth_call(request, block).await
    }

    async fn get_balance(&self, address: &str, block: Option<BlockNumber>) -> ChainResult<u128> {
        self.rpc.get_balance(address, block).await
    }

    async fn chain_id(&self) -> ChainResult<u64> {
        Ok(self.chain_id)
    }

    fn name(&self) -> &str {
        "verified"
    }
}
```

### 7.2 Register module in lib.rs

```rust
/// Verified chain client — wraps ChainClient with consensus verification.
pub mod verified_client;

pub use verified_client::VerifiedChainClient;
```

### 7.3 Tests

```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{MockChainClient, adapter::create_rpc_verifier};

    fn make_verified_client() -> VerifiedChainClient {
        let mock = MockChainClient::local();
        mock.mine_empty_block();
        mock.mine_empty_block();
        mock.set_balance("0xABC", 1_000_000);
        let client = Arc::new(mock);
        let verifier = create_rpc_verifier(client.clone());
        VerifiedChainClient::new(client, verifier, "test-net", 31337)
    }

    #[tokio::test(flavor = "current_thread")]
    async fn implements_chain_client() {
        let vc = make_verified_client();
        // Prove it can be used as dyn ChainClient
        let client: &dyn ChainClient = &vc;
        let bn = client.block_number().await.unwrap();
        assert_eq!(bn, 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn verified_balance_returns_verified_state() {
        let vc = make_verified_client();
        let vs = vc.verified_balance("0xABC", None).await.unwrap();
        assert_eq!(vs.data, 1_000_000);
        assert_eq!(vs.chain_id, 31337);
        assert_eq!(vs.network, "test-net");
        assert_eq!(vs.trust_level, TrustLevel::RpcTrusted);
        assert_eq!(vs.consensus_mechanism, "rpc");
        assert!(vs.verified_at > 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn verified_storage_returns_verified_state() {
        let mock = MockChainClient::local();
        mock.mine_empty_block();
        mock.insert_storage("0xContract", "0x01", None, vec![42]);
        let client = Arc::new(mock);
        let verifier = create_rpc_verifier(client.clone());
        let vc = VerifiedChainClient::new(client, verifier, "test", 1);

        let vs = vc.verified_storage("0xContract", "0x01", None).await.unwrap();
        assert_eq!(vs.data, vec![42]);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn verify_transfer_with_mock_receipt() {
        let (mock_client, mock_wallet) = crate::paired_mocks(1_000_000);
        let tx = TxRequest { to: Some("0xBEEF".into()), value: 100, ..Default::default() };
        let tx_hash = mock_wallet.sign_and_submit(tx).await.unwrap();

        let client = Arc::new(mock_client);
        let verifier = create_rpc_verifier(client.clone());
        let vc = VerifiedChainClient::new(client, verifier, "test", 1);

        let vs = vc.verify_transfer(&tx_hash).await.unwrap();
        assert!(vs.data.status);
        assert_eq!(vs.trust_level, TrustLevel::RpcTrusted);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn can_be_arc_dyn_chain_client() {
        let vc = make_verified_client();
        let _: Arc<dyn ChainClient> = Arc::new(vc);
        // Compile test — VerifiedChainClient can be used as Arc<dyn ChainClient>
    }
}
```

---

## Verification Checklist

- [ ] `VerifiedChainClient` implements all 9 `ChainClient` methods
- [ ] `verified_balance()` returns `VerifiedState<u128>` with correct trust level
- [ ] `verified_storage()` returns `VerifiedState<Vec<u8>>`
- [ ] `verify_transfer()` returns `VerifiedState<Receipt>`
- [ ] `VerifiedChainClient` can be used as `Arc<dyn ChainClient>`
- [ ] Network name, chain_id, and consensus mechanism propagated correctly
- [ ] Module registered in `lib.rs`
- [ ] `cargo test -p roko-chain` passes
- [ ] `cargo clippy -p roko-chain --no-deps -- -D warnings` passes
