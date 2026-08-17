# 06 — Task Checklist

Status: `[ ]` not started, `[~]` in-progress, `[x]` done

Every task has the exact file, function, and code to write. Read [05-IMPLEMENTATION-PLAN.md](05-IMPLEMENTATION-PLAN.md) for full context on each phase.

---

## Phase 1: Core Types & Traits

- [ ] **1.1** Add `state_root: String` field to `ChainHeader`
  - File: `crates/roko-chain/src/types.rs` line 43
  - Add `pub state_root: String` after `parent` field
  - Doc comment: `/// State trie root hash (hex, 0x-prefixed). Needed for MPT proof verification.`

- [ ] **1.2** Update `MockChainClient` to populate `state_root`
  - File: `crates/roko-chain/src/mock.rs`
  - In `local()` genesis: `state_root: "0x0000000000000000000000000000000000000000000000000000000000000000".into()`
  - In `mine_empty_block()`: same zero hash
  - In `push_block()`: caller now must provide `state_root` in `ChainHeader`

- [ ] **1.3** Update `AlloyChainClient::get_block_header()` to populate `state_root`
  - File: `crates/roko-chain/src/alloy_impl.rs`
  - In `get_block_header()`: extract `block.header.state_root` from alloy response
  - Convert to hex string: `format!("{:?}", block.header.state_root)` or `format!("0x{}", hex::encode(block.header.state_root.as_slice()))`

- [ ] **1.4** Fix all `ChainHeader` construction sites in tests/other modules
  - Grep: `ChainHeader {` across `crates/roko-chain/src/`
  - Add `state_root: "0x00...00".into()` to each
  - Modules to check: `observer.rs`, `triage.rs`, `heartbeat_ext.rs`, test modules

- [ ] **1.5** Create `crates/roko-chain/src/consensus.rs`
  - Types: `TrustLevel` (enum: Cryptographic, RpcTrusted, Playback)
  - Types: `ConsensusProof` (enum: ThresholdBls, SyncCommittee, RpcTrusted, Playback)
  - Types: `TrustedHeader` (struct: number, hash, state_root, timestamp, consensus_proof)
  - Types: `ConsensusError` (enum: InvalidSignature, InsufficientParticipation, BlockUnavailable, NotSynced, Chain, Other)
  - Trait: `ConsensusVerifier` (5 methods: verify_finality, latest_finalized, mechanism, trust_level, is_healthy)
  - All types: derive Serialize + Deserialize where appropriate
  - Tests: serde roundtrips for TrustLevel, ConsensusProof, TrustedHeader

- [ ] **1.6** Create `crates/roko-chain/src/verified_state.rs`
  - Type: `VerifiedState<T: Serialize>` with fields: data, chain_id, network, block_number, block_hash, block_timestamp, trust_level, consensus_mechanism, consensus_proof_bytes, state_proof_bytes, verified_at
  - Tests: serde roundtrip with `VerifiedState<u128>` and `VerifiedState<String>`

- [ ] **1.7** Create `crates/roko-chain/src/adapter.rs`
  - Type: `ChainBackendConfig` (struct with all config fields)
  - Trait: `ChainAdapter` (consensus_type, create_verifier)
  - Function: `adapter_for_consensus(mechanism: &str) -> Option<Box<dyn ChainAdapter>>`
  - Impl: `RpcOnlyAdapter` (always returns RpcOnlyVerifier stub)
  - Impl: `PlaybackAdapter` (reads from playback_file config)
  - Tests: adapter_for_consensus("rpc") returns Some, adapter_for_consensus("unknown") returns None

- [ ] **1.8** Register new modules in `crates/roko-chain/src/lib.rs`
  - Add: `pub mod consensus;`
  - Add: `pub mod adapter;`
  - Add: `pub mod verified_state;`
  - Add pub use statements for key types

- [ ] **1.9** Verify Phase 1 compiles and tests pass
  - Run: `cargo test -p roko-chain`
  - Run: `cargo clippy -p roko-chain --no-deps -- -D warnings`
  - Run: `cargo test --workspace` (ensure no breakage from ChainHeader change)

