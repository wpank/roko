//! The A-CTL controller ledger: `roko.controller/1` rows, cross-run in
//! `.roko/learn/controller.jsonl` (S01 v1.3 §5.10; S06 §5).
//!
//! Every row carries `{schema_version, record_id, ts, kind, run_id,
//! policy_version, arm}` ([`ControllerRecord`]) and one body per kind
//! ([`ControllerRow`]): `ev.sample`, `ev.breach`, `ev.restore`,
//! `homeostasis.episode`, `param.change`, `param.evaluate`, `recovery`,
//! `controller.mode` and `controller.hold`. Ids come from
//! [`record_id`], so readers dedupe these rows as they do S01's; being
//! cross-run, a row has no `seq`. `arm` is S01 §4.6's: the controller
//! steers the adaptive θ, `learned`.
//!
//! [`ControllerRecord::from_event`] turns the controller's events into rows
//! and [`write_jsonl`] and [`append`] write them for a replay. A live run
//! submits them through `TelemetryWriter` once that has a controller
//! variant (8122). The disturbance ground truth (`roko.disturbance/1`) is
//! not here: the controller's module cannot read it (8119).

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use roko_core::config::harness_params::{Block, Knob};
use roko_core::config::homeostasis::HomeostasisMode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::controller::{
    ChangeProposal, ControllerEvent, EpisodeOutcome, Evaluation, MoveReason, PriorSource,
};
use super::ev::{Ev, Side};
use super::policy::Violation;
use crate::telemetry::Arm;
use crate::telemetry::records::record_id;

/// `schema_version` of a controller row.
pub const CONTROLLER_SCHEMA: &str = "roko.controller/1";
/// The ledger, relative to the `.roko` directory.
pub const CONTROLLER_FILE: &str = "learn/controller.jsonl";
/// Passes for which the audit rate on a class doubles after a cost- or
/// verification-reducing move (S06 §4.6.5).
pub const AUDIT_BOOST_PASSES: u32 = 20;

/// `<roko_dir>/learn/controller.jsonl`.
#[must_use]
pub fn ledger_path(roko_dir: &Path) -> PathBuf {
    roko_dir.join(CONTROLLER_FILE)
}

/// What the rows of one stream share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Envelope {
    /// ISO-8601 UTC time of the rows.
    pub ts: String,
    /// The run whose resolution caused them; `None` for a replay.
    pub run_id: Option<String>,
    /// The S5 policy's `policy_version`.
    pub policy_version: u32,
    /// The arm the rows are about.
    pub arm: Arm,
    /// The position of the input in its stream, such as the resolution
    /// index: part of the record id, so a repeated item gets a new id.
    pub seq: u64,
}

/// One `roko.controller/1` row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ControllerRecord {
    /// [`CONTROLLER_SCHEMA`].
    pub schema_version: String,
    /// The id readers dedupe on.
    pub record_id: String,
    /// ISO-8601 UTC time of the row.
    pub ts: String,
    /// The run whose resolution caused the row; `null` for a replay.
    pub run_id: Option<String>,
    /// The S5 policy's `policy_version`.
    pub policy_version: u32,
    /// The arm the row is about: `learned`.
    pub arm: Arm,
    /// The kind and its fields.
    #[serde(flatten)]
    pub row: ControllerRow,
}

impl ControllerRecord {
    /// The row `row` in `envelope`, with its record id.
    #[must_use]
    pub fn new(envelope: &Envelope, row: ControllerRow) -> Self {
        let run = envelope.run_id.as_deref().unwrap_or("-");
        let seq = envelope.seq.to_string();
        Self {
            schema_version: CONTROLLER_SCHEMA.to_string(),
            record_id: record_id(CONTROLLER_SCHEMA, run, row.kind(), &row.item(), &seq),
            ts: envelope.ts.clone(),
            run_id: envelope.run_id.clone(),
            policy_version: envelope.policy_version,
            arm: envelope.arm,
            row,
        }
    }

