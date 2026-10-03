//! Incident records (S05 §4.7, DP6): what a confirmed false green, a gaming
//! finding, a weak oracle, a leaked canary or a faulty battery check leaves in
//! the vault, for a person to act on.
//!
//! - An incident is `<vault>/<workspace_id>/incidents/<incident_id>.json`,
//!   with an append-only status history: `open → triaged → {fix_task_created
//!   | dismissed_battery_fp | accepted_risk} → closed` ([`IncidentStatus`]).
//!   Opening it and every change append `audit.incident` to the ledger. Its
//!   id derives from its kind and its attempt, so the same finding opens one
//!   incident.
//! - Its evidence is content-addressed: the attempt's base and result trees,
//!   A1's findings, the hidden suite and how many of its tests failed (never
//!   a test's body or name), B3's verdict, the selection and result ids, and
//!   an [`IncidentLink`] to the immune ledger when one applies.
//! - A false green or a gaming finding about an attempt comes with a fix
//!   proposal, `<incident_id>.task.toml`: a ready `[[task]]` that says only
//!   "audit failed: <kind>", for an operator to add to a plan. Nothing
//!   dispatches it, and it never holds a hidden test's body or name.
//! - Repeated gaming by one (model, harness) pair gives an isolation
//!   proposal, `isolate-<model>-<harness>.json`, which only an operator
//!   confirms (decision 7102, question 6).
//! - `dismissed_battery_fp` retires the hidden suite that caused the incident.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use roko_core::audit_home::AuditVault;
use roko_core::immune::IncidentLink;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::hidden::{HiddenStore, SuiteState, private_dir, write_private};
use super::ledger::{AuditEvent, AuditLedger};
use super::policy::hex;

/// The suffix of an incident's fix proposal file.
pub const FIX_SUFFIX: &str = ".task.toml";

/// Gaming incidents of one (model, harness) pair that give an isolation
/// proposal.
pub const ISOLATION_AFTER: usize = 3;

/// What an incident is about (S05 §4.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentKind {
    /// The artifact fails its spec (Y).
    FalseGreen,
    /// The pass depended on manipulating an oracle (G), or the gate-gaming
    /// detector raised an alert.
    SpecGaming,
    /// The tests do not pin the change (W).
    WeakOracle,
    /// A hidden suite's canary was found where agents write.
    LeakCanary,
    /// A battery check was wrong.
    BatteryFault,
}

impl IncidentKind {
    /// The kind's name, as `audit.incident` records it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::FalseGreen => "false_green",
            Self::SpecGaming => "spec_gaming",
            Self::WeakOracle => "weak_oracle",
            Self::LeakCanary => "leak_canary",
            Self::BatteryFault => "battery_fault",
        }
    }

    /// Whether the kind comes with a fix proposal: a false green or gaming.
    #[must_use]
    pub const fn wants_a_fix(self) -> bool {
        matches!(self, Self::FalseGreen | Self::SpecGaming)
    }
}

/// Where an incident is in its life (S05 §4.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    /// Opened by an audit.
    Open,
    /// A person has looked at it.
    Triaged,
    /// Its fix task was added to a plan.
    FixTaskCreated,
    /// The battery was wrong: its hidden suite is retired.
    DismissedBatteryFp,
    /// Kept as a known risk.
    AcceptedRisk,
    /// Done.
    Closed,
}

impl IncidentStatus {
    /// The status's name, as `audit.incident` records it.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Triaged => "triaged",
            Self::FixTaskCreated => "fix_task_created",
            Self::DismissedBatteryFp => "dismissed_battery_fp",
            Self::AcceptedRisk => "accepted_risk",
            Self::Closed => "closed",
        }
    }

    /// Whether an incident may move from `self` to `next`.
    #[must_use]
    pub const fn can_move_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Open, Self::Triaged)
                | (
                    Self::Triaged,
                    Self::FixTaskCreated | Self::DismissedBatteryFp | Self::AcceptedRisk
                )
                | (
                    Self::FixTaskCreated | Self::DismissedBatteryFp | Self::AcceptedRisk,
                    Self::Closed
                )
        )
    }
}

