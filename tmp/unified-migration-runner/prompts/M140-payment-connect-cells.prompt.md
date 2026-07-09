# M140 — Payment Connect Cells (x402 + State Channels)

**[BLOCKED:chain]** -- Requires M131 (ChainConnector), M037 (Connect protocol), M012 (Cell trait). Chain deployment is Tier 6.

## Objective
Implement two payment Connect Cells: `X402ConnectCell` for per-request stateless micropayments (HTTP 402 handshake with ERC-3009 authorization), and `StateChannelConnectCell` for off-chain transact with cooperative close. Both implement the Cell + Connect protocol lifecycle (connect, health, disconnect), with domain-specific inherent methods for payment operations. A `PaymentRouteCell` selects between them based on interaction pattern.

## Scope
- Crates: `roko-chain`
- Files:
  - `crates/roko-chain/src/payment_cells.rs` (new)
  - `crates/roko-chain/src/lib.rs` (add module + re-exports)
- Depth doc: `tmp/unified-depth/18-registries/06-payments-and-settlement.md` SS1

## Steps
1. Check existing x402 implementation:
   ```bash
   grep -rn 'pub struct PaymentRequest\|pub struct PaymentAuthorization\|pub enum VerificationStatus\|pub struct X402Manager' crates/roko-chain/src/x402.rs
   ```
   **Expected**: `PaymentRequest` at `x402.rs:27` (fields: `recipient: Address`, `amount: u256`, `token: Address`, `nonce: u256`, `deadline: u64`, `reason: String`). `PaymentAuthorization` at `x402.rs:46` (fields: `from`, `to`, `value`, `valid_after`, `valid_before`, `nonce`, `v: u8`, `r: [u8; 32]`, `s: [u8; 32]`). `VerificationStatus` at `x402.rs:69` (enum: `Valid`, `Expired`, `NonceReused`, `AmountMismatch`, `InvalidSignature`, `InsufficientFunds`).

2. Check existing state channel types in phase2:
   ```bash
   grep -rn 'pub struct AgentPaymentChannel\|pub enum ChannelLifecycle\|pub struct ChannelParty' crates/roko-chain/src/phase2.rs crates/roko-chain/src/x402.rs
   ```
   **Expected**: `AgentPaymentChannel` in `phase2.rs` with fields: `channel_id`, `agent_a: ChannelParty`, `agent_b: ChannelParty`, `deposit_a: u256`, `deposit_b: u256`, `state: ChannelState` (struct with `nonce: u64` + balance fields), `challenge_window: u64`. `ChannelLifecycle` enum in `x402.rs`: `Open`, `Closing { close_requested_at: u64 }`, `Closed`, `Disputed` (no `Opening` variant). `ChannelParty` with `address` and `passport_id`.

3. Verify Connect and Route protocol traits:
   ```bash
   grep -rn 'pub trait Connect' crates/roko-core/src/traits.rs
   grep -rn 'pub trait Route' crates/roko-core/src/traits.rs
   ```
   **Expected**: `Connect` at `traits.rs:408` (supertrait of `Cell`, methods: `connect() -> Result<()>`, `health() -> bool`, `disconnect() -> Result<()>`). `Route` at `traits.rs:242` (sync: `select(&[Engram], &Context) -> Option<Selection>`, `feedback(&Outcome)`, `name() -> &str`).

   **IMPORTANT**: The `Connect` trait only has lifecycle methods (connect/health/disconnect). Payment-specific operations (`query_terms()`, `authorize_payment()`, `open_channel()`, `close_channel()`) are inherent methods on the Cell structs, NOT part of the Connect trait.