    /// The row of a controller event; `None` for a commit, which the
    /// guarded store logs instead (8113).
    #[must_use]
    pub fn from_event(envelope: &Envelope, event: &ControllerEvent) -> Option<Self> {
        ControllerRow::from_event(event).map(|row| Self::new(envelope, row))
    }
}

/// Which way an EV breached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Above its bound (E2, E3, E4).
    High,
    /// Below its bound (E1).
    Low,
}

impl Direction {
    /// The direction `ev` breaches in.
    #[must_use]
    pub const fn of(ev: Ev) -> Self {
        match ev.side() {
            Side::Lower => Self::Low,
            Side::Upper => Self::High,
        }
    }
}

/// Where an estimate sits against its band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BandPosition {
    /// Inside the inner band.
    Inside,
    /// Between the inner band and the outer bound.
    Inner,
    /// Beyond the outer bound.
    Outer,
}

/// An episode's two rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeEvent {
    /// The episode opened.
    Open,
    /// The episode closed.
    Close,
}

/// A HOLD's two rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HoldEvent {
    /// The controller entered HOLD.
    Enter,
    /// A person or an S5 change released it.
    Release,
}

/// Who acted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Actor {
    /// The controller.
    Controller,
    /// A person.
    Human,
    /// M2, the loop auditor: it can push M1 down to shadow, never up (8128).
    M2,
}

/// A `param.change`'s predicted drive change.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Predicted {
    /// The drive change the move's prior predicts: negative helps.
    pub d_drive: f64,
    /// Where the prior came from: `catalog` or `m3`.
    pub source: PriorSource,
}

/// A `param.change`'s validator verdicts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validator {
    /// The SafetyBox's verdict: `pass` or `fail`.
    pub safety_box: String,
    /// Whether θ sits on its ladders (the config validation, E42): `pass`
    /// or `fail`.
    pub e42: String,
}

/// The audit boost a cost- or verification-reducing move triggers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditCoupling {
    /// The task class the boost applies to: a tier for a B1 rung, `all`
    /// for a global knob.
    pub class: String,
    /// Passes the doubled audit rate lasts.
    pub boost_until_passes: u32,
}

/// A `param.change` row's fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamChange {
    /// `ch-0019`.
    pub change_id: String,
    /// The episode; `null` for a relaxation.
    pub episode_id: Option<String>,
    /// `shadow` (only logged) or `on` (applied).
    pub mode: HomeostasisMode,
    /// Whether θ was swapped.
    pub applied: bool,
    /// The knob's block.
    pub block: Block,
    /// The knob, as `field` or `field.key`.
    pub param: String,
    /// Its value before.
    pub from: Value,
    /// Its value after.
    pub to: Value,
    /// Why the change was made.
    pub reason: MoveReason,
    /// The drive change its prior predicts, for a guided move.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub predicted: Option<Predicted>,
    /// The validators' verdicts.
    pub validator: Validator,
    /// The audit boost it triggers, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_coupling: Option<AuditCoupling>,
}

impl From<&ChangeProposal> for ParamChange {
    fn from(change: &ChangeProposal) -> Self {
        let off_ladder = change
            .verdict
            .violations
            .iter()
            .any(|violation| matches!(violation, Violation::Inadmissible { .. }));
        let mode = if change.applied {
            HomeostasisMode::On
        } else {
            HomeostasisMode::Shadow
        };
        let class = match change.knob {
            Knob::TierFloor(tier) | Knob::TierCap(tier) => tier.to_string(),
            _ => "all".to_string(),
        };
        Self {
            change_id: change.change_id.clone(),
            episode_id: change.episode_id.clone(),
            mode,
            applied: change.applied,
            block: change.block,
            param: change.knob.to_string(),
            from: change.from.clone(),
            to: change.to.clone(),
            reason: change.reason,
            predicted: change.predicted.map(|prior| Predicted {
                d_drive: -prior.mean,
                source: prior.source,
            }),
            validator: Validator {
                safety_box: change.verdict.label().to_string(),
                e42: if off_ladder { "fail" } else { "pass" }.to_string(),
            },
            audit_coupling: change.audit_coupled.then_some(AuditCoupling {
                class,
                boost_until_passes: AUDIT_BOOST_PASSES,
            }),
        }
    }
}

