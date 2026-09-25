//! RAG-10: Retrieval outcome telemetry.
//!
//! Records each retrieval attempt alongside its gate-pass correlation so the
//! learning system can compare strategy effectiveness over time.
//!
//! # File layout
//!
//! Records are appended to `.roko/learn/retrieval-outcomes.jsonl`. Each line is
//! one [`RetrievalOutcomeRecord`] serialised as JSON.  Malformed lines are
//! silently skipped on read.
//!
//! # Retrieval strategies
//!
//! Three named strategies are tracked (matching RAG-11 experiment arms):
//! - `"hdc-only"` — HDC vector cosine search only.
//! - `"keyword"` — lexical keyword overlap scoring.
//! - `"hybrid"` — keyword scoring followed by HDC re-ranking.
//!
//! The strategy name is copied verbatim from whatever label the caller uses;
//! the store does not enforce the set.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Canonical path under `.roko/learn/` for retrieval outcome records.
pub const DEFAULT_RETRIEVAL_OUTCOMES_PATH: &str = ".roko/learn/retrieval-outcomes.jsonl";

/// Schema version stamped on every record for forward-compatibility.
pub const RETRIEVAL_OUTCOME_SCHEMA_VERSION: u32 = 1;

/// Lexical keyword overlap retrieval strategy.
pub const STRATEGY_KEYWORD: &str = "keyword";
/// HDC vector cosine search only (no keyword scoring).
pub const STRATEGY_HDC_ONLY: &str = "hdc-only";
/// Keyword scoring followed by HDC re-ranking.
pub const STRATEGY_HYBRID: &str = "hybrid";

/// All valid strategy names in a stable slice (used by the experiment arm table).
pub const KNOWN_STRATEGIES: &[&str] = &[STRATEGY_KEYWORD, STRATEGY_HDC_ONLY, STRATEGY_HYBRID];

// ─── Record ─────────────────────────────────────────────────────────────────

/// One immutable observation of a retrieval attempt and its downstream outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalOutcomeRecord {
    /// JSON schema version for forward-compatibility.
    pub schema_version: u32,

    /// Milliseconds since the Unix epoch when the retrieval completed.
    pub timestamp_ms: u64,

    /// Plan identifier owning the task that triggered the retrieval.
    pub plan_id: String,

    /// Task identifier within the plan.
    pub task_id: String,

    /// Free-text query derived from the task title and description.
    pub query: String,

    /// Strategy used: one of `"keyword"`, `"hdc-only"`, or `"hybrid"`.
    pub strategy: String,

    /// Number of knowledge entries returned by the retrieval.
    pub results_count: usize,

    /// Whether the subsequent gate(s) passed.
    ///
    /// `None` is written immediately after retrieval (before gate results are
    /// known). A follow-up record with `Some(true/false)` is appended once the
    /// task's verify steps complete. Consumers that want the settled correlation
    /// should prefer the last record for each `(plan_id, task_id)` pair.
    pub gate_passed: Option<bool>,

    /// Optional experiment assignment id linking this observation to a RAG-11
    /// retrieval strategy A/B experiment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experiment_assignment_id: Option<String>,

    /// Milliseconds taken to perform the retrieval (prompt assembly phase).
    ///
    /// Measures from immediately before `dispatcher().plan()` to immediately
    /// after it returns, covering prompt building and neuro knowledge lookup.
    /// Absent when the timing was not captured (e.g., legacy paths).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

impl RetrievalOutcomeRecord {
    /// Construct a pre-gate record (gate outcome not yet known).
    #[must_use]
    pub fn pre_gate(
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        query: impl Into<String>,
        strategy: impl Into<String>,
        results_count: usize,
    ) -> Self {
        Self {
            schema_version: RETRIEVAL_OUTCOME_SCHEMA_VERSION,
            timestamp_ms: current_timestamp_ms(),
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            query: query.into(),
            strategy: strategy.into(),
            results_count,
            gate_passed: None,
            experiment_assignment_id: None,
            latency_ms: None,
        }
    }

    /// Construct the post-gate settlement record for a previous pre-gate
    /// observation. Carries the same metadata but fills `gate_passed`.
    #[must_use]
    pub fn settled(
        plan_id: impl Into<String>,
        task_id: impl Into<String>,
        query: impl Into<String>,
        strategy: impl Into<String>,
        results_count: usize,
        gate_passed: bool,
    ) -> Self {
        Self {
            schema_version: RETRIEVAL_OUTCOME_SCHEMA_VERSION,
            timestamp_ms: current_timestamp_ms(),
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            query: query.into(),
            strategy: strategy.into(),
            results_count,
            gate_passed: Some(gate_passed),
            experiment_assignment_id: None,
            latency_ms: None,
        }
    }

    /// Attach an experiment assignment id.
    #[must_use]
    pub fn with_experiment_assignment(mut self, id: impl Into<String>) -> Self {
        self.experiment_assignment_id = Some(id.into());
        self
    }

    /// Attach the retrieval latency in milliseconds.
    #[must_use]
    pub fn with_latency_ms(mut self, latency_ms: u64) -> Self {
        self.latency_ms = Some(latency_ms);
        self
    }
}

