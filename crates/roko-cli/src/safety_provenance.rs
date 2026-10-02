//! The host's safety provenance sink for Graph plan runs (gap-ff95f5).
//!
//! [`GraphProvenanceSink`] implements [`SafetyProvenanceSink`] over the
//! workspace's forensic logs. Each tool call's intent and outcome become a
//! witness vertex in `.roko/witness.jsonl` and a hash-chained custody record
//! in `.roko/custody.jsonl`, both synced to disk before the call goes on, so
//! `roko knowledge custody list` and `verify` show what a run did. The sink
//! keeps the calls' taint lineage, and [`GraphProvenanceSink::summary`] is
//! what a Graph checkpoint stores under `roko.safety-provenance@1`.
//!
//! The digest key lives in `.roko/state/safety-provenance.key`. The sink
//! creates it on first use, readable by its owner only.

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use parking_lot::{Mutex, RwLock};
use roko_agent::safety::provenance::{Custody, CustodyLogger};
use roko_agent::safety::provenance_sink::{track_intent, track_outcome};
use roko_agent::safety::{
    ProvenanceAck, ProvenanceCall, ProvenanceError, ProvenanceIntent, ProvenanceOutcome,
    ProvenanceRecord, SafetyProvenanceSink, TaintTracker, VertexKind, WitnessDag, WitnessLogger,
    WitnessVertex,
};
use roko_core::ContentHash;
use roko_core::config::fingerprint::canonical_json;
use roko_fs::RokoLayout;
use serde::{Deserialize, Serialize};

use crate::custody::{chain_violations, log_chained};

/// File in `.roko/state/` holding the digest key.
const KEY_FILE: &str = "safety-provenance.key";

/// Serializes this process's appends to the workspace logs, so that a
/// record's witness vertex and custody record go in together. Other processes
/// wait on the custody log's file lock ([`log_chained`]).
static APPEND_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// What a Graph checkpoint keeps of its run's safety provenance, under
/// `roko.safety-provenance@1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyProvenanceSummary {
    /// Provenance records the run wrote.
    pub records: u64,
    /// Hash of the run's first record in `.roko/custody.jsonl`, from which a
    /// resume verifies the run's records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custody_root: Option<String>,
    /// Id of the run's last vertex in `.roko/witness.jsonl`.
    pub witness_head: Option<ContentHash>,
    /// Hash of the run's last record in `.roko/custody.jsonl`.
    pub custody_head: Option<String>,
    /// The taint index ([`TaintTracker::to_json`]).
    pub taint: serde_json::Value,
}

/// The safety provenance sinks of the runs in flight, by run id: a dispatch
/// records its tool calls with its run's sink. Clones share one registry.
#[derive(Debug, Clone, Default)]
pub struct ProvenanceSinks {
    runs: Arc<RwLock<HashMap<String, Arc<GraphProvenanceSink>>>>,
}

impl ProvenanceSinks {
    /// Record run `run_id`'s tool calls with `sink` until the returned
    /// registration drops.
    #[must_use]
    pub fn register(&self, run_id: &str, sink: Arc<GraphProvenanceSink>) -> ProvenanceRegistration {
        self.runs.write().insert(run_id.to_string(), sink);
        ProvenanceRegistration {
            sinks: self.clone(),
            run_id: run_id.to_string(),
        }
    }

    /// The sink of run `run_id`, while it is registered.
    #[must_use]
    pub fn for_run(&self, run_id: &str) -> Option<Arc<dyn SafetyProvenanceSink>> {
        let sink = self.runs.read().get(run_id).cloned()?;
        Some(sink)
    }
}

/// Keeps a run's sink registered with [`ProvenanceSinks`] until it drops.
#[derive(Debug)]
pub struct ProvenanceRegistration {
    sinks: ProvenanceSinks,
    run_id: String,
}

impl Drop for ProvenanceRegistration {
    fn drop(&mut self) {
        self.sinks.runs.write().remove(&self.run_id);
    }
}

