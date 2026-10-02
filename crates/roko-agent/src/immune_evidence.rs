//! Strict bounded storage for quarantined immune evidence.
//!
//! This ledger deliberately does not use the general append-only substrate:
//! an enforcement boundary must not replay an unbounded journal before it can
//! deny a suspicious provider or tool result.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use roko_core::{Body, ContentHash, Kind, Provenance, Signal};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub(crate) const IMMUNE_EVIDENCE_RELATIVE_PATH: &str = ".roko/immune/quarantine/evidence.json";
pub(crate) const AGENT_CONTROLS_RELATIVE_PATH: &str = ".roko/immune/agent-controls.json";
pub(crate) const AGENT_CONTROL_RELEASES_RELATIVE_PATH: &str =
    ".roko/immune/agent-control-releases.jsonl";
pub(crate) const AGENT_ISOLATION_CONTROL_KIND: &str = "roko.security.immune.agent_isolation";
/// Reason code of the controls the provider boundary writes.
pub(crate) const PROVIDER_CONTAINMENT_REASON: &str = "provider_output_immune_containment";
pub(crate) const MAX_IMMUNE_EVIDENCE_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_IMMUNE_EVIDENCE_SIGNALS: usize = 200;
pub(crate) const MAX_IMMUNE_LABEL_BYTES: usize = 256;

const IMMUNE_EVIDENCE_SCHEMA_VERSION: u32 = 1;
/// Format of the control ledger file itself: a map of controls by agent.
const AGENT_CONTROL_LEDGER_SCHEMA_VERSION: u32 = 1;
/// Schema of a control written before backlog 1108, with no expiry. Such a
/// control stays in force until an operator releases it.
const LEGACY_AGENT_CONTROL_SCHEMA_VERSION: u32 = 1;
/// Schema of a control that carries `isolated_at_ms` and `expires_at_ms`.
const AGENT_CONTROL_SCHEMA_VERSION: u32 = 2;
/// How long a new isolation control denies its agent: 7 days (decision 1107).
// TODO(backlog 1108): read `[safety] isolation_ttl_secs` once the config has
// a safety section.
pub(crate) const DEFAULT_ISOLATION_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const MAX_AGENT_CONTROL_LEDGER_BYTES: u64 = 4 * 1024 * 1024;
const MAX_AGENT_CONTROLS: usize = 512;
const MAX_SIGNAL_TAGS: usize = 128;
const MAX_CONTROL_REASON_BYTES: usize = 64;

#[derive(Clone, Debug, PartialEq, Serialize)]
struct ImmuneEvidenceLedger {
    schema_version: u32,
    signals: BTreeMap<String, Signal>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ImmuneEvidenceLedgerDecoded {
    schema_version: u32,
    signals: BTreeMap<String, Signal>,
}

impl<'de> Deserialize<'de> for ImmuneEvidenceLedger {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = Value::deserialize(deserializer)?;
        validate_immune_evidence_wire(&raw).map_err(serde::de::Error::custom)?;
        let decoded: ImmuneEvidenceLedgerDecoded =
            serde_json::from_value(raw).map_err(serde::de::Error::custom)?;
        Ok(Self {
            schema_version: decoded.schema_version,
            signals: decoded.signals,
        })
    }
}

impl Default for ImmuneEvidenceLedger {
    fn default() -> Self {
        Self {
            schema_version: IMMUNE_EVIDENCE_SCHEMA_VERSION,
            signals: BTreeMap::new(),
        }
    }
}

fn validate_immune_evidence_wire(raw: &Value) -> Result<(), String> {
    let ledger = exact_object(
        raw,
        "immune evidence ledger",
        &["schema_version", "signals"],
    )?;
    let signals = ledger
        .get("signals")
        .and_then(Value::as_object)
        .ok_or_else(|| "immune evidence ledger signals must be an object".to_string())?;
    for signal in signals.values() {
        validate_signal_wire(signal)?;
    }
    Ok(())
}

fn validate_signal_wire(raw: &Value) -> Result<(), String> {
    let signal = exact_object(
        raw,
        "immune evidence Signal",
        &[
            "id",
            "fingerprint",
            "kind",
            "body",
            "created_at_ms",
            "decay",
            "provenance",
            "score",
            "lineage",
            "tags",
            "attestation",
            "emotional_tag",
            "balance",
            "status",
            "access_count",
            "demurrage_paid",
        ],
    )?;

    if let Some(body) = signal.get("body") {
        // Validate only the Body envelope. `data` remains intentionally opaque
        // so arbitrary provider/tool JSON arrays and objects are preserved.
        exact_object(body, "immune evidence Body", &["format", "data"])?;
    }
    if let Some(provenance) = signal.get("provenance") {
        validate_provenance_wire(provenance)?;
    }
    if let Some(attestation) = non_null(signal.get("attestation")) {
        validate_attestation_wire(attestation)?;
    }
    if let Some(fingerprint) = non_null(signal.get("fingerprint")) {
        exact_object(
            fingerprint,
            "immune evidence fingerprint",
            &["vector", "encoder_version"],
        )?;
    }
    if let Some(score) = signal.get("score") {
        exact_object(
            score,
            "immune evidence score",
            &[
                "confidence",
                "novelty",
                "utility",
                "reputation",
                "precision",
                "salience",
                "coherence",
            ],
        )?;
    }
    if let Some(decay) = signal.get("decay") {
        exact_object(
            decay,
            "immune evidence decay",
            &["kind", "half_life_ms", "ttl_ms", "strength", "scale_ms"],
        )?;
    }
    if let Some(emotional_tag) = non_null(signal.get("emotional_tag")) {
        let emotional = exact_object(
            emotional_tag,
            "immune evidence emotional tag",
            &["pad", "intensity", "trigger", "mood_snapshot"],
        )?;
        for field in ["pad", "mood_snapshot"] {
            if let Some(pad) = emotional.get(field) {
                exact_object(
                    pad,
                    "immune evidence PAD vector",
                    &["pleasure", "arousal", "dominance"],
                )?;
            }
        }
    }
    Ok(())
}

