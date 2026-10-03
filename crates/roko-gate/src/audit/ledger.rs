//! The audit ledger (S05 §5).
//!
//! It is a SHA-256 hash chain of every audit event in
//! the vault, `<vault>/<workspace_id>/ledger/audit-YYYYMMDD.jsonl`, and a
//! redacted mirror in the workspace's agent-readable
//! `.roko/audit/audits.jsonl`.
//!
//! Every vault record carries `schema` (`roko.audit/1`), `record_id`,
//! `seq`, `at`, `prev_hash` and `record_hash`: SHA-256 over the RFC 8785
//! canonical JSON of the record without `record_hash`, chained through
//! `prev_hash` across the day files from [`GENESIS`]. `record_id` is S01's
//! id rule with SHA-256 (`roko_learn::telemetry::records::record_id`):
//! sha256(schema | attempt key | event | item | seq). [`verify_chain`]
//! walks every file and names the first record that breaks the chain.
//!
//! The vault chain is authoritative. The mirror keeps each record's hashes,
//! so it can be checked against the vault, but never holds a run key before
//! that run's `audit.key_reveal` (a key [`AuditLedger::commit_key`]
//! registered is scrubbed from every mirrored line until
//! [`AuditLedger::reveal_key`]), nor a hidden test's body or ids: an
//! `audit.result` loses its check detail, which may name failing hidden
//! tests. One process appends at a time: each append holds an exclusive
//! lock on `ledger/.lock` and re-reads the chain's tail when another writer
//! moved it.

use std::fs::OpenOptions;
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use roko_core::audit_home::AuditVault;
use roko_core::audit_types::{AuditLabels, Stratum};
use roko_core::config::fingerprint::canonical_json;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::policy::{RunKey, commitment, hex};

/// The schema of every ledger record.
pub const SCHEMA: &str = "roko.audit/1";
/// The `prev_hash` of the first record.
pub const GENESIS: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";
/// The redaction marker in mirrored lines.
pub const REDACTED: &str = "<redacted>";

