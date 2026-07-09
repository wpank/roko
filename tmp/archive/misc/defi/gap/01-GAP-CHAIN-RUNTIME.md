# 01 -- Chain runtime: agent-executable work batches

> **Batch count**: 7 (0.1 + 1.1–1.6) | **Total items**: ~35 | **Phase 0 batches**: 0.1, 1.1, 1.2
> **Primary crate**: `roko-chain`
> **Dependency root**: Batch 0.1 (test infra), Batch 1.1 (chain ops)

---

## Batch 0.1: Mirage-rs integration toolkit for DeFi agents

> **Effort**: M | **Depends on**: none | **Crate**: roko-chain
> **Branch**: `defi/batch-0.1-mirage-toolkit`

### Context

Every DeFi batch needs integration tests against real chain state — swaps, LP operations, protocol queries, risk simulations. Mocking these individually per batch is fragile and duplicative. The mirage-rs EVM simulator at `apps/mirage-rs/` already supports forking mainnet, advancing blocks, snapshot/revert, and scenario testing. But there's no ergonomic Rust harness for DeFi-specific testing patterns.

This batch creates a test toolkit in `roko-chain` (feature-gated behind `mirage-test`) that wraps mirage-rs primitives into DeFi-ready helpers. Every subsequent batch uses this toolkit for integration tests instead of building ad-hoc mocks.

The existing integration test infrastructure at `apps/mirage-rs/src/integration.rs` provides `spawn_mirage_test_instance()` and `MirageClient`. This batch wraps those into higher-level patterns: fork-at-block, simulate-swap, simulate-LP, get-pool-state, and a `TxSimulator` trait implementation.

### Read first

| File | Why |
|------|-----|
| `apps/mirage-rs/src/integration.rs` | `spawn_mirage_test_instance()`, `MirageClient::new()` — the primitives we wrap |
| `apps/mirage-rs/src/fork.rs` | Fork configuration, RPC URL handling, block number selection |
| `crates/roko-chain/src/heartbeat_ext.rs:151-194` | `TxSimulator` trait — the interface `MirageSimulator` implements |
| `crates/roko-chain/src/client.rs` | `ChainClient` trait — `MirageTestHarness` uses this for read operations |
| `crates/roko-chain/src/types.rs` | `TxRequest`, `Receipt`, `LogEntry` — types the harness produces |
| `crates/roko-chain/Cargo.toml` | Where to add the `mirage-test` feature gate |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

**0.1.1 — Add `mirage-test` feature gate to `roko-chain/Cargo.toml`**

```toml
[features]
default = ["alloy-backend"]
alloy-backend = ["alloy", "alloy-primitives", "alloy-sol-types"]
mirage-test = ["mirage-rs"]

[dev-dependencies]
mirage-rs = { path = "../../apps/mirage-rs" }
```

The feature gate ensures mirage-rs is only compiled for tests, never in production builds.

**0.1.2 — Create `MirageTestHarness` struct**

**File**: `crates/roko-chain/src/mirage_harness.rs` (CREATE)

```rust
//! Ergonomic test harness wrapping mirage-rs for DeFi integration tests.
//!
//! Feature-gated behind `mirage-test`. Provides helpers for forking mainnet
//! at a specific block, simulating swaps/LP operations, and querying pool state.

use mirage_rs::integration::{spawn_mirage_test_instance, MirageClient, MirageInstance};
use crate::types::{ChainResult, TxRequest, Receipt, LogEntry};

/// Test harness wrapping an ephemeral mirage-rs instance.
///
/// Manages the lifecycle of a forked EVM for integration tests.
/// Automatically shuts down on drop (best-effort) but prefer explicit
/// `shutdown()` for deterministic cleanup.
pub struct MirageTestHarness {
    instance: MirageInstance,
    client: MirageClient,
    fork_block: Option<u64>,
}

impl MirageTestHarness {
    /// Fork mainnet (or the given RPC) at a specific block.
    ///
    /// If `fork_block` is `None`, forks at latest.
    pub async fn fork_at_block(
        rpc_url: &str,
        fork_block: Option<u64>,
    ) -> ChainResult<Self> {
        let instance = spawn_mirage_test_instance(
            Some(rpc_url),
            fork_block.map(|b| b as usize),
        )
        .await
        .map_err(|e| crate::types::ChainError::Other(e.to_string()))?;

        let client = MirageClient::new(instance.config())
            .await
            .map_err(|e| crate::types::ChainError::Other(e.to_string()))?;

        Ok(Self { instance, client, fork_block })
    }

    /// Get a reference to the underlying `MirageClient` for raw RPC calls.
    pub fn client(&self) -> &MirageClient {
        &self.client
    }

    /// Take an EVM snapshot. Returns a snapshot ID for later revert.
    pub async fn snapshot(&self) -> ChainResult<String> {
        self.client
            .evm_snapshot()
            .await
            .map_err(|e| crate::types::ChainError::Other(e.to_string()))
    }

    /// Revert to a previous snapshot.
    pub async fn revert(&self, snapshot_id: &str) -> ChainResult<()> {
        self.client
            .evm_revert(snapshot_id.to_string())
            .await
            .map_err(|e| crate::types::ChainError::Other(e.to_string()))
    }

    /// Mine `n` blocks, advancing the chain.
    pub async fn mine_blocks(&self, n: u64) -> ChainResult<()> {
        for _ in 0..n {
            self.client
                .mirage_mine_block()
                .await
                .map_err(|e| crate::types::ChainError::Other(e.to_string()))?;
        }
        Ok(())
    }

    /// Shut down the mirage instance. Prefer this over relying on Drop.
    pub async fn shutdown(self) -> ChainResult<()> {
        self.instance
            .shutdown()
            .await
            .map_err(|e| crate::types::ChainError::Other(e.to_string()))
    }
}
```

**0.1.3 — Create DeFi helper functions**

**File**: `crates/roko-chain/src/mirage_harness.rs` (append to 0.1.2)

```rust
/// Uniswap V3 SwapRouter02 address on mainnet.
const UNISWAP_V3_ROUTER: &str = "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45";
/// Uniswap V3 Factory address on mainnet.
const UNISWAP_V3_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";

/// WETH address on mainnet.
const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
/// USDC address on mainnet.
const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

impl MirageTestHarness {
    /// Simulate a Uniswap V3 exactInputSingle swap without committing state.
    ///
    /// Uses `evm_snapshot` + `eth_sendTransaction` + `evm_revert` to simulate
    /// without side effects. Returns the output amount from the swap event.
    pub async fn simulate_swap(
        &self,
        token_in: &str,
        token_out: &str,
        amount_in: u128,
        fee_tier: u32,
    ) -> ChainResult<SwapSimResult> {
        let snap = self.snapshot().await?;

        // Encode exactInputSingle(ExactInputSingleParams) calldata
        // ExactInputSingleParams: tokenIn, tokenOut, fee, recipient, amountIn, amountOutMinimum, sqrtPriceLimitX96
        let calldata = alloy_sol_types::sol! {
            function exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))
                external payable returns (uint256 amountOut);
        };
        let params = (
            token_in.parse::<alloy_primitives::Address>().unwrap(),
            token_out.parse::<alloy_primitives::Address>().unwrap(),
            fee_tier,
            alloy_primitives::Address::ZERO, // recipient = self
            alloy_primitives::U256::from(amount_in),
            alloy_primitives::U256::ZERO,     // amountOutMinimum = 0 for simulation
            alloy_primitives::U256::ZERO,     // sqrtPriceLimitX96 = 0 (no limit)
        );
        let encoded = calldata::exactInputSingleCall::new(params).abi_encode();

        let receipt = self.client
            .eth_send_transaction_raw(UNISWAP_V3_ROUTER, &encoded)
            .await
            .map_err(|e| crate::types::ChainError::Other(e.to_string()))?;

        self.revert(&snap).await?;

        Ok(SwapSimResult {
            amount_out: receipt.output_amount(),
            gas_used: receipt.gas_used,
            success: receipt.status,
        })
    }

    /// Query the current state of a Uniswap V3 pool.
    ///
    /// Returns tick, sqrtPriceX96, liquidity, and fee growth accumulators.
    pub async fn get_pool_state(
        &self,
        token_a: &str,
        token_b: &str,
        fee_tier: u32,
    ) -> ChainResult<PoolSnapshot> {
        // Call factory.getPool(tokenA, tokenB, fee) to find pool address
        let factory_call = alloy_sol_types::sol! {
            function getPool(address, address, uint24) external view returns (address);
        };
        let pool_addr = self.client
            .eth_call_decode(UNISWAP_V3_FACTORY, &factory_call, (token_a, token_b, fee_tier))
            .await?;

        // Call pool.slot0() for tick + sqrtPriceX96
        let slot0_call = alloy_sol_types::sol! {
            function slot0() external view returns (uint160, int24, uint16, uint16, uint16, uint8, bool);
        };
        let (sqrt_price, tick, ..) = self.client
            .eth_call_decode(&pool_addr, &slot0_call, ())
            .await?;

        // Call pool.liquidity()
        let liq_call = alloy_sol_types::sol! {
            function liquidity() external view returns (uint128);
        };
        let liquidity = self.client
            .eth_call_decode(&pool_addr, &liq_call, ())
            .await?;

        Ok(PoolSnapshot {
            pool_address: pool_addr,
            sqrt_price_x96: sqrt_price,
            tick,
            liquidity,
            fee_tier,
        })
    }

    /// Simulate adding liquidity to a Uniswap V3 position.
    ///
    /// Uses snapshot/revert pattern like `simulate_swap`.
    pub async fn simulate_lp_add(
        &self,
        token_a: &str,
        token_b: &str,
        fee_tier: u32,
        tick_lower: i32,
        tick_upper: i32,
        amount_a: u128,
        amount_b: u128,
    ) -> ChainResult<LpSimResult> {
        let snap = self.snapshot().await?;

        // Encode mint() calldata via NonfungiblePositionManager
        // This is a placeholder — the actual encoding depends on the position manager ABI
        let receipt = self.client
            .eth_send_transaction_raw(
                "0xC36442b4a4522E871399CD717aBDD847Ab11FE88", // NonfungiblePositionManager
                &Self::encode_mint_params(token_a, token_b, fee_tier, tick_lower, tick_upper, amount_a, amount_b),
            )
            .await
            .map_err(|e| crate::types::ChainError::Other(e.to_string()))?;

        self.revert(&snap).await?;

        Ok(LpSimResult {
            token_id: receipt.return_data_u256(0),
            liquidity: receipt.return_data_u128(1),
            amount_a_used: receipt.return_data_u128(2),
            amount_b_used: receipt.return_data_u128(3),
            gas_used: receipt.gas_used,
        })
    }

    fn encode_mint_params(
        _token_a: &str, _token_b: &str, _fee_tier: u32,
        _tick_lower: i32, _tick_upper: i32,
        _amount_a: u128, _amount_b: u128,
    ) -> Vec<u8> {
        // Use alloy_sol_types::sol! to encode MintParams struct
        // Left as implementation detail — pattern matches simulate_swap encoding
        todo!("encode NonfungiblePositionManager.mint() calldata — follow simulate_swap pattern")
    }
}

/// Result of a simulated swap.
#[derive(Debug, Clone)]
pub struct SwapSimResult {
    /// Output token amount received.
    pub amount_out: u128,
    /// Gas consumed by the swap.
    pub gas_used: u64,
    /// Whether the transaction succeeded.
    pub success: bool,
}

/// Snapshot of a Uniswap V3 pool's on-chain state.
#[derive(Debug, Clone)]
pub struct PoolSnapshot {
    pub pool_address: String,
    pub sqrt_price_x96: u128,
    pub tick: i32,
    pub liquidity: u128,
    pub fee_tier: u32,
}

/// Result of a simulated LP add.
#[derive(Debug, Clone)]
pub struct LpSimResult {
    pub token_id: u128,
    pub liquidity: u128,
    pub amount_a_used: u128,
    pub amount_b_used: u128,
    pub gas_used: u64,
}
```