fn validate_provenance_wire(raw: &Value) -> Result<(), String> {
    let provenance = exact_object(
        raw,
        "immune evidence provenance",
        &[
            "author",
            "trust",
            "taint",
            "taint_info",
            "session",
            "taint_level",
            "trust_origin",
        ],
    )?;
    if let Some(taint) = non_null(provenance.get("taint"))
        && taint.is_object()
    {
        exact_object(
            taint,
            "immune evidence taint",
            &["kind", "detail", "threshold_ms", "inherited_from"],
        )?;
    }
    if let Some(taint_info) = non_null(provenance.get("taint_info")) {
        exact_object(
            taint_info,
            "immune evidence taint info",
            &["category", "detail", "inherited_from", "taint_level"],
        )?;
    }
    Ok(())
}

fn validate_attestation_wire(raw: &Value) -> Result<(), String> {
    let attestation = exact_object(
        raw,
        "immune evidence attestation",
        &["signature", "public_key", "chain_attestation"],
    )?;
    if let Some(chain) = non_null(attestation.get("chain_attestation")) {
        exact_object(
            chain,
            "immune evidence chain attestation",
            &["chain_id", "tx_hash", "block_number"],
        )?;
    }
    Ok(())
}

fn exact_object<'a>(
    raw: &'a Value,
    label: &str,
    allowed: &[&str],
) -> Result<&'a serde_json::Map<String, Value>, String> {
    let object = raw
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("{label} contains an unknown field"));
    }
    Ok(object)
}

fn non_null(value: Option<&Value>) -> Option<&Value> {
    value.filter(|value| !value.is_null())
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentControlLedger {
    schema_version: u32,
    controls: BTreeMap<String, Signal>,
}

impl Default for AgentControlLedger {
    fn default() -> Self {
        Self {
            schema_version: AGENT_CONTROL_LEDGER_SCHEMA_VERSION,
            controls: BTreeMap::new(),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentControlBody {
    schema_version: u32,
    agent_id: String,
    state: String,
    reason: String,
    #[serde(default)]
    isolated_at_ms: Option<u64>,
    #[serde(default)]
    expires_at_ms: Option<u64>,
}

pub(crate) fn immune_evidence_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(IMMUNE_EVIDENCE_RELATIVE_PATH)
}

pub(crate) fn agent_controls_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(AGENT_CONTROLS_RELATIVE_PATH)
}

/// The append-only audit file of released isolation controls beneath a
/// workspace root.
#[must_use]
pub fn agent_control_releases_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(AGENT_CONTROL_RELEASES_RELATIVE_PATH)
}

/// One isolation control, as [`list_agent_controls`] reports it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentControlEntry {
    /// Agent the control denies before dispatch.
    pub agent_id: String,
    /// Control state (`isolated`).
    pub state: String,
    /// Reason code: `provider_output_immune_containment` for a control the
    /// provider boundary wrote.
    pub reason: String,
    /// Full hex id of the control Signal.
    pub control_id: String,
    /// When the control was written, in Unix milliseconds; `None` for a
    /// control written before controls expired.
    pub isolated_at_ms: Option<u64>,
    /// When the control stops denying its agent, in Unix milliseconds;
    /// `None` for a control that stays until it is released.
    pub expires_at_ms: Option<u64>,
}

/// The audit record of one released control: one line of
/// `.roko/immune/agent-control-releases.jsonl`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleasedControl {
    /// When the control was released, in Unix milliseconds.
    pub released_at_ms: u64,
    /// Agent the control denied.
    pub agent_id: String,
    /// Full hex id of the released control Signal.
    pub control_id: String,
    /// Who released it.
    pub by: String,
    /// Why it was released.
    pub reason: String,
}

/// Build the isolation control Signal for `agent_id` with reason code
/// `reason`, written at `isolated_at_ms` and in force for `ttl`. The provider
/// boundary writes the same Signal, with reason
/// [`PROVIDER_CONTAINMENT_REASON`], for its own containments.
pub(crate) fn agent_isolation_control(
    agent_id: &str,
    reason: &str,
    isolated_at_ms: u64,
    ttl: Duration,
) -> io::Result<Signal> {
    validate_boundary_label(agent_id, "agent ID")?;
    validate_control_reason(reason)?;
    let ttl_ms = u64::try_from(ttl.as_millis()).unwrap_or(u64::MAX);
    if ttl_ms == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "an isolation control needs a positive lifetime",
        ));
    }
    control_signal(&crate::immune_boundary::AgentIsolationControl {
        schema_version: AGENT_CONTROL_SCHEMA_VERSION,
        agent_id: agent_id.to_string(),
        state: "isolated".to_string(),
        reason: reason.to_string(),
        isolated_at_ms: Some(isolated_at_ms),
        expires_at_ms: Some(isolated_at_ms.saturating_add(ttl_ms)),
    })
}

/// The deterministic control the provider boundary wrote for `agent_id`
/// before backlog 1108 (schema 1, no expiry). Version 1 receipts bind it.
pub(crate) fn legacy_agent_isolation_control(agent_id: &str) -> io::Result<Signal> {
    validate_boundary_label(agent_id, "agent ID")?;
    control_signal(&crate::immune_boundary::AgentIsolationControl {
        schema_version: LEGACY_AGENT_CONTROL_SCHEMA_VERSION,
        agent_id: agent_id.to_string(),
        state: "isolated".to_string(),
        reason: PROVIDER_CONTAINMENT_REASON.to_string(),
        isolated_at_ms: None,
        expires_at_ms: None,
    })
}

