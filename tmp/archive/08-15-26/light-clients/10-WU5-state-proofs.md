# WU-5: EVM State Proof Verification

**Layer**: 1
**Depends on**: WU-1 (core types), WU-3 (ChainHeader state_root)
**Blocks**: WU-7
**Estimated effort**: 2-3 hours
**Crate**: `crates/roko-chain`

---

## Overview

Verify `eth_getProof` MPT (Merkle Patricia Trie) proofs against a trusted `state_root`. This is the shared verification layer — ALL EVM chains use the same state proof format. Tempo, Ethereum, Base, Optimism, daeji — they all return `eth_getProof` responses that are verified identically.

**Feature-gated**: This entire module requires `alloy-backend` feature.

---

## Pre-read

- `07-DEPENDENCY-REFERENCE.md` — Section 2 (alloy-trie API)
- `crates/roko-chain/src/alloy_impl.rs` — existing AlloyChainClient, `provider()` method
- `08-EXISTING-CODE-REFERENCE.md` — Section 4 (AlloyChainClient)

---

## Tasks

### 5.1 Add `alloy-trie` dependency

**File**: `crates/roko-chain/Cargo.toml`

Add to `[dependencies]`:
```toml
alloy-trie = { version = "0.9", optional = true }
```

Add to the `alloy-backend` feature:
```toml
[features]
alloy-backend = ["dep:alloy", "dep:alloy-primitives", "dep:alloy-trie", "dep:reqwest"]
```

### 5.2 Create `crates/roko-chain/src/state_proof.rs`

**IMPORTANT**: Before implementing, verify the exact `alloy-trie` v0.9 API. Run:
```bash
cargo doc -p alloy-trie --open
```
Or check `docs.rs/alloy-trie/0.9`. The `verify_proof` function signature may differ from what's sketched below.

**File**: `crates/roko-chain/src/state_proof.rs`

```rust
//! EVM state proof verification via Merkle Patricia Trie.
//!
//! Verifies `eth_getProof` responses against a trusted `state_root`.
//! Shared across all EVM chains — Tempo, Ethereum, L2s all use the same format.
//!
//! # Feature gate
//!
//! This module requires the `alloy-backend` feature.

#![cfg(feature = "alloy-backend")]

use alloy::rpc::types::EIP1186AccountProofResponse;
use alloy_primitives::{keccak256, Address, B256};
use crate::types::ChainError;

/// Result of verifying an account proof against a state root.
#[derive(Debug, Clone)]
pub struct VerifiedAccount {
    /// Account address (hex, 0x-prefixed).
    pub address: String,
    /// Native balance in wei.
    pub balance: u128,
    /// Account nonce.
    pub nonce: u64,
    /// Code hash (keccak256 of account bytecode).
    pub code_hash: [u8; 32],
    /// Storage root hash (root of account's storage trie).
    pub storage_hash: [u8; 32],
}

/// Result of verifying a storage slot proof.
#[derive(Debug, Clone)]
pub struct VerifiedStorageSlot {
    /// Account address.
    pub address: String,
    /// Storage slot key.
    pub slot: String,
    /// Storage value at the slot.
    pub value: Vec<u8>,
}

/// Verify an account's existence and state against a trusted state root
/// using the MPT proof from `eth_getProof`.
///
/// # Arguments
/// * `state_root` — The trusted state root (from a verified block header)
/// * `proof_response` — The `eth_getProof` response from an RPC provider
///
/// # How it works
/// 1. The state trie key for an account is `keccak256(address)`
/// 2. The proof nodes form a path from the state root to the account's RLP-encoded data
/// 3. We verify this path is consistent (each node hashes to its parent's expected value)
pub fn verify_account_proof(
    state_root: &[u8; 32],
    proof_response: &EIP1186AccountProofResponse,
) -> Result<VerifiedAccount, ChainError> {
    let root = B256::from_slice(state_root);

    // The account key in the state trie is keccak256(address)
    let _address_hash = keccak256(proof_response.address.as_slice());

    // Convert proof nodes to the format expected by alloy-trie
    let _proof_nodes: Vec<Vec<u8>> = proof_response
        .account_proof
        .iter()
        .map(|b| b.to_vec())
        .collect();

    // ===================================================================
    // IMPORTANT: The exact alloy-trie API call goes here.
    //
    // Check docs.rs/alloy-trie/0.9 for the correct verify_proof signature.
    //
    // Likely something like:
    //   alloy_trie::proof::verify_proof(root, Nibbles::unpack(address_hash), ...)
    //
    // If the API has changed, adapt the call accordingly.
    // The conceptual verification is:
    //   Given root + key + proof_nodes → verify the account exists in the trie
    // ===================================================================

    // For now, verify that the proof is non-empty (basic sanity)
    if proof_response.account_proof.is_empty() {
        return Err(ChainError::Rpc("empty account proof".into()));
    }

    // TODO: Replace this with actual alloy-trie verification once API is confirmed.
    // The balance/nonce/etc are still returned from the proof response —
    // full MPT verification will ensure they match the state root.
    let _ = root; // suppress unused warning until real verification

    Ok(VerifiedAccount {
        address: format!("{:?}", proof_response.address),
        balance: proof_response.balance.to::<u128>(),
        nonce: proof_response.nonce,
        code_hash: proof_response.code_hash.0,
        storage_hash: proof_response.storage_hash.0,
    })
}

/// Verify a storage slot value against a trusted storage root.
///
/// # Arguments
/// * `storage_root` — The account's storage root (from `verify_account_proof`)
/// * `slot` — The storage slot to verify
/// * `proof_response` — The full `eth_getProof` response (contains storage proofs)
pub fn verify_storage_proof(
    storage_root: &[u8; 32],
    slot: &B256,
    proof_response: &EIP1186AccountProofResponse,
) -> Result<VerifiedStorageSlot, ChainError> {
    let _root = B256::from_slice(storage_root);
    let _slot_hash = keccak256(slot.as_slice());

    // Find the matching storage proof
    let storage_proof = proof_response
        .storage_proof
        .iter()
        .find(|sp| sp.key.as_b256() == *slot)
        .ok_or_else(|| ChainError::Rpc(format!("no storage proof for slot {slot}")))?;

    let _proof_nodes: Vec<Vec<u8>> = storage_proof
        .proof
        .iter()
        .map(|b| b.to_vec())
        .collect();

    // ===================================================================
    // IMPORTANT: Same as above — call alloy-trie verify_proof here.
    // Key is keccak256(slot), root is the storage_root from the account proof.
    // ===================================================================

    if storage_proof.proof.is_empty() {
        return Err(ChainError::Rpc("empty storage proof".into()));
    }

    Ok(VerifiedStorageSlot {
        address: format!("{:?}", proof_response.address),
        slot: format!("{slot}"),
        value: storage_proof.value.to_be_bytes_vec(),
    })
}
```

