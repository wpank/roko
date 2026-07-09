# 02 -- Tool infrastructure: agent-executable work batches

> **Batch count**: 5 | **Total items**: ~28 | **Phase 0 batches**: 2.1, 2.2
> **Primary crate**: `roko-chain` (tool handlers), `roko-std` (handler registry wiring)
> **Dependency root**: Batch 2.1

---

## Batch 2.1: VenueAdapter trait + mock implementation

> **Effort**: M | **Depends on**: 1.2 | **Crate**: roko-chain
> **Branch**: `defi/batch-2.1-venue-adapter`

### Context

The 14 chain tools in `crates/roko-chain/src/tools.rs` are `ToolDef` registrations with JSON Schema parameters, but no runtime handlers. When an agent invokes `chain.swap`, the handler lookup in `crates/roko-std/src/tool/handlers.rs:26` returns `None` because the match block only covers the 16 standard tools (read_file, write_file, etc.). The chain tools are visible in the registry but cannot execute.

The tool definitions at line 260 (`chain.swap`) hardcode Uniswap-specific parameters (V3 fee tiers, tick ranges). To support multiple DEX venues with the same tool surface, the PRD specifies a `VenueAdapter` trait that normalizes protocol-specific calls behind a common interface.

This batch creates the `VenueAdapter` trait with 6 methods (swap, add_liquidity, remove_liquidity, get_pool_state, get_quote, capabilities), defines the parameter and result types, and provides a `MockVenueAdapter` for testing. The mock returns deterministic results without network calls, letting downstream tool handlers be tested without a chain backend.

Beyond the mock, this batch also defines a `MirageVenueAdapter` as the first real adapter. It wraps a mirage-rs instance, allowing agents to `spawn_mirage_test_instance()`, fork mainnet, and test swap/LP operations against real Uniswap state without touching live chains. The `MirageVenueAdapter` implements the full `VenueAdapter` trait by translating adapter calls into eth_call/eth_sendTransaction against the forked mirage instance. This gives integration tests a realistic venue backend and gives agents a safe sandbox for validating trade logic before live execution.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/tools.rs` | All 14 chain tool definitions; `chain.swap` at line 260, `chain.add_liquidity` at line 312 -- understand the parameter schemas the adapter must accept |
| `crates/roko-chain/src/client.rs` | `ChainClient` trait -- adapters use this for read operations |
| `crates/roko-chain/src/wallet.rs` | `ChainWallet` trait -- adapters use this for write operations |
| `crates/roko-chain/src/types.rs` | `TxRequest`, `Receipt`, `ChainResult` -- the tx types adapters produce |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Create `VenueAdapter` trait and types

**File**: `crates/roko-chain/src/venue.rs` (CREATE)

**What to write**:

```rust
//! [`VenueAdapter`] trait for protocol-normalized DeFi operations.
//!
//! A venue adapter translates generic DeFi operations (swap, add/remove
//! liquidity, pool queries) into protocol-specific contract calls. One
//! tool definition (e.g. `chain.swap`) dispatches to the appropriate
//! adapter based on a `venue` parameter.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::types::ChainResult;

/// Parameters for a swap operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapParams {
    /// Input token address.
    pub token_in: String,
    /// Output token address.
    pub token_out: String,
    /// Amount of input token (in smallest unit).
    pub amount_in: u128,
    /// Minimum output amount (slippage protection).
    pub amount_out_min: u128,
    /// Recipient address. Defaults to wallet address if None.
    pub recipient: Option<String>,
    /// Unix timestamp deadline. None means adapter chooses a default.
    pub deadline: Option<u64>,
}

/// Result of a swap operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapResult {
    /// Transaction hash.
    pub tx_hash: String,
    /// Actual output amount received.
    pub amount_out: u128,
    /// Gas used by the transaction.
    pub gas_used: u64,
    /// Price impact as a fraction (0.01 = 1%).
    pub price_impact: f64,
}

/// Parameters for adding liquidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddLiquidityParams {
    /// First token address.
    pub token_a: String,
    /// Second token address.
    pub token_b: String,
    /// Amount of token A.
    pub amount_a: u128,
    /// Amount of token B.
    pub amount_b: u128,
    /// Lower tick bound (concentrated liquidity). None for full-range.
    pub tick_lower: Option<i32>,
    /// Upper tick bound (concentrated liquidity). None for full-range.
    pub tick_upper: Option<i32>,
    /// Fee tier in basis points (e.g. 3000 = 0.3%).
    pub fee: Option<u32>,
}

/// Parameters for removing liquidity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveLiquidityParams {
    /// Position identifier (e.g. V3 NFT token ID, V2 LP token address).
    pub position_id: String,
    /// Amount of liquidity to remove. u128::MAX for full removal.
    pub liquidity: u128,
    /// Minimum token A received (slippage).
    pub amount_a_min: u128,
    /// Minimum token B received (slippage).
    pub amount_b_min: u128,
}

/// Result of a liquidity operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityResult {
    /// Transaction hash.
    pub tx_hash: String,
    /// Actual amount of token A deposited/received.
    pub amount_a: u128,
    /// Actual amount of token B deposited/received.
    pub amount_b: u128,
    /// Liquidity minted or burned.
    pub liquidity: u128,
    /// Gas used.
    pub gas_used: u64,
}

/// Parameters for a swap quote (read-only, no tx).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteParams {
    /// Input token address.
    pub token_in: String,
    /// Output token address.
    pub token_out: String,
    /// Amount of input token.
    pub amount_in: u128,
    /// Fee tier in basis points. None means adapter picks the best route.
    pub fee: Option<u32>,
}

/// Result of a swap quote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResult {
    /// Expected output amount.
    pub amount_out: u128,
    /// Estimated price impact as a fraction.
    pub price_impact: f64,
    /// Route description (human-readable).
    pub route: String,
    /// Estimated gas cost.
    pub estimated_gas: u64,
}

/// Current pool state snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolState {
    /// Pool contract address.
    pub address: String,
    /// Token 0 address.
    pub token0: String,
    /// Token 1 address.
    pub token1: String,
    /// Reserve or liquidity of token 0.
    pub reserve0: u128,
    /// Reserve or liquidity of token 1.
    pub reserve1: u128,
    /// Fee tier in basis points.
    pub fee: u32,
    /// Current tick (V3) or 0 (V2).
    pub tick: i32,
    /// Total liquidity in the pool.
    pub liquidity: u128,
}

/// Pool identifier: either a direct address or a token pair + fee.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolId {
    /// Direct pool contract address.
    Address(String),
    /// Token pair + fee tier (adapter computes the pool address).
    Pair {
        token_a: String,
        token_b: String,
        fee: u32,
    },
}

/// What operations a venue supports.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VenueCapabilities {
    /// Supports token swaps.
    pub swap: bool,
    /// Supports adding liquidity.
    pub add_liquidity: bool,
    /// Supports removing liquidity.
    pub remove_liquidity: bool,
    /// Supports concentrated (tick-range) liquidity.
    pub concentrated_liquidity: bool,
    /// Supports swap quotes without execution.
    pub quotes: bool,
}

/// Protocol-normalized interface for DeFi venue operations.
///
/// Each implementation adapts a specific protocol (Uniswap V2, V3, V4,
/// Curve, Balancer, Aave, etc.) behind this common interface. The tool
/// handlers dispatch to the appropriate adapter based on a `venue` parameter.
#[async_trait]
pub trait VenueAdapter: Send + Sync {
    /// Execute a token swap.
    async fn swap(&self, params: &SwapParams) -> ChainResult<SwapResult>;

    /// Add liquidity to a pool.
    async fn add_liquidity(&self, params: &AddLiquidityParams) -> ChainResult<LiquidityResult>;

    /// Remove liquidity from a position.
    async fn remove_liquidity(
        &self,
        params: &RemoveLiquidityParams,
    ) -> ChainResult<LiquidityResult>;

    /// Get current pool state.
    async fn get_pool_state(&self, pool: &PoolId) -> ChainResult<PoolState>;

    /// Get a swap quote without executing.
    async fn get_quote(&self, params: &QuoteParams) -> ChainResult<QuoteResult>;

    /// Capabilities this venue supports.
    fn capabilities(&self) -> VenueCapabilities;

    /// Venue name for routing and display (e.g. "uniswap_v3", "curve").
    fn name(&self) -> &str;
}

/// Mock venue adapter for testing. Returns deterministic results.
pub struct MockVenueAdapter {
    venue_name: String,
    caps: VenueCapabilities,
}

impl MockVenueAdapter {
    /// Create a mock adapter with all capabilities enabled.
    pub fn full(name: impl Into<String>) -> Self {
        Self {
            venue_name: name.into(),
            caps: VenueCapabilities {
                swap: true,
                add_liquidity: true,
                remove_liquidity: true,
                concentrated_liquidity: true,
                quotes: true,
            },
        }
    }

    /// Create a mock adapter with specific capabilities.
    pub fn with_caps(name: impl Into<String>, caps: VenueCapabilities) -> Self {
        Self {
            venue_name: name.into(),
            caps,
        }
    }
}

#[async_trait]
impl VenueAdapter for MockVenueAdapter {
    async fn swap(&self, params: &SwapParams) -> ChainResult<SwapResult> {
        Ok(SwapResult {
            tx_hash: "0xmock_swap_hash".into(),
            amount_out: params.amount_out_min + 100, // slightly better than minimum
            gas_used: 150_000,
            price_impact: 0.003,
        })
    }

