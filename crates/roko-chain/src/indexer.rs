//! Read-only registry client and restart-safe chain event indexer.
//!
//! The indexer deliberately depends on [`ChainClient`] rather than an Alloy
//! provider. It therefore works with the optional JSON-RPC backend, Mirage
//! adapters, and deterministic mocks without making chain support mandatory.
//! State is an atomically replaced JSONL journal: a checkpoint record followed
//! by normalized event records. The indexer never signs or submits a transaction.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{BlockNumber, CallResult, ChainClient, ChainError, LogEntry, TxRequest};

const INDEXER_SCHEMA_VERSION: u32 = 1;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// One configured registry contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryContract {
    /// Human-readable stable name, such as `identity` or `knowledge`.
    pub name: String,
    /// EVM contract address.
    pub address: String,
    /// Optional topic-zero allowlist. Empty means all events from the address.
    #[serde(default)]
    pub topics: Vec<String>,
}

/// Provider-neutral read client for a named set of registry contracts.
#[derive(Clone)]
pub struct RegistryClient {
    chain: Arc<dyn ChainClient>,
    contracts: Arc<HashMap<String, RegistryContract>>,
}

impl std::fmt::Debug for RegistryClient {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RegistryClient")
            .field("chain", &self.chain.name())
            .field("contracts", &self.contracts)
            .finish()
    }
}

impl RegistryClient {
    /// Build a read client, rejecting duplicate names, duplicate addresses, and
    /// empty descriptors before any network request is made.
    pub fn new(
        chain: Arc<dyn ChainClient>,
        contracts: Vec<RegistryContract>,
    ) -> Result<Self, IndexerError> {
        let contracts = validate_contracts(contracts)?;
        Ok(Self {
            chain,
            contracts: Arc::new(contracts),
        })
    }

    /// Current chain tip.
    pub async fn block_number(&self) -> Result<BlockNumber, IndexerError> {
        self.chain.block_number().await.map_err(IndexerError::Chain)
    }

    /// Execute a read-only contract call by configured registry name.
    pub async fn call(
        &self,
        contract: &str,
        calldata: Vec<u8>,
        block: Option<BlockNumber>,
    ) -> Result<CallResult, IndexerError> {
        let contract = self
            .contracts
            .get(contract)
            .ok_or_else(|| IndexerError::UnknownContract(contract.to_owned()))?;
        self.chain
            .eth_call(
                &TxRequest {
                    to: Some(contract.address.clone()),
                    data: calldata,
                    ..TxRequest::default()
                },
                block,
            )
            .await
            .map_err(IndexerError::Chain)
    }

    /// Fetch raw registry logs over an explicit inclusive block range.
    pub async fn logs(
        &self,
        from: BlockNumber,
        to: BlockNumber,
    ) -> Result<Vec<LogEntry>, IndexerError> {
        if to < from {
            return Err(IndexerError::InvalidRange { from, to });
        }
        let addresses = self
            .contracts
            .values()
            .map(|contract| contract.address.clone())
            .collect::<Vec<_>>();
        let topics = backend_topics(self.contracts());
        let logs = self
            .chain
            .get_logs(from, to, &addresses, &topics)
            .await
            .map_err(IndexerError::Chain)?;
        Ok(logs
            .into_iter()
            .filter(|log| {
                contract_for_log(self.contracts(), log).is_some_and(|contract| {
                    contract.topics.is_empty()
                        || log.topics.first().is_some_and(|topic| {
                            contract
                                .topics
                                .iter()
                                .any(|allowed| allowed.eq_ignore_ascii_case(topic))
                        })
                })
            })
            .collect())
    }

    fn chain(&self) -> &dyn ChainClient {
        self.chain.as_ref()
    }

    fn contracts(&self) -> &HashMap<String, RegistryContract> {
        self.contracts.as_ref()
    }
}

/// Event-indexer persistence and finality settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventIndexerConfig {
    /// Registry contracts to index.
    pub contracts: Vec<RegistryContract>,
    /// Atomic JSONL journal location.
    pub store_path: PathBuf,
    /// First block to index when no journal exists.
    pub start_block: BlockNumber,
    /// Number of tip blocks withheld from indexing for finality.
    pub finality_confirmations: u64,
    /// Maximum blocks processed by one sync operation.
    pub max_batch_size: u64,
    /// Maximum normalized events retained in memory and the local journal.
    pub max_retained_events: usize,
}

/// One normalized registry event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedEvent {
    /// Monotonic journal sequence.
    pub sequence: u64,
    /// Configured contract name.
    pub contract: String,
    /// Emitting address.
    pub contract_address: String,
    /// Topic-zero hash, or `untyped` for a log without topics.
    pub event_type: String,
    /// Block containing the event.
    pub block_number: BlockNumber,
    /// Hash of the containing block.
    pub block_hash: String,
    /// Containing block timestamp.
    pub timestamp: u64,
    /// Stable index within the filtered logs returned for this block.
    pub log_index: u64,
    /// All indexed log topics.
    pub topics: Vec<String>,
    /// Non-indexed event bytes as lowercase hexadecimal.
    pub data_hex: String,
}

/// Durable progress marker committed atomically with the event journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexerCheckpoint {
    /// Next block that has not yet been indexed.
    pub next_block: BlockNumber,
    /// Last committed block number, if any.
    pub last_indexed_block: Option<BlockNumber>,
    /// Hash of the last committed block, used for parent-link validation.
    pub last_block_hash: Option<String>,
    /// Sequence assigned to the next event.
    pub next_sequence: u64,
}

/// Health projection returned by API adapters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexerStatus {
    /// Chain tip observed by the caller.
    pub chain_tip: Option<BlockNumber>,
    /// Last block durably indexed.
    pub last_indexed_block: Option<BlockNumber>,
    /// Number of blocks between tip and the checkpoint.
    pub lag_blocks: Option<u64>,
    /// Number of retained events.
    pub event_count: usize,
    /// Whether a chain adapter is configured.
    pub connected: bool,
}

