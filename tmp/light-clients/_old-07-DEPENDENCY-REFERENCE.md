# 07 — Dependency Reference

Complete API reference for all external dependencies needed by the light-client implementation. Agents implementing this should NOT need to look up crate documentation — everything is here.

---

## 1. alloy (already in workspace)

**Version**: `1` (workspace, features `"full"`)
**Crate**: `crates/roko-chain/Cargo.toml` — `alloy = { version = "1", features = ["full"], optional = true }`
**Feature gate**: `alloy-backend`

### Relevant types

```rust
use alloy::primitives::{Address, B256, Bytes, U256, keccak256};
use alloy::providers::{DynProvider, Provider, ProviderBuilder};
use alloy::rpc::types::EIP1186AccountProofResponse;
use alloy::network::EthereumWallet;
use alloy::signers::local::PrivateKeySigner;
```

### `Provider::get_proof()` — eth_getProof

This is the key method for state proof fetching.

```rust
// Usage:
let provider: Arc<DynProvider> = /* ... */;
let address: Address = "0x1234...".parse().unwrap();
let storage_keys: Vec<B256> = vec![]; // storage slots to prove
let block: BlockNumberOrTag = BlockNumberOrTag::Latest;

let proof: EIP1186AccountProofResponse = provider
    .get_proof(address, storage_keys)
    .block_id(block.into())
    .await?;
```

### `EIP1186AccountProofResponse` structure

```rust
pub struct EIP1186AccountProofResponse {
    pub address: Address,
    pub balance: U256,
    pub code_hash: B256,
    pub nonce: u64,
    pub storage_hash: B256,        // storage root for this account
    pub account_proof: Vec<Bytes>,  // MPT proof nodes (RLP-encoded trie nodes)
    pub storage_proof: Vec<StorageProof>,
}

pub struct StorageProof {
    pub key: JsonStorageKey,    // the storage slot
    pub value: U256,            // the value at that slot
    pub proof: Vec<Bytes>,      // MPT proof nodes for this storage slot
}
```

### `AlloyChainClient` (existing in roko-chain)

```rust
// File: crates/roko-chain/src/alloy_impl.rs
pub struct AlloyChainClient {
    provider: Arc<DynProvider>,
    name: String,
}

impl AlloyChainClient {
    pub fn http(rpc_url: &str) -> ChainResult<Self>
    pub fn provider(&self) -> Arc<DynProvider>  // share provider for eth_getProof
}
```

To get the provider for `eth_getProof` calls:
```rust
let client = AlloyChainClient::http("https://rpc.tempo.xyz")?;
let provider = client.provider(); // Arc<DynProvider>
let proof = provider.get_proof(address, keys).await?;
```

---

## 2. alloy-trie (NEW — to add)

**Version**: `0.9`
**Purpose**: Merkle Patricia Trie proof verification
**Add to**: `crates/roko-chain/Cargo.toml` under `alloy-backend` feature

### Cargo.toml

```toml
alloy-trie = { version = "0.9", optional = true }

[features]
alloy-backend = ["dep:alloy", "dep:alloy-primitives", "dep:alloy-trie", "dep:reqwest"]
```

### Core API

```rust
use alloy_trie::Nibbles;
use alloy_trie::proof::verify_proof;
```

### `verify_proof()` function

The exact signature as of alloy-trie 0.9 (CHECK docs.rs before implementing — this may have changed):

```rust
/// Verify a Merkle-Patricia proof against a trie root.
///
/// Arguments:
/// - `root`: The expected trie root hash (B256)
/// - `key`: The key as nibbles (use Nibbles::unpack(keccak256(raw_key)))
/// - `expected_value`: The expected value (None to verify non-existence)
/// - `proof`: The proof nodes (Vec of RLP-encoded trie nodes)
pub fn verify_proof(
    root: B256,
    key: Nibbles,
    expected_value: Option<Vec<u8>>,
    proof: &[Vec<u8>],
) -> Result<(), ProofVerificationError>;
```

### Usage for account proofs

```rust
use alloy_primitives::{B256, keccak256};
use alloy_trie::{Nibbles, proof::verify_proof};

fn verify_account(
    state_root: B256,
    address: Address,
    account_proof: &[Bytes],
) -> Result<(), ChainError> {
    // The key in the state trie is keccak256(address)
    let key = Nibbles::unpack(keccak256(address.as_slice()));

    // Convert Bytes to Vec<u8>
    let proof_nodes: Vec<Vec<u8>> = account_proof
        .iter()
        .map(|b| b.to_vec())
        .collect();

    verify_proof(state_root, key, None, &proof_nodes)
        .map_err(|e| ChainError::Rpc(format!("proof verification failed: {e}")))
}
```

