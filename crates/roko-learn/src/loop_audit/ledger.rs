//! Loop-audit rows (A-LOOP) and their append-only JSONL writer (S03 §5; backlog 5115).
//!
//! S01 §5.10 owns the schemas (R.S03-2); this module is their Rust form.
//!
//! - [`LoopAuditRecord`], `roko.loop_audit/1`: the cross-run loop-audit
//!   ledger, `.roko/learn/loop-audit.jsonl`, since a loop's state spans runs
//!   (its dwell is 24 h). Its kinds are `loop.registered`, `loop.health`,
//!   `loop.transition`, `loop.canary` and `loop.paired_replay`.
//! - [`FaultRecord`], `roko.fault/1`: the `loop.fault` ground truth, per run
//!   in `.roko/runs/<run_id>/faults.jsonl`, which only the evaluation joins.
//!
//! Both files take paths, so their placement stays the caller's. Rows append
//! under the `*.jsonl.lock` convention ([`roko_fs::log_rotation`]); the
//! reader gives the latest health row per loop and the transitions since a
//! time.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use super::spec::{AuditState, ReasonCode, ReceiptKind};
use super::state::Actor;

/// `schema_version` of a loop-audit row.
pub const LOOP_AUDIT_SCHEMA: &str = "roko.loop_audit/1";

/// `schema_version` of a fault row.
pub const FAULT_SCHEMA: &str = "roko.fault/1";

/// The cross-run ledger's file name, under `.roko/learn`.
pub const LOOP_AUDIT_FILE: &str = "loop-audit.jsonl";

/// A run's fault file name, under `.roko/runs/<run_id>`.
pub const FAULTS_FILE: &str = "faults.jsonl";

/// Size in MB at which an appended file rotates.
const LEDGER_MAX_MB: u64 = 256;

/// One `roko.loop_audit/1` row: the common fields and its kind's own.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LoopAuditRecord {
    /// Always [`LOOP_AUDIT_SCHEMA`].
    pub schema_version: String,
    /// The row's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    /// When it was written (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<String>,
    /// The loop, e.g. `L-know`.
    pub loop_id: String,
    /// The harness commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub harness_sha: Option<String>,
    /// The config fingerprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_hash: Option<String>,
    /// The assignment epoch, e.g. the UTC day.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_epoch: Option<String>,
    /// The run that produced the row: `Some(None)` writes `null`, for a row
    /// no run produced (a scheduled health pass); `None` leaves it out, as
    /// S01's abridged examples do.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub run_id: Option<Option<String>>,
    /// The kind and its fields.
    #[serde(flatten)]
    pub row: LoopAuditRow,
}

/// A field that may be `null`: present, even as `null`, is `Some`.
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

/// A loop-audit row's kind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum LoopAuditRow {
    /// The digested `LoopSpec`.
    #[serde(rename = "loop.registered")]
    Registered(RegisteredRow),
    /// One evaluation's health.
    #[serde(rename = "loop.health")]
    Health(HealthRow),
    /// A state change.
    #[serde(rename = "loop.transition")]
    Transition(TransitionRow),
    /// A canary trace.
    #[serde(rename = "loop.canary")]
    Canary(CanaryRow),
    /// One paired replay for ι_beh.
    #[serde(rename = "loop.paired_replay")]
    PairedReplay(PairedReplayRow),
}

/// `loop.registered`: the digested `LoopSpec` (S03 §4.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegisteredRow {
    /// Contract version.
    pub version: u32,
    /// The decision layer.
    pub layer: String,
    /// The parent loop, for a nested one.
    #[serde(default)]
    pub nested_in: Option<String>,
    /// The executed decision's code path.
    pub decision_point: String,
    /// The exposure receipt.
    pub receipt: ReceiptKind,
    /// `active`, `observe_only` or `retired`.
    pub lifecycle: String,
    /// Whether demotions change the executed policy.
    pub enforce: bool,
    /// Audited but never demoted.
    pub exempt: bool,
    /// Digest of the registered spec.
    pub spec_digest: String,
}