/// One entry of an incident's status history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatusChange {
    /// The status it entered.
    pub status: IncidentStatus,
    /// When.
    pub at: DateTime<Utc>,
    /// Who moved it: `audit` for the worker, else the operator.
    pub by: String,
    /// Why.
    pub note: String,
}

/// An incident's evidence: ids and hashes, never a hidden test's body or
/// name.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Evidence {
    /// The selection audited.
    pub sel_id: Option<String>,
    /// The audit's result.
    pub res_id: Option<String>,
    /// The tree the attempt started from.
    pub base_tree: Option<String>,
    /// The tree it left, which the audit judged.
    pub result_tree: Option<String>,
    /// A1's findings, which name the attempt's own paths only.
    pub findings: Vec<String>,
    /// The hidden suite that failed the attempt, by id.
    pub hidden_suite: Option<String>,
    /// How many of its tests failed; never which.
    pub hidden_failed: Option<u32>,
    /// B3's verdict.
    pub b3: Option<Value>,
    /// What else the audit noted, such as a gate-gaming alert.
    pub note: Option<String>,
    /// The related entry of the immune ledger, when one applies.
    pub immune_link: Option<IncidentLink>,
}

/// An incident record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Incident {
    /// `inc-` and 16 hex digits of its kind and attempt.
    pub incident_id: String,
    /// What it is about.
    pub kind: IncidentKind,
    /// S01's attempt key; `-` for one about a model.
    pub attempt_key: String,
    /// The run.
    pub run_id: String,
    /// The task.
    pub task_id: String,
    /// The implementing model.
    pub model: String,
    /// The harness it ran in.
    pub harness: String,
    /// What the audit found.
    pub evidence: Evidence,
    /// Its status history, oldest first; never empty.
    pub history: Vec<StatusChange>,
    /// Its fix proposal's file name, when it has one.
    pub fix_proposal: Option<String>,
}

impl Incident {
    /// Its current status.
    #[must_use]
    pub fn status(&self) -> IncidentStatus {
        self.history
            .last()
            .map_or(IncidentStatus::Open, |change| change.status)
    }
}

/// What opening an incident needs.
#[derive(Debug, Clone, PartialEq)]
pub struct IncidentDraft {
    /// What it is about.
    pub kind: IncidentKind,
    /// S01's attempt key, or `-`.
    pub attempt_key: String,
    /// The run.
    pub run_id: String,
    /// The task.
    pub task_id: String,
    /// The implementing model.
    pub model: String,
    /// The harness it ran in.
    pub harness: String,
    /// The paths the task names, for its fix proposal.
    pub files: Vec<String>,
    /// What the audit found.
    pub evidence: Evidence,
}

/// Why the store refused.
#[derive(Debug, thiserror::Error)]
pub enum IncidentError {
    /// A file could not be read or written.
    #[error("incident store: {0}")]
    Io(#[from] std::io::Error),
    /// A record does not parse.
    #[error("incident {0} does not parse: {1}")]
    Bad(String, String),
    /// No such incident.
    #[error("no incident {0}")]
    Unknown(String),
    /// A move the status history forbids.
    #[error("incident {incident_id} cannot move from {from} to {to}")]
    Illegal {
        /// The incident.
        incident_id: String,
        /// Its status.
        from: &'static str,
        /// The status asked for.
        to: &'static str,
    },
}

/// The incidents of one workspace, in its vault's `incidents/`.
#[derive(Debug, Clone)]
pub struct IncidentStore {
    dir: PathBuf,
}

impl IncidentStore {
    /// The store in `vault`'s `incidents/`.
    ///
    /// # Errors
    ///
    /// The directory cannot be created.
    pub fn open(vault: &AuditVault) -> std::io::Result<Self> {
        Self::open_dir(&vault.incidents_dir())
    }

