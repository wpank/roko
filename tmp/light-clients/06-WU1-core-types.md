# WU-1: Core Types & Traits

**Layer**: 0 (no dependencies — start immediately)
**Blocks**: WU-4, WU-5, WU-6, WU-7, WU-8
**Estimated effort**: 2-3 hours
**Crate**: `crates/roko-chain`

---

## Overview

Create the foundational types for the verified chain layer: the `ConsensusVerifier` trait (extension point per chain), `TrustedHeader` (what consensus produces), `VerifiedState<T>` (what agents consume), and the `ChainAdapter` registry (factory for verifiers).

These types have NO external dependencies — they use only types already in the workspace.

---

## Pre-read (understand before starting)

- `crates/roko-chain/src/types.rs` — existing chain types (`ChainHeader`, `ChainError`, `BlockNumber`)
- `crates/roko-chain/src/client.rs` — `ChainClient` trait (9 methods)
- `crates/roko-chain/src/lib.rs` — module registration pattern
- `crates/roko-chain/Cargo.toml` — current dependencies

---

## Tasks

### 1.1 Create `crates/roko-chain/src/consensus.rs`

This file defines the core consensus abstraction.

**Imports needed**:
```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use crate::types::{BlockNumber, ChainError};
```

**Types to define (in this order)**:

#### `TrustLevel` enum
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Verified via cryptographic consensus proof.
    Cryptographic,
    /// Returned by a trusted RPC, not independently verified.
    RpcTrusted,
    /// Synthetic/captured data for demo playback.
    Playback,
}
```

#### `ConsensusProof` enum
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusProof {
    /// BLS12-381 threshold signature (Tempo, daeji).
    ThresholdBls {
        /// BLS G2 point, 96 bytes.
        signature: Vec<u8>,
        /// BLS G1 point, 48 bytes.
        group_pubkey: Vec<u8>,
    },
    /// Ethereum sync committee aggregate signature.
    SyncCommittee {
        aggregate_signature: Vec<u8>,
        participation_bits: Vec<u8>,
        committee_period: u64,
    },
    /// No consensus verification — trusts the RPC provider.
    RpcTrusted,
    /// Deterministic playback from captured data.
    Playback {
        source_file: String,
    },
}
```