### Usage for storage proofs

```rust
fn verify_storage(
    storage_root: B256,   // from the account proof's storage_hash
    slot: B256,
    storage_proof: &[Bytes],
) -> Result<(), ChainError> {
    // The key in the storage trie is keccak256(slot)
    let key = Nibbles::unpack(keccak256(slot.as_slice()));

    let proof_nodes: Vec<Vec<u8>> = storage_proof
        .iter()
        .map(|b| b.to_vec())
        .collect();

    verify_proof(storage_root, key, None, &proof_nodes)
        .map_err(|e| ChainError::Rpc(format!("storage proof verification failed: {e}")))
}
```

### Important notes

- The `Nibbles` type converts a 32-byte hash into a 64-nibble path (each byte → 2 nibbles)
- Account trie key: `keccak256(address)` — 20-byte address → 32-byte hash → 64 nibbles
- Storage trie key: `keccak256(slot)` — 32-byte slot → 32-byte hash → 64 nibbles
- The proof nodes are RLP-encoded trie nodes (branch, extension, or leaf)
- Passing `None` for `expected_value` verifies the key exists in the trie without checking the specific value

---

## 3. commonware-cryptography (NEW — to add)

**Version**: Latest (check crates.io — was `2026.4.0` as of last check)
**Purpose**: BLS12-381 threshold signature verification for Tempo/daeji
**Feature gate**: `threshold-bls`

### Cargo.toml

```toml
commonware-cryptography = { version = "*", optional = true }

[features]
threshold-bls = ["dep:commonware-cryptography"]
```

### Core API

The Commonware cryptography crate provides BLS12-381 operations used by Tempo's Threshold Simplex consensus.

```rust
use commonware_cryptography::bls12381;
```

### Key types

```rust
// Public key (G1 point, 48 bytes compressed)
// Secret key
// Signature (G2 point, 96 bytes compressed)
```

### Verification

The core verification for threshold BLS is a bilinear pairing check:

```
e(signature, G2_generator) == e(H(message), group_pubkey)
```

Where:
- `signature` is the threshold BLS signature (96 bytes, G2 point)
- `message` is the block header hash (32 bytes)
- `group_pubkey` is the static group public key (48 bytes, G1 point)
- `H()` is hash-to-curve (BLS12-381 standard)

```rust
// Pseudocode — exact API depends on commonware version:
use commonware_cryptography::bls12381::{PublicKey, Signature};

fn verify_threshold_sig(
    group_pubkey: &PublicKey,   // 48 bytes
    message: &[u8; 32],        // block header hash
    signature: &Signature,      // 96 bytes
) -> bool {
    group_pubkey.verify(message, signature)
}
```

### Important notes

- **Static group key**: Tempo uses DKG (Distributed Key Generation) to create a group key. This key is STABLE across validator rotations (resharing preserves the group public key). A verifier only needs to store one 48-byte key.
- **Single pairing check**: ~1-2ms. Compare to Ethereum sync committee: 512-key aggregation + pairing ~50ms.
- **Cert format**: ~240 bytes total (96-byte signature + 48-byte pubkey + metadata)

### Open question

The exact Commonware API for BLS verification may use a `Scheme` trait pattern:

```rust
// Possible API pattern from commonware-cryptography:
pub trait Scheme: Send + Sync {
    type PublicKey;
    type Signature;

    fn verify(public_key: &Self::PublicKey, message: &[u8], signature: &Self::Signature) -> bool;
}
```

Check `docs.rs/commonware-cryptography` for the exact `bls12381` module API before implementing. The Commonware project updates frequently.

---

## 4. hex (NEW — to add, unconditional)

**Version**: `0.4`
**Purpose**: Hex encoding/decoding for byte arrays
**Unconditional**: Small dependency, needed by playback verifier

### Cargo.toml

```toml
hex = "0.4"
```

### API

```rust
use hex;

// Decode hex string to bytes
let bytes: Vec<u8> = hex::decode("deadbeef")?;
let bytes: Vec<u8> = hex::decode("0xdeadbeef".strip_prefix("0x").unwrap())?;

// Encode bytes to hex string
let hex_str: String = hex::encode(&[0xde, 0xad]);  // "dead"
```

---

## 5. ethereum-consensus OR helios (Phase 6 — Ethereum)

Two options for Ethereum sync committee verification. Choose one.

### Option A: helios (recommended for v1)

**Version**: `0.11`
**Purpose**: Complete Ethereum light client
**Feature gate**: `sync-committee`

