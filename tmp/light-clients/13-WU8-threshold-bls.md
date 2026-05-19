# WU-8: Threshold BLS Verifier (Tempo)

**Layer**: 2
**Depends on**: WU-4 (RpcOnlyVerifier pattern)
**Blocks**: WU-10
**Estimated effort**: 3-4 hours
**Crate**: `crates/roko-chain`

---

## Overview

Implement `ThresholdBlsVerifier` — a `ConsensusVerifier` for Tempo and daeji chains using BLS12-381 threshold signatures from Commonware. This is the first real cryptographic consensus verifier.

**Feature-gated**: `threshold-bls`

---

## Tempo Consensus Facts (researched 2026-05-04)

| Property | Value |
|----------|-------|
| Consensus | Commonware `threshold_simplex` (Simplex BFT dialect) |
| Signature scheme | BLS12-381 threshold, **MinSig** variant |
| Group public key | **48 bytes** (G1 compressed point), STATIC across validator rotations |
| Threshold signature | **96 bytes** (G2 compressed point) per finalized block |
| Threshold | 2f+1 out of 3f+1 validators (standard BFT) |
| Verification | Single bilinear pairing check: ~1-2ms |
| Finality | Deterministic, ~0.5 seconds. **Every block is finalized** (not just checkpoints) |
| Certificate size | ~240 bytes total (96B sig + round + parent + digest + overhead) |
| Underlying crate | `blst` (supranational, used by all major ETH2 clients) |
| Commonware version | `2026.4.0` (date-based versioning) |

The group key is created via DKG (Distributed Key Generation) and preserved across validator
resharing. Resharing maintains the same secret polynomial constant term — the group public key
is invariant. A verifier stores ONE 48-byte key and never needs to update it unless a full DKG
occurs (`is_next_full_dkg = true` in `OnchainDkgOutcome`), which is rare (only for catastrophic
security failures, not normal validator rotation).

### Certificate wire format

The finality proof is Commonware's `Finalization<S, D>`:

```rust
// commonware-consensus::simplex::types
pub struct Finalization<S: Scheme, D: Digest> {
    pub proposal: Proposal<D>,
    pub certificate: S::Certificate,  // For BLS threshold: single 96-byte G2 point
}

pub struct Proposal<D: Digest> {
    pub round: Round,   // u64 — consensus view/round number
    pub parent: View,   // u64 — parent view
    pub payload: D,     // 32-byte digest (likely Blake3 hash of block body)
}
```

For `bls12381_threshold`, `S::Certificate` = single 96-byte BLS12-381 G2 signature (the
recovered threshold signature via Lagrange interpolation). No list of individual validator
signatures — constant size regardless of validator count.

### RPC methods

| Method | Purpose |
|--------|---------|
| `consensus_getIdentityTransitionProof` | Returns DKG outcomes + BLS certificates for validator set changes. Supports `from_epoch` param. Use to fetch group public key. |
| `eth_getBlockByNumber` | Post TIP-1031 (v1.6.0 T3), block headers contain `consensus_context` with view number and finalization state |

**How to get the group public key**: Call `consensus_getIdentityTransitionProof` with
`{"from_epoch": 0}` → response includes `OnchainDkgOutcome.output` → extract the G1 constant
term of the public polynomial = the group public key.

### DKG struct (on-chain)

```rust
// tempo_dkg_onchain_artifacts (168 bytes)
pub struct OnchainDkgOutcome {
    pub epoch: Epoch,
    pub output: Output<MinSig, PublicKey>,  // contains public polynomial → group key
    pub next_players: Set<PublicKey>,       // validators for next epoch
    pub is_next_full_dkg: bool,            // reshare (key preserved) vs full DKG (new key)
}
```

---

## Pre-read

- `07-DEPENDENCY-REFERENCE.md` — Section 3 (commonware-cryptography API)
- `09-WU4-rpc-verifier.md` — same `ConsensusVerifier` pattern
- `crates/roko-chain/src/client.rs` — `ChainClient` for fetching headers

---

## Tasks

### 8.1 Add dependencies

**File**: `crates/roko-chain/Cargo.toml`