/// A `recovery` row's fields: the SASO scorecard of one episode on one EV
/// (8116 computes it).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryRow {
    /// The episode.
    pub episode_id: String,
    /// The EV the episode regulated.
    pub ev: Ev,
    /// Resolutions from the onset until D stays 0; `null` if it never does.
    pub settling_resolutions: Option<u32>,
    /// Spend over those resolutions.
    pub settling_usd: Option<f64>,
    /// The largest excursion past the target after the first recovery.
    pub overshoot: f64,
    /// The largest excursion of each EV that was not breached.
    pub collateral_max: BTreeMap<String, f64>,
    /// The drive left at the end.
    pub steady_state_error: f64,
    /// IAE = Σ D(t), the primary endpoint.
    pub iae: f64,
    /// Spend above the holdout arm's over the episode.
    pub adaptation_cost_usd: f64,
    /// Resolutions from onset to the confirmed breach, when the evaluator
    /// knows the onset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detection_delay_resolutions: Option<u32>,
    /// Wall seconds over the settling resolutions, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settling_secs: Option<f64>,
}

/// A controller row's kind and fields (S01 v1.3 §5.10, A-CTL).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ControllerRow {
    /// One EV's estimate over the window.
    #[serde(rename = "ev.sample")]
    EvSample {
        /// The EV.
        ev: Ev,
        /// Its estimate; `null`, never 0, when unmeasurable.
        value: Option<f64>,
        /// Its 90% interval.
        estimate: Option<(f64, f64)>,
        /// Where it sits against its band.
        band: BandPosition,
        /// The window's drive.
        drive: f64,
        /// Observations behind it.
        resolutions: usize,
    },
    /// An EV breached.
    #[serde(rename = "ev.breach")]
    EvBreach {
        /// The EV.
        ev: Ev,
        /// Which way.
        direction: Direction,
        /// Its estimate.
        value: Option<f64>,
        /// The bound it crossed.
        bound: f64,
        /// Resolutions since its estimate left the inner band.
        detect_delay_resolutions: u32,
        /// The open episode.
        episode_id: Option<String>,
    },
    /// A breached EV came back inside its inner band.
    #[serde(rename = "ev.restore")]
    EvRestore {
        /// The EV.
        ev: Ev,
        /// Its estimate.
        value: Option<f64>,
        /// Its inner bound.
        bound: f64,
        /// The open episode.
        episode_id: Option<String>,
    },
    /// An episode opened or closed.
    #[serde(rename = "homeostasis.episode")]
    Episode {
        /// The episode.
        episode_id: String,
        /// `open` or `close`.
        event: EpisodeEvent,
        /// The EVs that opened it.
        evs: Vec<Ev>,
        /// Search moves made.
        changes: u32,
        /// Net adaptation spend.
        adaptation_spend_usd: f64,
        /// How it ended; `null` on `open`.
        outcome: Option<EpisodeOutcome>,
    },
    /// θ changed, or would have in shadow.
    #[serde(rename = "param.change")]
    ParamChange(Box<ParamChange>),
    /// A change was judged.
    #[serde(rename = "param.evaluate")]
    ParamEvaluate {
        /// The change.
        change_id: String,
        /// `kept`, `rolled_back` or `unevaluable_in_shadow`.
        decision: Evaluation,
        /// D over the dwell minus D before the change.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        d_drive: Option<f64>,
    },
    /// An episode's SASO scorecard.
    #[serde(rename = "recovery")]
    Recovery(Box<RecoveryRow>),
    /// The mode changed.
    #[serde(rename = "controller.mode")]
    Mode {
        /// The mode before.
        from: HomeostasisMode,
        /// The mode after.
        to: HomeostasisMode,
        /// Who switched it: a person.
        actor: Actor,
    },
    /// The controller entered or left HOLD.
    #[serde(rename = "controller.hold")]
    Hold {
        /// `enter` or `release`.
        event: HoldEvent,
        /// Why: `max_changes`, `adaptation_spend`, `no_admissible_move`,
        /// `ack` or `policy_changed`.
        reason: String,
        /// The episode that ended in it.
        episode_id: Option<String>,
        /// Who acted.
        actor: Actor,
    },
}