4. Create `crates/roko-chain/src/payment_cells.rs`:

   **X402ConnectCell** (Cell + Connect):
   ```rust
   use crate::x402::{PaymentRequest, PaymentAuthorization, VerificationStatus};
   use crate::phase2::{Address, u256};
   use crate::connector::ChainConnector;
   use roko_core::cell::{Cell, CellId};
   use roko_core::traits::Connect;
   use roko_core::error::Result;

   /// Configuration for x402 micropayments.
   #[derive(Debug, Clone)]
   pub struct X402Config {
       /// Maximum amount per single request (safety limit).
       pub max_per_request_amount: u256,
       /// Supported token contract addresses.
       pub supported_tokens: Vec<Address>,
   }

   pub struct X402ConnectCell {
       id: CellId,
       config: X402Config,
       connector: Option<Arc<ChainConnector>>,
       /// Wallet address for balance checks.
       wallet_address: String,
       /// Simulated balance for testing.
       balance: std::sync::atomic::AtomicU64,
   }

   impl X402ConnectCell {
       pub fn new(id: CellId, config: X402Config) -> Self { ... }

       /// Parse payment terms from a 402 response (domain method, NOT on Connect trait).
       pub fn parse_payment_request(&self, headers: &std::collections::HashMap<String, String>) -> Option<PaymentRequest> { ... }

       /// Construct ERC-3009 transferWithAuthorization calldata (domain method).
       pub fn build_authorization(&self, request: &PaymentRequest, from: &Address) -> PaymentAuthorization { ... }

       /// Verify a payment authorization (domain method).
       pub fn verify_authorization(&self, auth: &PaymentAuthorization, request: &PaymentRequest) -> VerificationStatus { ... }
   }
   ```
   - Cell: `cell_name` = "x402-payment", `protocols` = `&["Connect"]`
   - Connect: `connect()` -> no-op (stateless protocol), returns Ok(()). `health()` -> checks balance > 0 (returns true if wallet has funds). `disconnect()` -> no-op, returns Ok(()).

   **StateChannelConnectCell** (Cell + Connect):
   ```rust
   use crate::phase2::{AgentPaymentChannel, ChannelParty};
   use crate::x402::ChannelLifecycle;

   /// Configuration for state channels.
   #[derive(Debug, Clone)]
   pub struct StateChannelConfig {
       /// Initial channel deposit in KORAI base units.
       pub channel_deposit: u256,
       /// Channel duration in blocks.
       pub channel_duration_blocks: u64,
       /// Counterparty address.
       pub counterparty_address: Address,
   }

   /// Off-chain state update (no gas cost).
   #[derive(Debug, Clone)]
   pub struct StateUpdate {
       /// Channel nonce (monotonically increasing).
       pub nonce: u64,
       /// Balance for party A after this update.
       pub balance_a: u256,
       /// Balance for party B after this update.
       pub balance_b: u256,
   }

   pub struct StateChannelConnectCell {
       id: CellId,
       config: StateChannelConfig,
       connector: Option<Arc<ChainConnector>>,
       /// Current channel state.
       channel: parking_lot::RwLock<Option<AgentPaymentChannel>>,
       /// Off-chain state updates (signed by both parties).
       state_updates: parking_lot::RwLock<Vec<StateUpdate>>,
   }

   impl StateChannelConnectCell {
       pub fn new(id: CellId, config: StateChannelConfig) -> Self { ... }

       /// Open channel (domain method -- would submit on-chain deposit via ChainConnector).
       pub fn open_channel(&self) -> Result<()> { ... }

       /// Off-chain state update (domain method -- no gas, no ChainConnector call).
       pub fn update_state(&self, update: StateUpdate) -> Result<()> { ... }

       /// Cooperative close (domain method -- submit final state on-chain).
       pub fn cooperative_close(&self) -> Result<()> { ... }

       /// Get current channel balance.
       pub fn channel_balance(&self) -> Option<(u256, u256)> { ... }
   }
   ```
   - Cell: `cell_name` = "state-channel", `protocols` = `&["Connect"]`
   - Connect: `connect()` -> calls `open_channel()` (on-chain deposit). `health()` -> checks channel lifecycle is `ChannelLifecycle::Open` and balance > 0 and blocks remaining > challenge_window. `disconnect()` -> calls `cooperative_close()` (on-chain final state submission).
   - Channel lifecycle: `Open -> Closing { close_requested_at } -> Closed` (or `Disputed`)

   **PaymentRouteCell** (Cell + Route):
   ```rust
   /// Configuration for payment routing.
   #[derive(Debug, Clone)]
   pub struct PaymentRouteConfig {
       /// Interaction count threshold: below this use x402, above use state channel.
       pub interaction_threshold: u32,
       /// Value threshold per interaction (below = x402, above = state channel).
       pub value_threshold: u256,
   }

   impl Default for PaymentRouteConfig {
       fn default() -> Self {
           Self {
               interaction_threshold: 5,
               value_threshold: 1000,
           }
       }
   }

   pub struct PaymentRouteCell {
       id: CellId,
       config: PaymentRouteConfig,
   }
   ```
   - Cell: `cell_name` = "payment-route", `protocols` = `&["Route"]`
   - Route: `select(candidates, ctx)` reads expected interaction count and value-per-interaction from context metadata:
     - Expected interactions < threshold -> select x402 (index 0)
     - Expected interactions >= threshold -> select state channel (index 1)
     - High value per interaction -> prefer state channel for gas savings

5. Add module to lib.rs:
   ```rust
   pub mod payment_cells;
   pub use payment_cells::{
       X402ConnectCell, X402Config,
       StateChannelConnectCell, StateChannelConfig, StateUpdate,
       PaymentRouteCell, PaymentRouteConfig,
   };
   ```

6. Write tests:
   - X402ConnectCell.build_authorization() constructs correct ERC-3009 fields (from, to, value match request)
   - X402ConnectCell.health() returns false when balance is zero
   - X402ConnectCell.verify_authorization() returns `Valid` for correct auth, `AmountMismatch` for wrong value
   - StateChannelConnectCell lifecycle: connect (open) -> update_state (off-chain) -> disconnect (close)
   - StateChannelConnectCell off-chain state updates do not require ChainConnector (no on-chain calls)
   - StateChannelConnectCell.health() returns false after disconnect
   - PaymentRouteCell selects x402 (index 0) for < 5 interactions, state channel (index 1) for >= 5

## Verification
```bash
cargo check -p roko-chain
cargo clippy -p roko-chain --no-deps -- -D warnings
cargo test -p roko-chain -- payment_cells
```

## What NOT to do
- Do NOT replace existing x402.rs -- wrap its types (PaymentRequest, PaymentAuthorization) in the Cell interface
- Do NOT add query()/execute() to the Connect trait -- payment operations are inherent methods
- Do NOT implement real ERC-3009 signature verification -- mock the wallet signing
- Do NOT implement Superfluid streaming (third payment protocol) -- defer to future batch
- Do NOT add Ethereum cryptography dependencies -- use mock signatures for tests
