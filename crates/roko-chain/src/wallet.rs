//! The [`ChainWallet`] trait: signs and submits transactions.
//!
//! A `ChainWallet` owns signing material (directly, via KMS, via Warden, …)
//! and exposes a minimal surface: address, balance, nonce, sign-and-submit,
//! and receipt polling. Richer fee / nonce management is delegated to the
//! concrete impl.

use crate::types::{BlockNumber, ChainResult, Receipt, TxHash, TxRequest};
use async_trait::async_trait;

/// Signs and submits transactions to a chain backend.
///
/// Pairs with a [`ChainClient`](crate::ChainClient) for reads. The
/// split mirrors §33.4.1 / §33.4.2 of the Roko parity checklist:
/// reads and writes are separable so tests can mock one without the other.
#[async_trait]
pub trait ChainWallet: Send + Sync {
    /// The wallet's on-chain address (hex, `0x`-prefixed).
    async fn address(&self) -> ChainResult<String>;

    /// Current balance at `block` (or latest if `None`), in wei.
    async fn balance(&self, block: Option<BlockNumber>) -> ChainResult<u128>;

    /// Next nonce the wallet will use.
    async fn nonce(&self) -> ChainResult<u64>;

    /// Sign and broadcast `tx`. Returns the resulting hash. Fields left
    /// `None` on `tx` (nonce, gas, fees) are filled in by the wallet.
    async fn sign_and_submit(&self, tx: TxRequest) -> ChainResult<TxHash>;

    /// Poll for a receipt with a hard timeout (milliseconds).
    async fn wait_for_receipt(&self, tx: &TxHash, timeout_ms: u64) -> ChainResult<Receipt>;

    /// Human-readable wallet/backend name (for logs/metrics).
    fn name(&self) -> &str;
}