/// Summary of one bounded sync operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncOutcome {
    /// First block considered, if the finality window permitted work.
    pub from_block: Option<BlockNumber>,
    /// Last committed block, if any.
    pub to_block: Option<BlockNumber>,
    /// Number of new events committed.
    pub indexed_events: usize,
    /// Finalized chain tip used for the operation.
    pub finalized_tip: Option<BlockNumber>,
}

/// Query filters for retained normalized events.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventQuery {
    /// Match one configured contract name.
    pub contract: Option<String>,
    /// Match one topic-zero event type.
    pub event_type: Option<String>,
    /// Inclusive lower block bound.
    pub from_block: Option<BlockNumber>,
    /// Inclusive upper block bound.
    pub to_block: Option<BlockNumber>,
    /// Maximum results after filtering.
    pub limit: usize,
}

/// Registry client and indexer failures.
#[derive(Debug, Error)]
pub enum IndexerError {
    /// Underlying chain operation failed.
    #[error("chain registry operation failed: {0}")]
    Chain(ChainError),
    /// Journal I/O failed.
    #[error("indexer storage failed: {0}")]
    Io(#[from] std::io::Error),
    /// Journal JSON failed validation or decoding.
    #[error("invalid indexer journal: {0}")]
    InvalidJournal(String),
    /// Contract configuration is invalid.
    #[error("invalid registry contract: {0}")]
    InvalidContract(String),
    /// A named contract was not configured.
    #[error("registry contract '{0}' is not configured")]
    UnknownContract(String),
    /// A read requested an inverted range.
    #[error("invalid block range {from}..={to}")]
    InvalidRange {
        /// Inclusive start.
        from: BlockNumber,
        /// Inclusive end.
        to: BlockNumber,
    },
    /// A block did not extend the committed parent hash.
    #[error(
        "chain reorganization detected at block {block}: expected parent {expected}, got {actual}"
    )]
    Reorg {
        /// Block whose parent did not match.
        block: BlockNumber,
        /// Previously committed hash.
        expected: String,
        /// Actual parent hash.
        actual: String,
    },
    /// The observed tip moved behind already committed progress.
    #[error("chain tip {tip} is behind committed block {last}")]
    ChainRewind {
        /// Latest block reported by the backend.
        tip: BlockNumber,
        /// Last block committed by the indexer.
        last: BlockNumber,
    },
    /// Previously committed progress is no longer inside the finality window.
    #[error("finalized tip {finalized_tip:?} is behind committed block {last}")]
    FinalityRegression {
        /// Finalized tip under the current configuration, if one exists.
        finalized_tip: Option<BlockNumber>,
        /// Last block committed by the indexer.
        last: BlockNumber,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "record", content = "value", rename_all = "snake_case")]
enum PersistedRecord {
    Checkpoint(PersistedCheckpoint),
    Event(IndexedEvent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedCheckpoint {
    schema_version: u32,
    start_block: BlockNumber,
    contract_set_hash: String,
    checkpoint: IndexerCheckpoint,
    blocks: Vec<IndexedBlock>,
    content_hash: String,
}

/// Header evidence retained for every block that owns a retained event, plus
/// the latest checkpoint block. This lets journal loading reject event records
/// whose block number or hash was altered independently of the checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct IndexedBlock {
    number: BlockNumber,
    hash: String,
    parent: String,
    timestamp: u64,
}

/// Read-only registry event indexer with crash-consistent local state.
pub struct EventIndexer {
    registry: RegistryClient,
    config: EventIndexerConfig,
    checkpoint: IndexerCheckpoint,
    events: Vec<IndexedEvent>,
    blocks: BTreeMap<BlockNumber, IndexedBlock>,
}

impl std::fmt::Debug for EventIndexer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EventIndexer")
            .field("registry", &self.registry)
            .field("config", &self.config)
            .field("checkpoint", &self.checkpoint)
            .field("event_count", &self.events.len())
            .field("block_evidence_count", &self.blocks.len())
            .finish()
    }
}

impl EventIndexer {
    /// Open or initialize an indexer. Existing journals are validated fully;
    /// corruption fails closed instead of silently discarding history.
    pub fn open(
        chain: Arc<dyn ChainClient>,
        config: EventIndexerConfig,
    ) -> Result<Self, IndexerError> {
        validate_indexer_config(&config)?;
        let registry = RegistryClient::new(chain, config.contracts.clone())?;
        let (checkpoint, events, blocks) = load_journal(
            &config.store_path,
            config.start_block,
            registry.contracts(),
            config.max_retained_events,
        )?;
        Ok(Self {
            registry,
            config,
            checkpoint,
            events,
            blocks,
        })
    }

    /// Explicitly replace a derived journal and return a ready indexer.
    ///
    /// Unlike [`Self::open`], this does not read the prior journal. Callers
    /// should expose it only behind an intentional administrative rebuild
    /// action; startup must continue to fail closed on corrupt state.
    pub fn rebuild_open(
        chain: Arc<dyn ChainClient>,
        config: EventIndexerConfig,
    ) -> Result<Self, IndexerError> {
        validate_indexer_config(&config)?;
        let registry = RegistryClient::new(chain, config.contracts.clone())?;
        let checkpoint = initial_checkpoint(config.start_block);
        let events = Vec::new();
        let blocks = BTreeMap::new();
        persist_journal(
            &config.store_path,
            config.start_block,
            registry.contracts(),
            &checkpoint,
            &events,
            &blocks,
        )?;
        Ok(Self {
            registry,
            config,
            checkpoint,
            events,
            blocks,
        })
    }