### 5.3 Register module in lib.rs

```rust
#[cfg(feature = "alloy-backend")]
/// EVM state proof (MPT) verification.
pub mod state_proof;
```

And conditionally export:
```rust
#[cfg(feature = "alloy-backend")]
pub use state_proof::{VerifiedAccount, VerifiedStorageSlot, verify_account_proof, verify_storage_proof};
```

### 5.4 Create or capture test fixtures

**Option A: Capture from live RPC** (preferred for real validation):
```bash
# Using curl against Tempo Moderato testnet:
curl -X POST https://rpc.moderato.tempo.xyz \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_getProof","params":["0x0000000000000000000000000000000000000000",[],  "latest"],"id":1}' \
  > crates/roko-chain/src/testdata/account_proof_tempo.json
```

**Option B: Use any EVM testnet** (Sepolia, Holesky) if Tempo is unavailable.

**Option C: Construct synthetic fixture** — create a minimal JSON file matching `EIP1186AccountProofResponse` structure with known values.

### 5.5 Tests

```rust
#[cfg(test)]
#[cfg(feature = "alloy-backend")]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_account_proof_rejected() {
        // Construct a minimal proof response with empty proof
        // This should fail verification
        // TODO: construct EIP1186AccountProofResponse with empty account_proof
    }

    #[test]
    fn missing_storage_proof_rejected() {
        // Request proof for slot X, but response only has proof for slot Y
        // verify_storage_proof should return error
    }

    // If captured fixture exists:
    #[test]
    fn verify_captured_account_proof() {
        let fixture_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/testdata/account_proof_tempo.json");
        if !fixture_path.exists() {
            eprintln!("skip: no captured proof fixture");
            return;
        }
        // Load and verify
    }
}
```

---

## Verification Checklist

- [ ] `alloy-trie = "0.9"` added to Cargo.toml
- [ ] `alloy-backend` feature includes `dep:alloy-trie`
- [ ] `state_proof.rs` compiles under `alloy-backend` feature
- [ ] `verify_account_proof()` returns `VerifiedAccount` with correct fields
- [ ] `verify_storage_proof()` returns `VerifiedStorageSlot` or clear error
- [ ] Empty proofs are rejected with error
- [ ] Module registered in `lib.rs` with feature gate
- [ ] `cargo test -p roko-chain --features alloy-backend` passes
- [ ] `cargo test -p roko-chain` also passes (module gated, doesn't affect default build)
- [ ] `cargo clippy -p roko-chain --features alloy-backend --no-deps -- -D warnings` passes

### Note on alloy-trie API

The exact `verify_proof()` signature MUST be checked against `docs.rs/alloy-trie/0.9` before implementing. The code above has TODO markers where the real API call goes. Do not ship without replacing the TODOs with actual MPT verification.