    async fn add_liquidity(&self, params: &AddLiquidityParams) -> ChainResult<LiquidityResult> {
        Ok(LiquidityResult {
            tx_hash: "0xmock_lp_add_hash".into(),
            amount_a: params.amount_a,
            amount_b: params.amount_b,
            liquidity: (params.amount_a as f64 * params.amount_b as f64).sqrt() as u128,
            gas_used: 250_000,
        })
    }

    async fn remove_liquidity(
        &self,
        params: &RemoveLiquidityParams,
    ) -> ChainResult<LiquidityResult> {
        Ok(LiquidityResult {
            tx_hash: "0xmock_lp_remove_hash".into(),
            amount_a: params.amount_a_min + 50,
            amount_b: params.amount_b_min + 50,
            liquidity: params.liquidity,
            gas_used: 200_000,
        })
    }

    async fn get_pool_state(&self, pool: &PoolId) -> ChainResult<PoolState> {
        let address = match pool {
            PoolId::Address(a) => a.clone(),
            PoolId::Pair { .. } => "0xmock_pool".into(),
        };
        Ok(PoolState {
            address,
            token0: "0xtoken0".into(),
            token1: "0xtoken1".into(),
            reserve0: 1_000_000_000_000_000_000,
            reserve1: 2_000_000_000,
            fee: 3000,
            tick: -100,
            liquidity: 500_000_000_000,
        })
    }

    async fn get_quote(&self, params: &QuoteParams) -> ChainResult<QuoteResult> {
        Ok(QuoteResult {
            amount_out: params.amount_in * 98 / 100, // 2% slippage
            price_impact: 0.005,
            route: format!(
                "{} -> {} via {}",
                params.token_in, params.token_out, self.venue_name
            ),
            estimated_gas: 150_000,
        })
    }

    fn capabilities(&self) -> VenueCapabilities {
        self.caps.clone()
    }

    fn name(&self) -> &str {
        &self.venue_name
    }
}
```

#### Item 2: Create `MirageVenueAdapter`

**File**: `crates/roko-chain/src/venue_mirage.rs` (CREATE)

**What to write**:

```rust
//! [`MirageVenueAdapter`] -- VenueAdapter backed by an ephemeral mirage-rs fork.
//!
//! Agents can spawn a mirage instance forked from any chain, then execute
//! swap/LP operations against real Uniswap state without touching live chains.
//! Use this for integration testing and pre-trade simulation.

use std::sync::Arc;
use crate::venue::{
    VenueAdapter, SwapParams, SwapResult, AddLiquidityParams,
    RemoveLiquidityParams, LiquidityResult, QuoteParams, QuoteResult,
    PoolState, VenueCapabilities,
};
use crate::types::ChainResult;

/// VenueAdapter implementation backed by mirage-rs.
///
/// Translates adapter method calls into eth_call / eth_sendTransaction
/// against a forked mirage instance. The instance must already be spawned
/// and connected before constructing this adapter.
pub struct MirageVenueAdapter {
    /// mirage-rs client connected to the ephemeral instance.
    client: mirage_rs::client::MirageClient,
    /// Uniswap V3 Router address on the forked chain.
    router_address: String,
}

impl MirageVenueAdapter {
    /// Create a new adapter from an already-connected MirageClient.
    pub fn new(client: mirage_rs::client::MirageClient, router_address: impl Into<String>) -> Self {
        Self { client, router_address: router_address.into() }
    }
}

#[async_trait::async_trait]
impl VenueAdapter for MirageVenueAdapter {
    fn name(&self) -> &str { "mirage" }

    fn capabilities(&self) -> VenueCapabilities {
        VenueCapabilities {
            supports_swap: true,
            supports_add_liquidity: true,
            supports_remove_liquidity: true,
            supports_flash_loan: false,
        }
    }

    async fn swap(&self, params: &SwapParams) -> ChainResult<SwapResult> {
        use alloy_sol_types::sol;

        sol! {
            function exactInputSingle(
                address tokenIn,
                address tokenOut,
                uint24 fee,
                address recipient,
                uint256 deadline,
                uint256 amountIn,
                uint256 amountOutMinimum,
                uint160 sqrtPriceLimitX96
            ) external payable returns (uint256 amountOut);
        }

        let recipient = params.recipient.clone()
            .unwrap_or_else(|| self.client.default_sender().unwrap_or_default());
        let deadline = params.deadline.unwrap_or(u64::MAX);

        let calldata = exactInputSingleCall {
            tokenIn: params.token_in.parse().map_err(|e| ChainError::Abi(format!("{e}")))?,
            tokenOut: params.token_out.parse().map_err(|e| ChainError::Abi(format!("{e}")))?,
            fee: 3000u32.into(), // default to 0.3% tier
            recipient: recipient.parse().map_err(|e| ChainError::Abi(format!("{e}")))?,
            deadline: U256::from(deadline),
            amountIn: U256::from(params.amount_in),
            amountOutMinimum: U256::from(params.amount_out_min),
            sqrtPriceLimitX96: U256::ZERO, // no price limit
        }.abi_encode();

        let receipt = self.client.eth_send_transaction_raw(
            &self.router_address, &calldata,
        ).await.map_err(|e| ChainError::Rpc(e.to_string()))?;

        // Decode amountOut from return data (first 32 bytes)
        let amount_out = if receipt.output.len() >= 32 {
            U256::from_be_slice(&receipt.output[..32]).try_into().unwrap_or(0u128)
        } else {
            0u128
        };

        Ok(SwapResult {
            tx_hash: receipt.tx_hash.clone(),
            amount_out,
            gas_used: receipt.gas_used,
            price_impact: if params.amount_in > 0 {
                1.0 - (amount_out as f64 / params.amount_in as f64)
            } else {
                0.0
            },
        })
    }

    async fn add_liquidity(&self, params: &AddLiquidityParams) -> ChainResult<LiquidityResult> {
        use alloy_sol_types::sol;

        sol! {
            function mint(
                address token0,
                address token1,
                uint24 fee,
                int24 tickLower,
                int24 tickUpper,
                uint256 amount0Desired,
                uint256 amount1Desired,
                uint256 amount0Min,
                uint256 amount1Min,
                address recipient,
                uint256 deadline
            ) external payable returns (
                uint256 tokenId,
                uint128 liquidity,
                uint256 amount0,
                uint256 amount1
            );
        }

        let tick_lower = params.tick_lower.unwrap_or(-887220); // full range
        let tick_upper = params.tick_upper.unwrap_or(887220);
        let fee = params.fee.unwrap_or(3000);
        let recipient = self.client.default_sender().unwrap_or_default();

        let calldata = mintCall {
            token0: params.token_a.parse().map_err(|e| ChainError::Abi(format!("{e}")))?,
            token1: params.token_b.parse().map_err(|e| ChainError::Abi(format!("{e}")))?,
            fee: fee.into(),
            tickLower: tick_lower.into(),
            tickUpper: tick_upper.into(),
            amount0Desired: U256::from(params.amount_a),
            amount1Desired: U256::from(params.amount_b),
            amount0Min: U256::ZERO,
            amount1Min: U256::ZERO,
            recipient: recipient.parse().map_err(|e| ChainError::Abi(format!("{e}")))?,
            deadline: U256::from(u64::MAX),
        }.abi_encode();

        // NonfungiblePositionManager address (mainnet canonical)
        let nfpm = "0xC36442b4a4522E871399CD717aBDD847Ab11FE88";
        let receipt = self.client.eth_send_transaction_raw(nfpm, &calldata)
            .await.map_err(|e| ChainError::Rpc(e.to_string()))?;

        // Decode (tokenId, liquidity, amount0, amount1) from return data
        let (liquidity, amount_a, amount_b) = if receipt.output.len() >= 128 {
            let liq = u128::from_be_bytes(receipt.output[48..64].try_into().unwrap_or([0; 16]));
            let a0: u128 = U256::from_be_slice(&receipt.output[64..96]).try_into().unwrap_or(0);
            let a1: u128 = U256::from_be_slice(&receipt.output[96..128]).try_into().unwrap_or(0);
            (liq, a0, a1)
        } else {
            (0, params.amount_a, params.amount_b)
        };

        Ok(LiquidityResult {
            tx_hash: receipt.tx_hash.clone(),
            amount_a,
            amount_b,
            liquidity,
            gas_used: receipt.gas_used,
        })
    }

    async fn remove_liquidity(&self, params: &RemoveLiquidityParams) -> ChainResult<LiquidityResult> {
        use alloy_sol_types::sol;

        sol! {
            function decreaseLiquidity(
                uint256 tokenId,
                uint128 liquidity,
                uint256 amount0Min,
                uint256 amount1Min,
                uint256 deadline
            ) external payable returns (uint256 amount0, uint256 amount1);
        }

        let token_id: u128 = params.position_id.parse()
            .map_err(|e| ChainError::Abi(format!("position_id: {e}")))?;

        let calldata = decreaseLiquidityCall {
            tokenId: U256::from(token_id),
            liquidity: params.liquidity,
            amount0Min: U256::from(params.amount_a_min),
            amount1Min: U256::from(params.amount_b_min),
            deadline: U256::from(u64::MAX),
        }.abi_encode();

        let nfpm = "0xC36442b4a4522E871399CD717aBDD847Ab11FE88";
        let receipt = self.client.eth_send_transaction_raw(nfpm, &calldata)
            .await.map_err(|e| ChainError::Rpc(e.to_string()))?;

        let (amount_a, amount_b) = if receipt.output.len() >= 64 {
            let a0: u128 = U256::from_be_slice(&receipt.output[..32]).try_into().unwrap_or(0);
            let a1: u128 = U256::from_be_slice(&receipt.output[32..64]).try_into().unwrap_or(0);
            (a0, a1)
        } else {
            (params.amount_a_min, params.amount_b_min)
        };

        Ok(LiquidityResult {
            tx_hash: receipt.tx_hash.clone(),
            amount_a,
            amount_b,
            liquidity: params.liquidity,
            gas_used: receipt.gas_used,
        })
    }