    /// Synchronize at most one configured batch, withholding unfinalized tip blocks.
    #[allow(clippy::too_many_lines)]
    pub async fn sync_once(&mut self) -> Result<SyncOutcome, IndexerError> {
        let chain_tip = self.registry.block_number().await?;
        if let Some(last) = self.checkpoint.last_indexed_block
            && chain_tip < last
        {
            return Err(IndexerError::ChainRewind {
                tip: chain_tip,
                last,
            });
        }
        let finalized_tip = chain_tip.checked_sub(self.config.finality_confirmations);
        if let Some(last) = self.checkpoint.last_indexed_block
            && finalized_tip.is_none_or(|tip| tip < last)
        {
            return Err(IndexerError::FinalityRegression {
                finalized_tip,
                last,
            });
        }
        let Some(finalized_tip) = finalized_tip else {
            return Ok(SyncOutcome {
                from_block: None,
                to_block: None,
                indexed_events: 0,
                finalized_tip: None,
            });
        };
        let from = self.checkpoint.next_block;
        if from > finalized_tip {
            return Ok(SyncOutcome {
                from_block: None,
                to_block: None,
                indexed_events: 0,
                finalized_tip: Some(finalized_tip),
            });
        }
        let to =
            finalized_tip.min(from.saturating_add(self.config.max_batch_size.saturating_sub(1)));

        let mut checkpoint = self.checkpoint.clone();
        // Retain only the tail that can be committed. A single backend call
        // still returns one block of logs, but an event-heavy batch must not
        // allocate a second unbounded collection before retention is applied.
        let mut pending = VecDeque::new();
        let mut indexed_events = 0_usize;
        let mut blocks = self.blocks.clone();
        let addresses = self
            .registry
            .contracts()
            .values()
            .map(|contract| contract.address.clone())
            .collect::<Vec<_>>();
        let topics = backend_topics(self.registry.contracts());

        for block in from..=to {
            let header = self
                .registry
                .chain()
                .get_block_header(block)
                .await
                .map_err(IndexerError::Chain)?;
            if header.number != block {
                return Err(IndexerError::InvalidJournal(format!(
                    "backend returned block {} for request {block}",
                    header.number
                )));
            }
            if header.hash.trim().is_empty() || (block > 0 && header.parent.trim().is_empty()) {
                return Err(IndexerError::InvalidJournal(format!(
                    "backend returned incomplete header for block {block}"
                )));
            }
            if let Some(expected) = checkpoint.last_block_hash.as_deref()
                && header.parent != expected
            {
                return Err(IndexerError::Reorg {
                    block,
                    expected: expected.to_owned(),
                    actual: header.parent,
                });
            }
            let logs = self
                .registry
                .chain()
                .get_logs(block, block, &addresses, &topics)
                .await
                .map_err(IndexerError::Chain)?;
            let mut block_has_event = false;
            for (log_index, log) in logs.into_iter().enumerate() {
                let Some(contract) = contract_for_log(self.registry.contracts(), &log) else {
                    continue;
                };
                if !contract.topics.is_empty()
                    && !log.topics.first().is_some_and(|topic| {
                        contract
                            .topics
                            .iter()
                            .any(|allowed| allowed.eq_ignore_ascii_case(topic))
                    })
                {
                    continue;
                }
                if log.topics.iter().any(|topic| !looks_like_topic(topic)) {
                    return Err(IndexerError::InvalidJournal(format!(
                        "backend returned malformed topic for block {block}"
                    )));
                }
                let event = IndexedEvent {
                    sequence: checkpoint.next_sequence,
                    contract: contract.name.clone(),
                    contract_address: log.address,
                    event_type: log
                        .topics
                        .first()
                        .cloned()
                        .unwrap_or_else(|| "untyped".to_owned()),
                    block_number: block,
                    block_hash: header.hash.clone(),
                    timestamp: header.timestamp,
                    log_index: u64::try_from(log_index).unwrap_or(u64::MAX),
                    topics: log.topics,
                    data_hex: encode_hex(&log.data),
                };
                if pending.len() == self.config.max_retained_events {
                    pending.pop_front();
                }
                pending.push_back(event);
                indexed_events = indexed_events.saturating_add(1);
                block_has_event = true;
                checkpoint.next_sequence =
                    checkpoint.next_sequence.checked_add(1).ok_or_else(|| {
                        IndexerError::InvalidJournal("event sequence overflow".to_owned())
                    })?;
            }
            checkpoint.last_indexed_block = Some(block);
            checkpoint.last_block_hash = Some(header.hash);
            checkpoint.next_block = block.checked_add(1).ok_or_else(|| {
                IndexerError::InvalidJournal("block checkpoint overflow".to_owned())
            })?;
            if block_has_event || block == to {
                blocks.insert(
                    block,
                    IndexedBlock {
                        number: block,
                        hash: checkpoint
                            .last_block_hash
                            .clone()
                            .expect("hash assigned above"),
                        parent: header.parent,
                        timestamp: header.timestamp,
                    },
                );
            }
        }

        let mut events = self.events.clone();
        events.extend(pending);
        if events.len() > self.config.max_retained_events {
            let remove = events.len() - self.config.max_retained_events;
            events.drain(..remove);
        }
        let retained_event_blocks = events
            .iter()
            .map(|event| event.block_number)
            .collect::<std::collections::HashSet<_>>();
        blocks.retain(|block, _| {
            retained_event_blocks.contains(block) || Some(*block) == checkpoint.last_indexed_block
        });
        persist_journal(
            &self.config.store_path,
            self.config.start_block,
            self.registry.contracts(),
            &checkpoint,
            &events,
            &blocks,
        )?;
        self.checkpoint = checkpoint;
        self.events = events;
        self.blocks = blocks;
        Ok(SyncOutcome {
            from_block: Some(from),
            to_block: Some(to),
            indexed_events,
            finalized_tip: Some(finalized_tip),
        })
    }

    /// Reset durable progress to the configured start block.
    pub fn rebuild(&mut self) -> Result<(), IndexerError> {
        let checkpoint = initial_checkpoint(self.config.start_block);
        persist_journal(
            &self.config.store_path,
            self.config.start_block,
            self.registry.contracts(),
            &checkpoint,
            &[],
            &BTreeMap::new(),
        )?;
        self.checkpoint = checkpoint;
        self.events.clear();
        self.blocks.clear();
        Ok(())
    }

    /// Return a filtered, sequence-ordered event page.
    #[must_use]
    pub fn query(&self, query: &EventQuery) -> Vec<IndexedEvent> {
        let limit = if query.limit == 0 {
            100
        } else {
            query.limit.min(1_000)
        };
        self.events
            .iter()
            .filter(|event| {
                query
                    .contract
                    .as_ref()
                    .is_none_or(|contract| &event.contract == contract)
                    && query
                        .event_type
                        .as_ref()
                        .is_none_or(|event_type| &event.event_type == event_type)
                    && query
                        .from_block
                        .is_none_or(|from| event.block_number >= from)
                    && query.to_block.is_none_or(|to| event.block_number <= to)
            })
            .take(limit)
            .cloned()
            .collect()
    }

