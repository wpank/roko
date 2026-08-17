# 01 — Architecture

## The insight

What varies between chains is **consensus verification** — how you know a block header
is finalized. Once you trust a header, state verification is the same for all EVM chains:
`eth_getProof` returns an MPT proof verifiable against `header.state_root`.

So the abstraction boundary is: one trait for consensus (per-chain), one function for
state proofs (shared across all EVM chains), and one return type (`VerifiedState<T>`)
that wraps everything.

```
                  ┌───────────────────────────────┐
                  │        VerifiedState<T>        │  ← what agents consume
                  └───────────────┬───────────────┘
                                  │
                  ┌───────────────┴───────────────┐
                  │    EVM State Proof Verifier    │  ← shared: verify MPT proof
                  │    (eth_getProof + alloy)      │     against trusted state_root
                  └───────────────┬───────────────┘
                                  │
           ┌──────────────────────┼──────────────────────┐
           │                      │                      │
  ┌────────┴────────┐  ┌─────────┴─────────┐  ┌────────┴────────┐
  │ Threshold BLS   │  │ Sync Committee    │  │ RPC-Trusted     │
  │ (Tempo, daeji)  │  │ (Ethereum)        │  │ (any, fallback) │
  └─────────────────┘  └───────────────────┘  └─────────────────┘
       consensus             consensus              no consensus
       verification          verification           verification
```

Non-EVM chains (Cosmos, Solana) need their own state proof verifiers too, but EVM
covers Tempo, Ethereum, all L2s, and daeji.

---

## Core types

### `ConsensusVerifier` — the extension point

```rust
/// Verifies that a block header is finalized on a specific chain.
/// This is what varies between chains. One impl per consensus mechanism.
#[async_trait]
pub trait ConsensusVerifier: Send + Sync {
    /// Verify a block header is finalized. Returns the trusted header.
    async fn verify_finality(
        &self,
        block: BlockNumber,
    ) -> Result<TrustedHeader, ConsensusError>;

    /// Subscribe to new finalized headers.
    async fn subscribe_finalized(
        &self,
    ) -> Result<mpsc::Receiver<TrustedHeader>, ConsensusError>;

    /// The latest finalized block this verifier trusts.
    async fn latest_finalized(&self) -> Result<TrustedHeader, ConsensusError>;

    /// Consensus mechanism identifier for logging/UI.
    fn mechanism(&self) -> &str;

    /// Trust level this consensus mechanism provides.
    fn trust_level(&self) -> TrustLevel;

    /// Health check.
    async fn is_healthy(&self) -> bool;
}
```

### `TrustedHeader`

```rust
/// A block header whose finality has been verified by a ConsensusVerifier.
/// The state_root in this header is trusted — MPT proofs can be verified against it.
#[derive(Debug, Clone)]
pub struct TrustedHeader {
    pub number: u64,
    pub hash: [u8; 32],
    pub state_root: [u8; 32],
    pub timestamp: u64,
    pub consensus_proof: ConsensusProof,
}

/// The proof that this header is finalized. Chain-specific.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusProof {
    /// Tempo / daeji: BLS12-381 threshold signature (~240 bytes).
    ThresholdBls {
        signature: Vec<u8>,    // 96 bytes (G2 point)
        group_pubkey: Vec<u8>, // 48 bytes (G1 point)
    },
    /// Ethereum: sync committee aggregate signature.
    SyncCommittee {
        aggregate_signature: Vec<u8>,
        participation_bits: Vec<u8>,
        committee_period: u64,
    },
    /// Tendermint: 2/3+ stake-weighted Ed25519 signatures.
    Tendermint {
        commit_signatures: Vec<Vec<u8>>,
        validator_set_hash: [u8; 32],
    },
    /// No consensus verification — trusted RPC.
    RpcTrusted,
    /// Deterministic playback from captured data.
    Playback { source_file: String },
}
```