/// One audit event (S05 §5). Hidden-test bodies never appear in any.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "ev")]
pub enum AuditEvent {
    /// A run's commitment to its key, at open.
    #[serde(rename = "audit.key_commit")]
    KeyCommit {
        /// The run.
        run_id: String,
        /// sha256(K_run ‖ run_id).
        commitment: String,
    },
    /// A run's key, at close.
    #[serde(rename = "audit.key_reveal")]
    KeyReveal {
        /// The run.
        run_id: String,
        /// K_run as 64 hex digits.
        key_hex: String,
    },
    /// One green unit's draw (DP1).
    #[serde(rename = "audit.selection")]
    Selection {
        /// The selection's id.
        sel_id: String,
        /// S01's attempt key.
        attempt_key: String,
        /// The run.
        run_id: String,
        /// The task.
        task_id: String,
        /// The stratum it was drawn in.
        stratum: Stratum,
        /// π_i.
        pi: f64,
        /// x_i as "0x" and 16 hex digits.
        prf_u: String,
        /// Whether it was selected.
        selected: bool,
        /// The tree the attempt started from.
        base_tree: Option<String>,
        /// The tree it left, standing in for its accepted commit.
        result_tree: Option<String>,
    },
    /// An audit's labels.
    #[serde(rename = "audit.result")]
    Result {
        /// The selection audited.
        sel_id: String,
        /// The result's id.
        #[serde(default)]
        res_id: String,
        /// S01's attempt key.
        attempt_key: String,
        /// The labels.
        labels: AuditLabels,
        /// What phase A's diff checks found, one line each: A1 and the
        /// audit-only kinds, which name the attempt's own paths only.
        #[serde(default)]
        findings: Vec<String>,
        /// Each check's detail; never mirrored, as it may name hidden tests.
        checks: Value,
        /// The audit's spend.
        cost_usd: Option<f64>,
        /// Seconds the audit's checks ran, the CPU cap's measure.
        #[serde(default)]
        cpu_secs: Option<f64>,
        /// π_i·π_B, the inclusion probability of phase B's labels; phase A's
        /// is the selection's π_i.
        #[serde(default)]
        pi_eff: Option<f64>,
    },
    /// A window's estimate.
    #[serde(rename = "audit.estimate")]
    Estimate {
        /// The window.
        window: String,
        /// The stratum, or `None` for the whole window.
        stratum: Option<Stratum>,
        /// The estimate (`estimate::Estimate`).
        estimate: Value,
    },
    /// A policy knob moved (only ρ and λ adapt, S05 §4.9).
    #[serde(rename = "audit.policy_change")]
    PolicyChange {
        /// The knob.
        knob: String,
        /// Its old value.
        from: Value,
        /// Its new value.
        to: Value,
        /// Why.
        reason: String,
    },
    /// An incident record's status changed (S05 §4.7).
    #[serde(rename = "audit.incident")]
    Incident {
        /// The incident.
        incident_id: String,
        /// `false_green`, `spec_gaming`, `weak_oracle`, `leak_canary` or
        /// `battery_fault`.
        kind: String,
        /// The attempt it is about.
        attempt_key: String,
        /// Its new status.
        status: String,
    },
    /// A suite's canary was found where it must not be (SC4).
    #[serde(rename = "audit.leak_canary")]
    LeakCanary {
        /// Where: `prompt`, `output`, `diff`, `episodes`, …
        place: String,
        /// The suite, or `unknown`.
        suite_id: String,
    },
    /// Even the floor's audits no longer fit the budget (S05 §4.9).
    #[serde(rename = "audit.budget_exhausted")]
    BudgetExhausted {
        /// The run.
        run_id: String,
        /// Audit spend so far.
        spent_usd: f64,
        /// The limit.
        limit_usd: f64,
    },
    /// A hidden suite moved between lifecycle states (S05 §5).
    #[serde(rename = "audit.hidden_suite")]
    HiddenSuite {
        /// The suite.
        suite_id: String,
        /// Its task.
        task_id: String,
        /// The spec it was written from.
        spec_hash: String,
        /// The suite's content.
        suite_hash: String,
        /// The model that wrote it.
        author_model: String,
        /// That model's family.
        author_family: String,
        /// The implementer's family.
        implementer_family: String,
        /// The state it left; `None` when it was created.
        from: Option<String>,
        /// The state it entered.
        to: String,
        /// `shown_to_fixer`, `canary_hit`, `max_uses`, `age`, …
        reason: String,
        /// Audits that used it.
        uses: u32,
    },
}

impl AuditEvent {
    /// The event name, `audit.…`.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::KeyCommit { .. } => "audit.key_commit",
            Self::KeyReveal { .. } => "audit.key_reveal",
            Self::Selection { .. } => "audit.selection",
            Self::Result { .. } => "audit.result",
            Self::Estimate { .. } => "audit.estimate",
            Self::PolicyChange { .. } => "audit.policy_change",
            Self::Incident { .. } => "audit.incident",
            Self::LeakCanary { .. } => "audit.leak_canary",
            Self::BudgetExhausted { .. } => "audit.budget_exhausted",
            Self::HiddenSuite { .. } => "audit.hidden_suite",
        }
    }

    /// The attempt key and item of the record id.
    fn identity(&self) -> (&str, &str) {
        match self {
            Self::KeyCommit { run_id, .. }
            | Self::KeyReveal { run_id, .. }
            | Self::BudgetExhausted { run_id, .. } => ("-", run_id),
            Self::Selection {
                attempt_key,
                sel_id,
                ..
            }
            | Self::Result {
                attempt_key,
                sel_id,
                ..
            } => (attempt_key, sel_id),
            Self::Estimate { window, .. } => ("-", window),
            Self::PolicyChange { knob, .. } => ("-", knob),
            Self::Incident {
                attempt_key,
                incident_id,
                ..
            } => (attempt_key, incident_id),
            Self::LeakCanary { suite_id, .. } | Self::HiddenSuite { suite_id, .. } => {
                ("-", suite_id)
            }
        }
    }
}