/// A [`SafetyProvenanceSink`] over the workspace's witness and custody logs.
pub struct GraphProvenanceSink {
    key: [u8; 32],
    witness: WitnessLogger,
    custody: CustodyLogger,
    taint: TaintTracker,
    chain: Mutex<Chain>,
}

impl std::fmt::Debug for GraphProvenanceSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GraphProvenanceSink")
            .field("witness", &self.witness.path())
            .field("custody", &self.custody.path())
            .finish_non_exhaustive()
    }
}

/// The run's records so far: how many, where they sit in the logs, and the
/// witness vertex of each by [`record_identity`].
#[derive(Debug, Default)]
struct Chain {
    records: u64,
    /// The last vertex in the witness log: the next vertex's parent.
    last_vertex: Option<ContentHash>,
    /// The run's first custody record, which its resume verifies from.
    custody_root: Option<String>,
    /// The run's last witness vertex.
    witness_head: Option<ContentHash>,
    /// The run's last custody record.
    custody_head: Option<String>,
    recorded: HashMap<ContentHash, ContentHash>,
}

impl Chain {
    /// Note `provenance`, one of the run's records, written as witness
    /// vertex `vertex` and custody record `record`.
    fn note(&mut self, provenance: &ProvenanceRecord, vertex: ContentHash, record: &Custody) {
        self.records += 1;
        self.recorded.insert(record_identity(provenance), vertex);
        self.witness_head = Some(vertex);
        self.custody_head.clone_from(&record.hash);
        if self.custody_root.is_none() {
            self.custody_root.clone_from(&record.hash);
        }
    }
}

impl GraphProvenanceSink {
    /// Open the sink for the workspace at `workdir`: its digest key, and the
    /// last witness vertex, which the run's first vertex follows.
    ///
    /// # Errors
    ///
    /// Returns an error when the key cannot be read or created, or the
    /// witness log cannot be read.
    pub fn open(workdir: &Path) -> Result<Self> {
        let layout = RokoLayout::for_project(workdir);
        let key = load_or_create_key(&layout.state_dir().join(KEY_FILE))?;
        let witness = WitnessLogger::new(layout.witness_log());
        let custody = CustodyLogger::new(layout.custody_log());
        let chain = Chain {
            last_vertex: last_witness_id(witness.path())?,
            ..Chain::default()
        };
        Ok(Self {
            key,
            witness,
            custody,
            taint: TaintTracker::new(),
            chain: Mutex::new(chain),
        })
    }

    /// Open the sink for a run that starts fresh. The run's records extend
    /// the custody log from its last record, whatever the log held before:
    /// older builds and concurrent processes leave history that need not
    /// verify, and a resume checks only the run's own records. A history that
    /// does not verify is reported once, never refused.
    ///
    /// # Errors
    ///
    /// See [`Self::open`].
    pub fn start(workdir: &Path) -> Result<Self> {
        let sink = Self::open(workdir)?;
        let history = sink
            .custody
            .read_all()
            .map(|records| chain_violations(&records, None));
        match history {
            Ok(violations) => {
                if let Some(violation) = violations.first() {
                    tracing::warn!(
                        custody = %sink.custody.path().display(),
                        %violation,
                        "safety provenance: the custody log's history does not verify; \
                         this run's records are checked from its own first record on"
                    );
                }
            }
            Err(error) => tracing::warn!(
                custody = %sink.custody.path().display(),
                %error,
                "safety provenance: the custody log cannot be read"
            ),
        }
        Ok(sink)
    }