```toml
[dependencies]
commonware-cryptography = { version = "2026.4.0", optional = true }
blst = { version = "0.3", optional = true }  # Direct access for MinSig verification

[features]
threshold-bls = ["dep:commonware-cryptography", "dep:blst"]
```

**Note**: `commonware-cryptography` uses date-based versions (`2026.4.0`). Pin to exact version.
The `blst` crate provides direct access to `blst::min_sig::Signature::verify()` if the
commonware high-level API is insufficient.

### 8.2 Create `crates/roko-chain/src/threshold_bls.rs`

```rust
//! Threshold BLS12-381 consensus verifier for Tempo and daeji chains.
//!
//! Verifies finality certificates containing a single BLS threshold signature
//! over the block header hash. The group public key is static across validator
//! set rotations (resharing preserves it).
//!
//! # Feature gate
//! Requires the `threshold-bls` feature.

#![cfg(feature = "threshold-bls")]

use std::collections::BTreeMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::adapter::{ChainAdapter, ChainBackendConfig};
use crate::client::ChainClient;
use crate::consensus::*;
use crate::types::{BlockNumber, ChainError};

/// Threshold BLS consensus verifier.
///
/// Verifies Tempo/daeji finality certificates using BLS12-381 threshold signatures.
/// Stores a single group public key (48 bytes) that never changes.
pub struct ThresholdBlsVerifier {
    /// Static BLS12-381 group public key (G1 point, 48 bytes).
    group_pubkey_bytes: Vec<u8>,
    /// Chain ID for this network.
    chain_id: u64,
    /// RPC client for fetching block headers.
    rpc: Arc<dyn ChainClient>,
    /// Cache of already-verified headers.
    verified_headers: RwLock<BTreeMap<u64, TrustedHeader>>,
}

impl ThresholdBlsVerifier {
    /// Create a new verifier with the given group public key.
    ///
    /// # Arguments
    /// * `group_pubkey_hex` — 0x-prefixed hex of the 48-byte BLS G1 point
    /// * `chain_id` — EVM chain ID
    /// * `rpc` — Chain client for fetching headers
    pub fn new(
        group_pubkey_hex: &str,
        chain_id: u64,
        rpc: Arc<dyn ChainClient>,
    ) -> Result<Self, ChainError> {
        let stripped = group_pubkey_hex.strip_prefix("0x").unwrap_or(group_pubkey_hex);
        let pubkey_bytes = hex::decode(stripped)
            .map_err(|e| ChainError::Rpc(format!("invalid group pubkey hex: {e}")))?;

        if pubkey_bytes.len() != 48 {
            return Err(ChainError::Rpc(format!(
                "group pubkey must be 48 bytes, got {}",
                pubkey_bytes.len()
            )));
        }

        Ok(Self {
            group_pubkey_bytes: pubkey_bytes,
            chain_id,
            rpc,
            verified_headers: RwLock::new(BTreeMap::new()),
        })
    }

    /// Verify a BLS threshold signature over a block header hash.
    ///
    /// # BLS verification
    /// Check: `e(signature, G2_generator) == e(H(message), group_pubkey)`
    ///
    /// This is a single pairing check, ~1-2ms regardless of validator count.
    fn verify_bls_signature(
        &self,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), ConsensusError> {
        // BLS12-381 MinSig verification: signatures in G2 (96 bytes), pubkeys in G1 (48 bytes)
        // Pairing check: e(H(message), group_pk) == e(signature, G2_generator)
        //
        // Two approaches (use whichever compiles with the pinned version):
        //
        // Approach A: Direct blst (recommended — minimal API surface)
        //
        //   use blst::min_sig::{Signature as BlstSig, PublicKey as BlstPk};
        //   let sig = BlstSig::from_bytes(signature)
        //       .map_err(|_| ConsensusError::InvalidSignature("malformed BLS signature".into()))?;
        //   let pk = BlstPk::from_bytes(&self.group_pubkey_bytes)
        //       .map_err(|_| ConsensusError::InvalidSignature("malformed group public key".into()))?;
        //   let dst = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";  // verify DST with Tempo team
        //   let result = sig.verify(true, message, dst, &[], &pk, true);
        //   if result != blst::BLST_ERROR::BLST_SUCCESS {
        //       return Err(ConsensusError::InvalidSignature(
        //           format!("BLS verification failed: {result:?}")
        //       ));
        //   }
        //
        // Approach B: Via commonware-cryptography high-level API
        //
        //   use commonware_cryptography::bls12381::min_sig;
        //   // Check docs.rs/commonware-cryptography/2026.4.0 for exact types
        //
        // IMPORTANT: The DST (Domain Separation Tag) bytes must match what Tempo validators
        // use. The namespace is derived from the chain's base namespace via
        // Namespace::new(namespace_bytes).finalize — check Tempo source for the exact bytes.

        if signature.is_empty() {
            tracing::warn!("empty consensus cert — falling back to RPC trust");
            return Ok(());
        }

        // TODO: Uncomment one of the approaches above once dependencies compile.
        //       Until then, accept all non-empty signatures with a warning.
        tracing::warn!("threshold BLS verification not yet wired — accepting all signatures");
        let _ = message;
        Ok(())
    }

    /// Fetch the consensus certificate for a block.
    ///
    /// ## Research findings (2026-05-04)
    ///
    /// TIP-1031 (v1.6.0 T3 hardfork) encodes consensus metadata directly into
    /// Tempo block headers. Post-T3, `eth_getBlockByNumber` returns headers with
    /// a `consensus_context` field containing the view number and finalization state.
    ///
    /// The finalization certificate (96-byte BLS signature) may be:
    /// 1. Embedded in the block header's `consensus_context` field (post TIP-1031)
    /// 2. Available via `consensus_getIdentityTransitionProof` for epoch transitions
    /// 3. Not separately queryable for individual blocks (the header IS the proof)
    ///
    /// Strategy: Try to extract the BLS signature from the extended block header
    /// fields returned by `eth_getBlockByNumber`. If the header includes consensus
    /// metadata, use it. Otherwise, fall back to RPC trust.
    async fn fetch_consensus_cert(
        &self,
        _block: BlockNumber,
    ) -> Result<Vec<u8>, ConsensusError> {
        // TODO: Implementation steps:
        // 1. Call eth_getBlockByNumber with full=false
        // 2. Check for extra fields in the header (consensus_context, or extraData)
        // 3. If present, extract the 96-byte BLS finalization signature
        // 4. If not present, return empty (fall back to RPC trust)
        //
        // The ChainClient trait returns ChainHeader which currently doesn't have
        // a consensus_context field. Options:
        //   a) Add an `extra_data: Option<Vec<u8>>` field to ChainHeader
        //   b) Make a separate raw RPC call via reqwest (bypass ChainClient)
        //   c) Extend AlloyChainClient with a method that returns the full header
        tracing::warn!("consensus cert fetching not yet implemented — using RPC trust");
        Ok(vec![])
    }
}

#[async_trait]
impl ConsensusVerifier for ThresholdBlsVerifier {
    async fn verify_finality(&self, block: BlockNumber) -> Result<TrustedHeader, ConsensusError> {
        // Check cache first
        {
            let cache = self.verified_headers.read().await;
            if let Some(header) = cache.get(&block) {
                return Ok(header.clone());
            }
        }

        // Fetch header from RPC
        let chain_header = self.rpc.get_block_header(block).await?;

        // Fetch consensus certificate
        let cert = self.fetch_consensus_cert(block).await?;

        // Parse header hash
        let hash = parse_hex_to_bytes32(&chain_header.hash).unwrap_or([0u8; 32]);
        let state_root = parse_hex_to_bytes32(&chain_header.state_root).unwrap_or([0u8; 32]);

        // Verify BLS signature (if cert is available)
        if !cert.is_empty() {
            self.verify_bls_signature(&hash, &cert)?;
        }

        let trusted = TrustedHeader {
            number: chain_header.number,
            hash,
            state_root,
            timestamp: chain_header.timestamp,
            consensus_proof: ConsensusProof::ThresholdBls {
                signature: cert,
                group_pubkey: self.group_pubkey_bytes.clone(),
            },
        };

        // Cache
        self.verified_headers.write().await.insert(block, trusted.clone());

        Ok(trusted)
    }

    async fn latest_finalized(&self) -> Result<TrustedHeader, ConsensusError> {
        let block = self.rpc.block_number().await?;
        self.verify_finality(block).await
    }

    fn mechanism(&self) -> &str { "threshold_bls" }

    fn trust_level(&self) -> TrustLevel {
        // When BLS verification is wired, this should return Cryptographic
        // For now, since verification is stubbed, return RpcTrusted
        TrustLevel::RpcTrusted // TODO: Change to Cryptographic when verify_bls_signature is real
    }

    async fn is_healthy(&self) -> bool {
        self.rpc.block_number().await.is_ok()
    }
}

/// Adapter for creating ThresholdBlsVerifier instances.
pub struct ThresholdBlsAdapter;

impl ChainAdapter for ThresholdBlsAdapter {
    fn consensus_type(&self) -> &str { "threshold_bls" }

    fn create_verifier(
        &self,
        config: &ChainBackendConfig,
    ) -> Result<Arc<dyn ConsensusVerifier>, ChainError> {
        let pubkey = config.group_pubkey.as_ref()
            .ok_or_else(|| ChainError::Rpc("group_pubkey required for threshold_bls adapter".into()))?;

        let rpc_url = config.rpc_url.as_ref()
            .ok_or_else(|| ChainError::Rpc("rpc_url required for threshold_bls adapter".into()))?;

        #[cfg(feature = "alloy-backend")]
        {
            let client = Arc::new(crate::alloy_impl::AlloyChainClient::http(rpc_url)?);
            Ok(Arc::new(ThresholdBlsVerifier::new(pubkey, config.chain_id, client)?))
        }

        #[cfg(not(feature = "alloy-backend"))]
        {
            let _ = rpc_url;
            Err(ChainError::Unsupported(
                "threshold_bls adapter requires alloy-backend feature".into()
            ))
        }
    }
}

fn parse_hex_to_bytes32(hex_str: &str) -> Option<[u8; 32]> {
    let stripped = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    let bytes = hex::decode(stripped).ok()?;
    if bytes.len() != 32 { return None; }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Some(arr)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::MockChainClient;

    fn mock_pubkey() -> String {
        // 48 bytes of 0xAA as hex
        format!("0x{}", "aa".repeat(48))
    }

    #[test]
    fn new_verifier_valid_pubkey() {
        let mock = MockChainClient::local();
        let v = ThresholdBlsVerifier::new(&mock_pubkey(), 4217, Arc::new(mock));
        assert!(v.is_ok());
    }

    #[test]
    fn new_verifier_invalid_hex() {
        let mock = MockChainClient::local();
        let v = ThresholdBlsVerifier::new("0xZZZZ", 4217, Arc::new(mock));
        assert!(v.is_err());
    }

    #[test]
    fn new_verifier_wrong_length() {
        let mock = MockChainClient::local();
        let v = ThresholdBlsVerifier::new("0xaabb", 4217, Arc::new(mock));
        assert!(v.is_err());
        assert!(format!("{}", v.unwrap_err()).contains("48 bytes"));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn verify_finality_returns_header() {
        let mock = MockChainClient::local();
        mock.mine_empty_block();
        let v = ThresholdBlsVerifier::new(&mock_pubkey(), 4217, Arc::new(mock)).unwrap();

        let h = v.verify_finality(1).await.unwrap();
        assert_eq!(h.number, 1);
        assert!(matches!(h.consensus_proof, ConsensusProof::ThresholdBls { .. }));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn caches_verified_headers() {
        let mock = MockChainClient::local();
        mock.mine_empty_block();
        let v = ThresholdBlsVerifier::new(&mock_pubkey(), 4217, Arc::new(mock)).unwrap();

        let h1 = v.verify_finality(1).await.unwrap();
        let h2 = v.verify_finality(1).await.unwrap(); // should hit cache
        assert_eq!(h1.number, h2.number);
    }

    #[test]
    fn mechanism_is_threshold_bls() {
        let mock = MockChainClient::local();
        let v = ThresholdBlsVerifier::new(&mock_pubkey(), 4217, Arc::new(mock)).unwrap();
        assert_eq!(v.mechanism(), "threshold_bls");
    }
}
```