/// A record as the vault holds it: the event and its chain fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LedgerRecord {
    /// [`SCHEMA`].
    pub schema: String,
    /// sha256(schema | attempt key | event | item | seq).
    pub record_id: String,
    /// The record's place in the chain, from 1.
    pub seq: u64,
    /// When it was appended (RFC 3339, UTC).
    pub at: String,
    /// The previous record's `record_hash`, or [`GENESIS`].
    pub prev_hash: String,
    /// SHA-256 of the record without this field.
    pub record_hash: String,
    /// The event.
    #[serde(flatten)]
    pub event: AuditEvent,
}

/// Where the chain breaks.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{file}:{line}: {reason}")]
pub struct ChainError {
    /// The day file.
    pub file: PathBuf,
    /// The 1-based line.
    pub line: usize,
    /// What is wrong.
    pub reason: String,
}

/// The audit ledger of one workspace.
#[derive(Debug)]
pub struct AuditLedger {
    dir: PathBuf,
    mirror: Option<PathBuf>,
    /// The tail this writer last saw: (file, its length, seq, record hash).
    tail: Option<(PathBuf, u64, u64, String)>,
    /// Keys committed and not yet revealed, as hex, scrubbed from the mirror.
    unrevealed: Vec<(String, String)>,
}

impl AuditLedger {
    /// The ledger in `vault`'s `ledger/`, created with mode 0700.
    ///
    /// # Errors
    ///
    /// The directory cannot be created.
    pub fn open(vault: &AuditVault) -> std::io::Result<Self> {
        Self::open_dir(&vault.ledger_dir())
    }