    /// Reopen the sink for run `run_id`, which a resume continues from what
    /// its checkpoint `stored`.
    ///
    /// The run's segment of the custody log, from the first record it wrote,
    /// must verify. The stored heads must be in their logs, every provenance
    /// record in the segment must name an intact witness vertex, and the
    /// stored taint index must cover what the run's records up to the stored
    /// custody head prove. The run's records after that head, written after
    /// the last checkpoint save, are tracked on top. Anything else is safety
    /// corruption, and the sink does not open rather than let the run go on
    /// as if nothing were tainted. History before the run's first record is
    /// not checked.
    ///
    /// # Errors
    ///
    /// Returns an error when the key or the logs cannot be read, or when any
    /// of those checks fails.
    pub fn resume(workdir: &Path, run_id: &str, stored: &SafetyProvenanceSummary) -> Result<Self> {
        let sink = Self::open(workdir)?;
        let taint = TaintTracker::from_json(stored.taint.clone())
            .context("safety provenance: read the stored taint index")?;
        let custody = sink
            .custody
            .read_all()
            .with_context(|| format!("read {}", sink.custody.path().display()))?;
        let witness = sink
            .witness
            .read_all()
            .with_context(|| format!("read {}", sink.witness.path().display()))?;
        let Some(root) = &stored.custody_root else {
            if stored.records > 0 {
                bail!(
                    "safety provenance: the checkpoint counts {} records but names no first \
                     custody record",
                    stored.records
                );
            }
            // Nothing was saved before the run stopped: track what it wrote
            // since, as far as the logs still show it.
            return Ok(sink.recover(run_id, &custody, &witness, taint));
        };
        let start = custody
            .iter()
            .position(|record| record.hash.as_ref() == Some(root))
            .with_context(|| {
                format!("safety provenance: the run's first custody record {root} is missing")
            })?;
        let segment = &custody[start..];
        let first_link = segment.first().and_then(|record| record.prev_hash.clone());
        if let Some(violation) = chain_violations(segment, first_link).first() {
            bail!(
                "safety provenance: the run's custody chain in {} is broken: {violation}",
                sink.custody.path().display()
            );
        }
        let committed = committed_records(stored, segment, &witness)?;
        // What the run's records up to the stored custody head prove.
        let proven = TaintTracker::new();
        let mut chain = Chain::default();
        for (index, record) in segment.iter().enumerate() {
            let Some((vertex, provenance)) = run_record(record, &witness, run_id)? else {
                continue;
            };
            chain.note(&provenance, vertex, record);
            track(
                if index < committed { &proven } else { &taint },
                &provenance,
            );
        }
        for (hash, level) in proven.levels() {
            if taint.get_level(&hash).is_none_or(|kept| kept < level) {
                bail!(
                    "safety provenance: the stored taint index puts {hash} below {level:?}, \
                     which the run's records prove"
                );
            }
        }
        Ok(sink.with_chain(chain, taint))
    }

    /// The sink of a run whose checkpoint saved none of its records: the
    /// run's records the logs still hold intact are tracked, and the first
    /// becomes its root. Nothing is verified, since nothing was saved to
    /// verify against, and tracking can only add taint.
    fn recover(
        self,
        run_id: &str,
        custody: &[Custody],
        witness: &WitnessDag,
        taint: TaintTracker,
    ) -> Self {
        let mut chain = Chain::default();
        for record in custody {
            if let Ok(Some((vertex, provenance))) = run_record(record, witness, run_id) {
                chain.note(&provenance, vertex, record);
                track(&taint, &provenance);
            }
        }
        self.with_chain(chain, taint)
    }

    /// This sink, carrying on from the run's records in `chain` and their
    /// taint.
    fn with_chain(self, mut chain: Chain, taint: TaintTracker) -> Self {
        {
            let mut current = self.chain.lock();
            chain.last_vertex = current.last_vertex;
            *current = chain;
        }
        Self { taint, ..self }
    }

    /// The taint lineage of the calls the sink recorded, a resumed run's
    /// earlier calls included.
    #[must_use]
    pub const fn taint(&self) -> &TaintTracker {
        &self.taint
    }

    /// What the checkpoint stores: the record count, the chains' heads and
    /// the taint index.
    #[must_use]
    pub fn summary(&self) -> SafetyProvenanceSummary {
        let chain = self.chain.lock();
        SafetyProvenanceSummary {
            records: chain.records,
            custody_root: chain.custody_root.clone(),
            witness_head: chain.witness_head,
            custody_head: chain.custody_head.clone(),
            taint: self.taint.to_json(),
        }
    }