impl ControllerRow {
    /// The row's `kind`.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::EvSample { .. } => "ev.sample",
            Self::EvBreach { .. } => "ev.breach",
            Self::EvRestore { .. } => "ev.restore",
            Self::Episode { .. } => "homeostasis.episode",
            Self::ParamChange(_) => "param.change",
            Self::ParamEvaluate { .. } => "param.evaluate",
            Self::Recovery(_) => "recovery",
            Self::Mode { .. } => "controller.mode",
            Self::Hold { .. } => "controller.hold",
        }
    }

    /// The row's identity within its kind, for its record id.
    fn item(&self) -> String {
        match self {
            Self::EvSample { ev, .. } | Self::EvBreach { ev, .. } | Self::EvRestore { ev, .. } => {
                ev.name().to_string()
            }
            Self::Episode {
                episode_id, event, ..
            } => format!("{episode_id}:{}", label(event)),
            Self::ParamChange(change) => change.change_id.clone(),
            Self::ParamEvaluate { change_id, .. } => change_id.clone(),
            Self::Recovery(recovery) => format!("{}:{}", recovery.episode_id, recovery.ev.name()),
            Self::Mode { from, to, .. } => format!("{}->{}", label(from), label(to)),
            Self::Hold { event, reason, .. } => format!("{}:{reason}", label(event)),
        }
    }

    /// The row of a controller event; `None` for a commit.
    #[must_use]
    pub fn from_event(event: &ControllerEvent) -> Option<Self> {
        let row = match event {
            ControllerEvent::Breach {
                ev,
                value,
                bound,
                detect_delay_resolutions,
                episode_id,
            } => Self::EvBreach {
                ev: *ev,
                direction: Direction::of(*ev),
                value: *value,
                bound: *bound,
                detect_delay_resolutions: *detect_delay_resolutions,
                episode_id: episode_id.clone(),
            },
            ControllerEvent::Restore {
                ev,
                value,
                bound,
                episode_id,
            } => Self::EvRestore {
                ev: *ev,
                value: *value,
                bound: *bound,
                episode_id: episode_id.clone(),
            },
            ControllerEvent::EpisodeOpen { episode_id, evs } => Self::Episode {
                episode_id: episode_id.clone(),
                event: EpisodeEvent::Open,
                evs: evs.clone(),
                changes: 0,
                adaptation_spend_usd: 0.0,
                outcome: None,
            },
            ControllerEvent::EpisodeClose {
                episode_id,
                evs,
                changes,
                adaptation_spend_usd,
                outcome,
            } => Self::Episode {
                episode_id: episode_id.clone(),
                event: EpisodeEvent::Close,
                evs: evs.clone(),
                changes: *changes,
                adaptation_spend_usd: *adaptation_spend_usd,
                outcome: Some(*outcome),
            },
            ControllerEvent::Change(change) => {
                Self::ParamChange(Box::new(ParamChange::from(change.as_ref())))
            }
            ControllerEvent::Evaluate {
                change_id,
                decision,
                d_drive,
            } => Self::ParamEvaluate {
                change_id: change_id.clone(),
                decision: *decision,
                d_drive: Some(*d_drive),
            },
            ControllerEvent::Hold { reason, episode_id } => Self::Hold {
                event: HoldEvent::Enter,
                reason: label(reason),
                episode_id: episode_id.clone(),
                actor: Actor::Controller,
            },
            ControllerEvent::Release { reason } => Self::Hold {
                event: HoldEvent::Release,
                reason: label(reason),
                episode_id: None,
                actor: Actor::Human,
            },
            ControllerEvent::Mode { from, to } => Self::Mode {
                from: *from,
                to: *to,
                actor: Actor::Human,
            },
            ControllerEvent::Commit { .. } => return None,
        };
        Some(row)
    }
}