    /// The ledger in `dir` (the vault's `ledger/` in production).
    ///
    /// # Errors
    ///
    /// The directory cannot be created.
    pub fn open_dir(dir: &Path) -> std::io::Result<Self> {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
        builder.create(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
            mirror: None,
            tail: None,
            unrevealed: Vec::new(),
        })
    }

    /// Mirror every record, redacted, to `path` (the workspace's
    /// `.roko/audit/audits.jsonl`).
    #[must_use]
    pub fn with_mirror(mut self, path: PathBuf) -> Self {
        self.mirror = Some(path);
        self
    }

    /// The ledger's directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Append `audit.key_commit` for `run_id`, and scrub `key` from the
    /// mirror until [`Self::reveal_key`].
    ///
    /// # Errors
    ///
    /// As [`Self::append`].
    pub fn commit_key(&mut self, run_id: &str, key: &RunKey) -> std::io::Result<LedgerRecord> {
        self.unrevealed.push((run_id.to_string(), key.to_hex()));
        self.append(AuditEvent::KeyCommit {
            run_id: run_id.to_string(),
            commitment: commitment(key, run_id),
        })
    }

    /// Append `audit.key_reveal` for `run_id`; its key may then appear in
    /// the mirror.
    ///
    /// # Errors
    ///
    /// As [`Self::append`].
    pub fn reveal_key(&mut self, run_id: &str, key: &RunKey) -> std::io::Result<LedgerRecord> {
        self.unrevealed.retain(|(run, _)| run != run_id);
        self.append(AuditEvent::KeyReveal {
            run_id: run_id.to_string(),
            key_hex: key.to_hex(),
        })
    }

    /// Append `event` to today's file, chained to the last record, and
    /// mirror it.
    ///
    /// # Errors
    ///
    /// The lock, a file or the mirror cannot be read or written.
    pub fn append(&mut self, event: AuditEvent) -> std::io::Result<LedgerRecord> {
        let lock = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.dir.join(".lock"))?;
        lock.lock()?;
        let appended = self.append_locked(event);
        lock.unlock()?;
        appended
    }

    fn append_locked(&mut self, event: AuditEvent) -> std::io::Result<LedgerRecord> {
        let now = chrono::Utc::now();
        let file = self
            .dir
            .join(format!("audit-{}.jsonl", now.format("%Y%m%d")));
        let (seq, prev_hash) = self.tail_state()?;
        let seq = seq + 1;
        let (attempt_key, item) = event.identity();
        let id_input = [SCHEMA, attempt_key, event.name(), item, &seq.to_string()].join("|");
        let mut record = LedgerRecord {
            schema: SCHEMA.to_string(),
            record_id: sha256_text(id_input.as_bytes()),
            seq,
            at: now.to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
            prev_hash,
            record_hash: String::new(),
            event,
        };
        record.record_hash = record_hash(&serde_json::to_value(&record)?);
        let mut line = serde_json::to_string(&record)?;
        line.push('\n');
        let mut handle = append_private(&file)?;
        handle.write_all(line.as_bytes())?;
        handle.flush()?;
        let length = handle.metadata()?.len();
        self.tail = Some((file, length, seq, record.record_hash.clone()));
        if let Some(mirror) = &self.mirror {
            let mut value = redact_for_mirror(&record)?;
            scrub(&mut value, &self.unrevealed);
            let mut line = serde_json::to_string(&value)?;
            line.push('\n');
            if let Some(parent) = mirror.parent() {
                std::fs::create_dir_all(parent)?;
            }
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(mirror)?
                .write_all(line.as_bytes())?;
        }
        Ok(record)
    }

    /// The last record's seq and hash: from memory when no other writer
    /// moved the tail, else from the newest day file.
    fn tail_state(&self) -> std::io::Result<(u64, String)> {
        let newest = day_files(&self.dir)?.pop();
        if let (Some((file, length, seq, hash)), Some(newest)) = (&self.tail, &newest)
            && file == newest
            && std::fs::metadata(newest)?.len() == *length
        {
            return Ok((*seq, hash.clone()));
        }
        let Some(newest) = newest else {
            return Ok((0, GENESIS.to_string()));
        };
        let Some(line) = last_line(&newest)? else {
            return Ok((0, GENESIS.to_string()));
        };
        let record: LedgerRecord = serde_json::from_str(&line)?;
        Ok((record.seq, record.record_hash))
    }
}

/// Walk every day file of the ledger in `dir`, in order.
///
/// Returns the
/// number of records, or the first record that breaks the chain: a line
/// that does not parse, a `record_hash` that does not match its record, a
/// `prev_hash` that is not the previous `record_hash`, or a `seq` out of
/// order.
///
/// # Errors
///
/// The first break, or a file that cannot be read.
pub fn verify_chain(dir: &Path) -> Result<u64, ChainError> {
    let files = day_files(dir).map_err(|error| ChainError {
        file: dir.to_path_buf(),
        line: 0,
        reason: error.to_string(),
    })?;
    let (mut previous, mut count) = (GENESIS.to_string(), 0_u64);
    for file in files {
        let text = std::fs::read_to_string(&file).map_err(|error| ChainError {
            file: file.clone(),
            line: 0,
            reason: error.to_string(),
        })?;
        for (index, line) in text.lines().enumerate() {
            let broken = |reason: String| ChainError {
                file: file.clone(),
                line: index + 1,
                reason,
            };
            let value: Value = serde_json::from_str(line)
                .map_err(|error| broken(format!("not a JSON record: {error}")))?;
            let record: LedgerRecord = serde_json::from_value(value.clone())
                .map_err(|error| broken(format!("not a ledger record: {error}")))?;
            if record_hash(&value) != record.record_hash {
                return Err(broken("the record does not match its record_hash".into()));
            }
            if record.prev_hash != previous {
                return Err(broken("prev_hash is not the previous record's hash".into()));
            }
            if record.seq != count + 1 {
                return Err(broken(format!("seq {} follows {count}", record.seq)));
            }
            previous = record.record_hash;
            count += 1;
        }
    }
    Ok(count)
}