### 8.3 Register module in lib.rs

```rust
#[cfg(feature = "threshold-bls")]
/// Threshold BLS12-381 consensus verifier for Tempo/daeji.
pub mod threshold_bls;
```

### 8.4 Wire into `adapter_for_consensus()`

In `adapter.rs`, the `#[cfg(feature = "threshold-bls")]` match arm should already be in place from WU-1. Verify it points to `crate::threshold_bls::ThresholdBlsAdapter`.

---

## Open Questions (researched 2026-05-04)

1. **Tempo cert RPC**: ~~Does `tempo_getConsensusCertificate(block)` exist?~~
   **ANSWERED**: No dedicated per-block cert RPC exists. TIP-1031 (v1.6.0 T3 hardfork) embeds
   consensus metadata directly into block headers. Post-T3, `eth_getBlockByNumber` returns
   headers with a `consensus_context` field. The finalization cert is likely embedded there.
   For epoch transitions, `consensus_getIdentityTransitionProof` (PR #1918) returns DKG outcomes
   and BLS certificates. **Action**: Extend `ChainHeader` with `extra_data: Option<Vec<u8>>`
   to capture the consensus context, or make a raw RPC call to extract it.

2. **Commonware BLS API**: ~~What's the exact `verify()` call?~~
   **ANSWERED**: Use `blst` directly (MinSig variant):
   ```rust
   use blst::min_sig::{Signature, PublicKey};
   let sig = Signature::from_bytes(&cert_96_bytes)?;
   let pk = PublicKey::from_bytes(&group_key_48_bytes)?;
   let result = sig.verify(true, &message, dst, &[], &pk, true);
   ```
   DST (Domain Separation Tag) needs confirmation from Tempo source — the namespace is derived
   via `Namespace::new(chain_namespace_bytes).finalize`. The signed message is
   `namespace.finalize || encode(Proposal { round, parent, payload })`.

3. **Group pubkey bootstrap**: ~~Where does a new verifier get the group public key from?~~
   **ANSWERED**: Three sources, in preference order:
   - **Config**: `group_pubkey` field in `[chain.backends.tempo]` (48-byte hex, 0x-prefixed)
   - **RPC**: Call `consensus_getIdentityTransitionProof({"from_epoch": 0})` → extract
     `OnchainDkgOutcome.output.public_polynomial[0]` (the G1 constant term)
   - **Genesis**: Part of the chain spec (`tempo_chainspec`), but not easily accessible via RPC

   **Moderato testnet key**: Not published in any indexed public doc. Must be fetched via RPC
   call above against `https://rpc.moderato.tempo.xyz`.

4. **Namespace bytes for Moderato**: The `Namespace::new(bytes)` call derives domain separation
   from the chain's base namespace. Likely the chain ID (42431) or a chain-specific constant.
   Must confirm from `tempo_chainspec` source or by testing against a known finalization.

5. **Block hash → payload mapping**: The `Proposal.payload` is a `D: Digest` (likely Blake3 of
   the block body). The mapping from Ethereum-style block hash (Keccak256) to the Commonware
   digest needs confirmation. They may be different values.

---

## Verification Checklist

- [ ] `commonware-cryptography` added to Cargo.toml
- [ ] `threshold-bls` feature defined
- [ ] `ThresholdBlsVerifier` implements `ConsensusVerifier`
- [ ] Constructor validates pubkey length (48 bytes)
- [ ] Invalid hex and wrong-length pubkeys produce clear errors
- [ ] `verify_finality()` returns `TrustedHeader` with `ConsensusProof::ThresholdBls`
- [ ] Header caching works (second call hits cache)
- [ ] `ThresholdBlsAdapter` implements `ChainAdapter`
- [ ] Module registered in `lib.rs` with feature gate
- [ ] `cargo test -p roko-chain --features threshold-bls` passes
- [ ] `cargo test -p roko-chain` also passes (module gated)
- [ ] TODO markers clearly indicate where real BLS verification goes
