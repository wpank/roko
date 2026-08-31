//! Backend-independent chain observation records and bounded live state.
//!
//! These types deliberately do not depend on an RPC implementation. Keeping
//! them outside the optional Alloy watcher lets local registries, APIs, and
//! trigger evidence compile in Roko's lean development build.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Information about a single block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct BlockInfo {
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: u64,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub tx_count: u32,
    pub base_fee_per_gas: Option<u64>,
}

/// Information about a single transaction.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct TxInfo {
    pub block_number: u64,
    pub tx_hash: String,
    pub from: String,
    pub to: Option<String>,
    pub value_wei: String,
    pub gas_used: u64,
    pub method_sig: Option<String>,
    pub success: bool,
}

/// A decoded contract event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ContractEventInfo {
    pub block_number: u64,
    pub tx_hash: String,
    pub log_index: u32,
    pub contract: String,
    pub event_name: String,
    pub decoded: serde_json::Value,
    #[serde(default)]
    pub raw_evidence_available: bool,
}

/// Raw EVM log evidence emitted alongside the compatibility decoded event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct RawLogInfo {
    pub block_number: u64,
    pub block_hash: String,
    pub tx_hash: String,
    pub log_index: u32,
    pub contract: String,
    pub topics: Vec<String>,
    pub data: String,
}

/// Reorganization evidence emitted before replacement blocks are replayed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub struct ChainReorgInfo {
    pub orphaned_block_hashes: Vec<String>,
}

/// Recent chain state ring buffers for REST endpoints.
#[derive(Debug, Default)]
pub struct ChainState {
    /// Latest block observed.
    pub latest_block: tokio::sync::RwLock<Option<BlockInfo>>,
    /// Ring of recent blocks (last 64).
    pub recent_blocks: tokio::sync::RwLock<VecDeque<BlockInfo>>,
    /// Ring of recent transactions (last 128).
    pub recent_txs: tokio::sync::RwLock<VecDeque<TxInfo>>,
    /// Ring of recent decoded events (last 128).
    pub recent_events: tokio::sync::RwLock<VecDeque<ContractEventInfo>>,
    /// Whether the watcher background task is running.
    pub watcher_running: std::sync::atomic::AtomicBool,
}

impl ChainState {
    const MAX_BLOCKS: usize = 64;
    const MAX_TXS: usize = 128;
    const MAX_EVENTS: usize = 128;

    /// Push a new block into the ring buffer.
    pub async fn push_block(&self, block: BlockInfo) {
        *self.latest_block.write().await = Some(block.clone());
        let mut ring = self.recent_blocks.write().await;
        ring.push_back(block);
        while ring.len() > Self::MAX_BLOCKS {
            ring.pop_front();
        }
    }

    /// Push a new transaction into the ring buffer.
    pub async fn push_tx(&self, tx: TxInfo) {
        let mut ring = self.recent_txs.write().await;
        ring.push_back(tx);
        while ring.len() > Self::MAX_TXS {
            ring.pop_front();
        }
    }

    /// Push a new event into the ring buffer.
    pub async fn push_event(&self, event: ContractEventInfo) {
        let mut ring = self.recent_events.write().await;
        ring.push_back(event);
        while ring.len() > Self::MAX_EVENTS {
            ring.pop_front();
        }
    }
}