/// Every record of the ledger in `dir`, in order, without checking the
/// chain ([`verify_chain`] does that); a line that does not parse is
/// skipped, and a ledger not yet written holds none.
///
/// # Errors
///
/// A day file cannot be read.
pub fn records(dir: &Path) -> std::io::Result<Vec<LedgerRecord>> {
    let files = match day_files(dir) {
        Ok(files) => files,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut records = Vec::new();
    for file in files {
        for line in std::fs::read_to_string(&file)?.lines() {
            if let Ok(record) = serde_json::from_str(line) {
                records.push(record);
            }
        }
    }
    Ok(records)
}

/// A record as the workspace mirror shows it: the same chain fields, with
/// what agents must not read removed.
///
/// # Errors
///
/// The record cannot be serialized.
pub fn redact_for_mirror(record: &LedgerRecord) -> serde_json::Result<Value> {
    let mut value = serde_json::to_value(record)?;
    if matches!(record.event, AuditEvent::Result { .. }) {
        value["checks"] = Value::String(REDACTED.to_string());
    }
    Ok(value)
}

/// Replace every unrevealed key in `value`'s strings.
fn scrub(value: &mut Value, unrevealed: &[(String, String)]) {
    match value {
        Value::String(text) => {
            for (_, key) in unrevealed {
                if text.contains(key.as_str()) {
                    *text = text.replace(key.as_str(), REDACTED);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|item| scrub(item, unrevealed)),
        Value::Object(fields) => fields.values_mut().for_each(|item| scrub(item, unrevealed)),
        _ => {}
    }
}

/// "sha256:" + SHA-256 of the canonical JSON of `record` without
/// `record_hash`.
fn record_hash(record: &Value) -> String {
    let mut body = record.clone();
    if let Some(fields) = body.as_object_mut() {
        fields.remove("record_hash");
    }
    sha256_text(canonical_json(&body).as_bytes())
}

fn sha256_text(bytes: &[u8]) -> String {
    format!("sha256:{}", hex(&Sha256::digest(bytes)))
}

/// The ledger's day files, oldest first.
fn day_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("audit-"))
                && path
                    .extension()
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("jsonl"))
        })
        .collect();
    files.sort();
    Ok(files)
}

/// The last non-empty line of `path`, read from its end.
fn last_line(path: &Path) -> std::io::Result<Option<String>> {
    let mut file = std::fs::File::open(path)?;
    let length = file.metadata()?.len();
    let mut start = length.saturating_sub(64 * 1024);
    loop {
        file.seek(SeekFrom::Start(start))?;
        let mut tail = Vec::new();
        file.read_to_end(&mut tail)?;
        let end = tail
            .iter()
            .rposition(|&byte| byte != b'\n')
            .map_or(0, |at| at + 1);
        let trimmed = &tail[..end];
        match trimmed.iter().rposition(|&byte| byte == b'\n') {
            Some(at) => return Ok(Some(String::from_utf8_lossy(&trimmed[at + 1..]).into())),
            None if start == 0 => {
                return Ok((!trimmed.is_empty()).then(|| String::from_utf8_lossy(trimmed).into()));
            }
            None => start = start.saturating_sub(64 * 1024),
        }
    }
}