    /// Current health projection using an optional freshly queried chain tip.
    #[must_use]
    pub fn status(&self, chain_tip: Option<BlockNumber>) -> IndexerStatus {
        IndexerStatus {
            chain_tip,
            last_indexed_block: self.checkpoint.last_indexed_block,
            lag_blocks: chain_tip.and_then(|tip| {
                self.checkpoint
                    .last_indexed_block
                    .map_or_else(|| Some(tip.saturating_add(1)), |last| tip.checked_sub(last))
            }),
            event_count: self.events.len(),
            connected: true,
        }
    }

    /// Borrow the durable checkpoint.
    #[must_use]
    pub fn checkpoint(&self) -> &IndexerCheckpoint {
        &self.checkpoint
    }
}

fn validate_indexer_config(config: &EventIndexerConfig) -> Result<(), IndexerError> {
    if config.max_batch_size == 0 || config.max_retained_events == 0 {
        return Err(IndexerError::InvalidJournal(
            "max_batch_size and max_retained_events must be nonzero".to_owned(),
        ));
    }
    Ok(())
}

fn validate_contracts(
    contracts: Vec<RegistryContract>,
) -> Result<HashMap<String, RegistryContract>, IndexerError> {
    if contracts.is_empty() {
        return Err(IndexerError::InvalidContract(
            "at least one registry contract is required".to_owned(),
        ));
    }
    let mut by_name = HashMap::with_capacity(contracts.len());
    let mut by_address = HashMap::<String, String>::with_capacity(contracts.len());
    for mut contract in contracts {
        contract.name = contract.name.trim().to_owned();
        contract.address = contract.address.trim().to_ascii_lowercase();
        contract.topics = contract
            .topics
            .into_iter()
            .map(|topic| topic.trim().to_ascii_lowercase())
            .collect();
        if contract.name.is_empty()
            || !looks_like_address(&contract.address)
            || contract.topics.iter().any(|topic| !looks_like_topic(topic))
        {
            return Err(IndexerError::InvalidContract(format!(
                "{} at {}",
                contract.name, contract.address
            )));
        }
        if by_address
            .insert(contract.address.clone(), contract.name.clone())
            .is_some()
            || by_name.insert(contract.name.clone(), contract).is_some()
        {
            return Err(IndexerError::InvalidContract(
                "contract names and addresses must be unique".to_owned(),
            ));
        }
    }
    Ok(by_name)
}