fn control_signal(control: &crate::immune_boundary::AgentIsolationControl) -> io::Result<Signal> {
    let body = Body::from_json(control)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    Ok(
        Signal::builder(Kind::Custom(AGENT_ISOLATION_CONTROL_KIND.to_string()))
            .body(body)
            .provenance(Provenance::trusted("immune-provider-boundary"))
            .tag("agent_id", &control.agent_id)
            .tag("control_state", "isolated")
            .build(),
    )
}

/// Isolate `agent_id` on an operator's behalf: write the same control the
/// provider boundary writes, with reason code `reason` (lowercase letters,
/// digits and underscores), in force for [`DEFAULT_ISOLATION_TTL`]. An agent
/// that is already isolated keeps its control. Returns the control in force.
///
/// # Errors
///
/// Fails for an invalid agent id or reason code, a full or invalid ledger,
/// or a filesystem error.
pub fn isolate_agent(
    workspace_root: &Path,
    agent_id: &str,
    reason: &str,
) -> io::Result<AgentControlEntry> {
    let now_ms = unix_now_ms();
    let control = agent_isolation_control(agent_id, reason, now_ms, DEFAULT_ISOLATION_TTL)?;
    let key = agent_control_key(agent_id);
    roko_fs::with_locked_json_transaction_bounded::<AgentControlLedger, _, io::Error, _>(
        &agent_controls_path(workspace_root),
        MAX_AGENT_CONTROL_LEDGER_BYTES,
        |ledger| {
            validate_agent_control_ledger(ledger)?;
            prune_expired_controls(ledger, now_ms)?;
            if let Some(existing) = ledger.controls.get(&key) {
                return control_entry(existing);
            }
            if ledger.controls.len() >= MAX_AGENT_CONTROLS {
                return Err(io::Error::other(format!(
                    "agent control ledger reached its {MAX_AGENT_CONTROLS}-entry capacity"
                )));
            }
            let entry = control_entry(&control)?;
            ledger.controls.insert(key, control);
            validate_agent_control_ledger(ledger)?;
            Ok(entry)
        },
    )
}

/// List the isolation controls in force, by agent id. Expired controls are
/// left out (the next write prunes them).
///
/// # Errors
///
/// Fails closed on an invalid or unreadable ledger.
pub fn list_agent_controls(workspace_root: &Path) -> io::Result<Vec<AgentControlEntry>> {
    let now_ms = unix_now_ms();
    let mut entries =
        roko_fs::with_locked_json_transaction_bounded::<AgentControlLedger, _, io::Error, _>(
            &agent_controls_path(workspace_root),
            MAX_AGENT_CONTROL_LEDGER_BYTES,
            |ledger| {
                validate_agent_control_ledger(ledger)?;
                let mut entries = Vec::new();
                for control in ledger.controls.values() {
                    if !is_expired(control, now_ms)? {
                        entries.push(control_entry(control)?);
                    }
                }
                Ok(entries)
            },
        )?;
    entries.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
    Ok(entries)
}

/// Release the isolation control of `agent_id`: remove it from the ledger
/// and append one audit line (time, agent id, control id, `by`, `reason`)
/// to [`agent_control_releases_path`], both under the ledger's lock. The
/// audit line is written first, so no release goes unrecorded. Releasing an
/// agent with no control in force (none, or an expired one) returns
/// `Ok(None)` and writes no audit line.
///
/// # Errors
///
/// [`io::ErrorKind::InvalidInput`] for a malformed agent id, `by` or
/// `reason` (each must be 1..=256 bytes without control characters);
/// [`io::ErrorKind::InvalidData`] for an invalid ledger; or a filesystem
/// error.
pub fn release_agent_control(
    workspace_root: &Path,
    agent_id: &str,
    by: &str,
    reason: &str,
) -> io::Result<Option<ReleasedControl>> {
    let invalid_input =
        |error: io::Error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string());
    validate_boundary_label(agent_id, "agent ID").map_err(invalid_input)?;
    validate_boundary_label(by, "release principal").map_err(invalid_input)?;
    validate_boundary_label(reason, "release reason").map_err(invalid_input)?;
    let key = agent_control_key(agent_id);
    let audit_path = agent_control_releases_path(workspace_root);
    let now_ms = unix_now_ms();
    roko_fs::with_locked_json_transaction_bounded::<AgentControlLedger, _, io::Error, _>(
        &agent_controls_path(workspace_root),
        MAX_AGENT_CONTROL_LEDGER_BYTES,
        |ledger| {
            validate_agent_control_ledger(ledger)?;
            prune_expired_controls(ledger, now_ms)?;
            let Some(control) = ledger.controls.get(&key) else {
                return Ok(None);
            };
            let released = ReleasedControl {
                released_at_ms: now_ms,
                agent_id: agent_id.to_string(),
                control_id: control.id.to_hex(),
                by: by.to_string(),
                reason: reason.to_string(),
            };
            append_release_audit(&audit_path, &released)?;
            ledger.controls.remove(&key);
            Ok(Some(released))
        },
    )
}

fn append_release_audit(path: &Path, released: &ReleasedControl) -> io::Result<()> {
    let mut line = serde_json::to_vec(released)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    line.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(&line)?;
    file.sync_data()
}

fn control_entry(control: &Signal) -> io::Result<AgentControlEntry> {
    let body = decode_control_body(control)?;
    Ok(AgentControlEntry {
        agent_id: body.agent_id,
        state: body.state,
        reason: body.reason,
        control_id: control.id.to_hex(),
        isolated_at_ms: body.isolated_at_ms,
        expires_at_ms: body.expires_at_ms,
    })
}