fn current_timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ─── Store ──────────────────────────────────────────────────────────────────

/// Append-only JSONL store for retrieval outcome records.
///
/// Backed by a single `.roko/learn/retrieval-outcomes.jsonl` file.  Each call
/// to [`Self::append`] writes one line atomically (or as close as the OS
/// allows without `O_APPEND` + `fsync` semantics).
#[derive(Debug, Clone)]
pub struct RetrievalOutcomeStore {
    path: PathBuf,
    fsync: bool,
}

impl RetrievalOutcomeStore {
    /// Create a store pointing at `path`.  Parent directories are not created
    /// automatically; use [`Self::open_creating`] when the directory may not
    /// exist yet.
    #[must_use]
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            fsync: true,
        }
    }

    /// Create parent directories and return a store at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if parent directory creation fails.
    pub async fn open_creating(path: impl Into<PathBuf>) -> io::Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(Self { path, fsync: true })
    }

    /// Disable `fsync` after writes (useful in tests and high-throughput paths).
    #[must_use]
    pub const fn without_fsync(mut self) -> Self {
        self.fsync = false;
        self
    }

    /// The path to the underlying JSONL file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append a single record as a JSON line.
    ///
    /// # Errors
    ///
    /// Returns an error for serialization or file I/O failures.
    pub async fn append(&self, record: &RetrievalOutcomeRecord) -> io::Result<()> {
        let mut line = serde_json::to_string(record)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
        line.push('\n');

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(line.as_bytes()).await?;
        if self.fsync {
            file.sync_data().await?;
        }
        Ok(())
    }

    /// Read all records from the store; malformed lines are silently skipped.
    ///
    /// # Errors
    ///
    /// Returns an error only for file open or read failures.  A missing file
    /// returns an empty vector.
    pub async fn read_all(&self) -> io::Result<Vec<RetrievalOutcomeRecord>> {
        read_retrieval_outcomes(&self.path).await
    }
}

/// Read retrieval outcome records from a JSONL file.
///
/// A missing file returns an empty vector; malformed lines are skipped.
///
/// # Errors
///
/// Returns an error only for file open/read failures.
pub async fn read_retrieval_outcomes(path: &Path) -> io::Result<Vec<RetrievalOutcomeRecord>> {
    let file = match tokio::fs::File::open(path).await {
        Ok(f) => f,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err),
    };
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut out = Vec::new();
    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<RetrievalOutcomeRecord>(trimmed) {
            out.push(record);
        }
    }
    Ok(out)
}

// ─── Aggregate helpers ───────────────────────────────────────────────────────

/// Per-strategy aggregate statistics derived from a slice of records.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RetrievalStrategyStats {
    /// Strategy label.
    pub strategy: String,
    /// Total observations (settled records only).
    pub observations: u64,
    /// Gate passes among settled records.
    pub gate_passes: u64,
    /// Gate-pass rate in `[0.0, 1.0]`.
    pub gate_pass_rate: f64,
    /// Average result count across all observations.
    pub avg_results_count: f64,
}