#### `TrustedHeader` struct
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedHeader {
    /// Block number.
    pub number: u64,
    /// Block hash (32 bytes).
    pub hash: [u8; 32],
    /// State trie root (32 bytes). Used for MPT proof verification.
    pub state_root: [u8; 32],
    /// Block timestamp (unix seconds).
    pub timestamp: u64,
    /// How this header's finality was proven.
    pub consensus_proof: ConsensusProof,
}
```

#### `ConsensusError` enum
```rust
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConsensusError {
    #[error("invalid signature")]
    InvalidSignature,

    #[error("insufficient participation: {participating}/{required}")]
    InsufficientParticipation {
        participating: usize,
        required: usize,
    },

    #[error("block {0} unavailable")]
    BlockUnavailable(u64),

    #[error("verifier not synced")]
    NotSynced,

    #[error("chain error: {0}")]
    Chain(#[from] ChainError),

    #[error("{0}")]
    Other(String),
}
```

#### `ConsensusVerifier` trait
```rust
/// Verifies that a block header is finalized on a specific chain.
/// One implementation per consensus mechanism.
#[async_trait]
pub trait ConsensusVerifier: Send + Sync {
    /// Verify a block header is finalized. Returns the trusted header.
    async fn verify_finality(
        &self,
        block: BlockNumber,
    ) -> Result<TrustedHeader, ConsensusError>;

    /// The latest finalized block this verifier trusts.
    async fn latest_finalized(&self) -> Result<TrustedHeader, ConsensusError>;

    /// Consensus mechanism identifier (for logging/UI).
    fn mechanism(&self) -> &str;

    /// Trust level this consensus mechanism provides.
    fn trust_level(&self) -> TrustLevel;

    /// Health check — returns true if verifier is operational.
    async fn is_healthy(&self) -> bool;
}
```

#### Tests (at bottom of file)
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn trust_level_serde_roundtrip() {
        for level in [TrustLevel::Cryptographic, TrustLevel::RpcTrusted, TrustLevel::Playback] {
            let json = serde_json::to_string(&level).unwrap();
            let back: TrustLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(back, level);
        }
    }

    #[test]
    fn consensus_proof_serde_roundtrip() {
        let proofs = vec![
            ConsensusProof::ThresholdBls {
                signature: vec![0u8; 96],
                group_pubkey: vec![0u8; 48],
            },
            ConsensusProof::RpcTrusted,
            ConsensusProof::Playback { source_file: "test.jsonl".into() },
        ];
        for proof in proofs {
            let json = serde_json::to_string(&proof).unwrap();
            let _back: ConsensusProof = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn trusted_header_construction() {
        let header = TrustedHeader {
            number: 42,
            hash: [1u8; 32],
            state_root: [2u8; 32],
            timestamp: 1700000000,
            consensus_proof: ConsensusProof::RpcTrusted,
        };
        assert_eq!(header.number, 42);
        let json = serde_json::to_string(&header).unwrap();
        let back: TrustedHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(back.hash, [1u8; 32]);
    }

    #[test]
    fn consensus_error_display() {
        let e = ConsensusError::InvalidSignature;
        assert!(format!("{e}").contains("invalid signature"));

        let e = ConsensusError::InsufficientParticipation { participating: 100, required: 342 };
        assert!(format!("{e}").contains("100/342"));

        let e = ConsensusError::BlockUnavailable(999);
        assert!(format!("{e}").contains("999"));
    }
}
```

---

### 1.2 Create `crates/roko-chain/src/verified_state.rs`

**Imports**:
```rust
use serde::{Deserialize, Serialize};
use crate::consensus::TrustLevel;
```

**Type**:
```rust
/// A piece of chain state with verification provenance.
/// This is what agents and the UI always consume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedState<T: Serialize> {
    /// The actual data.
    pub data: T,
    /// EVM chain ID.
    pub chain_id: u64,
    /// Human-readable network name (e.g. "tempo-mainnet").
    pub network: String,
    /// Block number the data was verified at.
    pub block_number: u64,
    /// Block hash (32 bytes).
    pub block_hash: [u8; 32],
    /// Block timestamp (unix seconds).
    pub block_timestamp: u64,
    /// Trust level of this verification.
    pub trust_level: TrustLevel,
    /// Consensus mechanism name (e.g. "threshold_bls", "rpc").
    pub consensus_mechanism: String,
    /// Serialized consensus proof bytes (for external audit). Empty for RPC-trusted.
    pub consensus_proof_bytes: Vec<u8>,
    /// Serialized state proof bytes (MPT nodes). Empty for RPC-trusted.
    pub state_proof_bytes: Vec<u8>,
    /// When this verification was performed (unix ms).
    pub verified_at: u64,
}

impl<T: Serialize> VerifiedState<T> {
    /// Whether this state was cryptographically verified.
    pub fn is_cryptographic(&self) -> bool {
        self.trust_level == TrustLevel::Cryptographic
    }

    /// Whether this state is from a playback/demo.
    pub fn is_playback(&self) -> bool {
        self.trust_level == TrustLevel::Playback
    }
}
```

**Tests**:
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn verified_state_u128_serde_roundtrip() {
        let vs = VerifiedState {
            data: 1_000_000u128,
            chain_id: 4217,
            network: "tempo-mainnet".into(),
            block_number: 100,
            block_hash: [0xAA; 32],
            block_timestamp: 1700000000,
            trust_level: TrustLevel::Cryptographic,
            consensus_mechanism: "threshold_bls".into(),
            consensus_proof_bytes: vec![1, 2, 3],
            state_proof_bytes: vec![4, 5, 6],
            verified_at: 1700000001000,
        };
        let json = serde_json::to_string(&vs).unwrap();
        let back: VerifiedState<u128> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.data, 1_000_000);
        assert_eq!(back.chain_id, 4217);
        assert!(back.is_cryptographic());
        assert!(!back.is_playback());
    }

    #[test]
    fn verified_state_string_construction() {
        let vs = VerifiedState {
            data: "hello".to_string(),
            chain_id: 1,
            network: "ethereum-mainnet".into(),
            block_number: 0,
            block_hash: [0; 32],
            block_timestamp: 0,
            trust_level: TrustLevel::RpcTrusted,
            consensus_mechanism: "rpc".into(),
            consensus_proof_bytes: vec![],
            state_proof_bytes: vec![],
            verified_at: 0,
        };
        assert!(!vs.is_cryptographic());
    }
}
```

---

### 1.3 Create `crates/roko-chain/src/adapter.rs`

**Imports**:
```rust
use std::sync::Arc;
use serde::Deserialize;
use crate::consensus::ConsensusVerifier;
use crate::types::ChainError;
```

**Types and traits**:

```rust
/// Configuration for a named chain backend. Parsed from `[chain.backends.*]` in roko.toml.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ChainBackendConfig {
    /// HTTP JSON-RPC endpoint.
    pub rpc_url: Option<String>,
    /// EVM chain ID.
    pub chain_id: u64,
    /// Consensus mechanism: "threshold_bls" | "sync_committee" | "rpc" | "playback"
    pub consensus: String,
    // -- Threshold BLS specific --
    /// BLS12-381 group public key (hex, 0x-prefixed, 48 bytes).
    pub group_pubkey: Option<String>,
    /// P2P peer addresses for cert subscription.
    pub peer_addrs: Option<Vec<String>>,
    // -- Sync committee specific --
    /// Beacon API endpoint.
    pub beacon_api_url: Option<String>,
    /// Weak subjectivity checkpoint root.
    pub checkpoint_root: Option<String>,
    // -- Playback specific --
    /// Path to JSONL capture file.
    pub playback_file: Option<std::path::PathBuf>,
    // -- Shared --
    /// RPC timeout in milliseconds.
    pub timeout_ms: Option<u64>,
    /// Max concurrent RPC requests.
    pub max_concurrent: Option<u32>,
}