fn decode_control_body(control: &Signal) -> io::Result<AgentControlBody> {
    control
        .body
        .as_json()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

/// Whether `control` expired by `now_ms`. A schema-1 control never expires.
fn is_expired(control: &Signal, now_ms: u64) -> io::Result<bool> {
    let expires_at_ms = decode_control_body(control)?.expires_at_ms;
    Ok(expires_at_ms.is_some_and(|expires_at_ms| expires_at_ms <= now_ms))
}

/// Remove the controls that expired by `now_ms`, so they neither deny their
/// agents nor count towards the ledger's capacity (decision 1107).
fn prune_expired_controls(ledger: &mut AgentControlLedger, now_ms: u64) -> io::Result<()> {
    let mut expired = Vec::new();
    for (key, control) in &ledger.controls {
        if is_expired(control, now_ms)? {
            expired.push(key.clone());
        }
    }
    if !expired.is_empty() {
        tracing::info!(
            pruned = expired.len(),
            "pruned expired immune isolation controls"
        );
    }
    for key in expired {
        ledger.controls.remove(&key);
    }
    Ok(())
}

/// Whether `control` is a valid isolation control for `agent_id` that is
/// still in force at `now_ms`.
pub(crate) fn is_live_agent_control(control: &Signal, agent_id: &str, now_ms: u64) -> bool {
    validate_agent_control_signal(control).is_ok_and(|controlled| controlled == agent_id)
        && is_expired(control, now_ms).is_ok_and(|expired| !expired)
}

pub(crate) fn unix_now_ms() -> u64 {
    let elapsed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}

/// A control's reason is a short code of lowercase ASCII letters, digits and
/// underscores, so it never carries provider or operator text.
fn validate_control_reason(reason: &str) -> io::Result<()> {
    let is_code = !reason.is_empty()
        && reason.len() <= MAX_CONTROL_REASON_BYTES
        && reason
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
    if is_code {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "agent control reason must be 1..={MAX_CONTROL_REASON_BYTES} lowercase letters, \
                 digits or underscores"
            ),
        ))
    }
}

pub(crate) fn validate_boundary_label(label: &str, field: &str) -> io::Result<()> {
    if label.trim().is_empty()
        || label.len() > MAX_IMMUNE_LABEL_BYTES
        || label.chars().any(char::is_control)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "immune {field} must contain 1..={MAX_IMMUNE_LABEL_BYTES} bytes without control characters"
            ),
        ));
    }
    Ok(())
}

pub(crate) fn persist_evidence_signals(
    workspace_root: &Path,
    signals: &[Signal],
) -> io::Result<()> {
    roko_fs::with_locked_json_transaction_bounded::<ImmuneEvidenceLedger, _, io::Error, _>(
        &immune_evidence_path(workspace_root),
        MAX_IMMUNE_EVIDENCE_BYTES,
        |ledger| {
            validate_ledger(ledger)?;
            for signal in signals {
                validate_signal(signal)?;
                let key = signal.id.to_string();
                match ledger.signals.get(&key) {
                    Some(existing) if existing == signal => {}
                    Some(_) => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "immune evidence hash collides with a different signal",
                        ));
                    }
                    None if ledger.signals.len() >= MAX_IMMUNE_EVIDENCE_SIGNALS => {
                        return Err(io::Error::other(format!(
                            "immune evidence ledger reached its {MAX_IMMUNE_EVIDENCE_SIGNALS}-entry capacity"
                        )));
                    }
                    None => {
                        ledger.signals.insert(key, signal.clone());
                    }
                }
            }
            validate_ledger(ledger)
        },
    )
}

/// Persist `control` and return the control in force for its agent: an
/// agent that is already isolated keeps its live control, so isolation is
/// monotonic. Expired controls are pruned first and do not count towards
/// the ledger's capacity.
pub(crate) fn persist_agent_control(
    workspace_root: &Path,
    control: &Signal,
) -> io::Result<Signal> {
    let agent_id = validate_agent_control_signal(control)?;
    let key = agent_control_key(&agent_id);
    let now_ms = unix_now_ms();
    roko_fs::with_locked_json_transaction_bounded::<AgentControlLedger, _, io::Error, _>(
        &agent_controls_path(workspace_root),
        MAX_AGENT_CONTROL_LEDGER_BYTES,
        |ledger| {
            validate_agent_control_ledger(ledger)?;
            prune_expired_controls(ledger, now_ms)?;
            if let Some(existing) = ledger.controls.get(&key) {
                return Ok(existing.clone());
            }
            if ledger.controls.len() >= MAX_AGENT_CONTROLS {
                return Err(io::Error::other(format!(
                    "agent control ledger reached its {MAX_AGENT_CONTROLS}-entry capacity"
                )));
            }
            ledger.controls.insert(key, control.clone());
            validate_agent_control_ledger(ledger)?;
            Ok(control.clone())
        },
    )
}

pub(crate) fn get_agent_control(
    workspace_root: &Path,
    agent_id: &str,
) -> io::Result<Option<Signal>> {
    validate_boundary_label(agent_id, "agent ID")?;
    let key = agent_control_key(agent_id);
    let now_ms = unix_now_ms();
    roko_fs::with_locked_json_transaction_bounded::<AgentControlLedger, _, io::Error, _>(
        &agent_controls_path(workspace_root),
        MAX_AGENT_CONTROL_LEDGER_BYTES,
        |ledger| {
            validate_agent_control_ledger(ledger)?;
            // Expired controls neither deny nor fill the ledger: only 512
            // live controls make unknown agents fail closed.
            prune_expired_controls(ledger, now_ms)?;
            let control = ledger.controls.get(&key).cloned();
            if control.is_none() && ledger.controls.len() >= MAX_AGENT_CONTROLS {
                return Err(io::Error::other(
                    "agent control authority is saturated; unknown agents fail closed",
                ));
            }
            Ok(control)
        },
    )
}