/// `loop.health`: one evaluation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthRow {
    /// The audit state.
    pub state: AuditState,
    /// The state's reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<ReasonCode>,
    /// The holdout rate.
    pub h: f64,
    /// Opportunities.
    pub n_opp: u64,
    /// Learned-arm opportunities.
    #[serde(rename = "n_L")]
    pub n_learned: u64,
    /// Default-arm opportunities.
    #[serde(rename = "n_D")]
    pub n_default: u64,
    /// ε and its decomposition.
    pub eps: EpsilonFields,
    /// ι and its floor.
    pub iota: IotaFields,
    /// β.
    pub beta: BetaFields,
    /// The layer's SRM e-value.
    pub srm_evalue: f64,
    /// The placebo loop has not moved.
    pub placebo_ok: bool,
    /// Where the reason comes from: `log`, `declared` or `measured`.
    pub evidence: String,
}

/// A health row's ε fields.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EpsilonFields {
    /// ε̂.
    pub est: f64,
    /// Its sequence's upper end.
    pub ucb: f64,
    /// ε_read.
    pub read: f64,
    /// ε_reach.
    pub reach: f64,
    /// ε_honest.
    pub honest: f64,
    /// ε_receipt.
    pub receipt: f64,
}

/// A health row's ι fields.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IotaFields {
    /// ι.
    pub act: f64,
    /// The A/A floor.
    pub aa: f64,
    /// ι_net.
    pub net: f64,
    /// ι_net's sequence's lower end.
    pub lcb: f64,
}

/// A health row's β fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BetaFields {
    /// β̂; `null` while β is not judged.
    #[serde(default)]
    pub est: Option<f64>,
    /// The sequence's lower end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lcb: Option<f64>,
    /// The sequence's upper end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ucb: Option<f64>,
    /// Why β is not judged, e.g. `eps_below_min`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// `loop.transition`: a state change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionRow {
    /// The state left.
    pub from: AuditState,
    /// The state entered.
    pub to: AuditState,
    /// The new state's reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<ReasonCode>,
    /// The rule that fired.
    pub rule: String,
    /// The evidence snapshot.
    pub evidence: String,
    /// `auditor` or `human`.
    pub actor: Actor,
    /// The repair it starts (S03 §4.7's R1–R4).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repair: Option<String>,
}

/// `loop.canary`: one canary trace.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanaryRow {
    /// The canary's nonce.
    pub nonce: String,
    /// It ran on a dry-run plan.
    pub dry_run: bool,
    /// The first probe that failed, e.g. `P4`; `null` when all passed.
    #[serde(default)]
    pub first_failure: Option<String>,
    /// The probes run, in order.
    pub probes: Vec<ProbeRow>,
    /// What the trace spent.
    pub cost_usd: f64,
}

/// One canary probe's result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProbeRow {
    /// The probe, `P1` to `P7`.
    pub p: String,
    /// It passed.
    pub ok: bool,
    /// What it saw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

/// `loop.paired_replay`: one ι_beh pair (S03 §4.4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairedReplayRow {
    /// The replayed attempt.
    pub attempt_key: String,
    /// `[learned, default]`, or `[default, default]` for the A/A floor.
    pub arms: [String; 2],
    /// The behaviours' distance.
    pub distance: f64,
    /// What the pair cost.
    pub cost_usd: f64,
}

/// A fault flag's kind (S03 §4.9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultKind {
    /// The reader returns empty state.
    Cut,
    /// The executed action differs from the learned one under its label.
    Mask,
    /// The state is replaced by constant posteriors.
    Degenerate,
    /// The arm is drawn after the decision.
    LabelOnly,
    /// The reader is pinned to an old state version.
    Stale,
    /// The receipt is dropped.
    Unlogged,
    /// The learned proposal is replaced by an adversarial one.
    Harmful,
}

/// What happened to a fault flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultEvent {
    /// The flag was set.
    Armed,
    /// A read site applied it to a decision.
    Hit,
    /// Its time or decision budget ran out.
    Expired,
    /// It was cleared before then.
    Cleared,
}

/// Who set a fault flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultActor {
    /// serve's admin route.
    Admin,
    /// `ROKO_FAULTS=1` in the CLI.
    Env,
}