    async fn get_pool_state(&self, pool_address: &str) -> ChainResult<PoolState> {
        use alloy_sol_types::sol;

        sol! {
            function getPool(address tokenA, address tokenB, uint24 fee)
                external view returns (address pool);
            function slot0() external view returns (
                uint160 sqrtPriceX96, int24 tick, uint16 observationIndex,
                uint16 observationCardinality, uint16 observationCardinalityNext,
                uint8 feeProtocol, bool unlocked
            );
            function liquidity() external view returns (uint128);
            function token0() external view returns (address);
            function token1() external view returns (address);
            function fee() external view returns (uint24);
        }

        // Read slot0 for tick and sqrtPrice
        let slot0_data = self.client.eth_call_raw(pool_address, &slot0Call {}.abi_encode())
            .await.map_err(|e| ChainError::Rpc(e.to_string()))?;
        let slot0_ret = slot0Call::abi_decode_returns(&slot0_data, true)
            .map_err(|e| ChainError::Abi(e.to_string()))?;

        // Read total liquidity
        let liq_data = self.client.eth_call_raw(pool_address, &liquidityCall {}.abi_encode())
            .await.map_err(|e| ChainError::Rpc(e.to_string()))?;
        let liq_ret = liquidityCall::abi_decode_returns(&liq_data, true)
            .map_err(|e| ChainError::Abi(e.to_string()))?;

        // Read token addresses
        let t0_data = self.client.eth_call_raw(pool_address, &token0Call {}.abi_encode())
            .await.map_err(|e| ChainError::Rpc(e.to_string()))?;
        let t0 = token0Call::abi_decode_returns(&t0_data, true)
            .map_err(|e| ChainError::Abi(e.to_string()))?;
        let t1_data = self.client.eth_call_raw(pool_address, &token1Call {}.abi_encode())
            .await.map_err(|e| ChainError::Rpc(e.to_string()))?;
        let t1 = token1Call::abi_decode_returns(&t1_data, true)
            .map_err(|e| ChainError::Abi(e.to_string()))?;

        // Read fee tier
        let fee_data = self.client.eth_call_raw(pool_address, &feeCall {}.abi_encode())
            .await.map_err(|e| ChainError::Rpc(e.to_string()))?;
        let fee_ret = feeCall::abi_decode_returns(&fee_data, true)
            .map_err(|e| ChainError::Abi(e.to_string()))?;

        Ok(PoolState {
            address: pool_address.to_string(),
            token0: format!("{:?}", t0._0),
            token1: format!("{:?}", t1._0),
            reserve0: 0, // V3 pools use liquidity, not reserves
            reserve1: 0,
            fee: fee_ret._0.try_into().unwrap_or(3000),
            tick: slot0_ret.tick.try_into().unwrap_or(0),
            liquidity: liq_ret._0,
        })
    }

    async fn get_quote(&self, params: &QuoteParams) -> ChainResult<QuoteResult> {
        use alloy_sol_types::sol;

        sol! {
            function quoteExactInputSingle(
                address tokenIn,
                address tokenOut,
                uint24 fee,
                uint256 amountIn,
                uint160 sqrtPriceLimitX96
            ) external returns (uint256 amountOut);
        }

        let fee = params.fee.unwrap_or(3000);

        let calldata = quoteExactInputSingleCall {
            tokenIn: params.token_in.parse().map_err(|e| ChainError::Abi(format!("{e}")))?,
            tokenOut: params.token_out.parse().map_err(|e| ChainError::Abi(format!("{e}")))?,
            fee: fee.into(),
            amountIn: U256::from(params.amount_in),
            sqrtPriceLimitX96: U256::ZERO,
        }.abi_encode();

        // Quoter V2 address (mainnet canonical)
        let quoter = "0x61fFE014bA17989E743c5F6cB21bF9697530B21e";
        let result = self.client.eth_call_raw(quoter, &calldata)
            .await.map_err(|e| ChainError::Rpc(e.to_string()))?;

        let amount_out: u128 = if result.len() >= 32 {
            U256::from_be_slice(&result[..32]).try_into().unwrap_or(0)
        } else {
            0
        };

        let price_impact = if params.amount_in > 0 && amount_out > 0 {
            1.0 - (amount_out as f64 / params.amount_in as f64)
        } else {
            0.0
        };

        Ok(QuoteResult {
            amount_out,
            price_impact,
            route: format!(
                "{} -> {} (fee: {}bps) via mirage fork",
                params.token_in, params.token_out, fee
            ),
            estimated_gas: 150_000, // typical V3 swap gas
        })
    }
}
```

Each method follows the same pattern: declare the ABI via `alloy_sol_types::sol!`, encode calldata with `abi_encode()`, call `self.client.eth_send_transaction_raw()` (writes) or `self.client.eth_call_raw()` (reads), and decode the return data. The `MirageVenueAdapter` should be feature-gated behind `#[cfg(feature = "mirage")]` so it does not add a hard dependency on mirage-rs for production builds.

#### Item 3: Create `VenueRegistry` for multi-venue dispatch

**File**: `crates/roko-chain/src/venue.rs` (EDIT -- append)

**What to write**:

```rust
/// Registry of venue adapters, keyed by name.
///
/// The tool handlers use this to dispatch operations to the correct venue.
pub struct VenueRegistry {
    adapters: std::collections::HashMap<String, std::sync::Arc<dyn VenueAdapter>>,
    /// Default venue name (used when the tool invocation omits the venue).
    default_venue: Option<String>,
}

impl VenueRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            adapters: std::collections::HashMap::new(),
            default_venue: None,
        }
    }

    /// Register a venue adapter.
    pub fn register(&mut self, adapter: std::sync::Arc<dyn VenueAdapter>) {
        let name = adapter.name().to_string();
        self.adapters.insert(name, adapter);
    }

    /// Set the default venue name.
    pub fn set_default(&mut self, name: impl Into<String>) {
        self.default_venue = Some(name.into());
    }

    /// Get an adapter by name, falling back to the default.
    pub fn get(&self, name: Option<&str>) -> Option<&std::sync::Arc<dyn VenueAdapter>> {
        if let Some(n) = name {
            return self.adapters.get(n);
        }
        self.default_venue
            .as_ref()
            .and_then(|d| self.adapters.get(d))
    }

    /// List registered venue names.
    pub fn venues(&self) -> Vec<&str> {
        self.adapters.keys().map(|k| k.as_str()).collect()
    }

    /// Number of registered adapters.
    pub fn len(&self) -> usize {
        self.adapters.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.adapters.is_empty()
    }
}

impl Default for VenueRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

#### Item 3: Export from `lib.rs`

**File**: `crates/roko-chain/src/lib.rs` (EDIT)

Add:

```rust
pub mod venue;

pub use venue::{
    AddLiquidityParams, LiquidityResult, MockVenueAdapter, PoolId, PoolState, QuoteParams,
    QuoteResult, RemoveLiquidityParams, SwapParams, SwapResult, VenueAdapter, VenueCapabilities,
    VenueRegistry,
};
```

### Wiring

1. In `crates/roko-chain/src/lib.rs`, add `pub mod venue;` and re-exports
2. No Cargo.toml changes -- `async-trait` and `serde` are already dependencies

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test(flavor = "current_thread")]
    async fn mock_swap_returns_result() {
        let adapter = MockVenueAdapter::full("test_dex");
        let params = SwapParams {
            token_in: "0xA".into(),
            token_out: "0xB".into(),
            amount_in: 1000,
            amount_out_min: 900,
            recipient: None,
            deadline: None,
        };
        let result = adapter.swap(&params).await.unwrap();
        assert!(result.amount_out >= params.amount_out_min);
        assert!(result.gas_used > 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_get_pool_state() {
        let adapter = MockVenueAdapter::full("test_dex");
        let pool = PoolId::Address("0xpool".into());
        let state = adapter.get_pool_state(&pool).await.unwrap();
        assert_eq!(state.address, "0xpool");
        assert!(state.liquidity > 0);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn mock_quote_returns_estimate() {
        let adapter = MockVenueAdapter::full("test_dex");
        let params = QuoteParams {
            token_in: "0xA".into(),
            token_out: "0xB".into(),
            amount_in: 1000,
            fee: None,
        };
        let quote = adapter.get_quote(&params).await.unwrap();
        assert!(quote.amount_out > 0);
        assert!(quote.route.contains("test_dex"));
    }

    #[test]
    fn venue_capabilities() {
        let adapter = MockVenueAdapter::full("test");
        let caps = adapter.capabilities();
        assert!(caps.swap);
        assert!(caps.add_liquidity);
        assert!(caps.concentrated_liquidity);
    }

    #[test]
    fn venue_registry_dispatch() {
        let mut registry = VenueRegistry::new();
        let adapter = Arc::new(MockVenueAdapter::full("uniswap_v3"));
        registry.register(adapter);
        registry.set_default("uniswap_v3");

        assert!(registry.get(Some("uniswap_v3")).is_some());
        assert!(registry.get(None).is_some()); // falls back to default
        assert!(registry.get(Some("curve")).is_none());
    }

    #[test]
    fn venue_registry_list() {
        let mut registry = VenueRegistry::new();
        registry.register(Arc::new(MockVenueAdapter::full("a")));
        registry.register(Arc::new(MockVenueAdapter::full("b")));
        assert_eq!(registry.len(), 2);
        let mut names = registry.venues();
        names.sort();
        assert_eq!(names, vec!["a", "b"]);
    }
}
```