**0.1.4 — Create `MirageSimulator` implementing `TxSimulator`**

**File**: `crates/roko-chain/src/mirage_simulator.rs` (CREATE)

```rust
//! [`MirageSimulator`] implements the `TxSimulator` trait from `heartbeat_ext.rs`
//! using an ephemeral mirage-rs fork for accurate pre-trade simulation.

use async_trait::async_trait;
use crate::mirage_harness::MirageTestHarness;
use crate::types::{ChainResult, TxRequest};

/// Pre-trade simulator backed by an ephemeral mirage-rs EVM fork.
///
/// Used by the heartbeat decision pipeline (step 5: SIMULATE) and by
/// `DeFiRiskEngine::simulate_trade()` to validate transactions before
/// signing and broadcasting.
///
/// Each `MirageSimulator` owns one `MirageTestHarness`. Create a new
/// simulator per simulation batch; they are not reused across ticks.
pub struct MirageSimulator {
    harness: MirageTestHarness,
}

/// Result of a transaction simulation.
#[derive(Debug, Clone)]
pub struct SimulateResult {
    /// Whether the transaction succeeded (did not revert).
    pub success: bool,
    /// Gas consumed.
    pub gas_used: u64,
    /// Output data from the transaction (decoded by caller).
    pub output: Vec<u8>,
    /// Logs emitted during execution.
    pub logs: Vec<crate::types::LogEntry>,
    /// State changes summary (balance deltas for sender/receiver).
    pub balance_deltas: Vec<BalanceDelta>,
}

/// Balance change observed during simulation.
#[derive(Debug, Clone)]
pub struct BalanceDelta {
    pub token: String,
    pub address: String,
    pub delta: i128, // positive = received, negative = sent
}

impl MirageSimulator {
    /// Create a new simulator by forking the given RPC at the current block.
    pub async fn new(rpc_url: &str) -> ChainResult<Self> {
        let harness = MirageTestHarness::fork_at_block(rpc_url, None).await?;
        Ok(Self { harness })
    }

    /// Create a simulator forked at a specific block.
    pub async fn at_block(rpc_url: &str, block: u64) -> ChainResult<Self> {
        let harness = MirageTestHarness::fork_at_block(rpc_url, Some(block)).await?;
        Ok(Self { harness })
    }

    /// Simulate a transaction using snapshot/revert for isolation.
    pub async fn simulate(&self, tx: &TxRequest) -> ChainResult<SimulateResult> {
        let snap = self.harness.snapshot().await?;

        let receipt = self.harness.client()
            .eth_send_transaction_raw(&tx.to, &tx.data)
            .await
            .map_err(|e| crate::types::ChainError::Other(e.to_string()))?;

        // Extract logs and balance changes from receipt before reverting
        let logs = receipt.logs.iter().map(|l| crate::types::LogEntry {
            address: format!("{:?}", l.address),
            topics: l.topics.iter().map(|t| format!("{:?}", t)).collect(),
            data: l.data.to_vec(),
            block_number: receipt.block_number,
            tx_hash: format!("{:?}", receipt.transaction_hash),
            log_index: l.log_index.unwrap_or(0),
        }).collect();

        self.harness.revert(&snap).await?;

        Ok(SimulateResult {
            success: receipt.status,
            gas_used: receipt.gas_used,
            output: receipt.output.to_vec(),
            logs,
            balance_deltas: vec![], // TODO: parse Transfer events for balance deltas
        })
    }

    /// Shut down the underlying mirage instance.
    pub async fn shutdown(self) -> ChainResult<()> {
        self.harness.shutdown().await
    }
}
```

**0.1.5 — Wire modules into `roko-chain/src/lib.rs`**

Add conditional module declarations:

```rust
#[cfg(feature = "mirage-test")]
pub mod mirage_harness;
#[cfg(feature = "mirage-test")]
pub mod mirage_simulator;
```

**0.1.6 — Integration test demonstrating the toolkit**

**File**: `crates/roko-chain/tests/mirage_toolkit.rs` (CREATE)

```rust
//! Integration tests demonstrating the mirage test toolkit.
//!
//! Run with: `cargo test -p roko-chain --features mirage-test --test mirage_toolkit`
//! Requires network access (forks mainnet via public RPC).

#![cfg(feature = "mirage-test")]

use roko_chain::mirage_harness::MirageTestHarness;
use roko_chain::mirage_simulator::MirageSimulator;

const PUBLIC_RPC: &str = "https://eth-rpc.publicnode.com";
// Block with known Uniswap V3 WETH/USDC activity
const FORK_BLOCK: u64 = 18_545_000;

#[tokio::test]
async fn harness_forks_and_queries_pool_state() {
    let harness = MirageTestHarness::fork_at_block(PUBLIC_RPC, Some(FORK_BLOCK))
        .await
        .expect("should fork mainnet");

    let pool = harness
        .get_pool_state(
            "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", // WETH
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
            3000, // 0.3% fee tier
        )
        .await
        .expect("should query pool state");

    assert_ne!(pool.liquidity, 0, "WETH/USDC pool should have liquidity");
    assert_ne!(pool.tick, 0, "tick should be non-zero");

    harness.shutdown().await.expect("clean shutdown");
}

#[tokio::test]
async fn simulator_snapshot_revert_is_isolated() {
    let sim = MirageSimulator::at_block(PUBLIC_RPC, FORK_BLOCK)
        .await
        .expect("should create simulator");

    // First simulation
    let result1 = sim.simulate(&test_tx()).await.expect("sim 1 should succeed");
    // Second simulation on same state (revert happened)
    let result2 = sim.simulate(&test_tx()).await.expect("sim 2 should succeed");

    // Both should produce identical results because state was reverted
    assert_eq!(result1.gas_used, result2.gas_used, "gas should match after revert");

    sim.shutdown().await.expect("clean shutdown");
}

fn test_tx() -> roko_chain::types::TxRequest {
    // Simple ETH transfer — always succeeds, deterministic gas
    roko_chain::types::TxRequest {
        to: "0x0000000000000000000000000000000000000001".to_string(),
        data: vec![],
        value: Some(1000),
        gas_limit: Some(21000),
    }
}
```

### Verification

```bash
# Feature gate compiles
cargo check -p roko-chain --features mirage-test

# Module declarations correct
grep -n 'mirage_harness\|mirage_simulator' crates/roko-chain/src/lib.rs

# Types are public
cargo doc -p roko-chain --features mirage-test --no-deps 2>&1 | grep -i 'mirage'

# Integration tests pass (requires network)
cargo test -p roko-chain --features mirage-test --test mirage_toolkit
```

