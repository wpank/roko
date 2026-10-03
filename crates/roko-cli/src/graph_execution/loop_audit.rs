//! The loop auditor's tick at each plan run's close (S03 §5; backlog 5126).
//!
//! When a checkpoint run closes and its attempt log is flushed,
//! [`audit_tick`] has [`LoopAuditor::observe_run`] fold every run's decision
//! rows into each measured loop's health, apply the state machine with the
//! dwell the ledger carries across runs, and append the `loop.health` and
//! `loop.transition` rows to `.roko/learn/loop-audit.jsonl`. Each row then
//! goes to the run's StateHub as `DashboardEvent::LoopHealth` or
//! `LoopTransition`, which the TUI and serve's SSE read.
//!
//! The tick only observes: it changes no executed policy, whatever
//! `[learning.audit] enforce` says (attempt open reads the ledger's states
//! for that). It calls no provider and writes nothing but its own ledger,
//! also when learning is frozen (gap-644040). An error is logged and never
//! fails the run.

use std::path::Path;

use chrono::{DateTime, Utc};
use roko_core::DashboardEvent;
use roko_core::config::learning::LearningAuditConfig;
use roko_learn::loop_audit::ledger::{LoopAuditRecord, LoopAuditRow};
use roko_learn::loop_audit::{LoopAuditor, Qualifier};

use crate::runner::graph_tui_bridge::GraphTuiBridge;

/// One tick at a time in this process: plans of a set that close together
/// would otherwise both read the ledger before either appends, and move a
/// loop twice.
static TICKING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Run the audit tick for checkpoint run `run_id` of the workspace
/// `workdir`, which just closed, and publish its rows through `bridge`.
pub fn audit_tick(
    workdir: &Path,
    config: &LearningAuditConfig,
    run_id: &str,
    bridge: &GraphTuiBridge,
) {
    tick(workdir, config, run_id, Utc::now(), bridge);
}

/// [`audit_tick`] at `now`. Returns how many rows it appended.
fn tick(
    workdir: &Path,
    config: &LearningAuditConfig,
    run_id: &str,
    now: DateTime<Utc>,
    bridge: &GraphTuiBridge,
) -> usize {
    let _ticking = TICKING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut auditor = match LoopAuditor::load(workdir, config) {
        Ok(auditor) => auditor,
        Err(error) => {
            tracing::warn!(run_id, %error, "loop registry unreadable; no loop-audit tick");
            return 0;
        }
    };
    let observations = match auditor.observe_run(workdir, run_id, now) {
        Ok(observations) => observations,
        Err(error) => {
            tracing::warn!(
                run_id,
                %error,
                "loop-audit ledger not written; the tick publishes nothing"
            );
            return 0;
        }
    };
    let mut rows = 0;
    for observation in &observations {
        for record in observation.records() {
            rows += 1;
            if let Some(event) = loop_event(record, &observation.qualifiers) {
                bridge.publish_event(event);
            }
        }
    }
    tracing::debug!(run_id, rows, "loop-audit tick appended its rows");
    rows
}

