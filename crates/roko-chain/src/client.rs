//! The [`ChainClient`] trait: read-only access to an EVM-compatible chain.
//!
//! Implementations may wrap a JSON-RPC provider (alloy), an in-process fork
//! simulator (mirage-rs), or an in-memory mock ([`crate::MockChainClient`]).

use crate::types::{
    BlockNumber, CallResult, ChainHeader, ChainResult, LogEntry, Receipt, TxHash, TxRequest,
};
use async_trait::async_trait;

/// Read-only view of an EVM-compatible chain.
///
/// Mirrors the §33.4.1 parity-checklist contract: blocks, receipts, logs,
/// storage, and `eth_call` simulation. Writes live on [`crate::ChainWallet`];
/// this trait deliberately has no signing surface.
///
/// # Concurrency
///
/// All methods are `async` and implementations are `Send + Sync`. Clients
/// are expected to be cheap to clone / share via `Arc<dyn ChainClient>`.
#[async_trait]
pub trait ChainClient: Send + Sync {
    /// Current chain-tip block number.
    async fn block_number(&self) -> ChainResult<BlockNumber>;

    /// Fetch a block header by number.
    async fn get_block_header(&self, number: BlockNumber) -> ChainResult<ChainHeader>;

    /// Look up a transaction receipt by hash. `Ok(None)` if not yet mined.
    async fn get_receipt(&self, tx: &TxHash) -> ChainResult<Option<Receipt>>;

    /// Fetch logs in `[from, to]` (inclusive) filtered by address + topic.
    ///
    /// An empty `addresses` slice means "any address"; an empty `topics`
    /// slice means "any topic[0]".
    async fn get_logs(
        &self,
        from: BlockNumber,
        to: BlockNumber,
        addresses: &[String],
        topics: &[String],
    ) -> ChainResult<Vec<LogEntry>>;

    /// Raw storage slot read. `block == None` means "latest".
    async fn get_storage_at(
        &self,
        address: &str,
        slot: &str,
        block: Option<BlockNumber>,
    ) -> ChainResult<Vec<u8>>;

    /// Simulate a call against the given block (or latest if `None`).
    async fn eth_call(
        &self,
        request: &TxRequest,
        block: Option<BlockNumber>,
    ) -> ChainResult<CallResult>;

    /// EVM chain id.
    async fn chain_id(&self) -> ChainResult<u64>;

    /// Human-readable backend name (for logs/metrics).
    fn name(&self) -> &str;
}