### Acceptance criteria

- `[ ]` `mirage-test` feature gate in `roko-chain/Cargo.toml` — `mirage-rs` only compiled when feature enabled
- `[ ]` `MirageTestHarness` struct with `fork_at_block()`, `snapshot()`, `revert()`, `mine_blocks()`, `shutdown()`
- `[ ]` DeFi helpers: `simulate_swap()`, `get_pool_state()`, `simulate_lp_add()` on `MirageTestHarness`
- `[ ]` `MirageSimulator` struct implementing `simulate()` with snapshot/revert isolation
- `[ ]` `SimulateResult` type with `success`, `gas_used`, `output`, `logs`, `balance_deltas`
- `[ ]` `SwapSimResult`, `PoolSnapshot`, `LpSimResult` helper types
- `[ ]` Modules conditionally compiled (`#[cfg(feature = "mirage-test")]`)
- `[ ]` Integration test demonstrating fork → query → simulate → revert → shutdown
- `[ ]` `cargo check -p roko-chain` (without feature) still compiles — no mirage leakage

### Commit message

```
feat(roko-chain): add mirage-rs integration toolkit for DeFi testing

MirageTestHarness wraps spawn_mirage_test_instance() with DeFi-specific
helpers (simulate_swap, get_pool_state, simulate_lp_add). MirageSimulator
implements TxSimulator via snapshot/revert for pre-trade validation.
Feature-gated behind `mirage-test` to avoid production dependency.
```

---

## Batch 1.1: Implement `get_logs` on `AlloyChainClient`

> **Effort**: S | **Depends on**: none | **Crate**: roko-chain
> **Branch**: `defi/batch-1.1-alloy-get-logs`

### Context

The `ChainClient` trait declares `get_logs(from, to, addresses, topics)` at `crates/roko-chain/src/client.rs:40`. The mock implementation in `crates/roko-chain/src/mock.rs:184` works -- it filters inserted logs by address and topic. But the Alloy implementation at `crates/roko-chain/src/alloy_impl.rs:147` returns `ChainError::Unsupported("get_logs")`.

Every downstream chain component depends on `get_logs`: the block observer uses it for gap backfill, the triage pipeline needs historical events, and the DeFi tool handlers need log data for position queries. This batch implements the method using alloy's `eth_getLogs` JSON-RPC call.

The alloy provider already exposes `get_logs` through its `Provider` trait. The work is converting our `from`/`to`/`addresses`/`topics` parameters into an alloy `Filter` struct and mapping the returned `Log` values back to our `LogEntry` type. The conversion pattern already exists in `get_receipt` at line 112, which maps alloy logs to `LogEntry`.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/alloy_impl.rs` | Current Alloy client; see `get_receipt` at line 112 for the `Log` -> `LogEntry` conversion pattern |
| `crates/roko-chain/src/client.rs` | `ChainClient` trait definition; `get_logs` signature at line 40 |
| `crates/roko-chain/src/types.rs` | `LogEntry` struct at line 103, `ChainError` at line 121 |
| `crates/roko-chain/src/mock.rs:184` | Working mock `get_logs` -- match this behavior |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Implement `get_logs` on `AlloyChainClient`

**File**: `crates/roko-chain/src/alloy_impl.rs` (EDIT)

**What to write**:

Replace the stub at line 147 with a working implementation. Build an alloy `Filter` from the parameters, call `self.provider.get_logs(&filter)`, and convert the result to `Vec<LogEntry>`.

```rust
async fn get_logs(
    &self,
    from: BlockNumber,
    to: BlockNumber,
    addresses: &[String],
    topics: &[String],
) -> ChainResult<Vec<LogEntry>> {
    use alloy::rpc::types::eth::Filter;

    let mut filter = Filter::new().from_block(from).to_block(to);

    if !addresses.is_empty() {
        let addrs: Vec<Address> = addresses
            .iter()
            .map(|a| parse_hex_address(a))
            .collect::<ChainResult<Vec<_>>>()?;
        filter = filter.address(addrs);
    }

    if !topics.is_empty() {
        let topic_hashes: Vec<B256> = topics
            .iter()
            .map(|t| parse_hex_b256(t))
            .collect::<ChainResult<Vec<_>>>()?;
        filter = filter.event_signature(topic_hashes);
    }

    let logs = self.provider.get_logs(&filter).await.map_err(to_rpc_err)?;

    Ok(logs
        .into_iter()
        .map(|l| LogEntry {
            address: format!("{:#x}", l.inner.address),
            topics: l
                .inner
                .data
                .topics()
                .iter()
                .map(|t| format!("{t:#x}"))
                .collect(),
            data: l.inner.data.data.to_vec(),
        })
        .collect())
}
```

**Reuse**: `crates/roko-chain/src/alloy_impl.rs:123` -- the `Log` to `LogEntry` mapping in `get_receipt` is identical. Extract into a helper if you prefer, but inline is fine for two call sites.

**Do NOT**: change the `ChainClient` trait signature. The existing `get_logs` parameter list is correct.

#### Item 2: Add integration-ready unit tests

**File**: `crates/roko-chain/src/alloy_impl.rs` (EDIT -- append to existing `#[cfg(test)]` block, or create one if absent)

**What to write**:

These tests verify the Filter construction logic. They cannot hit a real RPC in CI, so they test the conversion helpers and the mock contract.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_address_valid() {
        let addr = parse_hex_address("0x0000000000000000000000000000000000000001");
        assert!(addr.is_ok());
    }

    #[test]
    fn parse_hex_address_invalid() {
        let addr = parse_hex_address("not-an-address");
        assert!(addr.is_err());
    }

    #[test]
    fn parse_hex_b256_valid() {
        let hash = parse_hex_b256(
            "0x0000000000000000000000000000000000000000000000000000000000000001",
        );
        assert!(hash.is_ok());
    }

    #[test]
    fn tx_request_to_alloy_minimal() {
        let tx = TxRequest {
            to: Some("0x0000000000000000000000000000000000000001".into()),
            value: 1000,
            ..Default::default()
        };
        let result = tx_request_to_alloy(&tx);
        assert!(result.is_ok());
    }
}
```

### Wiring

No new modules or config changes. This edits an existing method body in an existing file.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_address_valid() {
        let addr = parse_hex_address("0x0000000000000000000000000000000000000001");
        assert!(addr.is_ok());
    }

    #[test]
    fn parse_hex_address_invalid() {
        assert!(parse_hex_address("nope").is_err());
    }

    #[test]
    fn parse_hex_b256_valid() {
        let h = parse_hex_b256(
            "0x0000000000000000000000000000000000000000000000000000000000000001",
        );
        assert!(h.is_ok());
    }
}
```

#### Item 3: Mirage-rs integration test for `get_logs`

**File**: `crates/roko-chain/tests/mirage_get_logs.rs` (CREATE)

**What to write**:

This test spawns an ephemeral mirage-rs instance forked from mainnet, calls `get_logs` against a known Uniswap V3 pool to retrieve Swap events, and verifies the returned `LogEntry` values match expected topic hashes. Gate this behind a `mirage` feature so CI can skip it without an RPC endpoint.

```rust
//! Integration test: get_logs against a forked chain via mirage-rs.

#[cfg(feature = "mirage")]
mod mirage_get_logs {
    use mirage_rs::integration::spawn_mirage_test_instance;
    use mirage_rs::client::MirageClient;
    use roko_chain::client::ChainClient;
    use roko_chain::alloy_impl::AlloyChainClient;

    #[tokio::test]
    async fn get_logs_returns_swap_events_from_forked_chain() {
        let rpc_url = "https://eth-rpc.publicnode.com";
        let fork_block = 18_000_000u64;

        let mut instance = spawn_mirage_test_instance(
            Some(rpc_url), Some(fork_block),
        ).await.expect("mirage instance");

        let client = AlloyChainClient::new(&instance.rpc_url())
            .await
            .expect("alloy client");

        // Uniswap V3 USDC/WETH pool
        let pool = "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640";
        // Swap topic
        let swap_topic = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

        let logs = client
            .get_logs(fork_block - 10, fork_block, &[pool.into()], &[swap_topic.into()])
            .await
            .expect("get_logs");

        assert!(!logs.is_empty(), "should find Swap events in recent blocks");
        for log in &logs {
            assert_eq!(log.address.to_lowercase(), pool.to_lowercase());
            assert!(log.topics.iter().any(|t| t == swap_topic));
        }

        instance.shutdown().await.expect("shutdown");
    }
}
```

Add to `crates/roko-chain/Cargo.toml` under `[dev-dependencies]`:
```toml
mirage-rs = { path = "../../apps/mirage-rs", optional = true }
```

And under `[features]`:
```toml
mirage = ["dep:mirage-rs"]
```

### Verification