---

## Phase 2: RPC-Only & Playback Verifiers

- [ ] **2.1** Add `hex = "0.4"` to `crates/roko-chain/Cargo.toml`
  - Under `[dependencies]`, unconditional (small dep, needed by playback hex parsing)

- [ ] **2.2** Implement `RpcOnlyVerifier` in `adapter.rs`
  - Struct: `RpcOnlyVerifier` (holds `Arc<dyn ChainClient>`)
  - Constructor: `new(client: Arc<dyn ChainClient>) -> Self`
  - `verify_finality()`: fetches header via client, returns TrustedHeader with ConsensusProof::RpcTrusted
  - `latest_finalized()`: calls `client.block_number()` then `verify_finality()`
  - `mechanism()`: returns `"rpc"`
  - `trust_level()`: returns `TrustLevel::RpcTrusted`
  - `is_healthy()`: calls `client.block_number()`, returns true if Ok

- [ ] **2.3** Create `crates/roko-chain/src/playback.rs`
  - Type: `PlaybackEntry` (serde-tagged enum: Header, Proof, Receipt)
  - Struct: `PlaybackVerifier` (BTreeMap<u64, TrustedHeader>, source String)
  - `from_file(path: &Path)`: read JSONL, parse headers, build map
  - `verify_finality(block)`: lookup in map, return TrustedHeader or BlockUnavailable
  - `latest_finalized()`: return last header in map
  - `mechanism()`: `"playback"`
  - `trust_level()`: `TrustLevel::Playback`
  - Helper: `hex_to_bytes32()` function

- [ ] **2.4** Create test fixture: `crates/roko-chain/src/testdata/demo-playback.jsonl`
  - 3-5 header entries with realistic-looking hashes and state roots
  - 1-2 proof entries
  - 1 receipt entry

- [ ] **2.5** Register `playback` module in `lib.rs`
  - Add: `pub mod playback;`
  - Add pub use for `PlaybackVerifier`, `PlaybackEntry`

- [ ] **2.6** Tests for Phase 2
  - `playback::tests::load_from_file` — reads test fixture, verifies header count
  - `playback::tests::verify_known_block` — returns correct TrustedHeader
  - `playback::tests::verify_unknown_block` — returns BlockUnavailable error
  - `playback::tests::latest_finalized` — returns highest block number
  - `adapter::tests::rpc_only_verifier` — construct with MockChainClient, verify_finality works
  - `adapter::tests::rpc_only_health` — is_healthy returns true with working mock

- [ ] **2.7** Verify Phase 2
  - `cargo test -p roko-chain`
  - `cargo clippy -p roko-chain --no-deps -- -D warnings`

---

## Phase 3: EVM State Proof Verification

- [ ] **3.1** Add `alloy-trie` dependency to `crates/roko-chain/Cargo.toml`
  - `alloy-trie = { version = "0.9", optional = true }`
  - Add to `alloy-backend` feature: `"dep:alloy-trie"`

- [ ] **3.2** Create `crates/roko-chain/src/state_proof.rs`
  - Gate: `#[cfg(feature = "alloy-backend")]` on the whole module
  - Function: `verify_account_proof(state_root: &[u8; 32], proof: &EIP1186AccountProofResponse) -> Result<VerifiedAccount>`
  - Function: `verify_storage_proof(storage_root: &[u8; 32], slot: &B256, proof: &EIP1186AccountProofResponse) -> Result<VerifiedStorageSlot>`
  - Type: `VerifiedAccount` (address, balance, nonce, code_hash, storage_hash)
  - Type: `VerifiedStorageSlot` (address, slot, value)
  - **IMPORTANT**: Check exact `alloy-trie` v0.9 API at `docs.rs/alloy-trie` before implementing. The `verify_proof()` signature may differ from the sketch in 05-IMPLEMENTATION-PLAN.md.