/// Factory for creating `ConsensusVerifier` instances from config.
pub trait ChainAdapter: Send + Sync {
    /// Consensus type identifier.
    fn consensus_type(&self) -> &str;

    /// Create a verifier from config.
    fn create_verifier(
        &self,
        config: &ChainBackendConfig,
    ) -> Result<Arc<dyn ConsensusVerifier>, ChainError>;
}

/// Look up an adapter for a consensus mechanism string.
/// Returns None for unknown mechanisms.
pub fn adapter_for_consensus(mechanism: &str) -> Option<Box<dyn ChainAdapter>> {
    match mechanism {
        "rpc" => Some(Box::new(RpcOnlyAdapter)),
        "playback" => Some(Box::new(PlaybackAdapter)),
        #[cfg(feature = "threshold-bls")]
        "threshold_bls" => Some(Box::new(crate::threshold_bls::ThresholdBlsAdapter)),
        _ => None,
    }
}

/// Adapter that creates RPC-only verifiers (no consensus verification).
pub struct RpcOnlyAdapter;

impl ChainAdapter for RpcOnlyAdapter {
    fn consensus_type(&self) -> &str { "rpc" }

    fn create_verifier(
        &self,
        _config: &ChainBackendConfig,
    ) -> Result<Arc<dyn ConsensusVerifier>, ChainError> {
        // RpcOnlyVerifier is implemented in WU-4
        // For now, return an error indicating it's not yet ready
        Err(ChainError::Unsupported("RpcOnlyVerifier not yet implemented — see WU-4".into()))
    }
}

/// Adapter that creates playback verifiers from JSONL capture files.
pub struct PlaybackAdapter;

impl ChainAdapter for PlaybackAdapter {
    fn consensus_type(&self) -> &str { "playback" }

    fn create_verifier(
        &self,
        _config: &ChainBackendConfig,
    ) -> Result<Arc<dyn ConsensusVerifier>, ChainError> {
        // PlaybackVerifier is implemented in WU-2
        Err(ChainError::Unsupported("PlaybackVerifier not yet implemented — see WU-2".into()))
    }
}
```

**Tests**:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_for_rpc_returns_some() {
        assert!(adapter_for_consensus("rpc").is_some());
    }

    #[test]
    fn adapter_for_playback_returns_some() {
        assert!(adapter_for_consensus("playback").is_some());
    }

    #[test]
    fn adapter_for_unknown_returns_none() {
        assert!(adapter_for_consensus("quantum").is_none());
    }

    #[test]
    fn chain_backend_config_deserialize() {
        let toml = r#"
            rpc_url = "https://rpc.tempo.xyz"
            chain_id = 4217
            consensus = "threshold_bls"
            group_pubkey = "0xabcdef"
        "#;
        let config: ChainBackendConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.chain_id, 4217);
        assert_eq!(config.consensus, "threshold_bls");
        assert_eq!(config.group_pubkey.as_deref(), Some("0xabcdef"));
    }
}
```

**Note**: Add `toml` to dev-dependencies if not already present for the config deserialization test.

---

### 1.4 Register modules in `crates/roko-chain/src/lib.rs`

Add after existing module declarations (near line 77):
```rust
/// Consensus verification abstraction — the extension point for light clients.
pub mod consensus;
/// Chain adapter registry — factory for creating consensus verifiers.
pub mod adapter;
/// Verified state return type — wraps data with verification provenance.
pub mod verified_state;
```

Add to the pub use section (after existing pub use statements):
```rust
pub use adapter::{ChainAdapter, ChainBackendConfig, adapter_for_consensus};
pub use consensus::{
    ConsensusError, ConsensusProof, ConsensusVerifier, TrustLevel, TrustedHeader,
};
pub use verified_state::VerifiedState;
```

---

## Verification Checklist

- [ ] `cargo test -p roko-chain` — all new + existing tests pass
- [ ] `cargo clippy -p roko-chain --no-deps -- -D warnings` — no warnings
- [ ] `cargo test --workspace` — no breakage in other crates
- [ ] All three new files exist: `consensus.rs`, `verified_state.rs`, `adapter.rs`
- [ ] All types are re-exported from `lib.rs`
- [ ] `TrustLevel`, `ConsensusProof`, `TrustedHeader` all serialize/deserialize
- [ ] `adapter_for_consensus("rpc")` returns `Some`
- [ ] `adapter_for_consensus("unknown")` returns `None`