    /// The store in `dir` (the vault's `incidents/` in production).
    ///
    /// # Errors
    ///
    /// The directory cannot be created.
    pub fn open_dir(dir: &Path) -> std::io::Result<Self> {
        private_dir(dir)?;
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }

    /// The store's directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Open the incident `draft` describes, with its fix proposal when its
    /// kind wants one, and log it; the incident already open for the same
    /// kind and attempt when there is one. Returns it, and whether it is
    /// new.
    ///
    /// # Errors
    ///
    /// A file or the ledger cannot be read or written.
    pub fn open_incident(
        &self,
        ledger: &mut AuditLedger,
        draft: IncidentDraft,
    ) -> Result<(Incident, bool), IncidentError> {
        let incident_id = incident_id(draft.kind, &draft.attempt_key, &draft.model);
        if let Some(incident) = self.get(&incident_id)? {
            return Ok((incident, false));
        }
        // An incident about a model, not an attempt, has no task to fix.
        let fix = draft.kind.wants_a_fix() && draft.attempt_key != "-";
        let fix_proposal = fix.then(|| {
            let name = format!("{incident_id}{FIX_SUFFIX}");
            (name, fix_task(&incident_id, &draft))
        });
        if let Some((name, text)) = &fix_proposal {
            write_private(&self.dir.join(name), text.as_bytes())?;
        }
        let incident = Incident {
            incident_id,
            kind: draft.kind,
            attempt_key: draft.attempt_key,
            run_id: draft.run_id,
            task_id: draft.task_id,
            model: draft.model,
            harness: draft.harness,
            evidence: draft.evidence,
            history: vec![StatusChange {
                status: IncidentStatus::Open,
                at: Utc::now(),
                by: "audit".to_string(),
                note: String::new(),
            }],
            fix_proposal: fix_proposal.map(|(name, _)| name),
        };
        self.save(&incident)?;
        log(ledger, &incident)?;
        Ok((incident, true))
    }

    /// Move incident `incident_id` to `to`, by `by`, and log it. Dismissing
    /// it as a battery false positive retires its hidden suite in `hidden`.
    ///
    /// # Errors
    ///
    /// An unknown incident, a move [`IncidentStatus::can_move_to`] forbids,
    /// or a file or the ledger cannot be read or written.
    pub fn transition(
        &self,
        ledger: &mut AuditLedger,
        incident_id: &str,
        to: IncidentStatus,
        by: &str,
        note: &str,
        hidden: Option<&HiddenStore>,
    ) -> Result<Incident, IncidentError> {
        let mut incident = self
            .get(incident_id)?
            .ok_or_else(|| IncidentError::Unknown(incident_id.to_string()))?;
        let from = incident.status();
        if !from.can_move_to(to) {
            return Err(IncidentError::Illegal {
                incident_id: incident_id.to_string(),
                from: from.label(),
                to: to.label(),
            });
        }
        incident.history.push(StatusChange {
            status: to,
            at: Utc::now(),
            by: by.to_string(),
            note: note.to_string(),
        });
        self.save(&incident)?;
        log(ledger, &incident)?;
        let suite = incident.evidence.hidden_suite.as_deref();
        if to == IncidentStatus::DismissedBatteryFp
            && let (Some(store), Some(suite)) = (hidden, suite)
        {
            let retired = store.transition(ledger, suite, SuiteState::Retired, "battery_fp");
            if let Err(error) = retired {
                tracing::warn!(%suite, %error, "the suite of a dismissed incident was not retired");
            }
        }
        Ok(incident)
    }

    /// Incident `incident_id`, when there is one.
    ///
    /// # Errors
    ///
    /// Its record cannot be read or does not parse.
    pub fn get(&self, incident_id: &str) -> Result<Option<Incident>, IncidentError> {
        let path = self.dir.join(format!("{incident_id}.json"));
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|error| IncidentError::Bad(incident_id.to_string(), error.to_string()))
    }