- [ ] **3.3** Register `state_proof` module in `lib.rs`
  - `#[cfg(feature = "alloy-backend")] pub mod state_proof;`

- [ ] **3.4** Capture test fixtures from Tempo testnet
  - Script: call `eth_getProof` on Tempo Moderato (chain 42431) for a known address
  - Save response as `crates/roko-chain/src/testdata/account_proof_tempo.json`
  - Also save the block header (with state_root) as `testdata/block_header_tempo.json`
  - Alternative: use any EVM testnet (Sepolia, Holesky) if Tempo unavailable

- [ ] **3.5** Tests for Phase 3 (feature-gated)
  - `#[cfg(feature = "alloy-backend")]` test: verify captured proof against captured state root
  - Test: invalid proof (wrong state root) fails verification
  - Test: missing storage proof for requested slot returns error

- [ ] **3.6** Verify Phase 3
  - `cargo test -p roko-chain --features alloy-backend`
  - `cargo clippy -p roko-chain --features alloy-backend --no-deps -- -D warnings`
  - Also: `cargo test -p roko-chain` (no features — state_proof module gated, should still pass)

---

## Phase 4: Verified Chain Client

- [ ] **4.1** Create `crates/roko-chain/src/verified_client.rs`
  - Struct: `VerifiedChainClient` (rpc, consensus, network, chain_id, proof_provider)
  - Constructor: `new(rpc, consensus, network, chain_id) -> Self`
  - Builder: `with_proof_provider(self, provider) -> Self` (feature-gated)
  - Method: `verified_balance(address, block) -> ChainResult<VerifiedState<u128>>`
  - Method: `verified_storage(address, slot, block) -> ChainResult<VerifiedState<Vec<u8>>>`
  - Method: `verify_transfer(tx_hash) -> ChainResult<VerifiedState<Receipt>>`
  - Impl `ChainClient` for `VerifiedChainClient` (delegate all 9 methods to inner rpc)

- [ ] **4.2** Register `verified_client` module in `lib.rs`
  - `pub mod verified_client;`
  - `pub use verified_client::VerifiedChainClient;`

- [ ] **4.3** Tests for Phase 4
  - Construct with MockChainClient + RpcOnlyVerifier
  - Test `get_balance()` (ChainClient impl) returns mock value
  - Test `verified_balance()` returns VerifiedState with RpcTrusted trust level
  - Test `verify_transfer()` with mock receipt
  - Compile test: `let _: Arc<dyn ChainClient> = Arc::new(VerifiedChainClient::new(...));`

- [ ] **4.4** Verify Phase 4
  - `cargo test -p roko-chain`
  - `cargo clippy -p roko-chain --no-deps -- -D warnings`

---

## Phase 5: Threshold BLS Verifier

- [ ] **5.1** Add `commonware-cryptography` to `Cargo.toml`
  - `commonware-cryptography = { version = "*", optional = true }` (pin version after testing)
  - Feature: `threshold-bls = ["dep:commonware-cryptography"]`

- [ ] **5.2** Create `crates/roko-chain/src/threshold_bls.rs`
  - Gate: `#[cfg(feature = "threshold-bls")]`
  - Struct: `ThresholdBlsVerifier` (group_pubkey, chain_id, rpc, verified_headers cache)
  - Constructor: `new(group_pubkey_hex, chain_id, rpc) -> Result<Self>`
  - Private: `verify_cert(header_hash, signature) -> Result<()>`
  - Private: `fetch_consensus_cert(block) -> Result<ConsensusCert>` — initially stub/placeholder
  - Impl `ConsensusVerifier` for `ThresholdBlsVerifier`
  - Struct: `ThresholdBlsAdapter` implementing `ChainAdapter`

- [ ] **5.3** Register in `adapter_for_consensus()`
  - Add `#[cfg(feature = "threshold-bls")]` match arm for `"threshold_bls"`

- [ ] **5.4** Register module in `lib.rs`
  - `#[cfg(feature = "threshold-bls")] pub mod threshold_bls;`

