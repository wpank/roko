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

use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use parking_lot::Mutex;
use roko_agent::safety::provenance::{Custody, CustodyLogger};
use roko_agent::safety::provenance_sink::{track_intent, track_outcome};
use roko_agent::safety::{
    ProvenanceAck, ProvenanceCall, ProvenanceError, ProvenanceIntent, ProvenanceOutcome,
    ProvenanceRecord, SafetyProvenanceSink, TaintTracker, VertexKind, WitnessLogger, WitnessVertex,
};
use roko_core::ContentHash;
use roko_fs::RokoLayout;
use serde::{Deserialize, Serialize};

use crate::custody::log_chained;

/// File in `.roko/state/` holding the digest key.
const KEY_FILE: &str = "safety-provenance.key";

/// What a Graph checkpoint keeps of its run's safety provenance, under
/// `roko.safety-provenance@1`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SafetyProvenanceSummary {
    /// Provenance records the sink wrote.
    pub records: u64,
    /// Id of the last vertex in `.roko/witness.jsonl`.
    pub witness_head: Option<ContentHash>,
    /// Hash of the last record in `.roko/custody.jsonl`.
    pub custody_head: Option<String>,
    /// The taint index ([`TaintTracker::to_json`]).
    pub taint: serde_json::Value,
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

/// How many records the sink wrote, and where the logs' chains end.
#[derive(Debug, Default)]
struct Chain {
    records: u64,
    witness_head: Option<ContentHash>,
    custody_head: Option<String>,
}

impl GraphProvenanceSink {
    /// Open the sink for the workspace at `workdir`: its digest key, and the
    /// ends of its witness and custody chains, which new records extend.
    ///
    /// # Errors
    ///
    /// Returns an error when the key cannot be read or created, or a log
    /// cannot be read.
    pub fn open(workdir: &Path) -> Result<Self> {
        let layout = RokoLayout::for_project(workdir);
        let key = load_or_create_key(&layout.state_dir().join(KEY_FILE))?;
        let witness = WitnessLogger::new(layout.witness_log());
        let custody = CustodyLogger::new(layout.custody_log());
        let custody_head = custody
            .read_all()
            .with_context(|| format!("read {}", custody.path().display()))?
            .last()
            .and_then(|record| record.hash.clone());
        let chain = Chain {
            witness_head: last_witness_id(witness.path())?,
            custody_head,
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

    /// What the checkpoint stores: the record count, the chains' heads and
    /// the taint index.
    #[must_use]
    pub fn summary(&self) -> SafetyProvenanceSummary {
        let chain = self.chain.lock();
        SafetyProvenanceSummary {
            records: chain.records,
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
        chain.witness_head = Some(vertex.id);
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
        let parents = chain.witness_head.into_iter().collect();
        let record = ProvenanceRecord::Intent(intent.clone());
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
        drop(chain);
        track_intent(&self.taint, intent);
        Ok(ProvenanceAck {
            record_id: vertex.to_hex(),
        })
    }

    fn record_outcome(&self, outcome: &ProvenanceOutcome) -> Result<(), ProvenanceError> {
        let mut chain = self.chain.lock();
        let mut parents: Vec<ContentHash> = chain.witness_head.into_iter().collect();
        // An outcome follows its intent's vertex, whose id is the intent's
        // acknowledgement.
        if let Some(intent) = outcome.intent.as_deref().and_then(ContentHash::from_hex)
            && !parents.contains(&intent)
        {
            parents.push(intent);
        }
        let record = ProvenanceRecord::Outcome(outcome.clone());
        let digest = outcome.result_digest;
        self.append(
            &mut chain,
            VertexKind::Resolution,
            &outcome.call,
            parents,
            &record,
            digest,
        )
        .map_err(|error| ProvenanceError(format!("record the outcome: {error}")))?;
        drop(chain);
        track_outcome(&self.taint, outcome);
        Ok(())
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

        // A reopened sink keeps the key and extends both chains.
        let reopened = GraphProvenanceSink::open(dir.path()).expect("reopen the sink");
        assert_eq!(reopened.digest_key(), key);
        let reopened_summary = reopened.summary();
        assert_eq!(reopened_summary.witness_head, Some(head));
        assert_eq!(reopened_summary.custody_head, summary.custody_head);
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