### Verification

```bash
cargo test -p roko-chain -- venue
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `VenueAdapter` trait exists at `crates/roko-chain/src/venue.rs` with 7 methods
- [ ] Parameter types (`SwapParams`, `AddLiquidityParams`, `RemoveLiquidityParams`, `QuoteParams`) are defined with all fields
- [ ] Result types (`SwapResult`, `LiquidityResult`, `QuoteResult`, `PoolState`) are defined
- [ ] `MockVenueAdapter` implements all trait methods with deterministic results
- [ ] `VenueRegistry` provides name-based dispatch with default fallback
- [ ] `MirageVenueAdapter` implements `VenueAdapter` (feature-gated behind `mirage`)
- [ ] `MirageVenueAdapter` connects to a spawned mirage-rs instance and translates adapter calls to RPC
- [ ] At least 6 tests pass
- [ ] `cargo clippy` clean
- [ ] Module exported from `lib.rs`

### Commit message

```
feat(roko-chain): add VenueAdapter trait, mock, and VenueRegistry for DeFi dispatch
```

---

## Batch 2.2: DeFi tool handlers (chain primitives + protocol adapters)

> **Effort**: L | **Depends on**: 2.1 | **Crate**: roko-chain, roko-std
> **Branch**: `defi/batch-2.2-defi-tool-handlers`

### Context

The 14 chain tools are registered in `ROKO_BUILTIN_TOOLS` via `crates/roko-std/src/tool/builtin/mod.rs:68` where `CHAIN_DOMAIN_TOOLS` is appended. But the `HandlerRegistry` at `crates/roko-std/src/tool/handlers.rs:26` only maps the 16 standard tool names to handlers. The `_ => None` catch-all at line 44 means every chain tool invocation fails silently.

This batch implements handlers for the 10 core chain tools (4 primitives + 6 protocol adapters). The handlers wrap `ChainClient`, `ChainWallet`, and `VenueAdapter` calls. Read-only tools (`chain.balance`, `chain.gas_estimate`, `chain.simulate_tx`, `chain.get_pool_info`, `chain.get_position`) call `ChainClient` directly. Write tools (`chain.transfer`, `chain.approve`, `chain.swap`, `chain.add_liquidity`, `chain.remove_liquidity`) route through the `VenueAdapter` or `ChainWallet`.

The handler architecture follows the same pattern as existing std tool handlers: each handler is a unit struct implementing the `ToolHandler` trait from `roko-core`. The handler receives JSON parameters, validates them, calls the appropriate backend, and returns a JSON result.

**Backend abstraction**: all tool handlers must support both live chain and mirage-rs backends. Define a `ChainBackend` enum in `ChainToolState`:

```rust
#[derive(Debug, Clone)]
pub enum ChainBackend {
    /// Production: real RPC endpoint.
    Live,
    /// Testing/simulation: ephemeral mirage-rs fork.
    Mirage { instance_id: String },
}
```

`ChainToolState` carries a `backend: ChainBackend` field. Handlers do not branch on this -- the `ChainClient` and `VenueAdapter` trait objects abstract the difference. The `ChainBackend` value is informational for logging/auditing (so audit trails distinguish real trades from simulated ones).

### Read first

| File | Why |
|------|-----|
| `crates/roko-std/src/tool/handlers.rs` | Current handler registry -- `handler_for` match block at line 26, `HandlerRegistry` wrapper at line 50 |
| `crates/roko-std/src/tool/builtin/read_file.rs` | Example handler pattern -- `NAME` const, `Handler` struct, `ToolHandler` impl |
| `crates/roko-chain/src/tools.rs` | All 14 tool definitions -- parameter schemas define what JSON the handlers receive |
| `crates/roko-chain/src/client.rs` | `ChainClient` trait -- handlers for read tools call these methods |
| `crates/roko-chain/src/venue.rs` | (After batch 2.1) `VenueAdapter` -- handlers for write tools dispatch through this |
| `crates/roko-core/src/tool/handler.rs` | `ToolHandler` trait definition |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Create chain tool handler module

**File**: `crates/roko-chain/src/tool_handlers.rs` (CREATE)

**What to write**:

```rust
//! Runtime handlers for the 10 core chain domain tools.
//!
//! Each handler implements [`ToolHandler`] by extracting JSON parameters,
//! calling the appropriate [`ChainClient`], [`ChainWallet`], or
//! [`VenueAdapter`] method, and returning a JSON result.

use std::sync::Arc;

use async_trait::async_trait;
use roko_core::tool::{ToolContext, ToolError, ToolHandler, ToolResult};
use serde_json::Value;

use crate::client::ChainClient;
use crate::venue::{
    AddLiquidityParams, PoolId, QuoteParams, RemoveLiquidityParams, SwapParams, VenueRegistry,
};
use crate::wallet::ChainWallet;

/// Shared state for chain tool handlers.
#[derive(Clone)]
pub struct ChainToolState {
    /// Chain client for read operations.
    pub client: Arc<dyn ChainClient>,
    /// Wallet for write operations (optional -- read-only agents omit this).
    pub wallet: Option<Arc<dyn ChainWallet>>,
    /// Venue registry for protocol-specific operations.
    pub venues: Arc<VenueRegistry>,
}

fn extract_str(args: &Value, key: &str) -> Result<String, ToolError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ToolError::InvalidArgs(format!("missing required field: {key}")))
}