```toml
helios = { version = "0.11", optional = true }

[features]
sync-committee = ["dep:helios"]
```

**Pros**: Complete solution, maintained by a16z, handles committee rotations
**Cons**: Large dependency tree

```rust
use helios::ethereum::{EthereumClientBuilder, EthereumClient};

async fn create_verifier(
    execution_rpc: &str,
    consensus_rpc: &str,
    checkpoint: &str,
) -> Result<EthereumClient> {
    let client = EthereumClientBuilder::new()
        .execution_rpc(execution_rpc)
        .consensus_rpc(consensus_rpc)
        .checkpoint(checkpoint)
        .build()
        .await?;
    Ok(client)
}
```

### Option B: ethereum-consensus (for custom impl)

**Version**: Latest
**Purpose**: Beacon chain types and BLS verification

```toml
ethereum-consensus = { version = "*", optional = true }
```

Requires implementing committee tracking, rotation, and aggregate BLS verification manually. More work but smaller dependency.

---

## 6. mpp (Phase 10 — Tempo Machine Payments)

**Version**: `0.9`
**Purpose**: Machine Payments Protocol for agent-to-service payments
**Feature gate**: `mpp`

### Cargo.toml

```toml
mpp = { version = "0.9", optional = true }

[features]
mpp = ["dep:mpp"]
```

### Core API

```rust
use mpp::{Client, PaymentMiddleware, TempoProvider};

// Create MPP client
let client = Client::new(TempoProvider::new(
    "https://rpc.tempo.xyz",
    wallet_private_key,
))?;

// One-time payment
let response = client.pay(
    "https://api.service.com/data",
    amount_in_token_units,
    token_contract_address,
).await?;

// The response includes:
// - The service's HTTP response body
// - The on-chain transaction hash
// - The settlement details
```

### Payment modes

| Mode | Method | When to use |
|------|--------|-------------|
| One-time | `client.pay(url, amount, token)` | Single API call |
| Session | `client.open_session(url, budget, token)` | Multi-request session |
| Streaming | `client.stream(url, token, rate)` | Per-token billing (SSE) |

### `PaymentMiddleware` (for server-side)

```rust
// Wrap an HTTP client to automatically handle 402 responses
let middleware = PaymentMiddleware::new(client);
let response = middleware.request(url).await?;
// If service returns 402, middleware automatically pays and retries
```

### Important notes

- MPP uses Tempo's TIP-20 tokens (USDC on Tempo)
- Settlement is ~500ms (Tempo block time)
- The `tx_hash` in the response can be verified via the light client
- `mpp-rs` v0.9.2 as of 2026-05 — check for newer versions

---

## 7. Existing roko-chain Dependencies (for reference)

These are already in the workspace and available:

| Crate | Use in light-client work |
|-------|--------------------------|
| `async-trait` | All trait impls with async methods |
| `tokio` (sync feature) | `RwLock`, `mpsc`, `Mutex` for shared state |
| `serde` + `serde_json` | Serialize/deserialize all types |
| `thiserror` | `ConsensusError` derive |
| `blake3` | Hashing (available but may not be needed — keccak256 from alloy-primitives is the EVM hash) |
| `parking_lot` | Sync mutex/rwlock for non-async contexts |
| `alloy-primitives` | `Address`, `B256`, `U256`, `keccak256` |

---

## 8. Tempo Chain Facts (for implementation)

| Property | Value |
|----------|-------|
| Mainnet RPC | `https://rpc.tempo.xyz` |
| Testnet (Moderato) RPC | `https://rpc.moderato.tempo.xyz` |
| Mainnet chain ID | `4217` |
| Testnet chain ID | `42431` |
| Consensus | Threshold Simplex (Commonware) |
| Signature scheme | BLS12-381 threshold |
| Signature size | 96 bytes (G2 point) |
| Group pubkey size | 48 bytes (G1 point) |
| Finality cert size | ~240 bytes total |
| Block time | ~0.5s deterministic finality |
| Execution | Reth SDK (EVM, standard `eth_getProof`) |
| State proofs | Standard MPT (`eth_getProof`) |
| Gas model | No native gas — fees via TIP-20 + Fee AMM |
| Token standard | TIP-20 (ERC-20 + memo, RBAC, compliance hooks) |
| Payment standard | MPP (Machine Payments Protocol) |

### Existing roko config section

From `roko.toml`:
```toml
[chain]
rpc_url = "http://127.0.0.1:8545"
chain_id = 31337
```

The new `[chain.backends.*]` sections will coexist with this existing section. The existing `[chain]` section is for the default local devnet.