fn looks_like_address(value: &str) -> bool {
    value.len() == 42
        && value.starts_with("0x")
        && value[2..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn looks_like_topic(value: &str) -> bool {
    value.len() == 66
        && value.starts_with("0x")
        && value[2..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn contract_for_log<'a>(
    contracts: &'a HashMap<String, RegistryContract>,
    log: &LogEntry,
) -> Option<&'a RegistryContract> {
    contracts
        .values()
        .find(|contract| contract.address.eq_ignore_ascii_case(&log.address))
}

/// A backend topic filter is a union. If any contract accepts every topic, the
/// backend must also receive an empty filter; the per-contract allowlists are
/// enforced locally after the broad query.
fn backend_topics(contracts: &HashMap<String, RegistryContract>) -> Vec<String> {
    if contracts
        .values()
        .any(|contract| contract.topics.is_empty())
    {
        return Vec::new();
    }
    let mut topics = contracts
        .values()
        .flat_map(|contract| contract.topics.iter().cloned())
        .collect::<Vec<_>>();
    topics.sort();
    topics.dedup();
    topics
}

fn initial_checkpoint(start_block: BlockNumber) -> IndexerCheckpoint {
    IndexerCheckpoint {
        next_block: start_block,
        last_indexed_block: None,
        last_block_hash: None,
        next_sequence: 0,
    }
}

#[allow(clippy::too_many_lines)]
fn load_journal(
    path: &Path,
    start_block: BlockNumber,
    contracts: &HashMap<String, RegistryContract>,
    max_retained_events: usize,
) -> Result<
    (
        IndexerCheckpoint,
        Vec<IndexedEvent>,
        BTreeMap<BlockNumber, IndexedBlock>,
    ),
    IndexerError,
> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((initial_checkpoint(start_block), Vec::new(), BTreeMap::new()));
        }
        Err(error) => return Err(error.into()),
    };
    let mut records = BufReader::new(file).lines();
    let first = records
        .next()
        .ok_or_else(|| IndexerError::InvalidJournal("journal is empty".to_owned()))??;
    let (recorded_start_block, recorded_contract_set_hash, checkpoint, blocks, content_hash) =
        match serde_json::from_str::<PersistedRecord>(&first)
            .map_err(|error| IndexerError::InvalidJournal(error.to_string()))?
        {
            PersistedRecord::Checkpoint(record)
                if record.schema_version == INDEXER_SCHEMA_VERSION =>
            {
                let block_count = record.blocks.len();
                let blocks = record
                    .blocks
                    .into_iter()
                    .map(|block| (block.number, block))
                    .collect::<BTreeMap<_, _>>();
                if blocks.len() != block_count {
                    return Err(IndexerError::InvalidJournal(
                        "duplicate block evidence".to_owned(),
                    ));
                }
                (
                    record.start_block,
                    record.contract_set_hash,
                    record.checkpoint,
                    blocks,
                    record.content_hash,
                )
            }
            PersistedRecord::Checkpoint(record) => {
                return Err(IndexerError::InvalidJournal(format!(
                    "unsupported schema {}",
                    record.schema_version
                )));
            }
            PersistedRecord::Event(_) => {
                return Err(IndexerError::InvalidJournal(
                    "first record is not a checkpoint".to_owned(),
                ));
            }
        };
    let mut events = Vec::new();
    for line in records {
        let line = line?;
        let event = match serde_json::from_str::<PersistedRecord>(&line)
            .map_err(|error| IndexerError::InvalidJournal(error.to_string()))?
        {
            PersistedRecord::Event(event) => event,
            PersistedRecord::Checkpoint(_) => {
                return Err(IndexerError::InvalidJournal(
                    "duplicate checkpoint record".to_owned(),
                ));
            }
        };
        let expected = events
            .last()
            .map_or(Ok(event.sequence), |previous: &IndexedEvent| {
                previous.sequence.checked_add(1).ok_or_else(|| {
                    IndexerError::InvalidJournal("event sequence overflow".to_owned())
                })
            })?;
        let contract_valid = contracts.get(&event.contract).is_some_and(|contract| {
            contract
                .address
                .eq_ignore_ascii_case(&event.contract_address)
                && (contract.topics.is_empty()
                    || contract
                        .topics
                        .iter()
                        .any(|topic| topic.eq_ignore_ascii_case(&event.event_type)))
        });
        let block_valid = blocks.get(&event.block_number).is_some_and(|block| {
            let same_hash = block.hash == event.block_hash;
            let same_timestamp = block.timestamp == event.timestamp;
            same_hash && same_timestamp
        });
        let topic_valid = event.topics.first().map_or_else(
            || event.event_type == "untyped",
            |topic| {
                topic.eq_ignore_ascii_case(&event.event_type)
                    && event.topics.iter().all(|topic| looks_like_topic(topic))
            },
        );
        let ordering_valid = events.last().is_none_or(|previous| {
            (event.block_number, event.log_index) > (previous.block_number, previous.log_index)
        });
        if event.sequence != expected
            || !contract_valid
            || !block_valid
            || !topic_valid
            || !ordering_valid
            || !looks_like_data_hex(&event.data_hex)
            || event.block_number >= checkpoint.next_block
            || event.sequence >= checkpoint.next_sequence
        {
            return Err(IndexerError::InvalidJournal(
                "event/checkpoint sequence mismatch".to_owned(),
            ));
        }
        events.push(event);
    }
    let expected_blocks = events
        .iter()
        .map(|event| event.block_number)
        .chain(checkpoint.last_indexed_block)
        .collect::<HashSet<_>>();
    let evidence_exact = blocks.len() == expected_blocks.len()
        && blocks.keys().all(|block| expected_blocks.contains(block));
    let evidence_links_valid = blocks
        .values()
        .zip(blocks.values().skip(1))
        .all(|(left, right)| {
            right.number != left.number.saturating_add(1)
                || right.parent.eq_ignore_ascii_case(&left.hash)
        });
    let initial_checkpoint_valid = if checkpoint.last_indexed_block.is_none() {
        checkpoint.next_block == start_block
            && checkpoint.last_block_hash.is_none()
            && checkpoint.next_sequence == 0
            && events.is_empty()
            && blocks.is_empty()
    } else {
        true
    };
    let expected_contract_set_hash = contract_configuration_hash(contracts)?;
    let configuration_valid = recorded_start_block == start_block
        && recorded_contract_set_hash == expected_contract_set_hash;
    let integrity_valid = journal_content_hash(
        recorded_start_block,
        &recorded_contract_set_hash,
        &checkpoint,
        &events,
        &blocks,
    )? == content_hash;
    if !configuration_valid
        || !initial_checkpoint_valid
        || events.len() > max_retained_events
        || events
            .last()
            .is_some_and(|event| event.sequence.checked_add(1) != Some(checkpoint.next_sequence))
        || events.is_empty() && checkpoint.next_sequence != 0
        || checkpoint
            .last_indexed_block
            .map(|block| block.saturating_add(1))
            != checkpoint.last_indexed_block.map(|_| checkpoint.next_block)
        || checkpoint.last_indexed_block.is_some_and(|last| {
            blocks
                .get(&last)
                .is_none_or(|block| Some(&block.hash) != checkpoint.last_block_hash.as_ref())
        })
        || !evidence_exact
        || !evidence_links_valid
        || !integrity_valid
        || blocks.values().any(|block| {
            block.number >= checkpoint.next_block
                || (block.number > 0 && block.parent.trim().is_empty())
                || block.hash.trim().is_empty()
        })
    {
        return Err(IndexerError::InvalidJournal(
            "checkpoint does not match journal contents".to_owned(),
        ));
    }
    Ok((checkpoint, events, blocks))
}

fn persist_journal(
    path: &Path,
    start_block: BlockNumber,
    contracts: &HashMap<String, RegistryContract>,
    checkpoint: &IndexerCheckpoint,
    events: &[IndexedEvent],
    blocks: &BTreeMap<BlockNumber, IndexedBlock>,
) -> Result<(), IndexerError> {
    let mut payload = Vec::new();
    let contract_set_hash = contract_configuration_hash(contracts)?;
    let content_hash =
        journal_content_hash(start_block, &contract_set_hash, checkpoint, events, blocks)?;
    serde_json::to_writer(
        &mut payload,
        &PersistedRecord::Checkpoint(PersistedCheckpoint {
            schema_version: INDEXER_SCHEMA_VERSION,
            start_block,
            contract_set_hash,
            checkpoint: checkpoint.clone(),
            blocks: blocks.values().cloned().collect(),
            content_hash,
        }),
    )
    .map_err(|error| IndexerError::InvalidJournal(error.to_string()))?;
    payload.push(b'\n');
    for event in events {
        serde_json::to_writer(&mut payload, &PersistedRecord::Event(event.clone()))
            .map_err(|error| IndexerError::InvalidJournal(error.to_string()))?;
        payload.push(b'\n');
    }
    durable_atomic_write(path, &payload)?;
    Ok(())
}

