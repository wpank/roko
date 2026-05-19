# 08 — Existing Code Reference

Complete reference to existing roko-chain code that the light-client implementation must integrate with. Agents should read this before modifying any existing file.

---

## 1. ChainClient Trait

**File**: `crates/roko-chain/src/client.rs`

```rust
#[async_trait]
pub trait ChainClient: Send + Sync {
    async fn block_number(&self) -> ChainResult<BlockNumber>;
    async fn get_block_header(&self, number: BlockNumber) -> ChainResult<ChainHeader>;
    async fn get_receipt(&self, tx: &TxHash) -> ChainResult<Option<Receipt>>;
    async fn get_logs(
        &self, from: BlockNumber, to: BlockNumber,
        addresses: &[String], topics: &[String],
    ) -> ChainResult<Vec<LogEntry>>;
    async fn get_storage_at(
        &self, address: &str, slot: &str, block: Option<BlockNumber>,
    ) -> ChainResult<Vec<u8>>;
    async fn eth_call(
        &self, request: &TxRequest, block: Option<BlockNumber>,
    ) -> ChainResult<CallResult>;
    async fn get_balance(&self, address: &str, block: Option<BlockNumber>) -> ChainResult<u128>;
    async fn chain_id(&self) -> ChainResult<u64>;
    fn name(&self) -> &str;
}
```

**Key point**: `VerifiedChainClient` MUST implement this trait so it's a drop-in replacement. All 9 methods must be implemented (delegate to inner `rpc` client).

---

## 2. ChainWallet Trait

**File**: `crates/roko-chain/src/wallet.rs`

```rust
#[async_trait]
pub trait ChainWallet: Send + Sync {
    async fn address(&self) -> ChainResult<String>;
    async fn balance(&self, block: Option<BlockNumber>) -> ChainResult<u128>;
    async fn nonce(&self) -> ChainResult<u64>;
    async fn sign_and_submit(&self, tx: TxRequest) -> ChainResult<TxHash>;
    async fn wait_for_receipt(&self, tx: &TxHash, timeout_ms: u64) -> ChainResult<Receipt>;
    fn name(&self) -> &str;
}
```

**Key point**: `MppClient` needs a `ChainWallet` for signing payments. The `ChainToolHandler` needs an optional wallet for `chain.transfer` and write operations.

---

## 3. ChainHeader (needs modification)

**File**: `crates/roko-chain/src/types.rs` lines 42-52

Current:
```rust
pub struct ChainHeader {
    pub number: BlockNumber,
    pub hash: String,
    pub parent: String,
    pub timestamp: u64,
}
```

After modification (Phase 1.1):
```rust
pub struct ChainHeader {
    pub number: BlockNumber,
    pub hash: String,
    pub parent: String,
    pub timestamp: u64,
    pub state_root: String,  // NEW
}
```

**Impact**: Every place that constructs a `ChainHeader` must be updated. Search: `ChainHeader {` in all files under `crates/roko-chain/src/`.

---

## 4. AlloyChainClient

**File**: `crates/roko-chain/src/alloy_impl.rs`

```rust
#[derive(Clone)]
pub struct AlloyChainClient {
    provider: Arc<DynProvider>,
    name: String,
}

impl AlloyChainClient {
    pub fn http(rpc_url: &str) -> ChainResult<Self>
    pub fn provider(&self) -> Arc<DynProvider>
}
```

**Key point**: `AlloyChainClient::provider()` returns the alloy provider needed for `eth_getProof` calls. The `VerifiedChainClient` needs access to this provider for state proof fetching (beyond what `ChainClient` exposes).

The `get_block_header()` implementation currently:
```rust
async fn get_block_header(&self, number: BlockNumber) -> ChainResult<ChainHeader> {
    let block = self.provider
        .get_block_by_number(BlockNumberOrTag::Number(number), /* full txs */ false)
        .await
        .map_err(to_rpc_err)?
        .ok_or_else(|| ChainError::Rpc(format!("block {number} not found")))?;

    Ok(ChainHeader {
        number: block.header.number,
        hash: format!("{:?}", block.header.hash),
        parent: format!("{:?}", block.header.parent_hash),
        timestamp: block.header.timestamp,
        // state_root is available: block.header.state_root
    })
}
```