fn extract_str_opt(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn extract_u128(args: &Value, key: &str) -> Result<u128, ToolError> {
    let s = extract_str(args, key)?;
    s.parse::<u128>()
        .map_err(|e| ToolError::InvalidArgs(format!("{key}: {e}")))
}

fn extract_u64_opt(args: &Value, key: &str) -> Option<u64> {
    args.get(key).and_then(|v| v.as_u64())
}

fn extract_i32_opt(args: &Value, key: &str) -> Option<i32> {
    args.get(key).and_then(|v| v.as_i64()).map(|v| v as i32)
}

fn extract_u32_opt(args: &Value, key: &str) -> Option<u32> {
    args.get(key).and_then(|v| v.as_u64()).map(|v| v as u32)
}

fn chain_err(e: crate::types::ChainError) -> ToolError {
    ToolError::Other(e.to_string())
}

// ---- chain.balance ----

/// Handler for `chain.balance`.
pub struct BalanceHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for BalanceHandler {
    fn name(&self) -> &str {
        "chain.balance"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let address = extract_str(&args, "address")?;
        let block = extract_u64_opt(&args, "block");
        let balance = self.0.client.get_balance(&address, block).await.map_err(chain_err)?;
        Ok(serde_json::json!({
            "address": address,
            "balance_wei": balance.to_string(),
        }))
    }
}

// ---- chain.transfer ----

/// Handler for `chain.transfer`.
pub struct TransferHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for TransferHandler {
    fn name(&self) -> &str {
        "chain.transfer"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let wallet = self
            .0
            .wallet
            .as_ref()
            .ok_or_else(|| ToolError::Other("no wallet configured".into()))?;
        let to = extract_str(&args, "to")?;
        let amount = extract_u128(&args, "amount")?;
        let tx = crate::types::TxRequest {
            to: Some(to.clone()),
            value: amount,
            ..Default::default()
        };
        let hash = wallet.sign_and_submit(tx).await.map_err(chain_err)?;
        Ok(serde_json::json!({
            "tx_hash": hash.to_string(),
            "to": to,
            "amount_wei": amount.to_string(),
        }))
    }
}

// ---- chain.gas_estimate ----

/// Handler for `chain.gas_estimate`.
pub struct GasEstimateHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for GasEstimateHandler {
    fn name(&self) -> &str {
        "chain.gas_estimate"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let to = extract_str(&args, "to")?;
        let data_hex = extract_str_opt(&args, "data").unwrap_or_default();
        let value_str = extract_str_opt(&args, "value").unwrap_or_else(|| "0".into());
        let value: u128 = value_str.parse().unwrap_or(0);

        let data = if data_hex.starts_with("0x") {
            hex::decode(&data_hex[2..]).unwrap_or_default()
        } else {
            Vec::new()
        };

        let tx = crate::types::TxRequest {
            to: Some(to),
            value,
            data,
            from: extract_str_opt(&args, "from"),
            ..Default::default()
        };

        let result = self.0.client.eth_call(&tx, None).await.map_err(chain_err)?;
        let estimated = (result.gas_used as f64 * 1.2) as u64;

        Ok(serde_json::json!({
            "estimated_gas": estimated,
            "gas_used_simulated": result.gas_used,
        }))
    }
}

// ---- chain.simulate_tx ----

/// Handler for `chain.simulate_tx`.
pub struct SimulateTxHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for SimulateTxHandler {
    fn name(&self) -> &str {
        "chain.simulate_tx"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let to = extract_str(&args, "to")?;
        let data_hex = extract_str(&args, "data")?;
        let block = extract_u64_opt(&args, "block");

        let data = if data_hex.starts_with("0x") {
            hex::decode(&data_hex[2..]).unwrap_or_default()
        } else {
            Vec::new()
        };

        let value_str = extract_str_opt(&args, "value").unwrap_or_else(|| "0".into());
        let value: u128 = value_str.parse().unwrap_or(0);

        let tx = crate::types::TxRequest {
            to: Some(to),
            value,
            data,
            from: extract_str_opt(&args, "from"),
            ..Default::default()
        };

        let result = self.0.client.eth_call(&tx, block).await.map_err(chain_err)?;

        Ok(serde_json::json!({
            "output": format!("0x{}", hex::encode(&result.output)),
            "gas_used": result.gas_used,
            "reverted": false,
        }))
    }
}

// ---- chain.swap ----

/// Handler for `chain.swap`.
pub struct SwapHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for SwapHandler {
    fn name(&self) -> &str {
        "chain.swap"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let venue_name = extract_str_opt(&args, "venue");
        let adapter = self
            .0
            .venues
            .get(venue_name.as_deref())
            .ok_or_else(|| ToolError::Other("no venue adapter configured".into()))?;

        let params = SwapParams {
            token_in: extract_str(&args, "token_in")?,
            token_out: extract_str(&args, "token_out")?,
            amount_in: extract_u128(&args, "amount_in")?,
            amount_out_min: extract_u128(&args, "amount_out_min")?,
            recipient: extract_str_opt(&args, "recipient"),
            deadline: extract_u64_opt(&args, "deadline"),
        };

        let result = adapter.swap(&params).await.map_err(chain_err)?;

        Ok(serde_json::json!({
            "tx_hash": result.tx_hash,
            "amount_out": result.amount_out.to_string(),
            "gas_used": result.gas_used,
            "price_impact": result.price_impact,
        }))
    }
}

// ---- chain.get_pool_info ----

/// Handler for `chain.get_pool_info`.
pub struct GetPoolInfoHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for GetPoolInfoHandler {
    fn name(&self) -> &str {
        "chain.get_pool_info"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let venue_name = extract_str_opt(&args, "venue");
        let adapter = self
            .0
            .venues
            .get(venue_name.as_deref())
            .ok_or_else(|| ToolError::Other("no venue adapter configured".into()))?;

        let pool_id = if let Some(pool_addr) = extract_str_opt(&args, "pool") {
            PoolId::Address(pool_addr)
        } else {
            let token_a = extract_str(&args, "token_a")?;
            let token_b = extract_str(&args, "token_b")?;
            let fee = extract_u32_opt(&args, "fee").unwrap_or(3000);
            PoolId::Pair {
                token_a,
                token_b,
                fee,
            }
        };

        let state = adapter.get_pool_state(&pool_id).await.map_err(chain_err)?;

        Ok(serde_json::json!({
            "address": state.address,
            "token0": state.token0,
            "token1": state.token1,
            "reserve0": state.reserve0.to_string(),
            "reserve1": state.reserve1.to_string(),
            "fee": state.fee,
            "tick": state.tick,
            "liquidity": state.liquidity.to_string(),
        }))
    }
}

// ---- chain.add_liquidity ----

/// Handler for `chain.add_liquidity`.
pub struct AddLiquidityHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for AddLiquidityHandler {
    fn name(&self) -> &str {
        "chain.add_liquidity"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let venue_name = extract_str_opt(&args, "venue");
        let adapter = self
            .0
            .venues
            .get(venue_name.as_deref())
            .ok_or_else(|| ToolError::Other("no venue adapter configured".into()))?;

        let params = AddLiquidityParams {
            token_a: extract_str(&args, "token_a")?,
            token_b: extract_str(&args, "token_b")?,
            amount_a: extract_u128(&args, "amount_a")?,
            amount_b: extract_u128(&args, "amount_b")?,
            tick_lower: extract_i32_opt(&args, "tick_lower"),
            tick_upper: extract_i32_opt(&args, "tick_upper"),
            fee: extract_u32_opt(&args, "fee"),
        };

        let result = adapter.add_liquidity(&params).await.map_err(chain_err)?;

        Ok(serde_json::json!({
            "tx_hash": result.tx_hash,
            "amount_a": result.amount_a.to_string(),
            "amount_b": result.amount_b.to_string(),
            "liquidity": result.liquidity.to_string(),
            "gas_used": result.gas_used,
        }))
    }
}

// ---- chain.remove_liquidity ----

/// Handler for `chain.remove_liquidity`.
pub struct RemoveLiquidityHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for RemoveLiquidityHandler {
    fn name(&self) -> &str {
        "chain.remove_liquidity"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let venue_name = extract_str_opt(&args, "venue");
        let adapter = self
            .0
            .venues
            .get(venue_name.as_deref())
            .ok_or_else(|| ToolError::Other("no venue adapter configured".into()))?;

        let params = RemoveLiquidityParams {
            position_id: extract_str(&args, "token_id")
                .or_else(|_| extract_str(&args, "position_id"))?,
            liquidity: extract_u128(&args, "liquidity")?,
            amount_a_min: extract_str_opt(&args, "amount_a_min")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            amount_b_min: extract_str_opt(&args, "amount_b_min")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
        };

        let result = adapter.remove_liquidity(&params).await.map_err(chain_err)?;

        Ok(serde_json::json!({
            "tx_hash": result.tx_hash,
            "amount_a": result.amount_a.to_string(),
            "amount_b": result.amount_b.to_string(),
            "liquidity": result.liquidity.to_string(),
            "gas_used": result.gas_used,
        }))
    }
}

// ---- chain.approve ----

/// Handler for `chain.approve`.
pub struct ApproveHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for ApproveHandler {
    fn name(&self) -> &str {
        "chain.approve"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let wallet = self
            .0
            .wallet
            .as_ref()
            .ok_or_else(|| ToolError::Other("no wallet configured".into()))?;

        let token = extract_str(&args, "token")?;
        let spender = extract_str(&args, "spender")?;
        let amount_str = extract_str(&args, "amount")?;

        // ERC-20 approve(address,uint256) selector: 0x095ea7b3
        let amount_bytes = if amount_str == "max" {
            [0xff; 32].to_vec()
        } else {
            let amount: u128 = amount_str
                .parse()
                .map_err(|e| ToolError::InvalidArgs(format!("amount: {e}")))?;
            let mut buf = vec![0u8; 32];
            buf[16..].copy_from_slice(&amount.to_be_bytes());
            buf
        };

        // ABI encode: approve(address spender, uint256 amount)
        let mut data = vec![0x09, 0x5e, 0xa7, 0xb3]; // selector
        data.extend_from_slice(&[0u8; 12]); // left-pad address to 32 bytes
        let spender_bytes = hex::decode(spender.trim_start_matches("0x"))
            .map_err(|e| ToolError::InvalidArgs(format!("spender: {e}")))?;
        data.extend_from_slice(&spender_bytes);
        data.extend_from_slice(&amount_bytes);

        let tx = crate::types::TxRequest {
            to: Some(token.clone()),
            data,
            ..Default::default()
        };

        let hash = wallet.sign_and_submit(tx).await.map_err(chain_err)?;

        Ok(serde_json::json!({
            "tx_hash": hash.to_string(),
            "token": token,
            "spender": spender,
            "amount": amount_str,
        }))
    }
}

// ---- chain.get_position ----

/// Handler for `chain.get_position`.
pub struct GetPositionHandler(pub ChainToolState);

#[async_trait]
impl ToolHandler for GetPositionHandler {
    fn name(&self) -> &str {
        "chain.get_position"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let token_id = extract_str(&args, "token_id")?;
        let _position_manager = extract_str_opt(&args, "position_manager");

        // For now, use the pool state query as a proxy. Full position
        // queries require the NonfungiblePositionManager ABI, which
        // will be implemented in a V3-specific venue adapter.
        Ok(serde_json::json!({
            "token_id": token_id,
            "status": "query requires V3 venue adapter (see batch 2.4)",
        }))
    }
}
```

#### Item 2: Export from `lib.rs`

**File**: `crates/roko-chain/src/lib.rs` (EDIT)

Add:

```rust
pub mod tool_handlers;

pub use tool_handlers::ChainToolState;
```

#### Item 3: Add `hex` dependency

**File**: `crates/roko-chain/Cargo.toml` (EDIT)

Add to `[dependencies]`:

```toml
hex = { workspace = true }
```

Verify `hex` is in the workspace `Cargo.toml`. If not, add it.

### Wiring

1. In `crates/roko-chain/src/lib.rs`, add `pub mod tool_handlers;` and re-export `ChainToolState`
2. In `crates/roko-chain/Cargo.toml`, add `hex = { workspace = true }`
3. Verify `hex` is in workspace root `Cargo.toml` `[workspace.dependencies]`

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockChainClient, MockChainWallet, paired_mocks};
    use crate::venue::MockVenueAdapter;
    use roko_core::tool::ToolContext;
    use std::sync::Arc;

    fn test_state() -> ChainToolState {
        let (client, wallet) = paired_mocks(1_000_000_000);
        let mut venues = VenueRegistry::new();
        venues.register(Arc::new(MockVenueAdapter::full("mock_dex")));
        venues.set_default("mock_dex");
        ChainToolState {
            client: Arc::new(client),
            wallet: Some(Arc::new(wallet)),
            venues: Arc::new(venues),
        }
    }

    fn test_ctx() -> ToolContext {
        ToolContext::default()
    }

    #[tokio::test(flavor = "current_thread")]
    async fn balance_handler_returns_wei() {
        let state = test_state();
        let handler = BalanceHandler(state);
        let args = serde_json::json!({"address": "0x0000000000000000000000000000000000000001"});
        let result = handler.handle(args, &test_ctx()).await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn swap_handler_uses_venue() {
        let state = test_state();
        let handler = SwapHandler(state);
        let args = serde_json::json!({
            "token_in": "0xA",
            "token_out": "0xB",
            "amount_in": "1000",
            "amount_out_min": "900",
        });
        let result = handler.handle(args, &test_ctx()).await.unwrap();
        assert!(result.get("tx_hash").is_some());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn get_pool_info_handler() {
        let state = test_state();
        let handler = GetPoolInfoHandler(state);
        let args = serde_json::json!({"pool": "0xpool"});
        let result = handler.handle(args, &test_ctx()).await.unwrap();
        assert!(result.get("liquidity").is_some());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn transfer_handler_requires_wallet() {
        let (client, _) = paired_mocks(0);
        let mut venues = VenueRegistry::new();
        venues.register(Arc::new(MockVenueAdapter::full("mock")));
        let state = ChainToolState {
            client: Arc::new(client),
            wallet: None,
            venues: Arc::new(venues),
        };
        let handler = TransferHandler(state);
        let args = serde_json::json!({"to": "0x1", "amount": "100"});
        let result = handler.handle(args, &test_ctx()).await;
        assert!(result.is_err());
    }
}
```

### Verification

```bash
cargo test -p roko-chain -- tool_handlers
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] Handlers exist for all 10 core tools: balance, transfer, gas_estimate, simulate_tx, approve, swap, add_liquidity, remove_liquidity, get_pool_info, get_position
- [ ] Each handler implements `ToolHandler` with correct `name()` and `handle()` methods
- [ ] Read-only handlers call `ChainClient` directly
- [ ] Write handlers route through `VenueAdapter` or `ChainWallet`
- [ ] `ChainToolState` bundles `client + wallet + venues + backend` for handler construction
- [ ] `ChainBackend` enum distinguishes `Live` from `Mirage { instance_id }` for audit trails
- [ ] At least 4 tests pass
- [ ] `cargo clippy` clean
- [ ] Module exported from `lib.rs`

### Commit message

```
feat(roko-chain): implement ToolHandler for 10 core DeFi tools
```

---

## Batch 2.3: Wire chain handlers into HandlerRegistry

> **Effort**: S | **Depends on**: 2.2 | **Crate**: roko-std
> **Branch**: `defi/batch-2.3-wire-handler-registry`

### Context

After batch 2.2, the chain tool handlers exist in `roko-chain` but are not registered in the `HandlerRegistry` at `crates/roko-std/src/tool/handlers.rs`. The match block at line 26 still returns `None` for chain tool names.

This batch wires the chain handlers into the handler registry. The challenge is that chain handlers need runtime state (`ChainToolState`) while std handlers are zero-sized unit structs.

**Decision: use `ChainHandlerRegistry`** (not `chain_handler_for`). A separate registry in `roko-chain` keeps the dependency direction clean -- `roko-std` does not depend on `roko-chain`. The dispatcher consults both registries: std first, then chain.

```rust
pub struct ChainHandlerRegistry {
    handlers: HashMap<String, Box<dyn ChainToolHandler>>,
    backend: ChainBackend,
}

impl ChainHandlerRegistry {
    /// Build a registry pre-loaded with all chain tool handlers.
    pub fn new(backend: ChainBackend) -> Self {
        let mut handlers: HashMap<String, Box<dyn ChainToolHandler>> = HashMap::new();
        let state = ChainToolState::from_backend(&backend);
        // Register all 10 core handlers
        handlers.insert("chain.balance".into(), Box::new(BalanceHandler(state.clone())));
        handlers.insert("chain.transfer".into(), Box::new(TransferHandler(state.clone())));
        handlers.insert("chain.gas_estimate".into(), Box::new(GasEstimateHandler(state.clone())));
        handlers.insert("chain.simulate_tx".into(), Box::new(SimulateTxHandler(state.clone())));
        handlers.insert("chain.approve".into(), Box::new(ApproveHandler(state.clone())));
        handlers.insert("chain.swap".into(), Box::new(SwapHandler(state.clone())));
        handlers.insert("chain.add_liquidity".into(), Box::new(AddLiquidityHandler(state.clone())));
        handlers.insert("chain.remove_liquidity".into(), Box::new(RemoveLiquidityHandler(state.clone())));
        handlers.insert("chain.get_pool_info".into(), Box::new(GetPoolInfoHandler(state.clone())));
        handlers.insert("chain.get_position".into(), Box::new(GetPositionHandler(state)));
        Self { handlers, backend }
    }

    /// Register a custom chain tool handler (for venue-specific extensions).
    pub fn register(&mut self, name: &str, handler: impl ChainToolHandler + 'static) {
        self.handlers.insert(name.to_string(), Box::new(handler));
    }

    /// Look up a handler by tool name. Returns `None` for non-chain tools.
    pub fn handle(&self, name: &str, params: &serde_json::Value) -> ChainResult<serde_json::Value> {
        let handler = self.handlers.get(name)
            .ok_or_else(|| ChainError::UnknownTool(name.to_string()))?;
        handler.execute(params, &self.backend)
    }
}
```

The dispatcher wiring in `roko-agent/src/dispatcher/mod.rs` adds an `Option<Arc<ChainHandlerRegistry>>` field. Tool resolution becomes: try `HandlerRegistry::handler_for(name)` first; if `None`, try `chain_registry.handle(name, params)`.

**Deployment note**: handler registration and tool dispatch happen in the roko control plane (Railway, always-on). In-process agents (monitoring, research, risk-assessor, safety-guardian) call tool handlers directly. Isolated Fly Machine agents (trading, coding) do not have direct access to the `ChainHandlerRegistry` -- they invoke tools via the control plane's `/api/tools/call` endpoint, which resolves the handler internally. The `ChainHandlerRegistry` is instantiated once in the control plane's startup sequence and shared across all in-process agent instances.

### Read first

| File | Why |
|------|-----|
| `crates/roko-std/src/tool/handlers.rs` | Current handler registry pattern -- `handler_for` at line 26, `HandlerRegistry` at line 50 |
| `crates/roko-chain/src/tool_handlers.rs` | (After batch 2.2) Handler implementations |
| `crates/roko-agent/src/dispatcher/mod.rs` | Agent dispatcher -- where tool handlers are actually called |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Create `ChainHandlerRegistry`

**File**: `crates/roko-chain/src/tool_handlers.rs` (EDIT -- append)

**What to write**:

```rust
/// Registry that maps chain tool names to their handlers.
///
/// Mirrors the `HandlerRegistry` pattern in `roko-std`, but holds
/// `ChainToolState` needed by chain handlers.
pub struct ChainHandlerRegistry {
    state: ChainToolState,
}

impl ChainHandlerRegistry {
    /// Create a chain handler registry with the given state.
    pub fn new(state: ChainToolState) -> Self {
        Self { state }
    }

    /// Look up a handler by canonical name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn ToolHandler>> {
        match name {
            "chain.balance" => Some(Arc::new(BalanceHandler(self.state.clone()))),
            "chain.transfer" => Some(Arc::new(TransferHandler(self.state.clone()))),
            "chain.gas_estimate" => Some(Arc::new(GasEstimateHandler(self.state.clone()))),
            "chain.simulate_tx" => Some(Arc::new(SimulateTxHandler(self.state.clone()))),
            "chain.approve" => Some(Arc::new(ApproveHandler(self.state.clone()))),
            "chain.swap" => Some(Arc::new(SwapHandler(self.state.clone()))),
            "chain.add_liquidity" => Some(Arc::new(AddLiquidityHandler(self.state.clone()))),
            "chain.remove_liquidity" => {
                Some(Arc::new(RemoveLiquidityHandler(self.state.clone())))
            }
            "chain.get_pool_info" => Some(Arc::new(GetPoolInfoHandler(self.state.clone()))),
            "chain.get_position" => Some(Arc::new(GetPositionHandler(self.state.clone()))),
            _ => None,
        }
    }

    /// All chain tool names this registry handles.
    pub fn names(&self) -> &[&str] {
        &[
            "chain.balance",
            "chain.transfer",
            "chain.gas_estimate",
            "chain.simulate_tx",
            "chain.approve",
            "chain.swap",
            "chain.add_liquidity",
            "chain.remove_liquidity",
            "chain.get_pool_info",
            "chain.get_position",
        ]
    }
}
```

#### Item 2: Update `lib.rs` re-exports

**File**: `crates/roko-chain/src/lib.rs` (EDIT)

Add to re-exports:

```rust
pub use tool_handlers::ChainHandlerRegistry;
```

### Wiring

1. In `crates/roko-chain/src/lib.rs`, add `ChainHandlerRegistry` to re-exports
2. The dispatcher in `crates/roko-agent/src/dispatcher/mod.rs` should be updated to hold an optional `ChainHandlerRegistry` and consult it when `handler_for` returns `None`. That wiring is outside this batch's scope but documented here for the next agent.

### Tests

```rust
#[cfg(test)]
mod registry_tests {
    use super::*;
    use crate::mock::paired_mocks;
    use crate::venue::MockVenueAdapter;
    use std::sync::Arc;