    /// Every incident, oldest first; a record that does not parse is
    /// skipped.
    ///
    /// # Errors
    ///
    /// The directory cannot be read.
    pub fn list(&self) -> std::io::Result<Vec<Incident>> {
        let mut incidents = Vec::new();
        for entry in std::fs::read_dir(&self.dir)? {
            let path = entry?.path();
            let is_record = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("inc-") && name.ends_with(".json"));
            if !is_record {
                continue;
            }
            let parsed = std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str::<Incident>(&text).ok());
            incidents.extend(parsed);
        }
        incidents.sort_by_key(|incident| {
            incident
                .history
                .first()
                .map(|change| change.at)
                .unwrap_or_default()
        });
        Ok(incidents)
    }

    /// An isolation proposal for (`model`, `harness`) once it has at least
    /// [`ISOLATION_AFTER`] gaming incidents; its file, written once. Only an
    /// operator acts on it.
    ///
    /// # Errors
    ///
    /// The incidents cannot be listed, or the proposal cannot be written.
    pub fn propose_isolation(
        &self,
        model: &str,
        harness: &str,
    ) -> std::io::Result<Option<PathBuf>> {
        let gaming: Vec<String> = self
            .list()?
            .into_iter()
            .filter(|incident| {
                incident.kind == IncidentKind::SpecGaming
                    && incident.model == model
                    && incident.harness == harness
            })
            .map(|incident| incident.incident_id)
            .collect();
        if gaming.len() < ISOLATION_AFTER {
            return Ok(None);
        }
        let path = self.dir.join(isolation_file(model, harness));
        if path.exists() {
            return Ok(Some(path));
        }
        let proposal = serde_json::json!({
            "proposal": "isolate",
            "model": model,
            "harness": harness,
            "incidents": gaming,
            "proposed_at": Utc::now(),
            "confirmed": false,
            "note": "Repeated gaming: an operator confirms or rejects isolating this pair \
                     (S05 §4.7); nothing isolates it until then.",
        });
        write_private(&path, serde_json::to_string_pretty(&proposal)?.as_bytes())?;
        Ok(Some(path))
    }

    fn save(&self, incident: &Incident) -> std::io::Result<()> {
        let path = self.dir.join(format!("{}.json", incident.incident_id));
        write_private(&path, serde_json::to_string_pretty(incident)?.as_bytes())
    }
}

/// An incident's id: `inc-` and the first 16 hex digits of SHA-256 over its
/// kind, its attempt and, for an incident about a model, the model.
#[must_use]
pub fn incident_id(kind: IncidentKind, attempt_key: &str, model: &str) -> String {
    let subject = if attempt_key == "-" { model } else { attempt_key };
    let digest = Sha256::digest(format!("{}|{subject}", kind.label()).as_bytes());
    format!("inc-{}", &hex(&digest)[..16])
}

/// The isolation proposal's file name for (`model`, `harness`).
#[must_use]
pub fn isolation_file(model: &str, harness: &str) -> String {
    let safe = |text: &str| text.replace(|c: char| !c.is_ascii_alphanumeric() && c != '.', "_");
    format!("isolate-{}-{}.json", safe(model), safe(harness))
}

/// The fix proposal of `incident_id`: a ready `[[task]]` that says only
/// "audit failed: <kind>", for the task `draft` names. It names no hidden
/// test.
fn fix_task(incident_id: &str, draft: &IncidentDraft) -> String {
    let says = format!("audit failed: {}", draft.kind.label());
    let mut task = toml::map::Map::new();
    let id = format!("{}-audit-fix", draft.task_id);
    task.insert("id".into(), toml::Value::String(id));
    task.insert("title".into(), toml::Value::String(says.clone()));
    task.insert("description".into(), toml::Value::String(says));
    task.insert("role".into(), toml::Value::String("implementer".into()));
    let files = draft.files.iter().cloned().map(toml::Value::String).collect();
    task.insert("files".into(), toml::Value::Array(files));
    let mut root = toml::map::Map::new();
    let tasks = vec![toml::Value::Table(task)];
    root.insert("task".into(), toml::Value::Array(tasks));
    let body = toml::to_string(&toml::Value::Table(root)).unwrap_or_default();
    format!(
        "# Fix proposal for audit incident {incident_id} (S05 §4.7). Nothing runs it:\n\
         # add the task to a plan to act on it.\n{body}"
    )
}