```bash
cargo test -p roko-chain -- alloy_impl
cargo test -p roko-chain --features mirage -- mirage_get_logs  # requires RPC access
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `AlloyChainClient::get_logs` no longer returns `ChainError::Unsupported`
- [ ] Builds an alloy `Filter` with correct `from_block`, `to_block`, `address`, and topic filters
- [ ] Converts alloy `Log` to `LogEntry` using the same pattern as `get_receipt`
- [ ] Empty `addresses`/`topics` slices mean "match all" (no filter applied)
- [ ] At least 3 unit tests pass
- [ ] `cargo clippy` clean
- [ ] `cargo +nightly fmt --check` clean
- [ ] Mirage integration test passes with `--features mirage` (verifies `get_logs` against forked mainnet state)

### Commit message

```
feat(roko-chain): implement get_logs on AlloyChainClient via eth_getLogs
```

---

## Batch 1.2: WebSocket subscription + event bus integration

> **Effort**: L | **Depends on**: 1.1 | **Crate**: roko-chain
> **Branch**: `defi/batch-1.2-ws-event-bus`

### Context

The block observer at `crates/roko-chain/src/observer.rs` is a synchronous batch processor. It can filter logs against watched addresses and detect block gaps, but it has no network layer. You call `process_block(header, logs)` and feed it data yourself. There is no WebSocket subscription, no async polling loop, and no event channel.

This batch builds the async runtime loop that drives the observer. It adds a `ChainEventSource` that polls `ChainClient::block_number()` on a configurable interval, fetches new block headers and logs via `get_block_header` and `get_logs` (the method implemented in batch 1.1), runs them through `BlockObserver`, and sends matched `ObservedEvent` values into a `tokio::mpsc` channel. The receiver of that channel is the triage pipeline or any other downstream consumer.

The design uses polling rather than WebSocket subscription. WebSocket support requires adding `alloy`'s WS transport feature and a `WsConnect` provider variant, which is a separate concern. Polling against `block_number()` works with every `ChainClient` implementation including mocks, and the 1-second default poll interval is fast enough for DeFi event detection.

**Deployment note**: WS subscriptions and the `ChainEventSource` polling loop run in the roko control plane (Railway, always-on), not in Fly Machine agents. The control plane is a persistent process with in-process agents (monitoring, research, risk-assessor, safety-guardian). Fly Machine agents (trading, coding) receive chain events via the control plane API, not by running their own polling loops.

**Mirage testing**: use ephemeral mirage-rs instances to test the polling loop against a forked chain without depending on live RPC uptime. Spawn an instance, connect the `ChainEventSource` to it, advance blocks via `mirage_mine_block`, and verify events arrive through the channel.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/observer.rs` | `BlockObserver`, `BlockObserverConfig`, `ObservedEvent` -- the synchronous filter this batch wraps in an async loop |
| `crates/roko-chain/src/client.rs` | `ChainClient` trait -- `block_number()`, `get_block_header()`, `get_logs()` are the three methods the loop calls |
| `crates/roko-chain/src/triage.rs` | `TriagePipeline` -- downstream consumer of `ObservedEvent`; this batch does not wire triage, but understanding the consumer informs the channel design |
| `crates/roko-chain/src/types.rs` | `BlockNumber`, `ChainHeader`, `LogEntry` types used throughout |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Create `ChainEventSource` struct

**File**: `crates/roko-chain/src/event_source.rs` (CREATE)

**What to write**:

```rust
//! Async event source that polls a [`ChainClient`] and feeds matched events
//! through a [`tokio::sync::mpsc`] channel.

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::client::ChainClient;
use crate::observer::{BlockObserver, BlockObserverConfig, ObservedEvent};
use crate::types::{BlockNumber, ChainError, ChainResult};

/// Configuration for the chain event source polling loop.
#[derive(Debug, Clone)]
pub struct ChainEventSourceConfig {
    /// Observer configuration (watched addresses/topics, gap detection).
    pub observer: BlockObserverConfig,
    /// Poll interval in milliseconds. Default: 1000 (1 second).
    pub poll_interval_ms: u64,
    /// Channel buffer size for outbound events. Default: 1024.
    pub channel_buffer: usize,
    /// Start scanning from this block. `None` means start from chain tip.
    pub start_block: Option<BlockNumber>,
}

impl Default for ChainEventSourceConfig {
    fn default() -> Self {
        Self {
            observer: BlockObserverConfig::default(),
            poll_interval_ms: 1_000,
            channel_buffer: 1_024,
            start_block: None,
        }
    }
}

/// Handle returned by [`ChainEventSource::start`].
///
/// Drop this handle to signal the polling loop to stop.
pub struct ChainEventSourceHandle {
    /// Receive matched events from the polling loop.
    pub events: mpsc::Receiver<ObservedEvent>,
    /// Abort handle for the spawned task.
    abort: tokio::task::AbortHandle,
}

impl ChainEventSourceHandle {
    /// Signal the polling loop to stop.
    pub fn stop(&self) {
        self.abort.abort();
    }
}

impl Drop for ChainEventSourceHandle {
    fn drop(&mut self) {
        self.abort.abort();
    }
}

/// Async polling loop that drives a [`BlockObserver`] against a [`ChainClient`].
pub struct ChainEventSource;

impl ChainEventSource {
    /// Start the polling loop. Returns a handle with the event receiver.
    ///
    /// The loop runs in a spawned Tokio task. It polls `client.block_number()`
    /// at `config.poll_interval_ms`, fetches new headers and logs for any
    /// blocks after the last-processed block, runs them through the observer,
    /// and sends matched events into the channel.
    ///
    /// The loop stops when the handle is dropped or `stop()` is called.
    pub fn start(
        client: Arc<dyn ChainClient>,
        config: ChainEventSourceConfig,
    ) -> ChainEventSourceHandle {
        let (tx, rx) = mpsc::channel(config.channel_buffer);
        let poll_ms = config.poll_interval_ms;
        let start_block = config.start_block;
        let mut observer = BlockObserver::new(config.observer);

        let task = tokio::spawn(async move {
            let mut last_block: Option<BlockNumber> = start_block;

            loop {
                if let Err(_) = Self::poll_once(&client, &mut observer, &mut last_block, &tx).await
                {
                    // Log errors in production; here we just continue.
                }
                tokio::time::sleep(std::time::Duration::from_millis(poll_ms)).await;
            }
        });

        ChainEventSourceHandle {
            events: rx,
            abort: task.abort_handle(),
        }
    }

    async fn poll_once(
        client: &Arc<dyn ChainClient>,
        observer: &mut BlockObserver,
        last_block: &mut Option<BlockNumber>,
        tx: &mpsc::Sender<ObservedEvent>,
    ) -> ChainResult<()> {
        let tip = client.block_number().await?;

        let start = match *last_block {
            Some(lb) if lb >= tip => return Ok(()),
            Some(lb) => lb + 1,
            None => tip,
        };

        for block_num in start..=tip {
            let header = client.get_block_header(block_num).await?;

            let addresses: Vec<String> = observer.config.watched_addresses.clone();
            let topics: Vec<String> = observer.config.watched_topics.clone();
            let logs = client
                .get_logs(block_num, block_num, &addresses, &topics)
                .await
                .unwrap_or_default();

            let events = observer.process_block(&header, &logs);
            for event in events {
                if tx.send(event).await.is_err() {
                    return Err(ChainError::Rpc("event channel closed".into()));
                }
            }
        }

        *last_block = Some(tip);
        Ok(())
    }
}
```

**Reuse**: `crates/roko-chain/src/observer.rs:159` -- `BlockObserver::new()` and `process_block()` are the core filtering logic. This batch wraps them; it does not duplicate them.

**Do NOT**: add WebSocket support here. That requires alloy WS feature flags and a different provider type. Polling is the correct first step.

#### Item 2: Add `gap_backfill` method

**File**: `crates/roko-chain/src/event_source.rs` (EDIT -- add method to `ChainEventSource`)

**What to write**:

```rust
impl ChainEventSource {
    /// Backfill gaps detected by the observer using `get_logs`.
    ///
    /// Call this after starting the event source to fill any blocks
    /// that were missed during downtime.
    pub async fn backfill_gaps(
        client: &Arc<dyn ChainClient>,
        observer: &mut BlockObserver,
        tx: &mpsc::Sender<ObservedEvent>,
    ) -> ChainResult<usize> {
        let gaps = observer.pending_gaps();
        let mut filled = 0;

        for block_num in gaps {
            let header = client.get_block_header(block_num).await?;
            let addresses: Vec<String> = observer.config.watched_addresses.clone();
            let topics: Vec<String> = observer.config.watched_topics.clone();
            let logs = client
                .get_logs(block_num, block_num, &addresses, &topics)
                .await
                .unwrap_or_default();

            let events = observer.process_block(&header, &logs);
            for event in events {
                if tx.send(event).await.is_err() {
                    return Err(ChainError::Rpc("event channel closed".into()));
                }
            }
            filled += 1;
        }

        Ok(filled)
    }
}
```

#### Item 3: Export module from `lib.rs`

**File**: `crates/roko-chain/src/lib.rs` (EDIT)

**What to write**:

Add the module declaration and re-exports after the existing `observer` line:

```rust
pub mod event_source;

pub use event_source::{ChainEventSource, ChainEventSourceConfig, ChainEventSourceHandle};
```

### Wiring