### `TrustLevel`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Verified via consensus proof (BLS threshold, sync committee, etc.).
    Cryptographic,
    /// Returned by a trusted RPC, not independently verified.
    RpcTrusted,
    /// Synthetic/captured data for demo playback.
    Playback,
}
```

Simplified from the prior design: removed `LocalFollower` (it's just `RpcTrusted`
with a local URL) and `Mock` (renamed to `Playback` for clarity).

### `VerifiedState<T>`

```rust
/// A piece of chain state with verification provenance.
/// This is what agents and the UI always consume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedState<T: Serialize> {
    pub data: T,
    pub chain_id: u64,
    pub network: String,
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub block_timestamp: u64,
    pub trust_level: TrustLevel,
    pub consensus_mechanism: String,
    /// Serialized consensus proof for external audit. Empty for RPC-trusted.
    pub consensus_proof_bytes: Vec<u8>,
    /// Serialized state proof (MPT nodes). Empty for RPC-trusted.
    pub state_proof_bytes: Vec<u8>,
    pub verified_at: u64,  // unix timestamp ms
}
```

---

## Wiring into roko-chain

### Extending `ChainClient`

`ChainClient` (existing) is unverified RPC. Rather than replacing it, we wrap it:

```rust
/// A ChainClient that verifies every response via consensus + state proofs.
/// Implements ChainClient so it's a drop-in replacement anywhere ChainClient is used.
pub struct VerifiedChainClient {
    /// The underlying RPC client (AlloyChainClient or any ChainClient impl).
    rpc: Arc<dyn ChainClient>,
    /// Consensus verifier for this chain.
    consensus: Arc<dyn ConsensusVerifier>,
    /// Network name for VerifiedState metadata.
    network: String,
    /// Alloy provider for eth_getProof calls (needed beyond ChainClient's surface).
    proof_provider: Arc<DynProvider>,
}

#[async_trait]
impl ChainClient for VerifiedChainClient {
    async fn get_balance(&self, address: &str, block: Option<u64>) -> ChainResult<u128> {
        // 1. Get trusted header for the block
        let header = self.consensus.verify_finality(block.unwrap_or(/* latest */)).await?;
        // 2. Get MPT proof via eth_getProof
        let proof = self.proof_provider.get_proof(address, &[], block).await?;
        // 3. Verify MPT proof against header.state_root
        verify_account_proof(&header.state_root, address, &proof)?;
        // 4. Return the verified balance
        Ok(proof.balance.to::<u128>())
    }
    // ... same pattern for get_storage_at, eth_call, etc.
}
```

Because `VerifiedChainClient` implements `ChainClient`, it slots directly into the
existing `AgentState.chain_client` slot without changing any call sites.

### Activating the `chain_client` slot

Currently `AgentState.chain_client` is only read for stats. Wire it into routes:

```rust
// NEW: in roko-agent-server/src/features/chain.rs
pub fn chain_routes(state: &AgentState) -> Router {
    Router::new()
        .route("/chain/balance", post(handle_balance))
        .route("/chain/storage", post(handle_storage))
        .route("/chain/head", get(handle_head))
        .route("/chain/backends", get(handle_backends))
        .route("/chain/verify-transfer", post(handle_verify_transfer))
        // etc.
}
```

### Wiring chain tool handlers

The 17 `ToolDef` entries in `roko-chain/src/tools.rs` need dispatch handlers. These
go in a new `ChainToolHandler` that bridges `ToolDef` → `ChainClient`:

```rust
pub struct ChainToolHandler {
    client: Arc<dyn ChainClient>,  // could be VerifiedChainClient
    wallet: Option<Arc<dyn ChainWallet>>,
}

impl ChainToolHandler {
    pub async fn dispatch(&self, tool_name: &str, args: &Value) -> ToolResult {
        match tool_name {
            "chain.balance" => {
                let addr = args["address"].as_str()?;
                let balance = self.client.get_balance(addr, None).await?;
                Ok(ToolOutput::text(format!("{balance}")))
            }
            "chain.transfer" => { /* uses self.wallet */ }
            "chain.post_insight" => { /* calls InsightBoard contract via eth_call */ }
            // ... 14 more
            _ => Err(ToolError::NotFound(tool_name.into())),
        }
    }
}
```

Register in `ToolDispatcher` alongside existing tools.

### Driving `BlockObserver` with an async loop

`BlockObserver` is a pure filter — it needs someone to feed it blocks. Add a
`ChainWatcherTask` that polls or subscribes and feeds the observer:

```rust
pub struct ChainWatcherTask {
    client: Arc<dyn ChainClient>,
    observer: Mutex<BlockObserver>,
    consensus: Option<Arc<dyn ConsensusVerifier>>,
    event_tx: mpsc::Sender<ObservedEvent>,
    poll_interval: Duration,
}

impl ChainWatcherTask {
    pub async fn run(self) {
        let mut last_block = 0;
        loop {
            // If we have a consensus verifier, subscribe to finalized heads
            // Otherwise, poll block_number() on an interval
            let current = self.client.block_number().await.unwrap_or(last_block);
            if current > last_block {
                for n in (last_block + 1)..=current {
                    let header = self.client.get_block_header(n).await?;
                    let logs = self.client.get_logs(n, n, &[], &[]).await.unwrap_or_default();
                    let events = self.observer.lock().process_block(&header, &logs);
                    for event in events {
                        self.event_tx.send(event).await.ok();
                    }
                }
                last_block = current;
            }
            tokio::time::sleep(self.poll_interval).await;
        }
    }
}
```

When `consensus` is available, use `subscribe_finalized()` instead of polling for
verified-only event processing.

---

## Adapter registry

Mirror the LLM provider pattern. Static dispatch, one adapter per consensus mechanism:

```rust
pub trait ChainAdapter: Send + Sync {
    fn consensus_type(&self) -> &str;