**Modification needed**: Add `state_root: format!("{:?}", block.header.state_root)` to the ChainHeader construction.

---

## 5. MockChainClient

**File**: `crates/roko-chain/src/mock.rs`

```rust
pub struct MockChainClient {
    state: Arc<RwLock<MockChainState>>,
    name: String,
}

impl MockChainClient {
    pub fn local() -> Self   // genesis block, chain_id 1
    pub fn with_chain_id(self, id: u64) -> Self
    pub fn push_block(&self, header: ChainHeader)
    pub fn mine_empty_block(&self) -> BlockNumber
    pub fn insert_receipt(&self, receipt: Receipt)
    pub fn insert_log(&self, log: LogEntry)
    pub fn set_balance(&self, address: &str, amount: u128)
    pub fn insert_storage(&self, address, slot, block, value)
}

pub fn paired_mocks(balance: u128) -> (MockChainClient, MockChainWallet)
```

**Key point**: `MockChainClient` is essential for testing. All light-client tests should use it. The `local()` constructor creates a genesis block — must be updated to include `state_root`.

---

## 6. ChainError

**File**: `crates/roko-chain/src/types.rs`

```rust
#[derive(Clone, Debug, thiserror::Error)]
pub enum ChainError {
    #[error("rpc error: {0}")]
    Rpc(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("offline; no reachable RPC")]
    Offline,
    #[error("insufficient funds (have {have}, need {need})")]
    InsufficientFunds { have: u128, need: u128 },
    #[error("nonce gap: expected {expected}, got {got}")]
    NonceGap { expected: u64, got: u64 },
    #[error("invalid address: {0}")]
    InvalidAddress(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
}
```

**Key point**: New consensus/verification errors should use `ConsensusError` (separate type in `consensus.rs`). `ChainError::Rpc(...)` can wrap consensus errors at the `ChainClient` boundary. Don't add consensus-specific variants to `ChainError` — keep it for RPC/wallet errors.

---

## 7. Chain Tool Definitions

**File**: `crates/roko-chain/src/tools.rs`

17 tools defined in `CHAIN_DOMAIN_TOOLS` lazy static array:

| Index | Name | Category |
|-------|------|----------|
| 0 | `chain.balance` | Layer 1 |
| 1 | `chain.transfer` | Layer 1 |
| 2 | `chain.approve` | Layer 2 |
| 3 | `chain.swap` | Layer 2 |
| 4 | `chain.add_liquidity` | Layer 2 |
| 5 | `chain.remove_liquidity` | Layer 2 |
| 6 | `chain.get_pool_info` | Layer 2 |
| 7 | `chain.get_position` | Layer 2 |
| 8 | `chain.simulate_tx` | Layer 1 |
| 9 | `chain.gas_estimate` | Layer 1 |
| 10 | `chain.wallet_create` | Wallet |
| 11 | `chain.wallet_list` | Wallet |
| 12 | `chain.wallet_info` | Wallet |
| 13 | `chain.wallet_export_address` | Wallet |
| 14 | `chain.post_insight` | Knowledge |
| 15 | `chain.search_insights` | Knowledge |
| 16 | `chain.confirm_insight` | Knowledge |

Each tool is a `ToolDef` (from `roko-core`) with:
- `name`, `description`, `parameters` (JSON schema)
- `category`, `permission`, `timeout_ms`, `concurrency`, `idempotent`
- `source`, `metadata`

**Key point**: Currently metadata-only — no dispatch handlers exist. Phase 7 creates `ChainToolHandler` that maps tool names to actual ChainClient/ChainWallet calls.

---

## 8. BlockObserver

**File**: `crates/roko-chain/src/observer.rs`

```rust
pub struct BlockObserver {
    config: BlockObserverConfig,
    filters: Vec<AddressFilter>,
    // ...
}

pub struct BlockObserverConfig {
    pub gap_threshold: u64,
    pub max_events_per_block: usize,
}

pub struct AddressFilter {
    pub address: String,
    pub topics: Vec<String>,
}

pub enum ObservedEvent {
    Transfer { from: String, to: String, amount: u128, block: u64 },
    Log { address: String, topics: Vec<String>, data: Vec<u8>, block: u64 },
    Gap { from: u64, to: u64 },
}
```