1. In `crates/roko-chain/src/lib.rs`, add: `pub mod event_source;`
2. In `crates/roko-chain/src/lib.rs`, add re-exports: `pub use event_source::{ChainEventSource, ChainEventSourceConfig, ChainEventSourceHandle};`
3. In `crates/roko-chain/Cargo.toml`, ensure `tokio` has `time` feature (already present via workspace)

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockChainClient;
    use crate::types::LogEntry;

    #[tokio::test(flavor = "current_thread")]
    async fn poll_once_processes_new_blocks() {
        let client = Arc::new(MockChainClient::local());
        client.mine_empty_block(); // block 1
        client.insert_log(LogEntry {
            address: "0xcafe".into(),
            topics: vec!["0xabcd".into()],
            data: vec![1, 2, 3],
        });

        let config = BlockObserverConfig {
            watched_addresses: vec!["0xcafe".into()],
            ..Default::default()
        };
        let mut observer = BlockObserver::new(config);
        let (tx, mut rx) = mpsc::channel(16);
        let mut last_block = Some(0u64);

        ChainEventSource::poll_once(
            &(Arc::clone(&client) as Arc<dyn ChainClient>),
            &mut observer,
            &mut last_block,
            &tx,
        )
        .await
        .unwrap();

        assert_eq!(last_block, Some(1));
        // Events may or may not arrive depending on mock log filtering
    }

    #[tokio::test(flavor = "current_thread")]
    async fn poll_once_skips_when_no_new_blocks() {
        let client = Arc::new(MockChainClient::local());
        let config = BlockObserverConfig::default();
        let mut observer = BlockObserver::new(config);
        let (tx, _rx) = mpsc::channel(16);
        let mut last_block = Some(0u64);

        let result = ChainEventSource::poll_once(
            &(Arc::clone(&client) as Arc<dyn ChainClient>),
            &mut observer,
            &mut last_block,
            &tx,
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(last_block, Some(0));
    }

    #[test]
    fn config_defaults() {
        let config = ChainEventSourceConfig::default();
        assert_eq!(config.poll_interval_ms, 1_000);
        assert_eq!(config.channel_buffer, 1_024);
        assert!(config.start_block.is_none());
    }
}
```

### Verification

```bash
cargo test -p roko-chain -- event_source
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `ChainEventSource` struct exists at `crates/roko-chain/src/event_source.rs`
- [ ] `start()` spawns a Tokio task that polls `ChainClient::block_number()` and processes new blocks through `BlockObserver`
- [ ] Matched `ObservedEvent` values are sent through a `tokio::mpsc` channel
- [ ] `backfill_gaps()` fills blocks identified by `BlockObserver::pending_gaps()`
- [ ] Handle drop aborts the polling task
- [ ] At least 3 tests pass
- [ ] `cargo clippy` clean
- [ ] `cargo +nightly fmt --check` clean
- [ ] Module exported from `lib.rs`

### Commit message

```
feat(roko-chain): add ChainEventSource async polling loop with gap backfill
```

---

## Batch 1.3: Triage pipeline enrichment

> **Effort**: M | **Depends on**: 1.2 | **Crate**: roko-chain
> **Branch**: `defi/batch-1.3-triage-enrichment`

### Context

The triage pipeline at `crates/roko-chain/src/triage.rs` has a working 4-stage structure: rule filter, anomaly scoring, contextual enrichment, and curiosity scoring. The `MidasRScorer` at line 88 is a simplified frequency tracker using per-address event counts with EMA smoothing, and the enrichment stage at line 231 attaches labels from the `known_contracts` and `known_topics` maps.

This batch upgrades the enrichment stage to be DeFi-aware. It adds protocol family detection (matching addresses against known Uniswap, Aave, Lido, and ERC-20 patterns), event type classification from topic hashes (Transfer, Swap, Mint, Burn, Approval, Sync), and a value extraction step that reads the log data field to pull out transferred amounts. These enrichments make the triage results actionable for downstream DeFi tools.

The curiosity scoring stage (line 258) also gets a boost: protocol-matched events get a higher weight than raw unknown events, and high-value transfers (above a configurable threshold) get flagged.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/triage.rs` | Full triage pipeline -- `TriageConfig` at line 17, `TriagePipeline` at line 160, `EventEnrichment` at line 61 |
| `crates/roko-chain/src/observer.rs:137` | `ObservedEvent` struct -- input to triage |
| `crates/roko-chain/src/types.rs:103` | `LogEntry` -- the log struct that enrichment inspects |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Add `ProtocolFamily` enum

**File**: `crates/roko-chain/src/triage.rs` (EDIT)

**What to write**:

Add after the `TriageAction` enum at line 82:

```rust
/// Known protocol families for DeFi event classification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolFamily {
    /// ERC-20 token operations (Transfer, Approval).
    Erc20,
    /// Uniswap V2 (Swap, Mint, Burn, Sync).
    UniswapV2,
    /// Uniswap V3 (Swap, Mint, Burn, Collect).
    UniswapV3,
    /// Aave V3 (Supply, Borrow, Repay, Liquidation).
    AaveV3,
    /// Wrapped ETH (Deposit, Withdrawal).
    Weth,
    /// Unknown protocol.
    Unknown,
}
```

#### Item 2: Add well-known topic hash constants

**File**: `crates/roko-chain/src/triage.rs` (EDIT)

**What to write**:

Add after the `ProtocolFamily` enum:

```rust
/// Well-known EVM event topic hashes for DeFi classification.
pub mod well_known_topics {
    /// ERC-20 Transfer(address,address,uint256)
    pub const TRANSFER: &str =
        "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
    /// ERC-20 Approval(address,address,uint256)
    pub const APPROVAL: &str =
        "0x8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925";
    /// Uniswap V2 Swap(address,uint256,uint256,uint256,uint256,address)
    pub const UNISWAP_V2_SWAP: &str =
        "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
    /// Uniswap V3 Swap(address,address,int256,int256,uint160,uint128,int24)
    pub const UNISWAP_V3_SWAP: &str =
        "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";
    /// Uniswap V2 Sync(uint112,uint112)
    pub const SYNC: &str =
        "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";
    /// WETH Deposit(address,uint256)
    pub const WETH_DEPOSIT: &str =
        "0xe1fffcc4923d04b559f4d29a8bfc6cda04eb5b0d3c460751c2402c5c5cc9109c";
}
```

#### Item 3: Extend `EventEnrichment` with protocol family

**File**: `crates/roko-chain/src/triage.rs` (EDIT)

**What to write**:

Add a field to `EventEnrichment` at line 61:

```rust
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EventEnrichment {
    pub contract_label: Option<String>,
    pub event_type_label: Option<String>,
    pub domain_tags: Vec<String>,
    /// Protocol family detected from topic hash analysis.
    pub protocol_family: Option<ProtocolFamily>,
    /// Decoded value from log data (in wei), if applicable.
    pub value_wei: Option<u128>,
}
```

#### Item 4: Upgrade `stage_enrich` to detect protocol family

**File**: `crates/roko-chain/src/triage.rs` (EDIT)

**What to write**:

Replace the `stage_enrich` method at line 231:

```rust
fn stage_enrich(&self, event: &ObservedEvent) -> EventEnrichment {
    let contract_label = self
        .config
        .known_contracts
        .get(&event.log.address.to_lowercase())
        .cloned();
    let event_type_label = event
        .log
        .topics
        .iter()
        .find_map(|t| self.config.known_topics.get(&t.to_lowercase()).cloned());

    let mut domain_tags = Vec::new();
    if contract_label.is_some() {
        domain_tags.push("known_contract".to_string());
    }
    if event_type_label.is_some() {
        domain_tags.push("known_event".to_string());
    }

    // Detect protocol family from topic[0]
    let protocol_family = event.log.topics.first().map(|t| {
        let t_lower = t.to_lowercase();
        match t_lower.as_str() {
            well_known_topics::TRANSFER | well_known_topics::APPROVAL => ProtocolFamily::Erc20,
            well_known_topics::UNISWAP_V2_SWAP | well_known_topics::SYNC => {
                ProtocolFamily::UniswapV2
            }
            well_known_topics::UNISWAP_V3_SWAP => ProtocolFamily::UniswapV3,
            well_known_topics::WETH_DEPOSIT => ProtocolFamily::Weth,
            _ => ProtocolFamily::Unknown,
        }
    });

    if protocol_family.as_ref().is_some_and(|p| *p != ProtocolFamily::Unknown) {
        domain_tags.push("defi_protocol".to_string());
    }

    // Extract value from Transfer data (last 32 bytes = uint256 amount)
    let value_wei = if event.log.data.len() >= 32 {
        let bytes = &event.log.data[event.log.data.len() - 32..];
        // Read as big-endian u128 (sufficient for most token amounts)
        if bytes[..16].iter().all(|&b| b == 0) {
            Some(u128::from_be_bytes(bytes[16..32].try_into().unwrap_or([0; 16])))
        } else {
            None // Value exceeds u128
        }
    } else {
        None
    };

    EventEnrichment {
        contract_label,
        event_type_label,
        domain_tags,
        protocol_family,
        value_wei,
    }
}
```

#### Item 5: Update `stage_curiosity` to use protocol family

**File**: `crates/roko-chain/src/triage.rs` (EDIT)

**What to write**:

Update the `stage_curiosity` method signature to accept the enrichment:

```rust
fn stage_curiosity(
    &self,
    event: &ObservedEvent,
    rule_matched: bool,
    anomaly_score: f64,
    enrichment: &EventEnrichment,
) -> f64 {
    let mut score = 0.0;

    if rule_matched {
        score += 0.2;
    }

    score += anomaly_score * 0.3;

    let data_richness = (event.log.data.len() as f64 / 256.0).min(1.0);
    score += data_richness * 0.15;

    let topic_bonus = (event.log.topics.len() as f64 / 4.0).min(1.0);
    score += topic_bonus * 0.1;

    // Protocol family bonus
    if enrichment.protocol_family.as_ref().is_some_and(|p| *p != ProtocolFamily::Unknown) {
        score += 0.25;
    }

    score.clamp(0.0, 1.0)
}
```

Update the caller in `triage()` at line 191 to pass `&enrichment`.

### Wiring

1. Update the `triage()` method to pass `&enrichment` to `stage_curiosity`
2. Update `pub use triage::` in `lib.rs` to include `ProtocolFamily`

### Tests

```rust
#[test]
fn enrichment_detects_erc20_transfer() {
    let config = TriageConfig::default();
    let mut pipeline = TriagePipeline::new(config);
    let event = test_event("0xtoken", well_known_topics::TRANSFER);
    let result = pipeline.triage(event);
    assert_eq!(
        result.enrichment.protocol_family,
        Some(ProtocolFamily::Erc20)
    );
    assert!(result.enrichment.domain_tags.contains(&"defi_protocol".to_string()));
}