/// Aggregate per-strategy statistics from a slice of settled records.
///
/// Only records with `gate_passed == Some(_)` contribute to `observations`.
#[must_use]
pub fn strategy_stats(records: &[RetrievalOutcomeRecord]) -> Vec<RetrievalStrategyStats> {
    use std::collections::HashMap;

    let mut by_strategy: HashMap<&str, (u64, u64, u64)> = HashMap::new(); // (obs, passes, sum_count)
    for record in records {
        if let Some(passed) = record.gate_passed {
            let entry = by_strategy
                .entry(record.strategy.as_str())
                .or_insert((0, 0, 0));
            entry.0 += 1;
            if passed {
                entry.1 += 1;
            }
            entry.2 += record.results_count as u64;
        }
    }

    let mut stats: Vec<RetrievalStrategyStats> = by_strategy
        .into_iter()
        .map(
            |(strategy, (obs, passes, sum_count))| RetrievalStrategyStats {
                strategy: strategy.to_string(),
                observations: obs,
                gate_passes: passes,
                gate_pass_rate: if obs == 0 {
                    0.0
                } else {
                    passes as f64 / obs as f64
                },
                avg_results_count: if obs == 0 {
                    0.0
                } else {
                    sum_count as f64 / obs as f64
                },
            },
        )
        .collect();
    stats.sort_by(|a, b| {
        b.gate_pass_rate
            .partial_cmp(&a.gate_pass_rate)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.observations.cmp(&a.observations))
            .then_with(|| a.strategy.cmp(&b.strategy))
    });
    stats
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record(strategy: &str, gate_passed: Option<bool>) -> RetrievalOutcomeRecord {
        RetrievalOutcomeRecord {
            schema_version: RETRIEVAL_OUTCOME_SCHEMA_VERSION,
            timestamp_ms: 1_700_000_000_000,
            plan_id: "plan-1".into(),
            task_id: "task-1".into(),
            query: "implement auth".into(),
            strategy: strategy.to_string(),
            results_count: 3,
            gate_passed,
            experiment_assignment_id: None,
            latency_ms: None,
        }
    }

    #[test]
    fn pre_gate_record_has_no_gate_outcome() {
        let record =
            RetrievalOutcomeRecord::pre_gate("plan-1", "task-1", "query", STRATEGY_KEYWORD, 5);
        assert_eq!(record.gate_passed, None);
        assert_eq!(record.strategy, STRATEGY_KEYWORD);
        assert_eq!(record.results_count, 5);
        assert_eq!(record.schema_version, RETRIEVAL_OUTCOME_SCHEMA_VERSION);
    }

    #[test]
    fn settled_record_carries_gate_outcome() {
        let record = RetrievalOutcomeRecord::settled(
            "plan-1",
            "task-1",
            "query",
            STRATEGY_HDC_ONLY,
            2,
            true,
        );
        assert_eq!(record.gate_passed, Some(true));
        assert_eq!(record.strategy, STRATEGY_HDC_ONLY);
        assert_eq!(record.results_count, 2);
    }

    #[test]
    fn with_experiment_assignment_sets_field() {
        let record = RetrievalOutcomeRecord::pre_gate("p", "t", "q", STRATEGY_HYBRID, 0)
            .with_experiment_assignment("assign-42");
        assert_eq!(record.experiment_assignment_id, Some("assign-42".into()));
    }

    #[test]
    fn with_latency_ms_sets_field() {
        let record = RetrievalOutcomeRecord::pre_gate("p", "t", "q", STRATEGY_KEYWORD, 3)
            .with_latency_ms(42);
        assert_eq!(record.latency_ms, Some(42));
    }

    #[test]
    fn latency_ms_skipped_when_absent_in_json() {
        // latency_ms is optional — absent from JSON and absent in struct must round-trip.
        let record = sample_record(STRATEGY_KEYWORD, None);
        assert_eq!(record.latency_ms, None);
        let json = serde_json::to_string(&record).expect("serialize");
        assert!(
            !json.contains("latency_ms"),
            "absent latency_ms must not appear in JSON"
        );
        let back: RetrievalOutcomeRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.latency_ms, None);
    }

    #[test]
    fn latency_ms_present_in_json_when_set() {
        let record = RetrievalOutcomeRecord::pre_gate("p", "t", "q", STRATEGY_KEYWORD, 2)
            .with_latency_ms(123);
        let json = serde_json::to_string(&record).expect("serialize");
        assert!(
            json.contains("\"latency_ms\":123"),
            "latency_ms must appear in JSON"
        );
        let back: RetrievalOutcomeRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.latency_ms, Some(123));
    }

    #[test]
    fn strategy_stats_aggregates_correctly() {
        let records = vec![
            sample_record(STRATEGY_KEYWORD, Some(true)),
            sample_record(STRATEGY_KEYWORD, Some(true)),
            sample_record(STRATEGY_KEYWORD, Some(false)),
            sample_record(STRATEGY_HDC_ONLY, Some(true)),
            sample_record(STRATEGY_HDC_ONLY, Some(false)),
            // Pre-gate record — should NOT appear in stats.
            sample_record(STRATEGY_HYBRID, None),
        ];

        let stats = strategy_stats(&records);
        assert_eq!(stats.len(), 2, "hybrid pre-gate records excluded");

        let kw = stats
            .iter()
            .find(|s| s.strategy == STRATEGY_KEYWORD)
            .unwrap();
        assert_eq!(kw.observations, 3);
        assert_eq!(kw.gate_passes, 2);
        assert!((kw.gate_pass_rate - 2.0 / 3.0).abs() < 1e-9);

        let hdc = stats
            .iter()
            .find(|s| s.strategy == STRATEGY_HDC_ONLY)
            .unwrap();
        assert_eq!(hdc.observations, 2);
        assert_eq!(hdc.gate_passes, 1);
    }

    #[tokio::test]
    async fn store_roundtrips_jsonl() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("retrieval-outcomes.jsonl");
        let store = RetrievalOutcomeStore::open_creating(&path)
            .await
            .expect("open store")
            .without_fsync();

        let r1 = sample_record(STRATEGY_KEYWORD, None);
        let r2 = sample_record(STRATEGY_KEYWORD, Some(true));

        store.append(&r1).await.expect("append r1");
        store.append(&r2).await.expect("append r2");

        let loaded = store.read_all().await.expect("read_all");
        assert_eq!(loaded, vec![r1, r2]);
    }

    #[tokio::test]
    async fn store_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("nonexistent.jsonl");
        let store = RetrievalOutcomeStore::at(&path);
        let loaded = store.read_all().await.expect("read_all");
        assert!(loaded.is_empty());
    }

    #[test]
    fn record_serialises_and_deserialises() {
        let r = RetrievalOutcomeRecord::settled("p", "t", "q", STRATEGY_HYBRID, 4, false);
        let json = serde_json::to_string(&r).expect("serialize");
        let back: RetrievalOutcomeRecord = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(r, back);
    }
}