**Key point**: `BlockObserver` is a pure filter — it processes blocks fed to it and emits events. It does NOT poll or subscribe. The `ChainWatcherTask` (Phase 8) provides the async loop that feeds it blocks.

---

## 9. ChainWitnessEngine

**File**: `crates/roko-chain/src/witness.rs`

```rust
pub struct ChainWitnessEngine;

impl ChainWitnessEngine {
    pub async fn witness_on_chain(
        self,
        attestation: &mut Attestation,
        wallet: &dyn ChainWallet,
        client: &dyn ChainClient,
    ) -> ChainResult<TxHash>

    pub async fn verify_on_chain(
        self,
        attestation: &Attestation,
        client: &dyn ChainClient,
    ) -> ChainResult<bool>
}
```

**Constants**:
- `WITNESS_TO`: `"0x00000000000000000000000000000000000000c0"` — sink address for witness txs
- `WITNESS_MARKER`: `b"roko.attestation.witness:"`
- `DEFAULT_RECEIPT_TIMEOUT_MS`: `30_000`

**Key point**: The witness engine currently checks that a tx exists and its logs contain the expected marker. It does NOT verify the tx receipt against a block header via proof. Future enhancement: combine with `VerifiedChainClient` to get `VerifiedState<bool>` from `verify_on_chain()`.

---

## 10. X402 Manager (existing micropayments)

**File**: `crates/roko-chain/src/x402.rs`

```rust
pub struct X402Manager { /* ... */ }
pub struct PaymentRequest { pub recipient: Address, pub amount: u256, ... }
pub struct PaymentAuthorization { pub from: Address, ... pub v: u8, pub r: [u8; 32], pub s: [u8; 32] }
pub struct StateChannel { pub channel_id: [u8; 32], pub party_a: Address, ... }
pub enum VerificationStatus { Valid, Expired, NonceReused, ... }
```

**Key point**: X402 is the existing HTTP 402 micropayment system with off-chain state channels. MPP (Phase 10) is the Tempo-native payment standard. They coexist — X402 for generic HTTP 402, MPP for Tempo-specific payments. The light-client layer adds settlement verification to both.

---

## 11. Phase 2 Stubs

**File**: `crates/roko-chain/src/phase2.rs`

Contains many stub types for future use:
- `ConsensusType` enum: `CometBft`, `HotStuff`, `Custom(String)`
- `BlockHeader` struct (different from `ChainHeader` — has more fields)
- `BlockId` enum: `Number(u64)`, `Hash(B256)`, `Latest`
- `VerifyChainGate` struct (empty, for chain verification gate)
- `FraudProof`, `WorkProof`, `SlashingConfig`

**Key point**: These are NOT the types to use for the light-client implementation. Create fresh types in `consensus.rs` and `verified_state.rs`. The Phase 2 types are for the decentralized sequencer work — different from light-client consensus verification.

---

## 12. AgentState.chain_client

**Location**: `crates/roko-agent/` (agent state struct)

```rust
pub struct AgentState {
    // ...
    pub chain_client: Option<Arc<dyn ChainClient>>,
    // ...
}
```

**Current usage**: Only read for name string in stats display. NOT used for any chain operations.

**After light-client wiring**: Set to `Some(Arc::new(verified_chain_client))` during agent initialization when chain config exists. Then:
1. `ChainToolHandler` reads from this field
2. Agent sidecar chain routes use this field
3. MCP chain tools use this field

---

## 13. Module Registration Pattern

**File**: `crates/roko-chain/src/lib.rs`

New modules follow the existing pattern:
```rust
// Module declaration
pub mod consensus;

// Re-exports for convenience
pub use consensus::{ConsensusVerifier, TrustLevel, TrustedHeader, ConsensusProof, ConsensusError};
```

Feature-gated modules:
```rust
#[cfg(feature = "threshold-bls")]
pub mod threshold_bls;
```

---

## 14. Test Pattern

Existing tests in roko-chain use:
1. `MockChainClient::local()` for chain client
2. `MockChainWallet::funded(1_000_000_000_000_000_000)` for wallet
3. `paired_mocks(balance)` for client + wallet that auto-mint receipts
4. `#[tokio::test]` for async tests
5. `#[cfg(test)] mod tests { ... }` at bottom of each module

Follow this exact pattern for new tests.