#[test]
fn enrichment_detects_uniswap_v3_swap() {
    let config = TriageConfig::default();
    let mut pipeline = TriagePipeline::new(config);
    let event = test_event("0xpool", well_known_topics::UNISWAP_V3_SWAP);
    let result = pipeline.triage(event);
    assert_eq!(
        result.enrichment.protocol_family,
        Some(ProtocolFamily::UniswapV3)
    );
}

#[test]
fn enrichment_unknown_topic_is_unknown_family() {
    let config = TriageConfig::default();
    let mut pipeline = TriagePipeline::new(config);
    let event = test_event("0xany", "0x1234");
    let result = pipeline.triage(event);
    assert_eq!(
        result.enrichment.protocol_family,
        Some(ProtocolFamily::Unknown)
    );
}

#[test]
fn defi_protocol_boosts_curiosity() {
    let config = TriageConfig::default();
    let mut pipeline = TriagePipeline::new(config.clone());
    let defi = pipeline.triage(test_event("0xa", well_known_topics::TRANSFER));
    let mut pipeline2 = TriagePipeline::new(config);
    let unknown = pipeline2.triage(test_event("0xa", "0x1234"));
    assert!(
        defi.curiosity_score > unknown.curiosity_score,
        "DeFi protocol event should have higher curiosity"
    );
}
```

### Verification

```bash
cargo test -p roko-chain -- triage
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `ProtocolFamily` enum exists with 6 variants
- [ ] Well-known topic hash constants are correct (verifiable against the Ethereum ABI spec)
- [ ] `EventEnrichment` has `protocol_family` and `value_wei` fields
- [ ] `stage_enrich` detects ERC-20, Uniswap V2, Uniswap V3, WETH from topic[0]
- [ ] `stage_curiosity` weights protocol-matched events higher
- [ ] Existing triage tests still pass
- [ ] At least 4 new tests pass
- [ ] `cargo clippy` clean

### Commit message

```
feat(roko-chain): add DeFi protocol detection to triage enrichment stage
```

---

## Batch 1.4: Protocol state cache

> **Effort**: M | **Depends on**: 1.1 | **Crate**: roko-chain
> **Branch**: `defi/batch-1.4-protocol-state`

### Context

There is no protocol state tracking in `roko-chain`. The `ChainClient` trait provides raw reads (block headers, logs, storage slots, eth_call), but nothing caches or indexes protocol-level state -- pool reserves, token balances, liquidity positions, health factors.

DeFi tool handlers need quick access to current protocol state without issuing RPC calls on every invocation. For example, `chain.get_pool_info` needs current reserves and tick for a Uniswap pool. Without a cache, every tool call becomes a multi-call RPC round-trip.

This batch adds a `ProtocolStateCache` backed by a `DashMap` for concurrent read/write access. Entries are keyed by `(chain_id, protocol_address)` and store protocol-specific state structs. A `ProtocolStateReader` trait abstracts how state is fetched for each protocol family, and a `CachedProtocolState` struct wraps the cache with TTL-based expiry.

This is the hot layer only. Warm-layer persistence (redb snapshots) and cold-layer indexer integration are Phase 2 concerns.

**Mirage scenario testing**: validate the cache end-to-end by forking mainnet at a known block, reading Uniswap V3 pool state via `ProtocolStateReader`, and verifying the cache populates with correct reserves/tick values. Use mirage-rs scenario sets to define a "cache_populate" scenario that reads state for 3 known pools and asserts all entries are present and non-expired. This catches serialization bugs and TTL logic issues that unit tests with mocked data miss.

```rust
// Example scenario test structure
let mut instance = spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?;
let client = MirageClient::new(instance.config()).await?;
let set_id = client.mirage_begin_scenario_set("cache_populate").await?;
client.mirage_define_scenario(&set_id, "uniswap_v3_pools", "Read state for 3 pools").await?;
let job_id = client.mirage_run_scenario_set(&set_id, RunMode::Parallel).await?;
// Verify cache entries after scenario run
instance.shutdown().await?;
```

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/client.rs` | `ChainClient` trait -- the raw read methods that state readers call |
| `crates/roko-chain/src/types.rs` | `BlockNumber`, `CallResult` -- return types from client reads |
| `crates/roko-chain/src/tools.rs:411` | `chain.get_pool_info` tool def -- the consumer that needs cached state |
| `crates/roko-chain/Cargo.toml` | Current deps; `parking_lot` is present, `dashmap` is not -- needs adding |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Add `dashmap` dependency

**File**: `crates/roko-chain/Cargo.toml` (EDIT)

**What to write**:

Add to `[dependencies]`:

```toml
dashmap = { workspace = true }
```

Verify `dashmap` is in the workspace `Cargo.toml`. If not, add it there first.

#### Item 2: Create `ProtocolStateCache`

**File**: `crates/roko-chain/src/protocol_state.rs` (CREATE)

**What to write**:

```rust
//! Hot-layer protocol state cache backed by [`DashMap`].
//!
//! Caches protocol-level state (pool reserves, token balances, positions)
//! with TTL-based expiry. Concurrent reads are lock-free; writes shard
//! across DashMap buckets.

use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

/// Key for a cached protocol state entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProtocolStateKey {
    /// Chain ID (e.g. 1 for mainnet, 8453 for Base).
    pub chain_id: u64,
    /// Protocol contract address (0x-prefixed, lowercase).
    pub address: String,
}

impl ProtocolStateKey {
    /// Create a new key.
    pub fn new(chain_id: u64, address: impl Into<String>) -> Self {
        Self {
            chain_id,
            address: address.into().to_lowercase(),
        }
    }
}

/// Cached protocol state with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedState {
    /// The protocol-specific state data.
    pub data: ProtocolData,
    /// Block number at which this state was fetched.
    pub block_number: u64,
    /// When this entry was last refreshed.
    #[serde(skip)]
    pub fetched_at: Option<Instant>,
}

/// Protocol-specific state data variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolData {
    /// Uniswap V2 pair state.
    UniswapV2Pool {
        reserve0: u128,
        reserve1: u128,
        token0: String,
        token1: String,
    },
    /// Uniswap V3 pool state.
    UniswapV3Pool {
        sqrt_price_x96: u128,
        tick: i32,
        liquidity: u128,
        fee: u32,
        token0: String,
        token1: String,
    },
    /// ERC-20 token metadata.
    Erc20Token {
        symbol: String,
        decimals: u8,
        total_supply: u128,
    },
    /// Generic key-value state for protocols without a typed variant.
    Generic {
        fields: std::collections::HashMap<String, serde_json::Value>,
    },
}

/// TTL-backed protocol state cache.
#[derive(Debug)]
pub struct ProtocolStateCache {
    entries: DashMap<ProtocolStateKey, CachedState>,
    /// How long entries remain valid before refresh.
    ttl: Duration,
}

impl ProtocolStateCache {
    /// Create a cache with the given TTL.
    pub fn new(ttl: Duration) -> Self {
        Self {
            entries: DashMap::new(),
            ttl,
        }
    }

    /// Create a cache with a 12-second TTL (one Ethereum block).
    pub fn one_block() -> Self {
        Self::new(Duration::from_secs(12))
    }

    /// Get a cached entry if it exists and has not expired.
    pub fn get(&self, key: &ProtocolStateKey) -> Option<CachedState> {
        let entry = self.entries.get(key)?;
        if let Some(fetched) = entry.fetched_at {
            if fetched.elapsed() > self.ttl {
                return None;
            }
        }
        Some(entry.clone())
    }

    /// Insert or update a cache entry.
    pub fn put(&self, key: ProtocolStateKey, data: ProtocolData, block_number: u64) {
        self.entries.insert(
            key,
            CachedState {
                data,
                block_number,
                fetched_at: Some(Instant::now()),
            },
        );
    }

