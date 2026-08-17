# M131 — ChainConnector Cell (Connect Protocol Wrapper)

**[BLOCKED:chain]** -- Requires Phase 1 kernel (M012 Cell trait, M037 Connect protocol) and M076 (Solidity contracts). Chain deployment is Tier 6.

## Objective
Wrap the existing `ChainClient` (read) and `ChainWallet` (write) traits behind a unified `ChainConnector` Cell that implements the Connect protocol. This makes the chain a domain plugin -- identical in shape to any other Connector (database, LLM API, MCP). The Router selects among Connectors by kind, latency, and cost without knowing which one is a chain.

## Scope
- Crates: `roko-chain`, `roko-core`
- Files:
  - `crates/roko-chain/src/connector.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-export)
- Depth doc: `tmp/unified-depth/18-registries/01-chain-as-domain-plugin.md`

## Steps
1. Verify the Cell trait and Connect protocol exist in roko-core:
   ```bash
   grep -rn 'pub trait Cell' crates/roko-core/src/cell.rs
   grep -rn 'pub trait Connect' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Cell` at `crates/roko-core/src/cell.rs:14` with methods `cell_id()`, `cell_name()`, `cell_version()`, `protocols()`, `estimated_cost()`, `estimated_duration()`. `Connect` at `crates/roko-core/src/traits.rs:408` as a supertrait of `Cell` with methods `connect() -> Result<()>`, `health() -> bool`, `disconnect() -> Result<()>`.

2. Verify the existing ChainClient and ChainWallet traits:
   ```bash
   grep -rn 'pub trait ChainClient' crates/roko-chain/src/client.rs
   grep -rn 'pub trait ChainWallet' crates/roko-chain/src/wallet.rs
   grep -rn 'pub enum ChainError' crates/roko-chain/src/types.rs
   ```
   **Expected**: `ChainClient` at `client.rs:24` (async trait with `block_number`, `get_block_header`, `get_receipt`, `get_logs`, `get_storage_at`, `eth_call`, `get_balance`, `chain_id`, `name`). `ChainWallet` at `wallet.rs:17` (async trait with `address`, `balance`, `nonce`, `sign_and_submit`, `wait_for_receipt`, `name`). `ChainError` at `types.rs:122` (enum with `Rpc`, `Timeout`, `Offline`, `InsufficientFunds`, `NonceGap`, `InvalidAddress`, `Unsupported`).

3. Create `crates/roko-chain/src/connector.rs`:

   ```rust
   use std::sync::Arc;
   use std::time::Duration;
   use crate::client::ChainClient;
   use crate::wallet::ChainWallet;
   use crate::types::{ChainError, ChainResult, BlockNumber, TxHash, TxRequest, Receipt, LogEntry, CallResult};
   use roko_core::cell::{Cell, CellId, CellVersion};
   use roko_core::traits::Connect;
   use roko_core::error::Result;

   /// Connection state for the ChainConnector.
   #[derive(Debug, Clone, Copy, PartialEq, Eq)]
   pub enum ConnectorState {
       /// Not yet connected to any RPC endpoint.
       Disconnected,
       /// Connected and healthy.
       Connected,
       /// Connected but degraded (high latency or partial failures).
       Degraded,
   }

   /// Configuration for the ChainConnector.
   #[derive(Debug, Clone)]
   pub struct ChainConnectorConfig {
       /// Expected chain ID; connect() fails on mismatch.
       pub expected_chain_id: u64,
       /// RPC endpoints to try (in priority order).
       pub rpc_endpoints: Vec<String>,
       /// Number of confirmations before considering a tx final.
       pub confirmation_depth: u64,
       /// If true, no wallet is expected and write operations will fail.
       pub read_only: bool,
       /// Timeout for health check calls.
       pub health_timeout: Duration,
   }

   impl Default for ChainConnectorConfig {
       fn default() -> Self {
           Self {
               expected_chain_id: 1,
               rpc_endpoints: Vec::new(),
               confirmation_depth: 1,
               read_only: false,
               health_timeout: Duration::from_secs(5),
           }
       }
   }

   /// Unified Cell wrapping ChainClient (read) and ChainWallet (write).
   ///
   /// Implements the `Connect` protocol so the chain is a domain plugin.
   /// Domain-specific methods (`query_balance`, `submit_tx`, etc.) are
   /// additional inherent methods -- they do NOT live on the Connect trait,
   /// which only manages the connection lifecycle.
   pub struct ChainConnector {
       id: CellId,
       config: ChainConnectorConfig,
       client: Arc<dyn ChainClient>,
       wallet: Option<Arc<dyn ChainWallet>>,
       state: std::sync::atomic::AtomicU8, // 0=Disconnected, 1=Connected, 2=Degraded
   }
   ```

   - Implement `Cell` for `ChainConnector`: `cell_id`, `cell_name` = "chain-connector", `protocols` = `&["Connect"]`, `estimated_cost` = None
   - Implement `Connect` for `ChainConnector`:
     - `connect()` -> call `client.chain_id()`, compare with `config.expected_chain_id`, set state to Connected on match, return error on mismatch
     - `health()` -> call `client.block_number()` with timeout, return true if Ok within health_timeout
     - `disconnect()` -> transition to Disconnected state
   - Add domain-specific inherent methods (NOT on Connect trait):
     - `async fn query_balance(&self, address: &str) -> ChainResult<u128>` -> delegates to `client.get_balance()`
     - `async fn query_logs(&self, from: BlockNumber, to: BlockNumber, addresses: &[String], topics: &[String]) -> ChainResult<Vec<LogEntry>>` -> delegates to `client.get_logs()`
     - `async fn eth_call(&self, request: &TxRequest, block: Option<BlockNumber>) -> ChainResult<CallResult>` -> delegates to `client.eth_call()`
     - `async fn submit_tx(&self, tx: TxRequest) -> ChainResult<TxHash>` -> fails if wallet is None, else delegates to `wallet.sign_and_submit()`
     - `async fn wait_receipt(&self, tx: &TxHash, timeout_ms: u64) -> ChainResult<Receipt>` -> delegates to `wallet.wait_for_receipt()`
   - Implement `From<ChainError> for roko_core::error::RokoError` if needed, or map at callsite via `RokoError::substrate()`

4. Add module declaration and re-exports to `crates/roko-chain/src/lib.rs`:
   ```rust
   pub mod connector;
   pub use connector::{ChainConnector, ChainConnectorConfig, ConnectorState};
   ```

5. Write unit tests:
   - `connect()` with matching chain_id succeeds, state transitions to Connected
   - `connect()` with mismatched chain_id returns error, state stays Disconnected
   - `query_balance()` delegates to MockChainClient.get_balance()
   - `submit_tx()` fails when wallet is None (read-only mode)
   - `health()` returns false when client is offline (use MockChainClient to simulate)
   - Use `MockChainClient` and `MockChainWallet` from `crates/roko-chain/src/mock.rs`

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- connector
```

## What NOT to do
- Do NOT remove or modify ChainClient/ChainWallet traits -- ChainConnector wraps them
- Do NOT add query()/execute() to the Connect trait -- Connect only manages lifecycle; domain methods are inherent
- Do NOT implement Bus integration here -- that is the ChainBus (part of M138)
- Do NOT add alloy dependencies -- use the existing mock backend for tests (`MockChainClient`, `MockChainWallet`, `paired_mocks`)
- Do NOT wire into orchestrate.rs -- this batch only defines the Cell, not the runtime integration
