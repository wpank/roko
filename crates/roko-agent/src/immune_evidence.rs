//! Strict bounded storage for quarantined immune evidence.
//!
//! This ledger deliberately does not use the general append-only substrate:
//! an enforcement boundary must not replay an unbounded journal before it can
//! deny a suspicious provider or tool result.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use roko_core::{ContentHash, Kind, Signal};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub(crate) const IMMUNE_EVIDENCE_RELATIVE_PATH: &str = ".roko/immune/quarantine/evidence.json";
pub(crate) const AGENT_CONTROLS_RELATIVE_PATH: &str = ".roko/immune/agent-controls.json";
pub(crate) const AGENT_ISOLATION_CONTROL_KIND: &str = "roko.security.immune.agent_isolation";
pub(crate) const MAX_IMMUNE_EVIDENCE_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_IMMUNE_EVIDENCE_SIGNALS: usize = 200;
pub(crate) const MAX_IMMUNE_LABEL_BYTES: usize = 256;

const IMMUNE_EVIDENCE_SCHEMA_VERSION: u32 = 1;
const AGENT_CONTROL_SCHEMA_VERSION: u32 = 1;
const MAX_AGENT_CONTROL_LEDGER_BYTES: u64 = 4 * 1024 * 1024;
const MAX_AGENT_CONTROLS: usize = 512;
const MAX_SIGNAL_TAGS: usize = 128;

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
            schema_version: AGENT_CONTROL_SCHEMA_VERSION,
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
}

pub(crate) fn immune_evidence_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(IMMUNE_EVIDENCE_RELATIVE_PATH)
}

pub(crate) fn agent_controls_path(workspace_root: &Path) -> PathBuf {
    workspace_root.join(AGENT_CONTROLS_RELATIVE_PATH)
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

pub(crate) fn persist_agent_control(workspace_root: &Path, control: &Signal) -> io::Result<()> {
    let agent_id = validate_agent_control_signal(control)?;
    let key = agent_control_key(&agent_id);
    roko_fs::with_locked_json_transaction_bounded::<AgentControlLedger, _, io::Error, _>(
        &agent_controls_path(workspace_root),
        MAX_AGENT_CONTROL_LEDGER_BYTES,
        |ledger| {
            validate_agent_control_ledger(ledger)?;
            match ledger.controls.get(&key) {
                Some(existing) if existing == control => return Ok(()),
                Some(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "agent control conflicts with existing authority",
                    ));
                }
                None if ledger.controls.len() >= MAX_AGENT_CONTROLS => {
                    return Err(io::Error::other(format!(
                        "agent control ledger reached its {MAX_AGENT_CONTROLS}-entry capacity"
                    )));
                }
                None => {}
            }
            ledger.controls.insert(key, control.clone());
            validate_agent_control_ledger(ledger)
        },
    )
}

pub(crate) fn get_agent_control(
    workspace_root: &Path,
    agent_id: &str,
) -> io::Result<Option<Signal>> {
    validate_boundary_label(agent_id, "agent ID")?;
    let key = agent_control_key(agent_id);
    roko_fs::with_locked_json_transaction_bounded::<AgentControlLedger, _, io::Error, _>(
        &agent_controls_path(workspace_root),
        MAX_AGENT_CONTROL_LEDGER_BYTES,
        |ledger| {
            validate_agent_control_ledger(ledger)?;
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
    if ledger.schema_version != AGENT_CONTROL_SCHEMA_VERSION {
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
    if body.schema_version != AGENT_CONTROL_SCHEMA_VERSION
        || signal.attestation.is_some()
        || body.state != "isolated"
        || body.reason != "provider_output_immune_containment"
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
    use roko_core::{Body, Provenance};
    use tempfile::tempdir;

    use super::*;

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
                    "schema_version": AGENT_CONTROL_SCHEMA_VERSION,
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
            schema_version: AGENT_CONTROL_SCHEMA_VERSION,
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
}