    /// Append `record` about `call` to both logs, synced to disk: a witness
    /// vertex of `kind` after `parents`, and a custody record that names the
    /// vertex and carries `digest`. Returns the vertex id.
    fn append(
        &self,
        chain: &mut Chain,
        kind: VertexKind,
        call: &ProvenanceCall,
        parents: Vec<ContentHash>,
        record: &ProvenanceRecord,
        digest: Option<ContentHash>,
    ) -> std::io::Result<ContentHash> {
        let _serialized = APPEND_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let now = chrono::Utc::now().timestamp_millis();
        let principal = principal(call);
        // The sequence number keeps two identical records apart.
        let content = serde_json::json!({ "seq": chain.records + 1, "record": record });
        let at = u64::try_from(now).unwrap_or_default();
        let vertex = WitnessVertex::new(kind, principal.clone(), at, parents, content);
        self.witness.log(&vertex)?;
        sync(self.witness.path())?;
        let phase = match record {
            ProvenanceRecord::Intent(_) => "intent",
            ProvenanceRecord::Outcome(_) => "outcome",
        };
        let action = format!("tool_{phase}:{}", call.tool);
        let mut custody = Custody::new(action, principal, now, Vec::new());
        custody.witness = Some(vertex.id.to_hex());
        custody.result = digest.map(|digest| digest.to_hex());
        let custody_head = log_chained(&self.custody, custody)?;
        sync(self.custody.path())?;
        chain.records += 1;
        chain.last_vertex = Some(vertex.id);
        chain.witness_head = Some(vertex.id);
        if chain.custody_root.is_none() {
            chain.custody_root = Some(custody_head.clone());
        }
        chain.custody_head = Some(custody_head);
        Ok(vertex.id)
    }
}

impl SafetyProvenanceSink for GraphProvenanceSink {
    fn digest_key(&self) -> [u8; 32] {
        self.key
    }

    fn record_intent(&self, intent: &ProvenanceIntent) -> Result<ProvenanceAck, ProvenanceError> {
        let mut chain = self.chain.lock();
        let record = ProvenanceRecord::Intent(intent.clone());
        // An intent recorded before, say by an earlier process of the run, is
        // acknowledged again rather than written twice.
        let identity = record_identity(&record);
        if let Some(vertex) = chain.recorded.get(&identity) {
            return Ok(ProvenanceAck {
                record_id: vertex.to_hex(),
            });
        }
        let parents = chain.last_vertex.into_iter().collect();
        let digest = Some(intent.call.args_digest);
        let vertex = self
            .append(
                &mut chain,
                VertexKind::Decision,
                &intent.call,
                parents,
                &record,
                digest,
            )
            .map_err(|error| ProvenanceError(format!("record the intent: {error}")))?;
        chain.recorded.insert(identity, vertex);
        drop(chain);
        track_intent(&self.taint, intent);
        Ok(ProvenanceAck {
            record_id: vertex.to_hex(),
        })
    }

    fn record_outcome(&self, outcome: &ProvenanceOutcome) -> Result<(), ProvenanceError> {
        let mut chain = self.chain.lock();
        let record = ProvenanceRecord::Outcome(outcome.clone());
        let identity = record_identity(&record);
        if chain.recorded.contains_key(&identity) {
            return Ok(());
        }
        let mut parents: Vec<ContentHash> = chain.last_vertex.into_iter().collect();
        // An outcome follows its intent's vertex, whose id is the intent's
        // acknowledgement.
        if let Some(intent) = outcome.intent.as_deref().and_then(ContentHash::from_hex)
            && !parents.contains(&intent)
        {
            parents.push(intent);
        }
        let digest = outcome.result_digest;
        let vertex = self
            .append(
                &mut chain,
                VertexKind::Resolution,
                &outcome.call,
                parents,
                &record,
                digest,
            )
            .map_err(|error| ProvenanceError(format!("record the outcome: {error}")))?;
        chain.recorded.insert(identity, vertex);
        drop(chain);
        track_outcome(&self.taint, outcome);
        Ok(())
    }
}