- [ ] **5.5** Tests for Phase 5
  - Unit test with Commonware BLS test vectors (if available)
  - Integration test against Tempo testnet (optional)
  - Mock test: construct verifier, feed known cert, verify passes

- [ ] **5.6** Verify Phase 5
  - `cargo test -p roko-chain --features threshold-bls`
  - `cargo clippy -p roko-chain --features threshold-bls --no-deps -- -D warnings`

---

## Phase 6: Sync Committee Verifier (Ethereum)

- [ ] **6.1** Decide: custom implementation vs Helios wrapper
  - Helios (`helios-light-client` v0.1.0): large dep but complete solution
  - Custom: smaller dep (`ethereum-consensus`) but must implement committee tracking
  - **Recommendation**: Start with Helios wrapper, migrate to custom if dep is too heavy

- [ ] **6.2** Add dependency
  - If Helios: `helios = { version = "0.11", optional = true }`
  - If custom: `ethereum-consensus = { version = "*", optional = true }`
  - Feature: `sync-committee = ["dep:helios"]` or `["dep:ethereum-consensus"]`

- [ ] **6.3** Create `crates/roko-chain/src/sync_committee.rs`
  - Gate: `#[cfg(feature = "sync-committee")]`
  - Struct: `SyncCommitteeVerifier` (or `HeliosVerifier`)
  - Impl `ConsensusVerifier`
  - Struct: `SyncCommitteeAdapter` implementing `ChainAdapter`

- [ ] **6.4** Register in `adapter_for_consensus()` and `lib.rs`

- [ ] **6.5** Tests for Phase 6
  - Integration test against Ethereum Sepolia beacon API (optional)
  - Mock test with captured sync committee data

- [ ] **6.6** Verify Phase 6
  - `cargo test -p roko-chain --features sync-committee`

---

## Phase 7: Chain Tool Dispatch Handlers

- [ ] **7.1** Create `crates/roko-chain/src/tool_handler.rs`
  - Struct: `ChainToolHandler` (client, wallet, Optional VerifiedChainClient, Optional MppClient)
  - Method: `dispatch(tool_name, args) -> Result<Value, ChainError>`
  - Handlers for 8 core tools:
    - `chain.balance` — `client.get_balance()`
    - `chain.transfer` — `wallet.sign_and_submit()`
    - `chain.gas_estimate` — `client.eth_call()` + gas calculation
    - `chain.simulate_tx` — `client.eth_call()`
    - `chain.wallet_create` — local key generation
    - `chain.wallet_list` — read config
    - `chain.wallet_info` — `client.get_balance()` + `wallet.nonce()`
    - `chain.wallet_export_address` — `wallet.address()`

- [ ] **7.2** Add verified tool handlers (new tool defs)
  - `chain.verified_balance` — `verified_client.verified_balance()`
  - `chain.verified_storage` — `verified_client.verified_storage()`
  - `chain.verify_transfer` — `verified_client.verify_transfer()`
  - `chain.head` — `consensus.latest_finalized()`
  - `chain.backends` — list configured backends from config

- [ ] **7.3** Add new tool defs to `tools.rs`
  - Add 5 new entries to `CHAIN_DOMAIN_TOOLS` array
  - Update `CHAIN_TOOL_COUNT` from 17 to 22
  - Update `CHAIN_TOOL_NAMES` array

- [ ] **7.4** Register `tool_handler` module in `lib.rs`
  - `pub mod tool_handler;`
  - `pub use tool_handler::ChainToolHandler;`

- [ ] **7.5** Wire into ToolDispatcher in roko-agent
  - File: `crates/roko-agent/src/` — find ToolDispatcher construction
  - Add chain domain registration when `AgentState.chain_client` is Some
  - Pattern: `dispatcher.register_domain("chain", chain_handler);`

- [ ] **7.6** Tests for Phase 7
  - Test each handler with MockChainClient + MockChainWallet
  - Test dispatch routing: `dispatch("chain.balance", ...)` calls correct handler
  - Test unknown tool: `dispatch("chain.unknown", ...)` returns Unsupported
  - Test transfer without wallet: returns error about missing wallet