    /// Remove an entry.
    pub fn invalidate(&self, key: &ProtocolStateKey) {
        self.entries.remove(key);
    }

    /// Remove all expired entries.
    pub fn evict_expired(&self) {
        self.entries.retain(|_, v| {
            v.fetched_at
                .map_or(true, |t| t.elapsed() <= self.ttl)
        });
    }

    /// Number of entries in the cache.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
```

#### Item 3: Export from `lib.rs`

**File**: `crates/roko-chain/src/lib.rs` (EDIT)

Add:

```rust
pub mod protocol_state;

pub use protocol_state::{CachedState, ProtocolData, ProtocolStateCache, ProtocolStateKey};
```

### Wiring

1. In `crates/roko-chain/Cargo.toml`, add `dashmap = { workspace = true }` to `[dependencies]`
2. In `crates/roko-chain/src/lib.rs`, add `pub mod protocol_state;` and re-exports
3. Verify `dashmap` is in workspace root `Cargo.toml` `[workspace.dependencies]`

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_put_get_roundtrip() {
        let cache = ProtocolStateCache::one_block();
        let key = ProtocolStateKey::new(1, "0xcafe");
        cache.put(
            key.clone(),
            ProtocolData::Erc20Token {
                symbol: "USDC".into(),
                decimals: 6,
                total_supply: 1_000_000,
            },
            100,
        );
        let entry = cache.get(&key);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().block_number, 100);
    }

    #[test]
    fn cache_miss_on_unknown_key() {
        let cache = ProtocolStateCache::one_block();
        let key = ProtocolStateKey::new(1, "0xdead");
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn cache_invalidate_removes_entry() {
        let cache = ProtocolStateCache::one_block();
        let key = ProtocolStateKey::new(1, "0xcafe");
        cache.put(
            key.clone(),
            ProtocolData::Generic {
                fields: Default::default(),
            },
            50,
        );
        assert!(!cache.is_empty());
        cache.invalidate(&key);
        assert!(cache.is_empty());
    }

    #[test]
    fn cache_expired_entry_returns_none() {
        let cache = ProtocolStateCache::new(Duration::from_millis(1));
        let key = ProtocolStateKey::new(1, "0xcafe");
        cache.put(
            key.clone(),
            ProtocolData::Generic {
                fields: Default::default(),
            },
            50,
        );
        std::thread::sleep(Duration::from_millis(5));
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn protocol_state_key_normalizes_address() {
        let key = ProtocolStateKey::new(1, "0xCAFE");
        assert_eq!(key.address, "0xcafe");
    }
}
```

### Verification

```bash
cargo test -p roko-chain -- protocol_state
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `ProtocolStateCache` struct exists at `crates/roko-chain/src/protocol_state.rs`
- [ ] Uses `DashMap` for concurrent access
- [ ] TTL-based expiry via `Instant` comparison
- [ ] `ProtocolData` enum has variants for UniswapV2, UniswapV3, ERC-20, and Generic
- [ ] `put`/`get`/`invalidate`/`evict_expired` methods work correctly
- [ ] At least 5 tests pass
- [ ] `cargo clippy` clean
- [ ] Module exported from `lib.rs`
- [ ] Mirage scenario test forks mainnet, watches Uniswap contracts, verifies cache populates with correct protocol state

### Commit message

```
feat(roko-chain): add DashMap-backed protocol state cache with TTL expiry
```

---

## Batch 1.5: Wallet registry for multi-wallet management

> **Effort**: M | **Depends on**: 1.1 | **Crate**: roko-chain
> **Branch**: `defi/batch-1.5-wallet-registry`

### Context

The `ChainWallet` trait at `crates/roko-chain/src/wallet.rs:17` defines single-wallet operations: address, balance, nonce, sign-and-submit, wait-for-receipt. The `AlloyChainWallet` at `crates/roko-chain/src/alloy_impl.rs:216` implements it for a single private key.

The wallet management tools in `crates/roko-chain/src/tools.rs:486` (`chain.wallet_create`, `chain.wallet_list`, `chain.wallet_info`, `chain.wallet_export_address`) require a multi-wallet registry -- a container that holds multiple `ChainWallet` instances identified by label or address and supports creating new wallets at runtime.

This batch adds a `WalletRegistry` that maps wallet labels to `Arc<dyn ChainWallet>` instances. It provides create (generate new key), register (add existing wallet), list, get-by-label, and get-by-address operations. The registry is thread-safe via `RwLock` and does not store private keys directly -- it holds wallet trait objects that encapsulate their own signing material.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/wallet.rs` | `ChainWallet` trait -- the interface each wallet implements |
| `crates/roko-chain/src/alloy_impl.rs:216` | `AlloyChainWallet` -- the concrete implementation |
| `crates/roko-chain/src/mock.rs:264` | `MockChainWallet` -- the test implementation |
| `crates/roko-chain/src/tools.rs:486` | Wallet tool definitions -- the consumers of this registry |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Create `WalletRegistry`

**File**: `crates/roko-chain/src/wallet_registry.rs` (CREATE)

**What to write**:

```rust
//! Multi-wallet registry for agent wallet management.
//!
//! Maps human-readable labels to [`ChainWallet`] trait objects. Thread-safe
//! via `parking_lot::RwLock`.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::types::ChainResult;
use crate::wallet::ChainWallet;

/// Metadata about a registered wallet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletEntry {
    /// Human-readable label (e.g. "trading", "treasury").
    pub label: String,
    /// Wallet address (0x-prefixed hex).
    pub address: String,
    /// Target network name (e.g. "ethereum", "base").
    pub network: String,
}

/// Multi-wallet registry.
///
/// Holds `Arc<dyn ChainWallet>` instances keyed by label. Supports
/// concurrent reads and serialized writes.
pub struct WalletRegistry {
    wallets: RwLock<HashMap<String, Arc<dyn ChainWallet>>>,
    metadata: RwLock<HashMap<String, WalletEntry>>,
}

impl WalletRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            wallets: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
        }
    }

    /// Register an existing wallet under a label.
    pub async fn register(
        &self,
        label: impl Into<String>,
        network: impl Into<String>,
        wallet: Arc<dyn ChainWallet>,
    ) -> ChainResult<WalletEntry> {
        let label = label.into();
        let network = network.into();
        let address = wallet.address().await?;

        let entry = WalletEntry {
            label: label.clone(),
            address,
            network,
        };

        self.wallets.write().insert(label.clone(), wallet);
        self.metadata.write().insert(label, entry.clone());

        Ok(entry)
    }

    /// Get a wallet by label.
    pub fn get(&self, label: &str) -> Option<Arc<dyn ChainWallet>> {
        self.wallets.read().get(label).cloned()
    }

    /// Get a wallet by address (linear scan).
    pub fn get_by_address(&self, address: &str) -> Option<Arc<dyn ChainWallet>> {
        let addr_lower = address.to_lowercase();
        let meta = self.metadata.read();
        let label = meta
            .values()
            .find(|e| e.address.to_lowercase() == addr_lower)?
            .label
            .clone();
        drop(meta);
        self.wallets.read().get(&label).cloned()
    }

    /// List all registered wallet entries.
    pub fn list(&self) -> Vec<WalletEntry> {
        self.metadata.read().values().cloned().collect()
    }

    /// List entries filtered by network.
    pub fn list_by_network(&self, network: &str) -> Vec<WalletEntry> {
        self.metadata
            .read()
            .values()
            .filter(|e| e.network == network)
            .cloned()
            .collect()
    }

    /// Get metadata for a wallet by label.
    pub fn info(&self, label: &str) -> Option<WalletEntry> {
        self.metadata.read().get(label).cloned()
    }

    /// Number of registered wallets.
    pub fn len(&self) -> usize {
        self.wallets.read().len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.wallets.read().is_empty()
    }

    /// Remove a wallet by label.
    pub fn remove(&self, label: &str) -> bool {
        let removed = self.wallets.write().remove(label).is_some();
        self.metadata.write().remove(label);
        removed
    }
}

impl Default for WalletRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

#### Item 2: Export from `lib.rs`

**File**: `crates/roko-chain/src/lib.rs` (EDIT)

Add:

```rust
pub mod wallet_registry;

pub use wallet_registry::{WalletEntry, WalletRegistry};
```

### Wiring

1. In `crates/roko-chain/src/lib.rs`, add `pub mod wallet_registry;` and re-exports

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockChainWallet;

    #[tokio::test(flavor = "current_thread")]
    async fn register_and_get() {
        let registry = WalletRegistry::new();
        let wallet = Arc::new(MockChainWallet::funded(1_000_000));
        let entry = registry
            .register("trading", "ethereum", wallet)
            .await
            .unwrap();
        assert_eq!(entry.label, "trading");
        assert_eq!(entry.network, "ethereum");
        assert!(registry.get("trading").is_some());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn get_by_address() {
        let registry = WalletRegistry::new();
        let wallet = Arc::new(MockChainWallet::funded(1_000));
        let entry = registry
            .register("test", "base", wallet)
            .await
            .unwrap();
        let found = registry.get_by_address(&entry.address);
        assert!(found.is_some());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_returns_all_entries() {
        let registry = WalletRegistry::new();
        let w1 = Arc::new(MockChainWallet::funded(100));
        let w2 = Arc::new(
            MockChainWallet::funded(200).with_address("0x0000000000000000000000000000000000001234"),
        );
        registry.register("a", "ethereum", w1).await.unwrap();
        registry.register("b", "base", w2).await.unwrap();
        assert_eq!(registry.list().len(), 2);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn list_by_network_filters() {
        let registry = WalletRegistry::new();
        let w1 = Arc::new(MockChainWallet::funded(100));
        let w2 = Arc::new(
            MockChainWallet::funded(200).with_address("0x0000000000000000000000000000000000001234"),
        );
        registry.register("a", "ethereum", w1).await.unwrap();
        registry.register("b", "base", w2).await.unwrap();
        assert_eq!(registry.list_by_network("base").len(), 1);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn remove_wallet() {
        let registry = WalletRegistry::new();
        let wallet = Arc::new(MockChainWallet::funded(100));
        registry.register("temp", "ethereum", wallet).await.unwrap();
        assert!(registry.remove("temp"));
        assert!(registry.get("temp").is_none());
        assert_eq!(registry.len(), 0);
    }
}
```

### Verification

```bash
cargo test -p roko-chain -- wallet_registry
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `WalletRegistry` struct exists at `crates/roko-chain/src/wallet_registry.rs`
- [ ] `register`, `get`, `get_by_address`, `list`, `list_by_network`, `info`, `remove` methods work
- [ ] Thread-safe via `parking_lot::RwLock`
- [ ] `WalletEntry` has label, address, network fields
- [ ] At least 5 tests pass
- [ ] `cargo clippy` clean
- [ ] Module exported from `lib.rs`

### Commit message

```
feat(roko-chain): add WalletRegistry for multi-wallet management
```

---

## Batch 1.6: Heartbeat chain lag suppression

> **Effort**: S | **Depends on**: 1.2 | **Crate**: roko-chain
> **Branch**: `defi/batch-1.6-heartbeat-chain-lag`

### Context

The `ChainHeartbeatExtension` at `crates/roko-chain/src/heartbeat_ext.rs:151` runs SIMULATE + VALIDATE steps before a chain agent executes a transaction. The `PolicyCageConfig` (line 45) enforces position limits, daily volume caps, approved assets, and gas price checks. The `PolicyCageState` (line 75) tracks runtime counters.

The gap analysis identifies chain lag suppression as missing: when the chain client is behind the tip (blocks_behind > threshold), trading should be suppressed because the agent is operating on stale state. This batch adds a `blocks_behind` field to `PolicyCageState` and a `max_blocks_behind` field to `PolicyCageConfig`, then adds a lag check to the `validate` method.

This is a small, focused change. The `ChainEventSource` from batch 1.2 provides the `blocks_behind` value by comparing `last_processed_block` to `chain_tip`.

**Agent mode context**: chain lag suppression applies to persistent agents only (monitoring, risk-assessor, safety-guardian -- the in-process agents running in the roko control plane on Railway). Fly Machine agents (trading, coding) are ephemeral and do not maintain their own chain polling state. They receive chain data via the control plane API, so lag suppression is enforced at the control plane level before data reaches them. The `max_blocks_behind` config should be set per-agent based on the agent's archetype: persistent agents get tight thresholds (3-5 blocks), while ephemeral agents inherit the control plane's lag status implicitly.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/heartbeat_ext.rs` | Full file -- `PolicyCageConfig` at line 45, `PolicyCageState` at line 75, `validate` at line 227 |
| `crates/roko-chain/src/event_source.rs` | (After batch 1.2) `ChainEventSource` -- provides the blocks_behind value |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Add `max_blocks_behind` to `PolicyCageConfig`

**File**: `crates/roko-chain/src/heartbeat_ext.rs` (EDIT)

**What to write**:

Add field to `PolicyCageConfig` after `max_gas_gwei`:

```rust
/// Maximum number of blocks the agent can be behind chain tip before
/// suppressing trading. 0 disables this check.
pub max_blocks_behind: u64,
```

Update the `Default` impl to set `max_blocks_behind: 5`.

#### Item 2: Add `blocks_behind` to `PolicyCageState`

**File**: `crates/roko-chain/src/heartbeat_ext.rs` (EDIT)

**What to write**:

Add field to `PolicyCageState`:

```rust
/// How many blocks behind the chain tip the agent currently is.
pub blocks_behind: u64,
```

#### Item 3: Add lag check to `validate`

**File**: `crates/roko-chain/src/heartbeat_ext.rs` (EDIT)

**What to write**:

Add to the `validate` method, after the gas price check:

```rust
// Check chain lag
if config.max_blocks_behind > 0 && state.blocks_behind > config.max_blocks_behind {
    violations.push(PolicyViolation {
        constraint: "max_blocks_behind".into(),
        description: format!(
            "Chain lag ({} blocks behind) exceeds limit ({})",
            state.blocks_behind, config.max_blocks_behind
        ),
        severity: ViolationSeverity::Error,
    });
}
```

### Wiring

No new modules or config changes. This edits existing structs and methods in `heartbeat_ext.rs`.

### Tests

```rust
#[tokio::test(flavor = "current_thread")]
async fn validate_blocks_when_chain_lagging() {
    let config = PolicyCageConfig {
        max_blocks_behind: 3,
        ..Default::default()
    };
    let ext = ChainHeartbeatExtension::new(
        Arc::new(MockTxSimulator {
            outcome: SimulationOutcome::ok(21_000),
        }),
        config,
    );
    let tx = TxRequest::default();
    let state = PolicyCageState {
        blocks_behind: 10,
        ..Default::default()
    };
    let result = ext.pre_act_check(&tx, &state).await;
    assert!(!result.validate.passed);
    assert!(
        result
            .validate
            .violations
            .iter()
            .any(|v| v.constraint == "max_blocks_behind")
    );
}

#[tokio::test(flavor = "current_thread")]
async fn validate_passes_when_chain_caught_up() {
    let config = PolicyCageConfig {
        max_blocks_behind: 3,
        ..Default::default()
    };
    let ext = ChainHeartbeatExtension::new(
        Arc::new(MockTxSimulator {
            outcome: SimulationOutcome::ok(21_000),
        }),
        config,
    );
    let tx = TxRequest::default();
    let state = PolicyCageState {
        blocks_behind: 1,
        ..Default::default()
    };
    let result = ext.pre_act_check(&tx, &state).await;
    assert!(result.validate.passed);
}

#[tokio::test(flavor = "current_thread")]
async fn validate_skips_lag_check_when_disabled() {
    let config = PolicyCageConfig {
        max_blocks_behind: 0, // disabled
        ..Default::default()
    };
    let ext = ChainHeartbeatExtension::new(
        Arc::new(MockTxSimulator {
            outcome: SimulationOutcome::ok(21_000),
        }),
        config,
    );
    let tx = TxRequest::default();
    let state = PolicyCageState {
        blocks_behind: 100,
        ..Default::default()
    };
    let result = ext.pre_act_check(&tx, &state).await;
    assert!(result.validate.passed);
}
```

### Verification

```bash
cargo test -p roko-chain -- heartbeat_ext
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `PolicyCageConfig` has `max_blocks_behind` field (default 5)
- [ ] `PolicyCageState` has `blocks_behind` field (default 0)
- [ ] `validate` emits `ViolationSeverity::Error` when `blocks_behind > max_blocks_behind` and `max_blocks_behind > 0`
- [ ] Lag check is skipped when `max_blocks_behind == 0`
- [ ] Existing heartbeat tests still pass
- [ ] At least 3 new tests pass
- [ ] `cargo clippy` clean

### Commit message

```
feat(roko-chain): add chain lag suppression to PolicyCage heartbeat check
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives used

- **Connector**: `ChainRpcConnector` provides the live I/O layer — block reads, transaction submission, and event subscription against any EVM-compatible chain. `OracleConnector` streams off-chain price data into the agent environment. `ForkManager` exposes chain-state snapshots and reverts as a first-class connector, enabling safe simulation before on-chain execution.
- **Feed**: `ChainEventFeed` delivers a continuous stream of block events, contract events, and mempool activity that agents and recipes subscribe to. `OraclePriceFeed` wraps oracle sources into a normalized, continuously updating price stream.
- **Gate**: `TxSimulatorGate` runs in pre-action mode — it simulates every transaction against the forked chain state and blocks submission if the simulation reverts, surfaces an error, or breaches gas bounds.

### Authoring surfaces

- **Connector Manager** — configure chain RPC endpoints (URL, chain ID, auth token, rate limits, failover peers), oracle sources, and fork parameters (fork block, snapshot policy)
- **Feed Designer** — subscribe to chain events by contract address and event signature, set block range and sampling interval, configure aggregation and backfill behavior

### Shareable artifacts

- Connector configurations: chain RPC presets for Ethereum mainnet, Arbitrum, Base, Optimism, and other EVM chains — importable with one click
- Feed templates: standard event subscriptions for major protocols (Uniswap V3 swaps, Aave borrows/repays, Compound liquidations, Curve trades)

### Dashboard visibility

- **System → Connectors** — live chain RPC health panel showing latency, block height, sync lag, and failover status per configured chain
- **Pulse → Event Stream** — real-time chain event log with contract address and event signature filters