/// The name serde gives a unit variant.
fn label<T: Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(Value::String(name)) => name,
        _ => String::new(),
    }
}

/// Write `records` to `out`, one JSON line each.
///
/// # Errors
///
/// I/O and serialization errors.
pub fn write_jsonl<W: Write>(out: &mut W, records: &[ControllerRecord]) -> std::io::Result<()> {
    for record in records {
        serde_json::to_writer(&mut *out, record)?;
        out.write_all(b"\n")?;
    }
    Ok(())
}

/// Append `records` to the ledger at `path`, creating it and its directory.
/// One writer at a time: a live run goes through `TelemetryWriter` (8122).
///
/// # Errors
///
/// I/O and serialization errors.
pub fn append(path: &Path, records: &[ControllerRecord]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut lines = Vec::new();
    write_jsonl(&mut lines, records)?;
    file.write_all(&lines)
}

#[cfg(test)]
mod tests {
    use roko_core::config::RokoConfig;
    use roko_core::config::harness_params::HarnessParams;
    use roko_core::task::TaskTier;

    use super::*;
    use crate::homeostasis::controller::{MovePriorValue, ReleaseReason};
    use crate::homeostasis::policy::{ChangeKind, Verdict};

    /// S01 v1.3 §5.10's A-CTL example rows, which leave out the common
    /// fields but the schema.
    const EXAMPLES: [&str; 4] = [
        r#"{"schema_version":"roko.controller/1","kind":"ev.breach","ev":"usd_per_verified_success","direction":"high","value":0.19,"bound":0.12,"detect_delay_resolutions":6,"episode_id":"ep-0007"}"#,
        r#"{"schema_version":"roko.controller/1","kind":"param.change","change_id":"ch-0019","episode_id":"ep-0007","mode":"shadow","applied":false,"block":"B1","param":"tier_cap.standard","from":"strong","to":"mid","reason":"directed","predicted":{"d_drive":-0.21,"source":"m3"},"validator":{"safety_box":"pass","e42":"pass"},"audit_coupling":{"class":"standard","boost_until_passes":20}}"#,
        r#"{"schema_version":"roko.controller/1","kind":"param.evaluate","change_id":"ch-0019","decision":"unevaluable_in_shadow"}"#,
        r#"{"schema_version":"roko.controller/1","kind":"recovery","episode_id":"ep-0007","ev":"pass_rate","settling_resolutions":27,"settling_usd":2.1,"overshoot":0.04,"collateral_max":{"latency_p90_s":0.12},"steady_state_error":0.02,"iae":3.7,"adaptation_cost_usd":0.61}"#,
    ];

    fn envelope(seq: u64) -> Envelope {
        Envelope {
            ts: "2026-10-02T14:03:21.950Z".to_string(),
            run_id: None,
            policy_version: 1,
            arm: Arm::Learned,
            seq,
        }
    }

    fn json(record: &ControllerRecord) -> Value {
        serde_json::to_value(record).expect("a record serializes")
    }