- [ ] **7.7** Verify Phase 7
  - `cargo test -p roko-chain`
  - `cargo test -p roko-agent` (if wiring into ToolDispatcher)

---

## Phase 8: Block Observer Async Driver

- [ ] **8.1** Create `crates/roko-chain/src/watcher.rs`
  - Struct: `ChainWatcherTask` (client, observer, consensus, event_tx, poll_interval)
  - Method: `run(self)` — async loop: poll blocks, feed observer, emit events
  - Constructor: `new(client, observer_config, consensus, poll_interval)`
  - Uses `tokio::sync::mpsc::Sender<ObservedEvent>` for output

- [ ] **8.2** Wire into roko-serve / roko-cli startup
  - When chain config exists: `tokio::spawn(watcher.run())`
  - Pass event receiver to SSE endpoint in roko-serve

- [ ] **8.3** Tests for Phase 8
  - Construct with MockChainClient, mine blocks, verify events emitted
  - Test shutdown via drop/cancel

- [ ] **8.4** Verify Phase 8
  - `cargo test -p roko-chain`

---

## Phase 9: Config Integration

- [ ] **9.1** Add `ChainBackendsConfig` to roko-core config schema
  - File: `crates/roko-core/src/config/` — schema or chain section
  - Parse `[chain.backends.*]` table from `roko.toml`
  - Each entry becomes a `ChainBackendConfig`

- [ ] **9.2** Create backend factory function
  - `create_verified_client(config: &ChainBackendConfig) -> Result<Arc<VerifiedChainClient>>`
  - Steps: AlloyChainClient from rpc_url → adapter → verifier → VerifiedChainClient

- [ ] **9.3** Create backend pool/cache
  - `ChainBackendPool` that caches `VerifiedChainClient` by network name
  - `get_or_create(network: &str, config: &Config) -> Result<Arc<VerifiedChainClient>>`

- [ ] **9.4** Wire into orchestrator and agent startup
  - When agent needs chain access: look up network → get VerifiedChainClient
  - Set `AgentState.chain_client = Some(verified_client.clone())`

- [ ] **9.5** Tests for Phase 9
  - Parse example `roko.toml` with `[chain.backends.test]` section
  - Factory creates VerifiedChainClient with RpcOnly fallback
  - Pool caches and returns same instance

---

## Phase 10: MPP Integration

- [ ] **10.1** Add `mpp` dependency
  - `mpp = { version = "0.9", optional = true }`
  - Feature: `mpp = ["dep:mpp"]`

- [ ] **10.2** Create `crates/roko-chain/src/mpp_client.rs`
  - Gate: `#[cfg(feature = "mpp")]`
  - Struct: `MppClient` (inner mpp::Client, verifier, wallet)
  - Method: `pay_and_verify(service_url, amount, token) -> Result<VerifiedPayment>`
  - Method: `open_session(service_url, budget, token) -> Result<MppSession>`

- [ ] **10.3** Add `chain.mpp_pay` tool def and handler
  - New ToolDef in `tools.rs`
  - Handler in `tool_handler.rs` — delegates to MppClient

- [ ] **10.4** Register module in `lib.rs`

- [ ] **10.5** Tests for Phase 10
  - Mock MPP server + MockChainClient: pay, verify settlement
  - Test handler via dispatch

---

## Cross-Phase Verification

After each phase, run:
```bash
# No features (trait-only compilation)
cargo test -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings

# With alloy (state proofs + verified client)
cargo test -p roko-chain --features alloy-backend
cargo clippy -p roko-chain --features alloy-backend --no-deps -- -D warnings

# Full workspace (ensure no breakage)
cargo test --workspace
cargo clippy --workspace --no-deps -- -D warnings
```

After ALL phases, run:
```bash
# All features
cargo test -p roko-chain --all-features
cargo clippy -p roko-chain --all-features --no-deps -- -D warnings
```