/// `path` opened for appending, created with mode 0600.
fn append_private(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = OpenOptions::new();
    options.create(true).append(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(index: usize) -> AuditEvent {
        AuditEvent::Selection {
            sel_id: format!("sel-{index}"),
            attempt_key: format!("run-1:plan:T{index}:1"),
            run_id: "run-1".into(),
            task_id: format!("T{index}"),
            stratum: Stratum::default(),
            pi: 0.1,
            prf_u: format!("0x{index:016x}"),
            selected: index % 10 == 0,
            base_tree: Some("base".into()),
            result_tree: Some(format!("tree-{index}")),
        }
    }

    #[test]
    fn ledger_chain_verifies_after_10k_appends_and_catches_a_flipped_byte() {
        let temp = tempfile::tempdir().expect("tempdir");
        let dir = temp.path().join("ledger");
        let mut ledger = AuditLedger::open_dir(&dir).expect("a ledger");
        for index in 0..10_000 {
            ledger.append(selection(index)).expect("append");
        }
        assert_eq!(verify_chain(&dir), Ok(10_000));

        // A second writer continues the same chain.
        let mut other = AuditLedger::open_dir(&dir).expect("another writer");
        let record = other.append(selection(10_000)).expect("append");
        assert_eq!(record.seq, 10_001);
        ledger
            .append(selection(10_001))
            .expect("the first writer sees the new tail");
        assert_eq!(verify_chain(&dir), Ok(10_002));

        // Flip one byte of record 5,000: the chain breaks exactly there.
        let file = day_files(&dir).expect("files").pop().expect("a day file");
        let mut text = std::fs::read_to_string(&file).expect("read");
        let start = text.match_indices('\n').nth(4_998).expect("line 5,000").0 + 1;
        let at = start + text[start..].find("tree-").expect("a tree") + 5;
        let flipped = if &text[at..=at] == "4" { "5" } else { "4" };
        text.replace_range(at..=at, flipped);
        std::fs::write(&file, text).expect("write");
        let error = verify_chain(&dir).expect_err("a broken chain");
        assert_eq!(error.line, 5_000, "{error}");
        assert!(error.reason.contains("record_hash"), "{error}");
    }

    #[test]
    fn the_workspace_mirror_never_holds_an_unrevealed_key() {
        let temp = tempfile::tempdir().expect("tempdir");
        let mirror = temp.path().join("work/.roko/audit/audits.jsonl");
        let dir = temp.path().join("vault/ledger");
        let mut ledger = AuditLedger::open_dir(&dir)
            .expect("a ledger")
            .with_mirror(mirror.clone());
        let key = RunKey::from_bytes([42; 32]);
        ledger.commit_key("run-1", &key).expect("commit");
        ledger.append(selection(0)).expect("select");
        // A careless event quotes the key: the mirror scrubs it.
        let reason = format!("debug {}", key.to_hex());
        let change = AuditEvent::PolicyChange {
            knob: "rho".into(),
            from: 0.1.into(),
            to: 0.15.into(),
            reason,
        };
        ledger.append(change).expect("policy change");
        let result = AuditEvent::Result {
            sel_id: "sel-0".into(),
            res_id: "res-0".into(),
            attempt_key: "run-1:plan:T0:1".into(),
            labels: AuditLabels::default(),
            findings: Vec::new(),
            checks: serde_json::json!({"b1": {"failed": ["test_hidden_upper_bound"]}}),
            cost_usd: Some(0.01),
            cpu_secs: None,
            pi_eff: None,
        };
        ledger.append(result).expect("result");
        let before = std::fs::read_to_string(&mirror).expect("the mirror");
        assert!(!before.contains(&key.to_hex()), "{before}");
        assert!(!before.contains("test_hidden_upper_bound"), "{before}");
        assert_eq!(before.lines().count(), 4);

        ledger.reveal_key("run-1", &key).expect("reveal");
        let after = std::fs::read_to_string(&mirror).expect("the mirror");
        let reveal = after.lines().last().expect("the reveal");
        assert!(reveal.contains(&key.to_hex()), "{reveal}");
        // The vault keeps everything, and its chain holds.
        assert_eq!(verify_chain(&dir), Ok(5));
        let vault = std::fs::read_to_string(day_files(&dir).expect("files").remove(0))
            .expect("the vault file");
        assert!(vault.contains("test_hidden_upper_bound"));
    }
}