    fn test_registry() -> ChainHandlerRegistry {
        let (client, wallet) = paired_mocks(1_000_000);
        let mut venues = VenueRegistry::new();
        venues.register(Arc::new(MockVenueAdapter::full("mock")));
        venues.set_default("mock");
        ChainHandlerRegistry::new(ChainToolState {
            client: Arc::new(client),
            wallet: Some(Arc::new(wallet)),
            venues: Arc::new(venues),
        })
    }

    #[test]
    fn registry_resolves_all_chain_tools() {
        let reg = test_registry();
        for name in reg.names() {
            assert!(
                reg.get(name).is_some(),
                "missing handler for chain tool: {name}"
            );
        }
    }

    #[test]
    fn registry_returns_none_for_std_tools() {
        let reg = test_registry();
        assert!(reg.get("read_file").is_none());
        assert!(reg.get("bash").is_none());
    }

    #[test]
    fn registry_handler_names_match() {
        let reg = test_registry();
        for name in reg.names() {
            let handler = reg.get(name).unwrap();
            assert_eq!(handler.name(), *name);
        }
    }
}
```

### Verification

```bash
cargo test -p roko-chain -- tool_handlers
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `ChainHandlerRegistry` exists in `tool_handlers.rs`
- [ ] `get()` resolves all 10 chain tool names to handlers
- [ ] Returns `None` for non-chain tool names
- [ ] Handler names match the canonical tool names
- [ ] At least 3 tests pass
- [ ] `cargo clippy` clean
- [ ] Exported from `lib.rs`