/// Append `incident`'s current status as `audit.incident`.
fn log(ledger: &mut AuditLedger, incident: &Incident) -> std::io::Result<()> {
    ledger.append(AuditEvent::Incident {
        incident_id: incident.incident_id.clone(),
        kind: incident.kind.label().to_string(),
        attempt_key: incident.attempt_key.clone(),
        status: incident.status().label().to_string(),
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::ledger::records;

    fn draft(kind: IncidentKind, n: u32) -> IncidentDraft {
        IncidentDraft {
            kind,
            attempt_key: format!("run:plan:t{n}:1"),
            run_id: "run".to_string(),
            task_id: format!("t{n}"),
            model: "glm-4.7".to_string(),
            harness: "roko".to_string(),
            files: vec!["src/lib.rs".to_string()],
            evidence: Evidence {
                hidden_suite: Some("hs-1".to_string()),
                hidden_failed: Some(2),
                ..Evidence::default()
            },
        }
    }

    #[test]
    fn an_incident_keeps_its_status_history_and_proposes_isolation() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = IncidentStore::open_dir(&temp.path().join("incidents")).expect("a store");
        let mut ledger = AuditLedger::open_dir(&temp.path().join("ledger")).expect("a ledger");

        let (incident, new) = store
            .open_incident(&mut ledger, draft(IncidentKind::WeakOracle, 1))
            .expect("opened");
        assert!(new);
        assert_eq!(incident.fix_proposal, None, "a weak oracle wants no fix task");
        let (again, new) = store
            .open_incident(&mut ledger, draft(IncidentKind::WeakOracle, 1))
            .expect("the same incident");
        assert!(!new);
        assert_eq!(again.incident_id, incident.incident_id);

        // open → triaged → accepted_risk → closed; nothing skips a step.
        let id = incident.incident_id.as_str();
        let closed = store.transition(&mut ledger, id, IncidentStatus::Closed, "me", "", None);
        assert!(matches!(closed, Err(IncidentError::Illegal { .. })));
        for status in [
            IncidentStatus::Triaged,
            IncidentStatus::AcceptedRisk,
            IncidentStatus::Closed,
        ] {
            store
                .transition(&mut ledger, id, status, "operator", "", None)
                .expect("a legal move");
        }
        let kept = store.get(id).expect("read").expect("the incident");
        let moves: Vec<IncidentStatus> = kept.history.iter().map(|change| change.status).collect();
        assert_eq!(
            moves,
            [
                IncidentStatus::Open,
                IncidentStatus::Triaged,
                IncidentStatus::AcceptedRisk,
                IncidentStatus::Closed,
            ]
        );
        let logged = records(ledger.dir())
            .expect("records")
            .iter()
            .filter(|record| matches!(record.event, AuditEvent::Incident { .. }))
            .count();
        assert_eq!(logged, 4, "opening and every move are logged");

        // Three gaming incidents of one pair give an isolation proposal,
        // which nothing confirms.
        for n in 2..4 {
            store
                .open_incident(&mut ledger, draft(IncidentKind::SpecGaming, n))
                .expect("opened");
        }
        assert_eq!(
            store.propose_isolation("glm-4.7", "roko").expect("listed"),
            None
        );
        store
            .open_incident(&mut ledger, draft(IncidentKind::SpecGaming, 4))
            .expect("opened");
        let proposal = store
            .propose_isolation("glm-4.7", "roko")
            .expect("written")
            .expect("a proposal");
        let text = std::fs::read_to_string(&proposal).expect("the proposal");
        let value: Value = serde_json::from_str(&text).expect("JSON");
        assert_eq!(value["confirmed"], false);
        assert_eq!(value["incidents"].as_array().map(Vec::len), Some(3));
        assert_eq!(store.list().expect("listed").len(), 4);
    }
}