/// How many records of the run's `segment` of the custody log the
/// checkpoint's summary `stored` covers: those up to and including its
/// custody head.
///
/// # Errors
///
/// Fails when a stored head is missing, or is not in its log.
fn committed_records(
    stored: &SafetyProvenanceSummary,
    segment: &[Custody],
    witness: &WitnessDag,
) -> Result<usize> {
    let (Some(custody_head), Some(witness_head)) = (&stored.custody_head, &stored.witness_head)
    else {
        bail!("safety provenance: the checkpoint names a first custody record but no heads");
    };
    if witness.get(witness_head).is_none() {
        bail!("safety provenance: the stored witness head {witness_head} is missing");
    }
    let index = segment
        .iter()
        .position(|record| record.hash.as_ref() == Some(custody_head))
        .with_context(|| {
            format!("safety provenance: the stored custody head {custody_head} is missing")
        })?;
    Ok(index + 1)
}

/// The provenance record that the custody record `record` names, with its
/// witness vertex, when it is one of run `run_id`'s.
///
/// # Errors
///
/// Fails when a provenance custody record names a witness vertex that is
/// missing, does not match its content, names a missing parent, or holds no
/// provenance record.
fn run_record(
    record: &Custody,
    witness: &WitnessDag,
    run_id: &str,
) -> Result<Option<(ContentHash, ProvenanceRecord)>> {
    let action = &record.action;
    let ours = ["tool_intent:", "tool_outcome:"];
    if !ours.iter().any(|prefix| action.starts_with(*prefix)) {
        return Ok(None);
    }
    let id = record
        .witness
        .as_deref()
        .and_then(ContentHash::from_hex)
        .with_context(|| {
            format!("safety provenance: custody record `{action}` names no witness vertex")
        })?;
    let vertex = witness
        .get(&id)
        .with_context(|| format!("safety provenance: witness vertex {id} is missing"))?;
    if !vertex.verify_id() {
        bail!("safety provenance: witness vertex {id} does not match its content");
    }
    let missing = vertex
        .parents
        .iter()
        .find(|parent| witness.get(parent).is_none());
    if let Some(parent) = missing {
        bail!("safety provenance: witness vertex {id} names a missing parent {parent}");
    }
    let provenance: ProvenanceRecord = serde_json::from_value(vertex.content["record"].clone())
        .with_context(|| format!("safety provenance: witness vertex {id} holds no record"))?;
    let call = match &provenance {
        ProvenanceRecord::Intent(intent) => &intent.call,
        ProvenanceRecord::Outcome(outcome) => &outcome.call,
    };
    let mine = call.run_id == run_id;
    Ok(mine.then_some((id, provenance)))
}

/// What makes two provenance records the same record: the hash of their
/// canonical JSON (RFC 8785). Recording the same record again, as a replay
/// would, then adds nothing to the logs.
fn record_identity(record: &ProvenanceRecord) -> ContentHash {
    let value = serde_json::to_value(record).unwrap_or_default();
    ContentHash::of(canonical_json(&value).as_bytes())
}

/// Track `record` in `tracker`.
fn track(tracker: &TaintTracker, record: &ProvenanceRecord) {
    match record {
        ProvenanceRecord::Intent(intent) => track_intent(tracker, intent),
        ProvenanceRecord::Outcome(outcome) => track_outcome(tracker, outcome),
    }
}

/// Who a record names as acting: the call's task, or roko itself.
fn principal(call: &ProvenanceCall) -> String {
    if call.task_id.is_empty() {
        "roko".to_string()
    } else {
        format!("task:{}", call.task_id)
    }
}

/// Flush the file at `path` to disk.
fn sync(path: &Path) -> std::io::Result<()> {
    OpenOptions::new().append(true).open(path)?.sync_data()
}

/// Id of the last vertex in the witness log at `path`; `None` when the log
/// is missing or holds none.
fn last_witness_id(path: &Path) -> Result<Option<ContentHash>> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    };
    Ok(content
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<WitnessVertex>(line.trim()).ok())
        .map(|vertex| vertex.id))
}