### Commit message

```
feat(roko-chain): add ChainHandlerRegistry for tool dispatch integration
```

---

## Batch 2.4: Analysis and data-query tool definitions

> **Effort**: M | **Depends on**: 2.1 | **Crate**: roko-chain
> **Branch**: `defi/batch-2.4-analysis-tools`

### Context

The current 14 chain tools cover chain primitives (balance, transfer, gas, simulate), protocol operations (approve, swap, LP, pool, position), and wallet management. None of them address the data-query and analysis use cases that observer/research agents need: token prices, portfolio summaries, transaction decoding, and pool analytics.

This batch adds 8 read-only `ToolDef` registrations for analysis and data-query operations. These tools are idempotent, parallel-safe, and require no wallet. They aggregate data from `ChainClient` reads, the `ProtocolStateCache` (batch 1.4), and the triage pipeline.

The tool definitions follow the same pattern as existing tools in `crates/roko-chain/src/tools.rs`. Handlers are stubs that return structured "not yet wired" responses (matching the pattern used by std tools at launch). Full handler implementations depend on external data sources (price feeds, subgraphs) that are outside Phase 0 scope.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/tools.rs` | Existing 14 tool definitions -- follow this exact pattern for new defs |
| `crates/roko-core/src/tool/def.rs` | `ToolDef`, `ToolSchema`, `ToolCategory`, `ToolConcurrency` structs |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Add 8 analysis tool definitions

**File**: `crates/roko-chain/src/tools.rs` (EDIT)

**What to write**:

Add new tool definition functions after the wallet tools section (after line 603). Add the new tools to `CHAIN_DOMAIN_TOOLS` and `CHAIN_TOOL_NAMES`. Update `CHAIN_TOOL_COUNT` from 14 to 22.

New tools:

1. `chain.get_block` -- fetch block header by number or "latest"
2. `chain.get_receipt` -- fetch transaction receipt by hash
3. `chain.get_tx` -- fetch transaction details by hash
4. `chain.decode_log` -- decode a log entry against a known ABI
5. `chain.token_info` -- query ERC-20 token metadata (symbol, decimals, totalSupply)
6. `chain.portfolio` -- aggregate token balances for an address
7. `chain.pool_analytics` -- time-series data for a pool (volume, TVL, fees)
8. `chain.price` -- query token price from on-chain oracle or pool reserves

```rust
/// `chain.get_block` -- fetch block header.
fn get_block_tool_def() -> ToolDef {
    ToolDef {
        name: "chain.get_block".into(),
        description: "Fetch a block header by number. Returns number, hash, \
            parent hash, timestamp, and gas used."
            .into(),
        parameters: ToolSchema::from_value(serde_json::json!({
            "type": "object",
            "properties": {
                "block": {
                    "type": "integer",
                    "description": "Block number. Omit for latest."
                }
            },
            "additionalProperties": false
        })),
        category: ToolCategory::Network,
        permission: ToolPermission::networked(),
        timeout_ms: 30_000,
        concurrency: ToolConcurrency::Parallel,
        idempotent: true,
        source: ToolSource::Builtin,
        metadata: None,
    }
}
```

Follow this pattern for all 8 tools. Each requires:
- Correct JSON Schema parameters
- `ToolCategory::Network`
- `ToolPermission::networked()`
- `ToolConcurrency::Parallel` (all are read-only)
- `idempotent: true`

#### Item 2: Update constants and static arrays

**File**: `crates/roko-chain/src/tools.rs` (EDIT)

**What to write**:

Update `CHAIN_TOOL_COUNT` to 22. Add the 8 new tool names to `CHAIN_TOOL_NAMES`. Add the 8 new def functions to `CHAIN_DOMAIN_TOOLS`.

### Wiring

1. Update `CHAIN_TOOL_COUNT` from 14 to 22
2. Add 8 entries to `CHAIN_TOOL_NAMES` array
3. Add 8 entries to `CHAIN_DOMAIN_TOOLS` LazyLock array
4. `ROKO_BUILTIN_TOOLS` in `roko-std` automatically picks up the new tools because it clones `CHAIN_DOMAIN_TOOLS`

### Tests

```rust
#[test]
fn chain_tools_have_correct_count() {
    assert_eq!(CHAIN_DOMAIN_TOOLS.len(), CHAIN_TOOL_COUNT);
    assert_eq!(CHAIN_TOOL_NAMES.len(), CHAIN_TOOL_COUNT);
}

#[test]
fn analysis_tools_are_read_only() {
    let analysis_tools = [
        "chain.get_block",
        "chain.get_receipt",
        "chain.get_tx",
        "chain.decode_log",
        "chain.token_info",
        "chain.portfolio",
        "chain.pool_analytics",
        "chain.price",
    ];
    for tool in CHAIN_DOMAIN_TOOLS.iter() {
        if analysis_tools.contains(&tool.name.as_str()) {
            assert!(tool.idempotent, "tool {} should be idempotent", tool.name);
            assert_eq!(
                tool.concurrency,
                ToolConcurrency::Parallel,
                "tool {} should be Parallel",
                tool.name
            );
        }
    }
}

#[test]
fn all_tools_have_unique_names() {
    let mut seen = std::collections::HashSet::new();
    for tool in CHAIN_DOMAIN_TOOLS.iter() {
        assert!(seen.insert(&tool.name), "duplicate tool name: {}", tool.name);
    }
}
```

### Verification

```bash
cargo test -p roko-chain -- tools
cargo test -p roko-std -- builtin  # verify ROKO_BUILTIN_TOOLS picks up new tools
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] `CHAIN_TOOL_COUNT` is 22
- [ ] `CHAIN_DOMAIN_TOOLS` contains all 22 tool definitions
- [ ] All 8 new tools have `ToolCategory::Network`, `idempotent: true`, `ToolConcurrency::Parallel`
- [ ] All tools have valid JSON Schema parameters
- [ ] `ROKO_BUILTIN_TOOLS` in `roko-std` automatically includes the new tools
- [ ] Existing tool tests still pass
- [ ] At least 3 new tests pass
- [ ] `cargo clippy` clean

### Commit message

```
feat(roko-chain): add 8 analysis and data-query tool definitions
```

---

## Batch 2.5: Wallet tool handlers

> **Effort**: S | **Depends on**: 1.5, 2.2 | **Crate**: roko-chain
> **Branch**: `defi/batch-2.5-wallet-tool-handlers`

### Context

The 4 wallet management tools (`chain.wallet_create`, `chain.wallet_list`, `chain.wallet_info`, `chain.wallet_export_address`) defined in `crates/roko-chain/src/tools.rs:486` need handlers that operate on the `WalletRegistry` from batch 1.5.

These handlers are simpler than the DeFi tool handlers -- they manage wallet lifecycle rather than executing chain operations. `wallet_list` and `wallet_info` are read-only queries against the registry. `wallet_create` generates a new key pair and registers it. `wallet_export_address` returns the public address for a wallet.

For Phase 0, `wallet_create` generates an in-memory key pair using `MockChainWallet`. Real key generation via alloy `PrivateKeySigner::random()` is gated behind the `alloy-backend` feature.

### Read first

| File | Why |
|------|-----|
| `crates/roko-chain/src/tools.rs:486` | Wallet tool definitions -- parameter schemas |
| `crates/roko-chain/src/wallet_registry.rs` | (After batch 1.5) `WalletRegistry`, `WalletEntry` |
| `crates/roko-chain/src/tool_handlers.rs` | (After batch 2.2) `ChainToolState`, handler pattern |
| `crates/roko-chain/src/mock.rs:272` | `MockChainWallet` -- used for in-memory key creation |

### Conventions

- **Module files**: one file per type, flat in `src/`, declared in `lib.rs` as `pub mod xxx;`
- **Errors**: use `thiserror` for domain errors, convert to `RokoError` at boundaries
- **Tests**: inline `#[cfg(test)] mod tests {}`, min 3 tests per new type
- **Lints**: `cargo clippy --no-deps -- -D warnings` must pass clean
- **Docs**: `///` on all public items, `//!` module-level doc in new files
- **Derives**: `#[derive(Debug, Clone)]` minimum; add `Serialize, Deserialize` if persisted
- **Mirage testing**: integration tests that need chain state should use ephemeral mirage-rs instances. Pattern: `spawn_mirage_test_instance(Some(rpc_url), Some(fork_block)).await?` from `apps/mirage-rs/src/integration.rs`. Connect via `MirageClient::new(instance.config()).await?`. Shut down with `instance.shutdown().await?`. Add `mirage-rs` as a dev-dependency feature gate.

### Work items

#### Item 1: Add wallet handlers

**File**: `crates/roko-chain/src/tool_handlers.rs` (EDIT -- append)