    fn create_verifier(
        &self,
        config: &ChainBackendConfig,
    ) -> Result<Arc<dyn ConsensusVerifier>, ChainError>;

    fn classify_error(&self, err: &ChainError) -> ChainErrorClass;
}

// Static registry
static THRESHOLD_BLS_ADAPTER: ThresholdBlsAdapter = ThresholdBlsAdapter;
static SYNC_COMMITTEE_ADAPTER: SyncCommitteeAdapter = SyncCommitteeAdapter;
static RPC_ONLY_ADAPTER: RpcOnlyAdapter = RpcOnlyAdapter;
static PLAYBACK_ADAPTER: PlaybackAdapter = PlaybackAdapter;

pub fn adapter_for_consensus(mechanism: &str) -> Option<&'static dyn ChainAdapter> {
    match mechanism {
        "threshold_bls" => Some(&THRESHOLD_BLS_ADAPTER),
        "sync_committee" => Some(&SYNC_COMMITTEE_ADAPTER),
        "rpc" => Some(&RPC_ONLY_ADAPTER),
        "playback" => Some(&PLAYBACK_ADAPTER),
        _ => None,
    }
}
```

---

## Config surface

```toml
# roko.toml — existing [chain] section stays for default/local chain

[chain]
rpc_url = "http://127.0.0.1:8545"
chain_id = 31337

# NEW: named chain backends
[chain.backends.tempo-mainnet]
rpc_url = "https://rpc.tempo.xyz"
chain_id = 4217
consensus = "threshold_bls"
# BLS group public key (48 bytes hex). Stable across validator rotations.
group_pubkey = "0x..."
# Optional: bootstrap peers for p2p cert subscription
peer_addrs = ["tempo-peer1:9000"]

[chain.backends.tempo-testnet]
rpc_url = "https://rpc.moderato.tempo.xyz"
chain_id = 42431
consensus = "threshold_bls"
group_pubkey = "0x..."

[chain.backends.ethereum-mainnet]
rpc_url = "https://eth.llamarpc.com"
chain_id = 1
consensus = "sync_committee"
beacon_api_url = "https://lodestar-mainnet.chainsafe.io"

[chain.backends.tempo-demo]
chain_id = 4217
consensus = "playback"
playback_file = ".roko/chain-captures/tempo-demo.jsonl"

# MPP config (Tempo Machine Payments Protocol)
[mpp]
wallet_key_env = "NUNCHI_MPP_WALLET_KEY"
default_network = "tempo-mainnet"
```

### `ChainBackendConfig` struct

```rust
pub struct ChainBackendConfig {
    pub rpc_url: Option<String>,
    pub chain_id: u64,
    pub consensus: String,           // "threshold_bls" | "sync_committee" | "rpc" | "playback"
    // Threshold BLS specific
    pub group_pubkey: Option<String>,
    pub peer_addrs: Option<Vec<String>>,
    // Sync committee specific
    pub beacon_api_url: Option<String>,
    pub checkpoint_root: Option<String>,
    // Playback
    pub playback_file: Option<PathBuf>,
    // Shared
    pub timeout_ms: Option<u64>,
    pub max_concurrent: Option<u32>,
}
```

---

## Feature gating

```toml
[features]
default = []
alloy-backend = ["dep:alloy", "dep:alloy-primitives", "dep:reqwest"]
# Consensus verifiers (each optional, brings its own deps)
threshold-bls = ["dep:commonware-cryptography"]
sync-committee = ["dep:ethereum-consensus"]
```

The core types (`VerifiedState`, `TrustLevel`, `ConsensusProof`, `ChainAdapter` trait)
are always compiled. Concrete consensus verifiers are feature-gated.

---

## Resolution flow

When an agent calls `chain.balance(network="tempo-mainnet", address="0xABC")`:

```
1. Resolve "tempo-mainnet" → ChainBackendConfig from roko.toml
2. Look up or create VerifiedChainClient for this network:
   a. Create AlloyChainClient from rpc_url
   b. Create ConsensusVerifier via adapter_for_consensus(config.consensus)
   c. Wrap in VerifiedChainClient
3. Call verified_client.get_balance("0xABC", None)
   a. ConsensusVerifier.latest_finalized() → TrustedHeader
   b. proof_provider.eth_getProof("0xABC", [], block) → MPT proof
   c. Verify MPT proof against TrustedHeader.state_root
   d. Return balance wrapped in VerifiedState<u128>
4. Format as tool output with trust badge
```