pub(crate) fn get_evidence_signal(
    workspace_root: &Path,
    id: &ContentHash,
) -> io::Result<Option<Signal>> {
    roko_fs::with_locked_json_transaction_bounded::<ImmuneEvidenceLedger, _, io::Error, _>(
        &immune_evidence_path(workspace_root),
        MAX_IMMUNE_EVIDENCE_BYTES,
        |ledger| {
            validate_ledger(ledger)?;
            Ok(ledger.signals.get(&id.to_string()).cloned())
        },
    )
}

pub(crate) fn query_evidence_signals(
    workspace_root: &Path,
    kind: &Kind,
    required_tag: Option<(&str, &str)>,
    limit: usize,
) -> io::Result<Vec<Signal>> {
    roko_fs::with_locked_json_transaction_bounded::<ImmuneEvidenceLedger, _, io::Error, _>(
        &immune_evidence_path(workspace_root),
        MAX_IMMUNE_EVIDENCE_BYTES,
        |ledger| {
            validate_ledger(ledger)?;
            Ok(ledger
                .signals
                .values()
                .filter(|signal| signal.is(kind))
                .filter(|signal| {
                    required_tag
                        .map(|(key, value)| signal.tag(key) == Some(value))
                        .unwrap_or(true)
                })
                .take(limit.min(MAX_IMMUNE_EVIDENCE_SIGNALS))
                .cloned()
                .collect())
        },
    )
}

fn validate_ledger(ledger: &ImmuneEvidenceLedger) -> io::Result<()> {
    let invalid = |message| io::Error::new(io::ErrorKind::InvalidData, message);
    if ledger.schema_version != IMMUNE_EVIDENCE_SCHEMA_VERSION {
        return Err(invalid("unsupported immune evidence ledger schema version"));
    }
    if ledger.signals.len() > MAX_IMMUNE_EVIDENCE_SIGNALS {
        return Err(invalid("immune evidence ledger exceeds entry capacity"));
    }
    for (key, signal) in &ledger.signals {
        validate_signal(signal)?;
        if *key != signal.id.to_string() {
            return Err(invalid(
                "immune evidence ledger key does not match its signal",
            ));
        }
        if signal.is(&Kind::Custom(
            crate::tool_immune::TOOL_BOUNDARY_RECORD_KIND.to_string(),
        )) {
            let record: crate::tool_immune::ToolBoundaryRecord = signal
                .body
                .as_json()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
            let evidence = ledger.signals.get(&record.output.to_string());
            let control = record
                .control
                .and_then(|control| ledger.signals.get(&control.to_string()));
            crate::tool_immune::validate_tool_boundary_receipt(signal, evidence, control)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        }
        if signal.is(&Kind::Custom(
            crate::immune_boundary::PROVIDER_BOUNDARY_RECORD_KIND.to_string(),
        )) {
            let record: crate::immune_boundary::ProviderBoundaryRecord = signal
                .body
                .as_json()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
            let evidence = ledger.signals.get(&record.output.to_string());
            crate::immune_boundary::validate_provider_boundary_receipt(signal, evidence, true)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        }
    }
    Ok(())
}

fn validate_signal(signal: &Signal) -> io::Result<()> {
    if signal.id != signal.content_hash() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "immune evidence signal content hash is invalid",
        ));
    }
    if signal.tags.len() > MAX_SIGNAL_TAGS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "immune evidence signal has too many tags",
        ));
    }
    for (key, value) in &signal.tags {
        validate_boundary_label(key, "tag key")?;
        validate_boundary_label(value, "tag value")?;
    }
    Ok(())
}

fn validate_agent_control_ledger(ledger: &AgentControlLedger) -> io::Result<()> {
    let invalid = |message| io::Error::new(io::ErrorKind::InvalidData, message);
    if ledger.schema_version != AGENT_CONTROL_LEDGER_SCHEMA_VERSION {
        return Err(invalid("unsupported agent control ledger schema version"));
    }
    if ledger.controls.len() > MAX_AGENT_CONTROLS {
        return Err(invalid("agent control ledger exceeds entry capacity"));
    }
    for (key, signal) in &ledger.controls {
        let agent_id = validate_agent_control_signal(signal)?;
        if *key != agent_control_key(&agent_id) {
            return Err(invalid("agent control ledger key does not match its body"));
        }
    }
    Ok(())
}

fn validate_agent_control_signal(signal: &Signal) -> io::Result<String> {
    validate_signal(signal)?;
    if !signal.is(&Kind::Custom(AGENT_ISOLATION_CONTROL_KIND.to_string())) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "agent control has an invalid Signal kind",
        ));
    }
    if signal.provenance != roko_core::Provenance::trusted("immune-provider-boundary") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "agent control has invalid boundary provenance",
        ));
    }
    let body: AgentControlBody = signal
        .body
        .as_json()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    validate_boundary_label(&body.agent_id, "agent ID")?;
    validate_control_reason(&body.reason)?;
    // A schema-1 control is the boundary's own, written before controls
    // expired; a schema-2 control carries a lifetime that ends after it
    // starts.
    let lifetime_valid = match body.schema_version {
        LEGACY_AGENT_CONTROL_SCHEMA_VERSION => {
            body.reason == PROVIDER_CONTAINMENT_REASON
                && body.isolated_at_ms.is_none()
                && body.expires_at_ms.is_none()
        }
        AGENT_CONTROL_SCHEMA_VERSION => matches!(
            (body.isolated_at_ms, body.expires_at_ms),
            (Some(isolated_at_ms), Some(expires_at_ms)) if isolated_at_ms < expires_at_ms
        ),
        _ => false,
    };
    if !lifetime_valid
        || signal.attestation.is_some()
        || body.state != "isolated"
        || signal.tag("agent_id") != Some(body.agent_id.as_str())
        || signal.tag("control_state") != Some("isolated")
        || signal.tags.len() != 2
        || !signal.lineage.is_empty()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "agent control Signal does not match its authority body",
        ));
    }
    Ok(body.agent_id)
}