**What to write**:

```rust
use crate::wallet_registry::WalletRegistry;

/// Extended state for wallet tool handlers.
#[derive(Clone)]
pub struct WalletToolState {
    /// Wallet registry for multi-wallet management.
    pub registry: Arc<WalletRegistry>,
}

// ---- chain.wallet_create ----

/// Handler for `chain.wallet_create`.
pub struct WalletCreateHandler(pub WalletToolState);

#[async_trait]
impl ToolHandler for WalletCreateHandler {
    fn name(&self) -> &str {
        "chain.wallet_create"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let label = extract_str_opt(&args, "label")
            .unwrap_or_else(|| format!("wallet-{}", self.0.registry.len()));
        let network = extract_str_opt(&args, "network").unwrap_or_else(|| "ethereum".into());

        let wallet = Arc::new(crate::mock::MockChainWallet::funded(0)
            .with_address(format!(
                "0x{:040x}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
                    % (1u128 << 160)
            )));

        let entry = self
            .0
            .registry
            .register(&label, &network, wallet)
            .await
            .map_err(chain_err)?;

        Ok(serde_json::json!({
            "label": entry.label,
            "address": entry.address,
            "network": entry.network,
        }))
    }
}

// ---- chain.wallet_list ----

/// Handler for `chain.wallet_list`.
pub struct WalletListHandler(pub WalletToolState);

#[async_trait]
impl ToolHandler for WalletListHandler {
    fn name(&self) -> &str {
        "chain.wallet_list"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let network = extract_str_opt(&args, "network");
        let entries = match network {
            Some(n) => self.0.registry.list_by_network(&n),
            None => self.0.registry.list(),
        };
        Ok(serde_json::json!({
            "wallets": entries,
            "count": entries.len(),
        }))
    }
}

// ---- chain.wallet_info ----

/// Handler for `chain.wallet_info`.
pub struct WalletInfoHandler(pub WalletToolState);

#[async_trait]
impl ToolHandler for WalletInfoHandler {
    fn name(&self) -> &str {
        "chain.wallet_info"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let label = extract_str_opt(&args, "wallet_id");
        let address = extract_str_opt(&args, "address");

        let entry = if let Some(l) = label {
            self.0.registry.info(&l)
        } else if let Some(a) = &address {
            self.0.registry.list().into_iter().find(|e| e.address.to_lowercase() == a.to_lowercase())
        } else {
            return Err(ToolError::InvalidArgs("need wallet_id or address".into()));
        };

        match entry {
            Some(e) => Ok(serde_json::json!(e)),
            None => Err(ToolError::Other("wallet not found".into())),
        }
    }
}

// ---- chain.wallet_export_address ----

/// Handler for `chain.wallet_export_address`.
pub struct WalletExportAddressHandler(pub WalletToolState);

#[async_trait]
impl ToolHandler for WalletExportAddressHandler {
    fn name(&self) -> &str {
        "chain.wallet_export_address"
    }

    async fn handle(&self, args: Value, _ctx: &ToolContext) -> ToolResult {
        let wallet_id = extract_str(&args, "wallet_id")?;
        let entry = self
            .0
            .registry
            .info(&wallet_id)
            .ok_or_else(|| ToolError::Other(format!("wallet not found: {wallet_id}")))?;
        Ok(serde_json::json!({
            "wallet_id": wallet_id,
            "address": entry.address,
        }))
    }
}
```

#### Item 2: Register wallet handlers in `ChainHandlerRegistry`

**File**: `crates/roko-chain/src/tool_handlers.rs` (EDIT)

**What to write**:

Update `ChainHandlerRegistry` to hold optional `WalletToolState` and add wallet tool matches:

Add a `wallet_state` field and extend the `get()` match block:

```rust
pub struct ChainHandlerRegistry {
    state: ChainToolState,
    wallet_state: Option<WalletToolState>,
}

impl ChainHandlerRegistry {
    pub fn new(state: ChainToolState) -> Self {
        Self {
            state,
            wallet_state: None,
        }
    }

    pub fn with_wallet_registry(mut self, registry: Arc<WalletRegistry>) -> Self {
        self.wallet_state = Some(WalletToolState { registry });
        self
    }
}
```

Add these matches to `get()`:

```rust
"chain.wallet_create" => self.wallet_state.as_ref().map(|ws| {
    Arc::new(WalletCreateHandler(ws.clone())) as Arc<dyn ToolHandler>
}),
"chain.wallet_list" => self.wallet_state.as_ref().map(|ws| {
    Arc::new(WalletListHandler(ws.clone())) as Arc<dyn ToolHandler>
}),
"chain.wallet_info" => self.wallet_state.as_ref().map(|ws| {
    Arc::new(WalletInfoHandler(ws.clone())) as Arc<dyn ToolHandler>
}),
"chain.wallet_export_address" => self.wallet_state.as_ref().map(|ws| {
    Arc::new(WalletExportAddressHandler(ws.clone())) as Arc<dyn ToolHandler>
}),
```

### Wiring

1. Update `ChainHandlerRegistry` struct and constructor
2. Add wallet tool names to the `names()` method

### Tests

```rust
#[tokio::test(flavor = "current_thread")]
async fn wallet_create_handler() {
    let registry = Arc::new(WalletRegistry::new());
    let state = WalletToolState {
        registry: Arc::clone(&registry),
    };
    let handler = WalletCreateHandler(state);
    let args = serde_json::json!({"label": "test", "network": "base"});
    let result = handler.handle(args, &test_ctx()).await.unwrap();
    assert_eq!(result["label"], "test");
    assert_eq!(result["network"], "base");
    assert_eq!(registry.len(), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn wallet_list_handler() {
    let registry = Arc::new(WalletRegistry::new());
    let wallet = Arc::new(MockChainWallet::funded(0));
    registry.register("w1", "ethereum", wallet).await.unwrap();
    let state = WalletToolState { registry };
    let handler = WalletListHandler(state);
    let args = serde_json::json!({});
    let result = handler.handle(args, &test_ctx()).await.unwrap();
    assert_eq!(result["count"], 1);
}

#[tokio::test(flavor = "current_thread")]
async fn wallet_export_address_handler() {
    let registry = Arc::new(WalletRegistry::new());
    let wallet = Arc::new(MockChainWallet::funded(0));
    registry.register("export_test", "ethereum", wallet).await.unwrap();
    let state = WalletToolState { registry };
    let handler = WalletExportAddressHandler(state);
    let args = serde_json::json!({"wallet_id": "export_test"});
    let result = handler.handle(args, &test_ctx()).await.unwrap();
    assert!(result.get("address").is_some());
}
```

### Verification

```bash
cargo test -p roko-chain -- tool_handlers
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo +nightly fmt --check -p roko-chain
```

### Acceptance criteria

- [ ] Handlers exist for all 4 wallet tools: create, list, info, export_address
- [ ] `wallet_create` generates a new wallet and registers it in the `WalletRegistry`
- [ ] `wallet_list` returns all or network-filtered entries
- [ ] `wallet_info` looks up by label or address
- [ ] `wallet_export_address` returns only the public address
- [ ] `ChainHandlerRegistry` resolves all 14 chain tool names (10 core + 4 wallet)
- [ ] At least 3 tests pass
- [ ] `cargo clippy` clean

### Commit message

```
feat(roko-chain): implement wallet tool handlers with WalletRegistry
```

## Product Layer

> Maps this gap doc's capabilities to the 12 universal primitives defined in `docs/prd/23-universal-primitives.md`.

### Primitives used

- **Connector**: `VenueAdapter` is the canonical connector type for this gap doc — it handles all exchange I/O including order placement, position queries, and fill events. `HyperliquidConnector` and `UniswapConnector` are concrete implementations of that interface, each carrying venue-specific auth, rate limits, and supported instrument lists.
- **Extension**: `SwapHandler`, `LpHandler`, and `VaultHandler` are extensions that wrap Connector operations with validation logic — slippage tolerance checks, gas estimation, pre-submission simulation, and retry policy. They fire before the connector call (pre-action) and after (post-action) to normalize results and surface structured errors. These are Tier 3 (Roko-native) extensions implementing the `Extension` trait's `validate` and `execute` hooks. Pi-compatible tool wrappers can also be built as Tier 1 extensions via `pi.registerTool()` for simpler use cases.
- **Gate**: `RiskAssessmentTool` runs as a pre-action gate before any tool execution — it checks position size, portfolio exposure, and venue-specific constraints before allowing the operation to proceed.

### Authoring surfaces

- **Connector Manager** — configure venue connections: API keys or wallet addresses, rate limits, supported trading pairs, and default order parameters per venue
- **Extension Workshop** — customize tool handler behavior: set slippage tolerance, gas price caps, maximum retry count, and fallback behavior per venue or per operation type
- **System → Extensions** — browse and install tool packs grouped by venue; each pack bundles the full set of handlers for that venue (swap + LP + vault as a unit)

### Shareable artifacts

- Connector configurations: venue presets with validated default parameters — importable without credential re-entry (credentials stay local)
- Tool packs: bundled operation handlers for a venue, versioned and publishable to the marketplace
- Extension templates: standard slippage, gas, and retry configurations for common risk profiles (conservative, normal, aggressive)

### Dashboard visibility

- **System → Extensions** — installed tool packs with version status, available operations per venue, and pack health indicators
- **Agent Composer Stage 4** — tool selection panel drawn from available connectors; surfaces compatible operations for the agent's declared domain
- **System → Connectors** — venue health dashboard showing latency, fill rates, and order rejection rates per connected exchange