    #[test]
    fn controller_rows_round_trip_s06_examples() {
        // Each example, with the common fields it leaves out, parses and
        // serializes back unchanged.
        let common = serde_json::json!({
            "record_id": "b3:0c1d",
            "ts": "2026-10-02T14:03:21.950Z",
            "run_id": "gr-7f3c2a91",
            "policy_version": 1,
            "arm": "learned"
        });
        for example in EXAMPLES {
            let mut row: Value = serde_json::from_str(example).expect("an example parses");
            for (key, value) in common.as_object().expect("an object") {
                row[key.as_str()] = value.clone();
            }
            let record: ControllerRecord =
                serde_json::from_value(row.clone()).expect("the example deserializes");
            assert_eq!(json(&record), row, "{example}");
            assert_eq!(record.schema_version, CONTROLLER_SCHEMA);
        }

        // Events become rows with stable ids: the same input the same id,
        // another position another id.
        let breach = ControllerEvent::Breach {
            ev: Ev::PassRate,
            value: Some(0.65),
            bound: 0.70,
            detect_delay_resolutions: 2,
            episode_id: None,
        };
        let record = ControllerRecord::from_event(&envelope(24), &breach).expect("a breach row");
        let row = json(&record);
        assert_eq!(row["kind"], "ev.breach");
        assert_eq!(row["direction"], "low");
        assert_eq!(row["run_id"], Value::Null);
        assert!(record.record_id.starts_with("b3:"));
        let again = ControllerRecord::from_event(&envelope(24), &breach).expect("the same row");
        assert_eq!(again.record_id, record.record_id);
        let later = ControllerRecord::from_event(&envelope(25), &breach).expect("a later row");
        assert_ne!(later.record_id, record.record_id);

        // A guided change becomes a param.change that parses back.
        let theta0 = HarnessParams::baseline(&RokoConfig::default());
        let proposal = ChangeProposal {
            change_id: "ch-0001".to_string(),
            episode_id: Some("ep-0001".to_string()),
            kind: ChangeKind::Search,
            reason: MoveReason::Directed,
            applied: false,
            knob: Knob::TierFloor(TaskTier::Focused),
            block: Block::B1,
            from: Value::from("cheap"),
            to: Value::from("mid"),
            verdict: Verdict::default(),
            predicted: Some(MovePriorValue {
                mean: 0.1,
                variance: 0.25,
                source: PriorSource::Catalog,
            }),
            audit_coupled: false,
            theta: theta0.clone(),
        };
        let change = ControllerEvent::Change(Box::new(proposal));
        let record = ControllerRecord::from_event(&envelope(24), &change).expect("a change row");
        let row = json(&record);
        assert_eq!(row["kind"], "param.change");
        assert_eq!(row["param"], "tier_floor.focused");
        assert_eq!(row["mode"], "shadow");
        assert_eq!(row["validator"]["safety_box"], "pass");
        assert_eq!(row["validator"]["e42"], "pass");
        assert_eq!(row["predicted"]["d_drive"], -0.1);
        assert_eq!(row["predicted"]["source"], "catalog");
        assert!(row.get("audit_coupling").is_none());
        let back: ControllerRecord = serde_json::from_value(row).expect("the row parses back");
        assert_eq!(back, record);

        // A commit is the guarded store's row; a release is a person's.
        let commit = ControllerEvent::Commit {
            theta: Box::new(theta0),
            reason: "homeostat:ep-0001/ch-0002".to_string(),
        };
        assert!(ControllerRecord::from_event(&envelope(30), &commit).is_none());
        let release = ControllerEvent::Release {
            reason: ReleaseReason::Ack,
        };
        let row = json(&ControllerRecord::from_event(&envelope(31), &release).expect("a row"));
        assert_eq!(row["kind"], "controller.hold");
        assert_eq!(row["event"], "release");
        assert_eq!(row["reason"], "ack");
        assert_eq!(row["actor"], "human");

        // Rows go out one JSON line each, and append to the ledger.
        let records = [record, later];
        let mut out = Vec::new();
        write_jsonl(&mut out, &records).expect("write");
        let text = String::from_utf8(out).expect("UTF-8");
        let lines: Vec<ControllerRecord> = text
            .lines()
            .map(|line| serde_json::from_str(line).expect("a line parses"))
            .collect();
        assert_eq!(lines, records);
        let dir = tempfile::tempdir().expect("tempdir");
        let path = ledger_path(dir.path());
        append(&path, &records[..1]).expect("first append");
        append(&path, &records[1..]).expect("second append");
        let written = std::fs::read_to_string(&path).expect("read the ledger");
        assert_eq!(written.lines().count(), 2);
    }
}