/// The StateHub event of ledger row `record`: `LoopHealth` for a health
/// row, with its evaluation's `qualifiers`, and `LoopTransition` for a
/// transition; `None` for the other kinds.
fn loop_event(record: &LoopAuditRecord, qualifiers: &[Qualifier]) -> Option<DashboardEvent> {
    let loop_id = record.loop_id.clone();
    let ts = record.ts.clone().unwrap_or_default();
    match &record.row {
        LoopAuditRow::Health(health) => Some(DashboardEvent::LoopHealth {
            loop_id,
            state: health.state.as_str().to_string(),
            reason: health.reason.map(|reason| reason.as_str().to_string()),
            qualifiers: qualifiers.iter().map(ToString::to_string).collect(),
            h: health.h,
            eps: Some(health.eps.est),
            iota_net: Some(health.iota.net),
            beta: health.beta.est,
            ts,
        }),
        LoopAuditRow::Transition(transition) => Some(DashboardEvent::LoopTransition {
            loop_id,
            from: transition.from.as_str().to_string(),
            to: transition.to.as_str().to_string(),
            reason: transition.reason.map(|reason| reason.as_str().to_string()),
            rule: transition.rule.clone(),
            actor: transition.actor.as_str().to_string(),
            ts,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use roko_learn::loop_audit::ledger::Ledger;
    use roko_learn::routing_log::DecisionState;
    use roko_learn::telemetry::records::{
        AuditFields, ContentProposals, DECISION_SCHEMA, DecisionAssignment, DecisionReceipt,
    };
    use roko_learn::telemetry::{
        Arm, Assignment, AssignmentUnit, AttemptIdentity, AttemptKey, ContentCandidate,
        ContentDecisionPoint, ContentDecisionRecord, RunFile, Stamped,
    };

    use super::*;
    use crate::runner::tui_bridge::TuiBridge;

    /// L-know's health row: on probation, with no reason.
    const KNOW_ON_PROBATION: &str = "L-know health probation -";
    /// L-play's health row once its missing receipts flag it.
    const PLAY_FLAGGED: &str = "L-play health flagged dormant:unlogged";
    /// L-play's move to `flagged`.
    const PLAY_FLAGGED_MOVE: &str = "L-play moved probation -> flagged";
    /// L-play's health row back on probation.
    const PLAY_ON_PROBATION: &str = "L-play health probation -";
    /// L-play's move back to probation.
    const PLAY_READMITTED: &str = "L-play moved flagged -> probation";

    /// Attempt `key`'s content decision at `point`, with S03's fields: the
    /// learned reader proposes `item`, which only the learned arm includes,
    /// and with `receipt` its rendered section was found in the request.
    fn content_row(
        key: &AttemptKey,
        point: ContentDecisionPoint,
        item: &str,
        arm: Arm,
        receipt: bool,
    ) -> ContentDecisionRecord {
        let layer = point.as_str();
        let learned = arm == Arm::Learned;
        let draw = Assignment {
            unit: AssignmentUnit::Chain,
            layer: layer.to_string(),
            salt_id: format!("{layer}@2026-10-03"),
            u: 0.5,
            h: 0.2,
            g: 0.0,
            arm,
            propensity: if learned { 0.8 } else { 0.2 },
        };
        let chosen = if learned {
            vec![item.to_string()]
        } else {
            Vec::new()
        };
        let candidate = ContentCandidate {
            id: item.to_string(),
            rank: Some(1),
            score: None,
            eligible: true,
            p: None,
        };
        let receipt = receipt.then(|| DecisionReceipt {
            kind: "content".to_string(),
            ok: true,
            request_hash: Some("b3:request".to_string()),
            exposure_hashes: vec!["b3:section".to_string()],
        });
        ContentDecisionRecord {
            identity: AttemptIdentity::new(key),
            decision_point: point,
            policy: "keyword_overlap_top3".to_string(),
            candidates: vec![candidate],
            chosen,
            chosen_propensity: Some(1.0),
            source: None,
            state: Some(DecisionState {
                read: true,
                version: "v1".to_string(),
                digest: "b3:state".to_string(),
                age_s: None,
                n_obs: 1,
            }),
            thresholds_digest: None,
            arm_set: None,
            proposals: Some(ContentProposals {
                learned: Some(vec![item.to_string()]),
                default: Some(Vec::new()),
            }),
            audit: AuditFields {
                layer: Some(layer.to_string()),
                assignment: Some(DecisionAssignment::new(draw, key, 1)),
                decided_at: Some(2),
                receipt,
                ..AuditFields::default()
            },
        }
    }

    /// Run `run_id` of the workspace `dir` with `chains` chains of one
    /// attempt, every fifth on the default arm. Each chain has an L-know row
    /// whose entry was found in the request, and an L-play row without a
    /// receipt.
    fn write_run(dir: &Path, run_id: &str, chains: usize) {
        let run_dir = dir.join(".roko/runs").join(run_id);
        std::fs::create_dir_all(&run_dir).expect("the run dir");
        let (mut lines, mut seq) = (String::new(), 0_u64);
        for chain in 0..chains {
            let key = AttemptKey::new(run_id, "plan", format!("t{chain}"), 1);
            let arm = if chain % 5 == 0 {
                Arm::Default
            } else {
                Arm::Learned
            };
            let know = content_row(&key, ContentDecisionPoint::Knowledge, "kn-1", arm, true);
            let play = content_row(&key, ContentDecisionPoint::Playbooks, "pb-1", arm, false);
            for record in [know, play] {
                seq += 1;
                let stamped = Stamped {
                    schema_version: DECISION_SCHEMA.to_string(),
                    record_id: format!("b3:{run_id}:{seq}"),
                    seq,
                    ts: "2026-10-03T09:00:00Z".to_string(),
                    record,
                };
                let line = serde_json::to_string(&stamped).expect("serialize a decision row");
                lines.push_str(&line);
                lines.push('\n');
            }
        }
        std::fs::write(RunFile::Decisions.path_in(&run_dir), lines)
            .expect("write the decision rows");
    }

    /// Ledger row `record` in a few words: its loop, and its state and
    /// reason or its move.
    fn row_summary(record: &LoopAuditRecord) -> String {
        let loop_id = &record.loop_id;
        match &record.row {
            LoopAuditRow::Health(health) => {
                let reason = health.reason.map_or("-", |reason| reason.as_str());
                format!("{loop_id} health {} {reason}", health.state.as_str())
            }
            LoopAuditRow::Transition(moved) => {
                let (from, to) = (moved.from.as_str(), moved.to.as_str());
                format!("{loop_id} moved {from} -> {to}")
            }
            _ => format!("{loop_id} other"),
        }
    }

    /// [`row_summary`] of each of `records`.
    fn summaries(records: &[LoopAuditRecord]) -> Vec<String> {
        records.iter().map(row_summary).collect()
    }

    /// The loop events `hub` saw, in order, in [`row_summary`]'s words.
    fn loop_events(hub: &crate::state_hub::SharedStateHub) -> Vec<String> {
        hub.subscribe_events_from(0)
            .replay
            .iter()
            .filter_map(|envelope| match &envelope.payload {
                DashboardEvent::LoopHealth {
                    loop_id,
                    state,
                    reason,
                    ..
                } => {
                    let reason = reason.as_deref().unwrap_or("-");
                    Some(format!("{loop_id} health {state} {reason}"))
                }
                DashboardEvent::LoopTransition {
                    loop_id, from, to, ..
                } => Some(format!("{loop_id} moved {from} -> {to}")),
                _ => None,
            })
            .collect()
    }

    /// S03 §5 (backlog 5126): the audit tick folds the runs' decision rows
    /// into each measured loop's health, appends a `loop.health` row per
    /// loop, and a `loop.transition` row for a loop that moved, to the
    /// loop-audit ledger, and publishes each row on the run's StateHub.
    /// L-play's rows carry no receipt, so once its learned arm has N_ε
    /// opportunities it is flagged `dormant:unlogged`; L-know's entries reach
    /// the request, and it stays on probation. A second tick reads L-play's
    /// state back from the ledger and moves nothing within the dwell; a
    /// third, after T_re and 100 more opportunities, re-admits it.
    #[test]
    fn audit_tick_appends_health_rows_and_publishes_events() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), "gr-audit-1", 40);
        let hub = crate::state_hub::shared_state_hub();
        let bridge = GraphTuiBridge::new(TuiBridge::new(hub.sender()));
        let config = LearningAuditConfig::default();
        let ledger = Ledger::in_learn_dir(&dir.path().join(".roko/learn"));
        let t0 = Utc
            .with_ymd_and_hms(2026, 10, 3, 10, 0, 0)
            .single()
            .expect("a valid time");

        assert_eq!(tick(dir.path(), &config, "gr-audit-1", t0, &bridge), 3);
        let rows = ledger.read().expect("read the ledger");
        let first = [KNOW_ON_PROBATION, PLAY_FLAGGED, PLAY_FLAGGED_MOVE];
        assert_eq!(summaries(&rows), first);
        let LoopAuditRow::Health(play) = &rows[1].row else {
            panic!("L-play's health row: {:?}", rows[1]);
        };
        assert_eq!((play.n_opp, play.n_learned, play.n_default), (40, 32, 8));
        let eps = (play.eps.est, play.eps.read, play.eps.receipt);
        assert_eq!(eps, (0.0, 1.0, 0.0));
        assert_eq!(play.beta.reason.as_deref(), Some("eps_below_min"));
        assert_eq!(play.h, 0.5, "a flagged loop runs at h_suspect");
        assert!(play.placebo_ok);
        assert_eq!(play.evidence, "measured");
        assert_eq!(rows[1].run_id, Some(Some("gr-audit-1".to_string())));
        assert_eq!(rows[1].audit_epoch.as_deref(), Some("2026-10-03"));
        assert_eq!(rows[1].ts.as_deref(), Some("2026-10-03T10:00:00Z"));
        let LoopAuditRow::Transition(moved) = &rows[2].row else {
            panic!("L-play's transition row: {:?}", rows[2]);
        };
        assert_eq!(moved.rule, "n_L ≥ N_ε and UCB(ε) < ε_min");
        // Each row reached the run's StateHub, in ledger order.
        assert_eq!(loop_events(&hub), first);

        // An hour later the state comes back from the ledger: L-play is not
        // flagged again, and its dwell holds it.
        let later = t0 + chrono::Duration::hours(1);
        assert_eq!(tick(dir.path(), &config, "gr-audit-1", later, &bridge), 2);
        let rows = ledger.read().expect("read the ledger");
        assert_eq!(summaries(&rows[3..]), [KNOW_ON_PROBATION, PLAY_FLAGGED]);

        // After T_re, with 100 more opportunities in another run, the dwell
        // is over and L-play re-enters probation.
        write_run(dir.path(), "gr-audit-2", 100);
        let after = t0 + chrono::Duration::days(15);
        assert_eq!(tick(dir.path(), &config, "gr-audit-2", after, &bridge), 3);
        let rows = ledger.read().expect("read the ledger");
        let third = [KNOW_ON_PROBATION, PLAY_ON_PROBATION, PLAY_READMITTED];
        assert_eq!(summaries(&rows[5..]), third);
        assert_eq!(loop_events(&hub).len(), 8);
    }
}