/// One `roko.fault/1` row: a fault flag's ground truth (S01 §5.10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FaultRecord {
    /// Always [`FAULT_SCHEMA`].
    pub schema_version: String,
    /// The row's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    /// The run writer's sequence number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    /// When it was written (RFC 3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<String>,
    /// Always `loop.fault`.
    pub kind: String,
    /// What happened.
    pub event: FaultEvent,
    /// The flag's id.
    pub fault_id: String,
    /// The faulted loop.
    pub loop_id: String,
    /// The fault.
    pub fault: FaultKind,
    /// The flag's time to live, at most 1800 s.
    pub ttl_s: u64,
    /// The decisions it may affect.
    pub max_decisions: u64,
    /// The decisions it affected so far.
    pub decisions_affected: u64,
    /// Who set it.
    pub actor: FaultActor,
    /// It runs on dry-run plans only.
    pub dry_run: bool,
    /// HARMFUL's spend cap for the run, at most $1.50 (S03 §4.9); absent for
    /// the dry-run kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spend_cap_usd: Option<f64>,
}

/// Append one row as a JSON line to `path`, under its `*.jsonl.lock`.
///
/// # Errors
///
/// Returns the serializer's or the file system's error.
pub fn append_row<T: Serialize>(path: &Path, row: &T) -> std::io::Result<()> {
    let line = serde_json::to_vec(row).map_err(std::io::Error::other)?;
    roko_fs::log_rotation::append_jsonl_line_sync(path, &line, LEDGER_MAX_MB).map(|_| ())
}

/// The cross-run loop-audit ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ledger {
    path: PathBuf,
}

impl Ledger {
    /// The ledger at `path`.
    #[must_use]
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The ledger of the learn directory `learn_dir`
    /// (`.roko/learn/loop-audit.jsonl`).
    #[must_use]
    pub fn in_learn_dir(learn_dir: &Path) -> Self {
        Self::at(learn_dir.join(LOOP_AUDIT_FILE))
    }

    /// Its file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Append `record`.
    ///
    /// # Errors
    ///
    /// Returns the serializer's or the file system's error.
    pub fn append(&self, record: &LoopAuditRecord) -> std::io::Result<()> {
        append_row(&self.path, record)
    }

    /// Every row, in order; a line that does not parse is skipped. A
    /// missing file has none.
    ///
    /// # Errors
    ///
    /// Returns the file system's error other than a missing file.
    pub fn read(&self) -> std::io::Result<Vec<LoopAuditRecord>> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        Ok(text
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect())
    }
}

/// The latest `loop.health` row of each loop in `records`.
#[must_use]
pub fn latest_health(records: &[LoopAuditRecord]) -> BTreeMap<&str, &HealthRow> {
    let mut latest = BTreeMap::new();
    for record in records {
        if let LoopAuditRow::Health(health) = &record.row {
            latest.insert(record.loop_id.as_str(), health);
        }
    }
    latest
}