fn agent_control_key(agent_id: &str) -> String {
    format!(
        "agent-control-{}",
        ContentHash::of(agent_id.as_bytes()).to_hex()
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tempfile::tempdir;

    use super::*;
    use crate::agent::{Agent, AgentResult};

    fn evidence(index: usize) -> Signal {
        Signal::builder(Kind::AgentOutput)
            .body(Body::text(format!("suspect-{index}")))
            .provenance(Provenance::external("test"))
            .tag("source", "test")
            .build()
    }

    fn agent_control(index: usize) -> Signal {
        let agent_id = format!("agent-{index}");
        Signal::builder(Kind::Custom(AGENT_ISOLATION_CONTROL_KIND.to_string()))
            .body(
                Body::from_json(&serde_json::json!({
                    "schema_version": LEGACY_AGENT_CONTROL_SCHEMA_VERSION,
                    "agent_id": agent_id.clone(),
                    "state": "isolated",
                    "reason": "provider_output_immune_containment",
                }))
                .unwrap(),
            )
            .provenance(Provenance::trusted("immune-provider-boundary"))
            .tag("agent_id", &agent_id)
            .tag("control_state", "isolated")
            .build()
    }

    #[test]
    fn malformed_ledger_is_preserved_and_fails_closed() {
        let workspace = tempdir().unwrap();
        let path = immune_evidence_path(workspace.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let malformed = b"{not-valid-evidence-json";
        std::fs::write(&path, malformed).unwrap();

        let error = persist_evidence_signals(workspace.path(), &[evidence(0)]).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(std::fs::read(path).unwrap(), malformed);
    }

    #[test]
    fn oversized_ledger_is_preserved_and_fails_closed() {
        let workspace = tempdir().unwrap();
        let path = immune_evidence_path(workspace.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let oversized = vec![b' '; (MAX_IMMUNE_EVIDENCE_BYTES + 1) as usize];
        std::fs::write(&path, &oversized).unwrap();

        let error = persist_evidence_signals(workspace.path(), &[evidence(0)]).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            std::fs::metadata(path).unwrap().len(),
            oversized.len() as u64
        );
    }

    #[test]
    fn unknown_outer_wire_fields_fail_closed_without_normalizing_arbitrary_body_json() {
        let workspace = tempdir().unwrap();
        let mut signal = Signal::builder(Kind::AgentOutput)
            .body(Body::Json(serde_json::json!({
                "provider_owned": [
                    {"unknown_nested_payload": true},
                    {"any_shape": {"is_preserved": [1, 2, 3]}}
                ]
            })))
            .provenance(Provenance::external("wire-test"))
            .build();
        let signing_key = roko_core::attestation::SigningKey::from_bytes(&[11; 32]);
        signal.attestation = Some(
            roko_core::attestation::sign(&signal, &signing_key).with_chain_attestation(
                roko_core::ChainAttestation {
                    chain_id: 7,
                    tx_hash: [8; 32],
                    block_number: 9,
                },
            ),
        );
        persist_evidence_signals(workspace.path(), &[signal]).unwrap();
        assert_eq!(
            query_evidence_signals(workspace.path(), &Kind::AgentOutput, None, 1)
                .unwrap()
                .len(),
            1,
            "arbitrary Body::Json objects and arrays must remain opaque"
        );

        let path = immune_evidence_path(workspace.path());
        let baseline: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        let assert_preserved_rejection = |mutated: &Value| {
            roko_fs::atomic_write_json(&path, mutated).unwrap();
            let before = std::fs::read(&path).unwrap();
            let error = query_evidence_signals(
                workspace.path(),
                &Kind::AgentOutput,
                None,
                MAX_IMMUNE_EVIDENCE_SIGNALS,
            )
            .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert_eq!(std::fs::read(&path).unwrap(), before);
        };

        let mut unknown_ledger = baseline.clone();
        unknown_ledger
            .as_object_mut()
            .unwrap()
            .insert("unknown_ledger_field".to_string(), Value::Bool(true));
        assert_preserved_rejection(&unknown_ledger);

        for envelope in [
            "signal",
            "body",
            "provenance",
            "taint",
            "score",
            "attestation",
            "chain",
        ] {
            let mut mutated = baseline.clone();
            let signal = mutated
                .get_mut("signals")
                .and_then(Value::as_object_mut)
                .unwrap()
                .values_mut()
                .next()
                .unwrap();
            let target = match envelope {
                "signal" => &mut *signal,
                "body" => signal.get_mut("body").unwrap(),
                "provenance" => signal.get_mut("provenance").unwrap(),
                "taint" => signal
                    .get_mut("provenance")
                    .and_then(|value| value.get_mut("taint"))
                    .unwrap(),
                "score" => signal.get_mut("score").unwrap(),
                "attestation" => signal.get_mut("attestation").unwrap(),
                "chain" => signal
                    .get_mut("attestation")
                    .and_then(|value| value.get_mut("chain_attestation"))
                    .unwrap(),
                _ => unreachable!(),
            };
            target
                .as_object_mut()
                .unwrap()
                .insert("unknown_envelope_field".to_string(), Value::Bool(true));
            assert_preserved_rejection(&mutated);
        }
    }

    #[test]
    fn full_ledger_rejects_new_evidence_without_overwrite() {
        let workspace = tempdir().unwrap();
        let signals = (0..MAX_IMMUNE_EVIDENCE_SIGNALS)
            .map(evidence)
            .collect::<Vec<_>>();
        persist_evidence_signals(workspace.path(), &signals).unwrap();
        let path = immune_evidence_path(workspace.path());
        let before = std::fs::read(&path).unwrap();

        let error =
            persist_evidence_signals(workspace.path(), &[evidence(usize::MAX)]).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(std::fs::read(path).unwrap(), before);
    }

    #[test]
    fn oversized_labels_are_rejected_before_persistence() {
        let workspace = tempdir().unwrap();
        let signal = Signal::builder(Kind::AgentOutput)
            .body(Body::text("suspect"))
            .tag("source", "x".repeat(MAX_IMMUNE_LABEL_BYTES + 1))
            .build();

        let error = persist_evidence_signals(workspace.path(), &[signal]).unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(!immune_evidence_path(workspace.path()).exists());
    }

    #[test]
    fn saturated_agent_control_authority_denies_unknown_agents() {
        let workspace = tempdir().unwrap();
        let controls = (0..MAX_AGENT_CONTROLS)
            .map(agent_control)
            .map(|signal| {
                let body: AgentControlBody = signal.body.as_json().unwrap();
                (agent_control_key(&body.agent_id), signal)
            })
            .collect();
        let ledger = AgentControlLedger {
            schema_version: AGENT_CONTROL_LEDGER_SCHEMA_VERSION,
            controls,
        };
        let path = agent_controls_path(workspace.path());
        roko_fs::atomic_write_json(&path, &ledger).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() < MAX_AGENT_CONTROL_LEDGER_BYTES);

        let error = get_agent_control(workspace.path(), "unknown-agent").unwrap_err();

        assert_eq!(error.kind(), io::ErrorKind::Other);
    }

    #[test]
    fn agent_isolation_control_merge_is_idempotent_and_monotonic() {
        let workspace = tempdir().unwrap();
        let control = agent_control(7);
        persist_agent_control(workspace.path(), &control).unwrap();
        persist_agent_control(workspace.path(), &control).unwrap();

        let body: AgentControlBody = control.body.as_json().unwrap();
        let restored = get_agent_control(workspace.path(), &body.agent_id)
            .unwrap()
            .unwrap();
        assert_eq!(restored, control);
        let ledger: AgentControlLedger = roko_fs::read_json_or_default_strict_bounded(
            &agent_controls_path(workspace.path()),
            MAX_AGENT_CONTROL_LEDGER_BYTES,
        )
        .unwrap();
        assert_eq!(ledger.controls.len(), 1);
    }

    /// Counts its runs and answers with text.
    struct CountingTextAgent(Arc<AtomicUsize>);

    #[async_trait::async_trait]
    impl Agent for CountingTextAgent {
        async fn run(&self, input: &Signal, _ctx: &roko_core::Context) -> AgentResult {
            self.0.fetch_add(1, Ordering::SeqCst);
            AgentResult::ok(input.derive(Kind::AgentOutput, Body::text("answer")).build())
        }

        fn name(&self) -> &str {
            "counting-text-agent"
        }
    }

    /// backlog 1104: an operator sees an isolation, releases it with an
    /// audit record, and the agent runs again.
    #[tokio::test]
    async fn released_control_lets_the_agent_run_again() {
        let workspace = tempdir().unwrap();
        let isolated =
            isolate_agent(workspace.path(), "plan/task#1", "operator_isolation").unwrap();
        assert_eq!(isolated.agent_id, "plan/task#1");
        assert_eq!(isolated.state, "isolated");
        assert_eq!(isolated.reason, "operator_isolation");
        assert_eq!(
            list_agent_controls(workspace.path()).unwrap(),
            vec![isolated.clone()]
        );

        let calls = Arc::new(AtomicUsize::new(0));
        let boundary = crate::immune_boundary::ImmuneScreenedAgent::durable(
            Box::new(CountingTextAgent(Arc::clone(&calls))),
            "plan/task#1",
            workspace.path(),
        );
        let prompt = Signal::builder(Kind::Prompt).body(Body::text("go")).build();
        let denied = boundary.run(&prompt, &roko_core::Context::now()).await;
        assert!(!denied.success);
        assert_eq!(denied.output.tag("immune_reason"), Some("agent_isolated"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let released = release_agent_control(
            workspace.path(),
            "plan/task#1",
            "operator",
            "blank answer, not tamper",
        )
        .unwrap()
        .expect("a control was released");
        assert_eq!(released.agent_id, "plan/task#1");
        assert_eq!(released.control_id, isolated.control_id);
        assert!(list_agent_controls(workspace.path()).unwrap().is_empty());

        let ran = boundary.run(&prompt, &roko_core::Context::now()).await;
        assert!(ran.success, "the released agent must reach its provider");
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        let audit = std::fs::read_to_string(agent_control_releases_path(workspace.path())).unwrap();
        let lines = audit.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 1, "{audit}");
        let recorded: ReleasedControl = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(recorded, released);
    }

    #[test]
    fn releasing_an_unknown_agent_changes_nothing() {
        let workspace = tempdir().unwrap();
        isolate_agent(workspace.path(), "kept-agent", "operator_isolation").unwrap();
        let path = agent_controls_path(workspace.path());
        let before = std::fs::read(&path).unwrap();

        let released =
            release_agent_control(workspace.path(), "unknown-agent", "operator", "nothing")
                .unwrap();

        assert_eq!(released, None);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(!agent_control_releases_path(workspace.path()).exists());
    }

    #[test]
    fn isolate_agent_is_idempotent_and_takes_only_reason_codes() {
        let workspace = tempdir().unwrap();
        let first = isolate_agent(workspace.path(), "agent-a", "operator_isolation").unwrap();
        let again = isolate_agent(workspace.path(), "agent-a", "another_reason").unwrap();
        assert_eq!(again, first, "an isolated agent keeps its control");
        assert_eq!(list_agent_controls(workspace.path()).unwrap().len(), 1);

        let error = isolate_agent(workspace.path(), "agent-b", "Free text!").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(list_agent_controls(workspace.path()).unwrap().len(), 1);
    }

    /// Write `controls` straight into the ledger, as an earlier run left it.
    fn write_control_ledger(workspace: &Path, controls: Vec<Signal>) {
        let controls = controls
            .into_iter()
            .map(|signal| {
                let body = decode_control_body(&signal).unwrap();
                (agent_control_key(&body.agent_id), signal)
            })
            .collect();
        let ledger = AgentControlLedger {
            schema_version: AGENT_CONTROL_LEDGER_SCHEMA_VERSION,
            controls,
        };
        roko_fs::atomic_write_json(&agent_controls_path(workspace), &ledger).unwrap();
    }

    /// A control written at 1 s past the epoch that expired 1 s later.
    fn expired_control(agent_id: &str) -> Signal {
        agent_isolation_control(
            agent_id,
            PROVIDER_CONTAINMENT_REASON,
            1_000,
            Duration::from_secs(1),
        )
        .unwrap()
    }

    /// backlog 1108: a control whose lifetime has passed no longer denies
    /// its agent, and the next ledger write prunes it.
    #[tokio::test]
    async fn expired_isolation_control_does_not_deny() {
        let workspace = tempdir().unwrap();
        write_control_ledger(workspace.path(), vec![expired_control("plan/task#1")]);
        assert!(
            list_agent_controls(workspace.path()).unwrap().is_empty(),
            "an expired control is not in force"
        );

        let calls = Arc::new(AtomicUsize::new(0));
        let boundary = crate::immune_boundary::ImmuneScreenedAgent::durable(
            Box::new(CountingTextAgent(Arc::clone(&calls))),
            "plan/task#1",
            workspace.path(),
        );
        let prompt = Signal::builder(Kind::Prompt).body(Body::text("go")).build();
        let ran = boundary.run(&prompt, &roko_core::Context::now()).await;
        assert!(ran.success, "an expired control must not deny");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let ledger: AgentControlLedger = roko_fs::read_json_or_default_strict_bounded(
            &agent_controls_path(workspace.path()),
            MAX_AGENT_CONTROL_LEDGER_BYTES,
        )
        .unwrap();
        assert!(ledger.controls.is_empty(), "the expired control was pruned");

        // A live control still denies, and carries its 7-day lifetime.
        let live = isolate_agent(workspace.path(), "plan/task#1", "operator_isolation").unwrap();
        let isolated_at_ms = live.isolated_at_ms.expect("a schema 2 control");
        let ttl_ms = u64::try_from(DEFAULT_ISOLATION_TTL.as_millis()).unwrap();
        assert_eq!(live.expires_at_ms, Some(isolated_at_ms + ttl_ms));
        let denied = boundary.run(&prompt, &roko_core::Context::now()).await;
        assert_eq!(denied.output.tag("immune_reason"), Some("agent_isolated"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// backlog 1108: a ledger full of expired controls is pruned before the
    /// capacity check, so an unknown agent runs instead of failing closed.
    #[tokio::test]
    async fn full_control_ledger_admits_unknown_agent_after_pruning() {
        let workspace = tempdir().unwrap();
        let expired = (0..MAX_AGENT_CONTROLS)
            .map(|index| expired_control(&format!("expired-agent-{index}")))
            .collect::<Vec<_>>();
        write_control_ledger(workspace.path(), expired);

        assert_eq!(
            get_agent_control(workspace.path(), "unknown-agent").unwrap(),
            None
        );

        write_control_ledger(
            workspace.path(),
            (0..MAX_AGENT_CONTROLS)
                .map(|index| expired_control(&format!("expired-agent-{index}")))
                .collect(),
        );
        let calls = Arc::new(AtomicUsize::new(0));
        let boundary = crate::immune_boundary::ImmuneScreenedAgent::durable(
            Box::new(CountingTextAgent(Arc::clone(&calls))),
            "unknown-agent",
            workspace.path(),
        );
        let prompt = Signal::builder(Kind::Prompt).body(Body::text("go")).build();
        let ran = boundary.run(&prompt, &roko_core::Context::now()).await;
        assert!(ran.success, "{:?}", ran.output.tag("immune_reason"));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(list_agent_controls(workspace.path()).unwrap().is_empty());
    }

    /// A control written before controls expired (schema 1) stays in force
    /// until it is released, and a schema 1 control with any other reason is
    /// invalid.
    #[test]
    fn legacy_control_never_expires() {
        let workspace = tempdir().unwrap();
        let legacy = legacy_agent_isolation_control("legacy-agent").unwrap();
        write_control_ledger(workspace.path(), vec![legacy.clone()]);

        assert_eq!(
            get_agent_control(workspace.path(), "legacy-agent").unwrap(),
            Some(legacy)
        );
        let listed = list_agent_controls(workspace.path()).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].expires_at_ms, None);

        let mut relabelled = agent_control(3);
        let mut body: Value = relabelled.body.as_json().unwrap();
        body["reason"] = Value::String("operator_isolation".to_string());
        relabelled.body = Body::Json(body);
        relabelled.id = relabelled.content_hash();
        assert!(validate_agent_control_signal(&relabelled).is_err());
    }
}