/// Read the digest key at `path`, or create it: 32 random bytes, readable by
/// their owner only.
fn load_or_create_key(path: &Path) -> Result<[u8; 32]> {
    match std::fs::read(path) {
        Ok(bytes) => return key_from(&bytes, path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(|| format!("read {}", path.display())),
    }
    let mut key = [0_u8; 32];
    key[..16].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    key[16..].copy_from_slice(uuid::Uuid::new_v4().as_bytes());
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(&key)
                .and_then(|()| file.sync_all())
                .with_context(|| format!("write {}", path.display()))?;
            Ok(key)
        }
        // Another process created the key first: use it.
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
            key_from(&bytes, path)
        }
        Err(error) => Err(error).with_context(|| format!("create {}", path.display())),
    }
}

/// The key in `bytes`, read from `path`.
fn key_from(bytes: &[u8], path: &Path) -> Result<[u8; 32]> {
    <[u8; 32]>::try_from(bytes)
        .map_err(|_| anyhow!("{} does not hold a 32-byte key", path.display()))
}

#[cfg(test)]
mod tests {
    use roko_agent::safety::ProvenanceVerdict;
    use roko_agent::safety::provenance_sink::arguments_digest;
    use roko_core::extension::CamelTaintLevel;
    use tempfile::tempdir;

    use super::*;

    fn call(key: &[u8; 32]) -> ProvenanceCall {
        ProvenanceCall {
            run_id: "run-1".to_string(),
            task_id: "T1".to_string(),
            attempt_id: "1".to_string(),
            turn_id: "1".to_string(),
            call_id: "call-1".to_string(),
            tool: "read_file".to_string(),
            args_digest: arguments_digest(key, &serde_json::json!({"path": "secret-plan.txt"})),
        }
    }

    #[test]
    fn graph_provenance_sink_writes_synced_witness_and_custody_chains() {
        let dir = tempdir().expect("tempdir");
        let sink = GraphProvenanceSink::open(dir.path()).expect("open the sink");
        let key = sink.digest_key();
        let intent = ProvenanceIntent {
            call: call(&key),
            taint: CamelTaintLevel::External,
        };
        let ack = sink.record_intent(&intent).expect("record the intent");
        let outcome = ProvenanceOutcome {
            call: intent.call.clone(),
            intent: Some(ack.record_id.clone()),
            verdict: ProvenanceVerdict::Succeeded,
            reason: None,
            result_digest: Some(ContentHash::keyed(&key, b"result")),
            taint: CamelTaintLevel::External,
        };
        sink.record_outcome(&outcome).expect("record the outcome");

        // The witness DAG links the outcome to its intent.
        let layout = RokoLayout::for_project(dir.path());
        let dag = WitnessLogger::new(layout.witness_log())
            .read_all()
            .expect("witness log");
        assert_eq!(dag.len(), 2);
        assert!(dag.verify_integrity().is_empty());
        let summary = sink.summary();
        let head = summary.witness_head.expect("witness head");
        let resolution = dag.get(&head).expect("the head vertex");
        assert_eq!(resolution.kind, VertexKind::Resolution);
        let intent_id = ContentHash::from_hex(&ack.record_id).expect("intent vertex id");
        assert_eq!(resolution.parents, [intent_id]);
        // The custody chain holds both records, and it verifies.
        let custody = CustodyLogger::new(layout.custody_log())
            .read_all()
            .expect("custody log");
        let actions: Vec<&str> = custody
            .iter()
            .map(|record| record.action.as_str())
            .collect();
        assert_eq!(actions, ["tool_intent:read_file", "tool_outcome:read_file"]);
        assert_eq!(summary.custody_head, custody[1].hash);
        crate::custody::cmd_custody_verify(dir.path()).expect("the custody chain verifies");
        // The taint index carries the lineage from the arguments to the result.
        assert_eq!(summary.records, 2);
        let taint = TaintTracker::from_json(summary.taint).expect("taint index");
        let result = outcome.result_digest.expect("result digest");
        assert_eq!(taint.derived_from(&result), [intent.call.args_digest]);
        // No argument text reaches the logs.
        for log in [layout.witness_log(), layout.custody_log()] {
            let text = std::fs::read_to_string(&log).expect("log");
            assert!(!text.contains("secret-plan"), "{text}");
        }

        // The key file is readable by its owner only.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let key_file = layout.state_dir().join(KEY_FILE);
            let mode = std::fs::metadata(&key_file)
                .expect("key file")
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }

        // A run that starts later keeps the key, and its first record follows
        // the logs' last vertex and record.
        let later = GraphProvenanceSink::start(dir.path()).expect("start another run");
        assert_eq!(later.digest_key(), key);
        let mut another = intent.clone();
        another.call.run_id = "run-2".to_string();
        let ack = later.record_intent(&another).expect("record the intent");
        let dag = WitnessLogger::new(layout.witness_log())
            .read_all()
            .expect("witness log");
        let id = ContentHash::from_hex(&ack.record_id).expect("vertex id");
        assert_eq!(dag.get(&id).expect("the new vertex").parents, [head]);
        crate::custody::cmd_custody_verify(dir.path()).expect("the custody chain verifies");
        let later_summary = later.summary();
        assert_eq!(later_summary.records, 1);
        assert_eq!(later_summary.custody_root, later_summary.custody_head);
    }

    #[test]
    fn graph_provenance_sink_records_a_replayed_record_once() {
        let dir = tempdir().expect("tempdir");
        let sink = GraphProvenanceSink::open(dir.path()).expect("open the sink");
        let key = sink.digest_key();
        let intent = ProvenanceIntent {
            call: call(&key),
            taint: CamelTaintLevel::Local,
        };
        let first = sink.record_intent(&intent).expect("record the intent");
        assert_eq!(sink.record_intent(&intent).expect("record it again"), first);
        let outcome = ProvenanceOutcome {
            call: intent.call.clone(),
            intent: Some(first.record_id.clone()),
            verdict: ProvenanceVerdict::Failed,
            reason: Some("timeout".to_string()),
            result_digest: None,
            taint: CamelTaintLevel::Local,
        };
        sink.record_outcome(&outcome).expect("record the outcome");
        sink.record_outcome(&outcome).expect("record it again");
        assert_eq!(sink.summary().records, 2);

        // A later process of the run acknowledges the same intent with its
        // first record, and writes nothing new.
        let stored = sink.summary();
        let resumed = GraphProvenanceSink::resume(dir.path(), "run-1", &stored).expect("resume");
        assert_eq!(
            resumed.record_intent(&intent).expect("replayed intent"),
            first
        );
        resumed.record_outcome(&outcome).expect("replayed outcome");
        let custody = CustodyLogger::new(RokoLayout::for_project(dir.path()).custody_log())
            .read_all()
            .expect("custody log");
        assert_eq!(custody.len(), 2);
        assert_eq!(resumed.summary().records, 2);
    }

    #[test]
    fn provenance_sinks_hold_a_runs_sink_while_it_is_registered() {
        let dir = tempdir().expect("tempdir");
        let sinks = ProvenanceSinks::default();
        let sink = Arc::new(GraphProvenanceSink::open(dir.path()).expect("open the sink"));
        assert!(sinks.for_run("run-1").is_none());
        let registration = sinks.register("run-1", Arc::clone(&sink));
        let shared = sinks.clone();
        let found = shared.for_run("run-1").expect("the run's sink");
        assert_eq!(found.digest_key(), sink.digest_key());
        assert!(shared.for_run("run-2").is_none());
        drop(registration);
        assert!(shared.for_run("run-1").is_none());
    }

    #[test]
    fn graph_provenance_sink_refuses_a_bad_key_file() {
        let dir = tempdir().expect("tempdir");
        let path = RokoLayout::for_project(dir.path())
            .state_dir()
            .join(KEY_FILE);
        std::fs::create_dir_all(path.parent().expect("state dir")).expect("state dir");
        std::fs::write(&path, b"short").expect("bad key");
        let error = GraphProvenanceSink::open(dir.path()).expect_err("a short key fails");
        assert!(error.to_string().contains("32-byte key"), "{error:#}");
    }
}