/// The `loop.transition` rows written at or after `since`; a row whose `ts`
/// is missing or does not parse is left out.
#[must_use]
pub fn transitions_since(
    records: &[LoopAuditRecord],
    since: DateTime<Utc>,
) -> Vec<&LoopAuditRecord> {
    records
        .iter()
        .filter(|record| matches!(record.row, LoopAuditRow::Transition(_)))
        .filter(|record| {
            record
                .ts
                .as_deref()
                .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
                .is_some_and(|ts| ts >= since)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// S01 §5.10's three example rows (S03 §5 cites them), verbatim.
    const EXAMPLES: [&str; 3] = [
        r#"{"schema_version":"roko.loop_audit/1","kind":"loop.health","loop_id":"L-know","state":"flagged","reason":"dormant:unlogged",
 "h":0.5,"n_opp":412,"n_L":331,"n_D":81,"eps":{"est":0.0,"ucb":0.02,"read":1.0,"reach":1.0,"honest":1.0,"receipt":0.0},
 "iota":{"act":0.94,"aa":0.0,"net":0.94,"lcb":0.9},"beta":{"est":null,"reason":"eps_below_min"},
 "srm_evalue":1.3,"placebo_ok":true,"evidence":"log"}"#,
        r#"{"schema_version":"roko.loop_audit/1","kind":"loop.transition","loop_id":"L-bid","from":"probation","to":"flagged",
 "reason":"dormant:degenerate","rule":"degeneracy: posterior spread 0 over 7 sections","evidence":"health:8f2c",
 "actor":"auditor","repair":"R4"}"#,
        r#"{"schema_version":"roko.loop_audit/1","kind":"loop.canary","loop_id":"L-rag11","nonce":"c-5d1e","dry_run":true,
 "first_failure":"P4","probes":[{"p":"P1","ok":true},{"p":"P2","ok":true},{"p":"P3","ok":true},{"p":"P4","ok":false,
 "evidence":"strategy drawn after plan()"}],"cost_usd":0.0}"#,
    ];

    /// S03 §5 / S01 §5.10: each example row parses into its kind and
    /// re-serializes to an equal JSON value, and the ledger file round-trips
    /// rows with the common fields, giving the latest health per loop and
    /// the transitions since a time.
    #[test]
    fn loop_audit_rows_round_trip_spec_examples() {
        let kinds = ["loop.health", "loop.transition", "loop.canary"];
        for (example, kind) in EXAMPLES.iter().zip(kinds) {
            let original: serde_json::Value = serde_json::from_str(example).expect("an example");
            let record: LoopAuditRecord = serde_json::from_str(example).expect("a row");
            let again = serde_json::to_value(&record).expect("serialize");
            assert_eq!(again, original, "{kind}");
            assert_eq!(again["kind"], kind);
        }

        let temp = tempfile::tempdir().expect("tempdir");
        let ledger = Ledger::in_learn_dir(temp.path());
        let mut health: LoopAuditRecord = serde_json::from_str(EXAMPLES[0]).expect("a row");
        health.ts = Some("2026-10-03T10:00:00Z".to_string());
        health.harness_sha = Some("e55d4c20f".to_string());
        health.run_id = Some(None);
        let mut later = health.clone();
        later.ts = Some("2026-10-03T11:00:00Z".to_string());
        if let LoopAuditRow::Health(row) = &mut later.row {
            row.h = 0.2;
        }
        let mut transition: LoopAuditRecord = serde_json::from_str(EXAMPLES[1]).expect("a row");
        transition.ts = Some("2026-10-03T10:30:00Z".to_string());
        for record in [&health, &transition, &later] {
            ledger.append(record).expect("append");
        }
        let read = ledger.read().expect("read");
        assert_eq!(read, [health.clone(), transition, later]);
        let line = std::fs::read_to_string(ledger.path()).expect("the file");
        assert!(
            line.lines()
                .next()
                .is_some_and(|first| first.contains(r#""run_id":null"#))
        );

        let latest = latest_health(&read);
        assert_eq!(latest.get("L-know").map(|row| row.h), Some(0.2));
        let since = DateTime::parse_from_rfc3339("2026-10-03T10:15:00Z")
            .expect("a time")
            .with_timezone(&Utc);
        let moved = transitions_since(&read, since);
        assert_eq!(moved.len(), 1);
        assert_eq!(moved[0].loop_id, "L-bid");
        let none = since + chrono::Duration::hours(1);
        assert!(transitions_since(&read, none).is_empty());
    }

    /// A fault row round-trips with its envelope.
    #[test]
    fn fault_rows_round_trip() {
        let row = FaultRecord {
            schema_version: FAULT_SCHEMA.to_string(),
            record_id: Some("f-1".to_string()),
            seq: Some(3),
            ts: Some("2026-10-03T10:00:00Z".to_string()),
            kind: "loop.fault".to_string(),
            event: FaultEvent::Armed,
            fault_id: "fault-1".to_string(),
            loop_id: "L-know".to_string(),
            fault: FaultKind::Cut,
            ttl_s: 600,
            max_decisions: 30,
            decisions_affected: 0,
            actor: FaultActor::Env,
            dry_run: true,
            spend_cap_usd: None,
        };
        let json = serde_json::to_value(&row).expect("serialize");
        assert_eq!(json["fault"], "cut");
        assert_eq!(json["event"], "armed");
        let again: FaultRecord = serde_json::from_value(json).expect("parse");
        assert_eq!(again, row);
    }
}