fn journal_content_hash(
    start_block: BlockNumber,
    contract_set_hash: &str,
    checkpoint: &IndexerCheckpoint,
    events: &[IndexedEvent],
    blocks: &BTreeMap<BlockNumber, IndexedBlock>,
) -> Result<String, IndexerError> {
    let blocks = blocks.values().collect::<Vec<_>>();
    let bytes = serde_json::to_vec(&(
        INDEXER_SCHEMA_VERSION,
        start_block,
        contract_set_hash,
        checkpoint,
        blocks,
        events,
    ))
    .map_err(|error| IndexerError::InvalidJournal(error.to_string()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn contract_configuration_hash(
    contracts: &HashMap<String, RegistryContract>,
) -> Result<String, IndexerError> {
    let mut contracts = contracts.values().collect::<Vec<_>>();
    contracts.sort_by(|left, right| left.name.cmp(&right.name));
    let bytes = serde_json::to_vec(&contracts)
        .map_err(|error| IndexerError::InvalidJournal(error.to_string()))?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

fn durable_atomic_write(path: &Path, payload: &[u8]) -> std::io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "journal has no parent")
    })?;
    std::fs::create_dir_all(parent)?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("events.jsonl");
    let temporary = parent.join(format!(
        ".{file_name}.tmp.{}.{}",
        std::process::id(),
        sequence
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(payload)?;
        file.sync_all()?;
        std::fs::rename(&temporary, path)?;
        File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::from("0x"), |mut output, byte| {
        let _ = write!(output, "{byte:02x}");
        output
    })
}

fn looks_like_data_hex(value: &str) -> bool {
    value.len() >= 2
        && value.len().is_multiple_of(2)
        && value.starts_with("0x")
        && value[2..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::sync::Mutex;

    use async_trait::async_trait;

    use super::*;
    use crate::{ChainHeader, ChainResult, Receipt, TxHash};

    const ADDRESS: &str = "0x1111111111111111111111111111111111111111";
    const ADDRESS_TWO: &str = "0x2222222222222222222222222222222222222222";
    const TOPIC: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const TOPIC_TWO: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    #[derive(Debug, Default)]
    struct TestChain {
        headers: BTreeMap<u64, ChainHeader>,
        logs: Mutex<HashMap<u64, Vec<LogEntry>>>,
        calls: Mutex<Vec<(TxRequest, Option<u64>)>>,
    }

    #[async_trait]
    impl ChainClient for TestChain {
        async fn block_number(&self) -> ChainResult<u64> {
            Ok(self.headers.keys().next_back().copied().unwrap_or(0))
        }

        async fn get_block_header(&self, number: u64) -> ChainResult<ChainHeader> {
            self.headers
                .get(&number)
                .cloned()
                .ok_or_else(|| ChainError::Rpc(format!("missing block {number}")))
        }

        async fn get_receipt(&self, _tx: &TxHash) -> ChainResult<Option<Receipt>> {
            Ok(None)
        }

        async fn get_logs(
            &self,
            from: u64,
            to: u64,
            addresses: &[String],
            topics: &[String],
        ) -> ChainResult<Vec<LogEntry>> {
            let logs = self.logs.lock().expect("logs lock");
            Ok((from..=to)
                .flat_map(|block| logs.get(&block).cloned().unwrap_or_default())
                .filter(|log| {
                    (addresses.is_empty() || addresses.iter().any(|item| item == &log.address))
                        && (topics.is_empty()
                            || log
                                .topics
                                .first()
                                .is_some_and(|topic| topics.contains(topic)))
                })
                .collect())
        }

        async fn get_storage_at(
            &self,
            _address: &str,
            _slot: &str,
            _block: Option<u64>,
        ) -> ChainResult<Vec<u8>> {
            Ok(Vec::new())
        }

        async fn eth_call(
            &self,
            request: &TxRequest,
            block: Option<u64>,
        ) -> ChainResult<CallResult> {
            self.calls
                .lock()
                .expect("calls lock")
                .push((request.clone(), block));
            Ok(CallResult {
                output: vec![1, 2, 3],
                gas_used: 42,
            })
        }

        async fn get_balance(&self, _address: &str, _block: Option<u64>) -> ChainResult<u128> {
            Ok(0)
        }

        async fn chain_id(&self) -> ChainResult<u64> {
            Ok(1)
        }

        fn name(&self) -> &str {
            "test-chain"
        }
    }

    fn chain(blocks: u64) -> Arc<TestChain> {
        let mut chain = TestChain::default();
        for number in 0..=blocks {
            chain.headers.insert(
                number,
                ChainHeader {
                    number,
                    hash: format!("0x{number:064x}"),
                    parent: if number == 0 {
                        "0x0".to_owned()
                    } else {
                        format!("0x{:064x}", number - 1)
                    },
                    timestamp: 1_000 + number,
                },
            );
        }
        Arc::new(chain)
    }

    fn config(path: PathBuf) -> EventIndexerConfig {
        EventIndexerConfig {
            contracts: vec![RegistryContract {
                name: "identity".to_owned(),
                address: ADDRESS.to_owned(),
                topics: vec![TOPIC.to_owned()],
            }],
            store_path: path,
            start_block: 0,
            finality_confirmations: 1,
            max_batch_size: 10,
            max_retained_events: 100,
        }
    }

    #[tokio::test]
    async fn sync_is_finality_aware_and_restart_durable() {
        let directory = tempfile::tempdir().expect("tempdir");
        let chain = chain(3);
        chain.logs.lock().expect("logs").insert(
            1,
            vec![LogEntry {
                address: ADDRESS.to_owned(),
                topics: vec![TOPIC.to_owned()],
                data: vec![0xde, 0xad],
            }],
        );
        let path = directory.path().join("events.jsonl");
        let erased: Arc<dyn ChainClient> = chain.clone();
        let mut indexer = EventIndexer::open(erased, config(path.clone())).expect("open");

        let outcome = indexer.sync_once().await.expect("sync");
        assert_eq!(outcome.to_block, Some(2));
        assert_eq!(outcome.indexed_events, 1);
        assert_eq!(indexer.query(&EventQuery::default())[0].data_hex, "0xdead");
        assert_eq!(indexer.checkpoint().next_block, 3);

        let erased: Arc<dyn ChainClient> = chain;
        let restored = EventIndexer::open(erased, config(path)).expect("restore");
        assert_eq!(restored.checkpoint().next_block, 3);
        assert_eq!(restored.query(&EventQuery::default()).len(), 1);
    }

    #[tokio::test]
    async fn reorg_fails_without_overwriting_committed_checkpoint() {
        let directory = tempfile::tempdir().expect("tempdir");
        let mut first = EventIndexer::open(
            chain(1),
            EventIndexerConfig {
                finality_confirmations: 0,
                ..config(directory.path().join("events.jsonl"))
            },
        )
        .expect("open");
        first.sync_once().await.expect("initial sync");
        let path = first.config.store_path.clone();
        let mut fork = TestChain::default();
        for number in 0..=2 {
            fork.headers.insert(
                number,
                ChainHeader {
                    number,
                    hash: format!("fork-{number}"),
                    parent: if number == 2 {
                        "unexpected-parent".to_owned()
                    } else if number == 0 {
                        "0x0".to_owned()
                    } else {
                        "0x0".to_owned()
                    },
                    timestamp: number,
                },
            );
        }
        let mut resumed = EventIndexer::open(
            Arc::new(fork),
            EventIndexerConfig {
                contracts: first.config.contracts.clone(),
                store_path: path,
                start_block: 0,
                finality_confirmations: 0,
                max_batch_size: 10,
                max_retained_events: 100,
            },
        )
        .expect("resume");
        let before = resumed.checkpoint().clone();
        assert!(matches!(
            resumed.sync_once().await,
            Err(IndexerError::Reorg { block: 2, .. })
        ));
        assert_eq!(resumed.checkpoint(), &before);
    }

    #[tokio::test]
    async fn tip_and_finality_regressions_fail_closed() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("events.jsonl");
        let mut first = EventIndexer::open(
            chain(2),
            EventIndexerConfig {
                finality_confirmations: 0,
                ..config(path.clone())
            },
        )
        .expect("open");
        first.sync_once().await.expect("initial sync");

        let mut rewound = EventIndexer::open(
            chain(1),
            EventIndexerConfig {
                finality_confirmations: 0,
                ..config(path.clone())
            },
        )
        .expect("rewound open");
        assert_eq!(rewound.status(Some(1)).lag_blocks, None);
        assert!(matches!(
            rewound.sync_once().await,
            Err(IndexerError::ChainRewind { tip: 1, last: 2 })
        ));

        let mut stricter_finality = EventIndexer::open(
            chain(2),
            EventIndexerConfig {
                finality_confirmations: 1,
                ..config(path)
            },
        )
        .expect("stricter-finality open");
        assert!(matches!(
            stricter_finality.sync_once().await,
            Err(IndexerError::FinalityRegression {
                finalized_tip: Some(1),
                last: 2,
            })
        ));
    }

    #[test]
    fn corrupt_journal_fails_closed() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("events.jsonl");
        std::fs::write(&path, b"not-json\n").expect("write fixture");
        assert!(matches!(
            EventIndexer::open(chain(0), config(path)),
            Err(IndexerError::InvalidJournal(_))
        ));
    }

    #[test]
    fn explicit_rebuild_recovers_a_corrupt_derived_journal() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("events.jsonl");
        std::fs::write(&path, b"not-json\n").expect("write fixture");
        let configured = config(path.clone());
        assert!(EventIndexer::open(chain(0), configured.clone()).is_err());

        let rebuilt = EventIndexer::rebuild_open(chain(0), configured.clone()).expect("rebuild");
        assert_eq!(rebuilt.checkpoint(), &initial_checkpoint(0));
        drop(rebuilt);
        EventIndexer::open(chain(0), configured).expect("reopen rebuilt journal");
    }

    #[test]
    fn journal_rejects_start_block_and_contract_configuration_drift() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("events.jsonl");
        let configured = config(path.clone());
        EventIndexer::rebuild_open(chain(0), configured.clone()).expect("initialize journal");

        let mut changed_start = configured.clone();
        changed_start.start_block = 10;
        assert!(matches!(
            EventIndexer::open(chain(0), changed_start),
            Err(IndexerError::InvalidJournal(_))
        ));

        let mut changed_contracts = configured;
        changed_contracts.contracts.push(RegistryContract {
            name: "knowledge".to_owned(),
            address: ADDRESS_TWO.to_owned(),
            topics: Vec::new(),
        });
        assert!(matches!(
            EventIndexer::open(chain(0), changed_contracts),
            Err(IndexerError::InvalidJournal(_))
        ));
    }

    #[tokio::test]
    async fn registry_client_executes_named_read_only_calls() {
        let chain = chain(0);
        let observed = chain.clone();
        let client = RegistryClient::new(
            chain,
            vec![RegistryContract {
                name: "knowledge".to_owned(),
                address: ADDRESS.to_owned(),
                topics: Vec::new(),
            }],
        )
        .expect("client");
        let result = client
            .call("knowledge", vec![0xaa], Some(7))
            .await
            .expect("call");
        assert_eq!(result.output, vec![1, 2, 3]);
        let calls = observed.calls.lock().expect("calls");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0.to.as_deref(), Some(ADDRESS));
        assert_eq!(calls[0].0.data, [0xaa]);
        assert_eq!(calls[0].1, Some(7));
        drop(calls);
        assert!(matches!(
            client.call("missing", Vec::new(), None).await,
            Err(IndexerError::UnknownContract(_))
        ));
    }

    #[tokio::test]
    async fn registry_log_query_preserves_address_topic_pairing() {
        let chain = chain(0);
        chain.logs.lock().expect("logs").insert(
            0,
            vec![
                LogEntry {
                    address: ADDRESS.to_owned(),
                    topics: vec![TOPIC.to_owned()],
                    data: vec![1],
                },
                LogEntry {
                    address: ADDRESS.to_owned(),
                    topics: vec![TOPIC_TWO.to_owned()],
                    data: vec![2],
                },
                LogEntry {
                    address: ADDRESS_TWO.to_owned(),
                    topics: vec![TOPIC_TWO.to_owned()],
                    data: vec![3],
                },
            ],
        );
        let client = RegistryClient::new(
            chain,
            vec![
                RegistryContract {
                    name: "identity".to_owned(),
                    address: ADDRESS.to_owned(),
                    topics: vec![TOPIC.to_owned()],
                },
                RegistryContract {
                    name: "knowledge".to_owned(),
                    address: ADDRESS_TWO.to_owned(),
                    topics: vec![TOPIC_TWO.to_owned()],
                },
            ],
        )
        .expect("client");
        let logs = client.logs(0, 0).await.expect("logs");
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].data, [1]);
        assert_eq!(logs[1].data, [3]);
    }

    #[tokio::test]
    async fn unrestricted_contract_disables_union_topic_filter_at_backend() {
        let chain = chain(0);
        chain.logs.lock().expect("logs").insert(
            0,
            vec![
                LogEntry {
                    address: ADDRESS.to_owned(),
                    topics: vec![TOPIC.to_owned()],
                    data: vec![1],
                },
                LogEntry {
                    address: ADDRESS_TWO.to_owned(),
                    topics: vec![TOPIC_TWO.to_owned()],
                    data: vec![2],
                },
                LogEntry {
                    address: ADDRESS.to_owned(),
                    topics: vec![TOPIC_TWO.to_owned()],
                    data: vec![3],
                },
            ],
        );
        let client = RegistryClient::new(
            chain,
            vec![
                RegistryContract {
                    name: "identity".to_owned(),
                    address: ADDRESS.to_owned(),
                    topics: vec![TOPIC.to_owned()],
                },
                RegistryContract {
                    name: "knowledge".to_owned(),
                    address: ADDRESS_TWO.to_owned(),
                    topics: Vec::new(),
                },
            ],
        )
        .expect("client");
        let logs = client.logs(0, 0).await.expect("logs");
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[1].data, [2]);
    }

    #[test]
    fn empty_contract_configuration_is_rejected() {
        assert!(matches!(
            RegistryClient::new(chain(0), Vec::new()),
            Err(IndexerError::InvalidContract(_))
        ));
    }

    #[tokio::test]
    async fn journal_retention_is_bounded_and_reloads_nonzero_sequences() {
        let directory = tempfile::tempdir().expect("tempdir");
        let chain = chain(2);
        for block in 1..=2 {
            chain.logs.lock().expect("logs").insert(
                block,
                vec![LogEntry {
                    address: ADDRESS.to_owned(),
                    topics: vec![TOPIC.to_owned()],
                    data: vec![block as u8],
                }],
            );
        }
        let path = directory.path().join("events.jsonl");
        let mut bounded = config(path.clone());
        bounded.finality_confirmations = 0;
        bounded.max_retained_events = 1;
        let erased: Arc<dyn ChainClient> = chain.clone();
        let mut indexer = EventIndexer::open(erased, bounded.clone()).expect("open");
        indexer.sync_once().await.expect("sync");
        let events = indexer.query(&EventQuery::default());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].sequence, 1);

        let erased: Arc<dyn ChainClient> = chain;
        let restored = EventIndexer::open(erased, bounded).expect("reload retained journal");
        assert_eq!(restored.query(&EventQuery::default())[0].sequence, 1);
        assert_eq!(std::fs::read_to_string(path).unwrap().lines().count(), 2);
    }

    #[tokio::test]
    async fn well_formed_event_record_tampering_fails_closed() {
        let directory = tempfile::tempdir().expect("tempdir");
        let chain = chain(1);
        chain.logs.lock().expect("logs").insert(
            1,
            vec![LogEntry {
                address: ADDRESS.to_owned(),
                topics: vec![TOPIC.to_owned()],
                data: vec![1],
            }],
        );
        let path = directory.path().join("events.jsonl");
        let mut configured = config(path.clone());
        configured.finality_confirmations = 0;
        let erased: Arc<dyn ChainClient> = chain.clone();
        let mut indexer = EventIndexer::open(erased, configured.clone()).expect("open");
        indexer.sync_once().await.expect("sync");

        let original_lines = std::fs::read_to_string(&path)
            .expect("journal")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let mut lines = original_lines.clone();
        let mut event: serde_json::Value = serde_json::from_str(&lines[1]).expect("event JSON");
        event["value"]["contract_address"] = serde_json::Value::String(ADDRESS_TWO.to_owned());
        lines[1] = serde_json::to_string(&event).expect("serialize tamper");
        std::fs::write(&path, format!("{}\n", lines.join("\n"))).expect("tamper journal");

        let erased: Arc<dyn ChainClient> = chain.clone();
        assert!(matches!(
            EventIndexer::open(erased, configured.clone()),
            Err(IndexerError::InvalidJournal(_))
        ));

        let mut lines = original_lines.clone();
        let mut event: serde_json::Value = serde_json::from_str(&lines[1]).expect("event JSON");
        event["value"]["block_hash"] = serde_json::Value::String("0xdeadbeef".to_owned());
        lines[1] = serde_json::to_string(&event).expect("serialize tamper");
        std::fs::write(&path, format!("{}\n", lines.join("\n"))).expect("tamper journal");

        let erased: Arc<dyn ChainClient> = chain;
        assert!(matches!(
            EventIndexer::open(erased.clone(), configured.clone()),
            Err(IndexerError::InvalidJournal(_))
        ));

        let mut lines = original_lines;
        let mut event: serde_json::Value = serde_json::from_str(&lines[1]).expect("event JSON");
        event["value"]["data_hex"] = serde_json::Value::String("0x02".to_owned());
        lines[1] = serde_json::to_string(&event).expect("serialize tamper");
        std::fs::write(&path, format!("{}\n", lines.join("\n"))).expect("tamper journal");
        assert!(matches!(
            EventIndexer::open(erased, configured),
            Err(IndexerError::InvalidJournal(_))
        ));
    }

    #[test]
    fn well_formed_checkpoint_cannot_skip_unindexed_blocks() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("events.jsonl");
        let mut checkpoint = initial_checkpoint(0);
        checkpoint.next_block = 99;
        let contracts = validate_contracts(config(path.clone()).contracts).expect("contracts");
        persist_journal(&path, 0, &contracts, &checkpoint, &[], &BTreeMap::new())
            .expect("journal fixture");

        assert!(matches!(
            EventIndexer::open(chain(100), config(path)),
            Err(IndexerError::InvalidJournal(_))
        ));
    }
}
