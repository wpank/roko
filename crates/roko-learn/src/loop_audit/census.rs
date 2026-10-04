//! Report-only loop census over the logs that exist today (S03 §0, §7 A1; backlog 5106).
//!
//! [`run`] reads a workspace's learning logs and the loop registry and gives
//! every registered loop one row: `(state, reason, qualifiers, evidence)` in
//! S03 §4.6's closed enum. Log rules read the files below; the registry's
//! declared findings add code-review facts, each printed with its pointer and
//! `verified_at`. The census spends nothing, makes no transitions and writes
//! nothing: [`render_json`] (`roko.loop_census/1`) and [`render_text`] print
//! it. A retired loop keeps its last reason, for its history, and no state.
//!
//! The measured mode (backlog 5123) reads every run's decision rows through
//! `telemetry::report` ([`measure`]) and folds the ones that carry S03's
//! fields (A-DEC) into each loop's opportunities: ε with its decomposition
//! and ι_net (S03 §4.4). Once a loop's learned arm has N_ε opportunities,
//! the measured verdict decides its reason; before that, its logs and its
//! declared findings do. Rows written before A-DEC count as
//! pre-instrumentation. The audit tick (backlog 5126) reads the same fold
//! through [`measure_at`], its sequences at the auditor's α/K.
//!
//! The regulators are measured from their own receipts (S03 §4.8; backlog
//! 5135): L-M1 from M1's `harness_policy` rows (`params_digest`: the θ the
//! attempt ran against the controller's θ, on S06's fixed holdout), L-M3
//! from the route rows its epochs write (`prediction_consumed`: the
//! attempt's prediction row), and L-M4 from later route rows in which
//! audit feedback left a candidate out (`audit_penalty_applied`: the
//! candidate's `audit_trust` reason, S05 DP4).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use roko_core::config::homeostasis::HomeostasisMode;
use serde::Serialize;
use serde_json::Value;

use super::arm_set::ArmSet;
use super::assign::takes_default;
use super::exposure::{
    Action, ExposureEstimate, ExposureEstimator, InfluenceEstimate, InfluenceEstimator,
    Opportunity, ReadStatus,
};
use super::ledger::EpsilonFields;
use super::spec::{AuditState, Lifecycle, LoopSpec, Qualifier, ReasonCode, Registry};
use super::state::AuditParams;
use crate::prompt_experiment::ExperimentStore;
use crate::routing_log::{DecisionState, RoutingDecisionLog};
use crate::runtime_feedback::LearningPaths;
use crate::telemetry::records::{
    AuditFields, ContentDecisionPoint, ContentDecisionRecord, DecisionSource, ExecutedModel,
    HarnessPolicyDecisionRecord, HarnessStamp,
};
use crate::telemetry::report::{RunRecords, undated};

/// Schema of [`render_json`]'s output.
pub const CENSUS_SCHEMA: &str = "roko.loop_census/1";

/// L-linucb learns when its observations reach this share of the router's.
pub const LINUCB_LEARNING_RATIO: f64 = 0.05;

/// Router observations the L-linucb ratio rule needs before it judges.
pub const MIN_ROUTER_OBSERVATIONS: u64 = 30;

/// The retired attention bidders' store, which 4217 left on disk, under the
/// learn directory.
const ATTENTION_BIDDERS: &str = "attention-bidders.json";
/// The prompt assembler's per-section outcomes, under the learn directory.
const SECTION_OUTCOMES: &str = "section-outcomes.jsonl";
/// Graph dispatch's retrieval outcomes, under the learn directory.
const RETRIEVAL_OUTCOMES: &str = "retrieval-outcomes.jsonl";
/// The retired `HoldoutExperiment`'s state, under the learn directory.
const HOLDOUT_STATE: &str = "holdout-state.json";
/// The run directories, under the `.roko` directory.
const RUNS_DIR: &str = "runs";
/// The loop whose layer a route row without a `loop_id` drew on.
const ROUTE_LOOP: &str = "L-route";
/// The loop M1's `harness_policy` rows belong to.
const HARNESS_LOOP: &str = "L-M1";
/// The loop whose epochs let the self-model choose on the route layer.
const SELF_MODEL_LOOP: &str = "L-M3";
/// The loop whose feedback later route rows show.
const AUDIT_FEEDBACK_LOOP: &str = "L-M4";
/// L-M4's layer, when a chain's arm set draws one.
const AUDIT_FEEDBACK_LAYER: &str = "audit_feedback";
/// A route candidate's `ineligible_reason` when audit trust left it out
/// (S05 DP4).
const AUDIT_TRUST_REASON: &str = "audit_trust";

/// Where a row's reason comes from (S03 §9.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
    /// The runs' decision rows measure it.
    Measured,
    /// The workspace's logs show it.
    Log,
    /// A code-review finding in the registry says it.
    Declared,
}

/// A registry finding as the census prints it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeclaredFinding {
    /// The fact, in a sentence.
    pub claim: String,
    /// `path::symbol` the claim is about.
    pub pointer: String,
    /// The commit the claim was checked at.
    pub verified_at: String,
    /// `verified_at` is not the harness's commit.
    pub stale: bool,
}

/// One registered loop's census row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CensusRow {
    /// Registry id, e.g. `L-route`.
    #[serde(rename = "loop")]
    pub loop_id: String,
    /// Where the loop stands in the registry.
    pub lifecycle: Lifecycle,
    /// The audit state: `flagged` with a reason, `probation` without one;
    /// `None` for a retired loop, which makes no transitions.
    pub state: Option<AuditState>,
    /// The reason that wins by S03 §4.6's precedence, if any.
    pub reason: Option<ReasonCode>,
    /// Qualifiers, printed after the reason.
    pub qualifiers: Vec<Qualifier>,
    /// Where the reason comes from.
    pub evidence: Option<Evidence>,
    /// What the logs show about the loop, in sentences.
    pub facts: Vec<String>,
    /// The loop's declared findings.
    pub findings: Vec<DeclaredFinding>,
    /// What the runs' decision rows measure, when any row is the loop's.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measured: Option<MeasuredLoop>,
}

/// One loop's exposure and influence over the runs' decision rows (S03
/// §4.4; backlog 5123).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MeasuredLoop {
    /// Opportunities with S03's fields, on both arms.
    pub n_opp: u64,
    /// Learned-arm opportunities.
    #[serde(rename = "n_L")]
    pub n_learned: u64,
    /// Default-arm opportunities.
    #[serde(rename = "n_D")]
    pub n_default: u64,
    /// The loop's decision rows written before S03's fields.
    pub pre_instrumentation: u64,
    /// ε and its decomposition over the learned-arm opportunities.
    pub eps: EpsilonFields,
    /// ι_net over every opportunity, net of the A/A floor.
    pub iota_net: f64,
    /// The learned arm has N_ε opportunities, so ε is judged.
    pub judged: bool,
    /// The dormant reason ε gives once judged, while it stays below ε_min.
    pub reason: Option<ReasonCode>,
}

/// One loop's measurement for the state machine (backlog 5126): what the
/// census prints, the estimates behind it, and the rows the structural
/// pre-checks read.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopMeasurement {
    /// What the census prints.
    pub measured: MeasuredLoop,
    /// ε and its decomposition, with its confidence sequence.
    pub exposure: ExposureEstimate,
    /// ι_net, with its confidence sequence.
    pub influence: InfluenceEstimate,
    /// The loop's decision rows with S03's fields, opportunities or not.
    pub rows: u64,
    /// Those rows whose arm was assigned at or after the decision.
    pub ordering_violations: u64,
}

/// The census of every registered loop, in registry order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CensusReport {
    /// Always [`CENSUS_SCHEMA`].
    pub schema: String,
    /// The harness's commit, which declared findings are judged against;
    /// `None` when unknown, and then no finding is stale.
    pub harness_sha: Option<String>,
    /// One row per registered loop.
    pub rows: Vec<CensusRow>,
}

impl CensusReport {
    /// The row of loop `id`, if registered.
    #[must_use]
    pub fn row(&self, id: &str) -> Option<&CensusRow> {
        self.rows.iter().find(|row| row.loop_id == id)
    }
}

/// The census of `workdir`'s `.roko` logs against `registry`.
#[must_use]
pub fn run(workdir: &Path, registry: &Registry, harness_sha: Option<&str>) -> CensusReport {
    run_in(&LearningPaths::for_project(workdir), registry, harness_sha)
}

/// The census of the logs at `paths`, and of the runs beside them in the
/// `.roko` directory, against `registry`.
#[must_use]
pub fn run_in(
    paths: &LearningPaths,
    registry: &Registry,
    harness_sha: Option<&str>,
) -> CensusReport {
    let runs = paths
        .root
        .parent()
        .map(|roko_dir| read_runs(&roko_dir.join(RUNS_DIR)))
        .unwrap_or_default();
    run_with(paths, &runs, registry, harness_sha)
}

/// The census of the logs at `paths` and of the run records `runs` against
/// `registry`.
#[must_use]
pub fn run_with(
    paths: &LearningPaths,
    runs: &[RunRecords],
    registry: &Registry,
    harness_sha: Option<&str>,
) -> CensusReport {
    let logs = Logs::read(paths);
    let measured = measure(runs);
    let rows = registry
        .loops()
        .iter()
        .map(|spec| {
            let measured = measured.get(spec.id.as_str());
            census_row(spec, &logs, measured, harness_sha)
        })
        .collect();
    CensusReport {
        schema: CENSUS_SCHEMA.to_string(),
        harness_sha: harness_sha.map(str::to_string),
        rows,
    }
}

/// The harness commit declared findings are judged against:
/// `ROKO_HARNESS_SHA` when set, else `git rev-parse HEAD` in `workdir`;
/// `None` when neither gives one.
#[must_use]
pub fn harness_sha(workdir: &Path) -> Option<String> {
    if let Ok(sha) = std::env::var("ROKO_HARNESS_SHA")
        && !sha.trim().is_empty()
    {
        return Some(sha.trim().to_string());
    }
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    let sha = String::from_utf8(output.stdout).ok()?.trim().to_string();
    (output.status.success() && !sha.is_empty()).then_some(sha)
}

/// `report` as `roko.loop_census/1` JSON, one trailing newline.
///
/// # Errors
///
/// Returns the serializer's error; the report's types always serialize.
pub fn render_json(report: &CensusReport) -> serde_json::Result<String> {
    let mut json = serde_json::to_string_pretty(report)?;
    json.push('\n');
    Ok(json)
}

/// `report` as a table: one line per loop, `id state reason qualifiers
/// evidence`.
#[must_use]
pub fn render_text(report: &CensusReport) -> String {
    let mut out = format!(
        "{:<16} {:<16} {:<24} {:<26} {}\n",
        "loop", "state", "reason", "qualifiers", "evidence"
    );
    for row in &report.rows {
        let state = match (&row.lifecycle, row.state) {
            (Lifecycle::Retired { by }, _) => format!("retired({by})"),
            (_, Some(state)) => serde_label(&state),
            (_, None) => "-".to_string(),
        };
        let reason = row.reason.map_or("-", ReasonCode::as_str);
        let qualifiers = if row.qualifiers.is_empty() {
            "-".to_string()
        } else {
            let labels: Vec<&str> = row.qualifiers.iter().map(|q| q.as_str()).collect();
            labels.join(",")
        };
        let evidence = row.evidence.map_or("-".to_string(), |e| serde_label(&e));
        out.push_str(&format!(
            "{:<16} {:<16} {:<24} {:<26} {}\n",
            row.loop_id, state, reason, qualifiers, evidence
        ));
    }
    out
}

/// The snake_case label serde gives a unit variant.
fn serde_label(value: &impl Serialize) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

/// One loop's row. A judged measurement decides the reason (S03 §9.11:
/// measured, then log, then declared); otherwise its log rules and declared
/// findings do, by S03 §4.6's precedence, log evidence first on a tie.
fn census_row(
    spec: &LoopSpec,
    logs: &Logs,
    measured: Option<&MeasuredLoop>,
    harness_sha: Option<&str>,
) -> CensusRow {
    let mut facts = Vec::new();
    let mut candidates: Vec<(ReasonCode, Evidence, bool)> = Vec::new();
    let mut qualifiers = Vec::new();
    if let Some(LogVerdict { reason, qualifier }) = logs.verdict(spec.id.as_str(), &mut facts) {
        candidates.extend(reason.map(|code| (code, Evidence::Log, false)));
        qualifiers.extend(qualifier);
    }
    let findings: Vec<DeclaredFinding> = spec
        .static_findings
        .iter()
        .map(|finding| DeclaredFinding {
            claim: finding.claim.clone(),
            pointer: finding.pointer.clone(),
            verified_at: finding.verified_at.clone(),
            stale: harness_sha.is_some_and(|sha| !same_commit(sha, &finding.verified_at)),
        })
        .collect();
    for (finding, printed) in spec.static_findings.iter().zip(&findings) {
        if let Some(code) = finding.reason {
            candidates.push((code, Evidence::Declared, printed.stale));
        }
        qualifiers.extend(finding.qualifier);
    }
    // The cheapest code wins; on a tie, log evidence beats a declaration.
    candidates
        .sort_by_key(|(code, evidence, _)| (code.precedence(), *evidence == Evidence::Declared));
    let mut winner = candidates.first().copied();
    let judged = measured.filter(|measured| measured.judged);
    if let Some(measured) = measured {
        facts.push(measured_fact(measured));
    }
    if let Some(measured) = judged {
        winner = measured
            .reason
            .map(|code| (code, Evidence::Measured, false));
    }
    if winner.is_some_and(|(_, evidence, stale)| evidence == Evidence::Declared && stale) {
        qualifiers.push(Qualifier::DeclaredStale);
    }
    qualifiers.sort_by_key(|qualifier| *qualifier as u8);
    qualifiers.dedup();
    let reason = winner.map(|(code, _, _)| code);
    let state = match spec.lifecycle {
        Lifecycle::Retired { .. } => None,
        _ if reason.is_some() => Some(AuditState::Flagged),
        _ => Some(AuditState::Probation),
    };
    CensusRow {
        loop_id: spec.id.as_str().to_string(),
        lifecycle: spec.lifecycle.clone(),
        state,
        reason,
        qualifiers,
        evidence: judged
            .map(|_| Evidence::Measured)
            .or(winner.map(|(_, evidence, _)| evidence)),
        facts,
        findings,
        measured: measured.cloned(),
    }
}

/// What the runs' decision rows show about a loop, in a sentence.
fn measured_fact(measured: &MeasuredLoop) -> String {
    let (n, learned) = (measured.n_opp, measured.n_learned);
    let pre = measured.pre_instrumentation;
    let (eps, iota) = (measured.eps, measured.iota_net);
    format!(
        "{n} opportunities measured, {learned} on the learned arm, and {pre} rows from before \
         S03's fields: ε {:.2} (read {:.2}, reach {:.2}, honest {:.2}, receipt {:.2}), \
         ι_net {iota:.2}",
        eps.est, eps.read, eps.reach, eps.honest, eps.receipt
    )
}

/// Every run directory under `runs_dir`, in name order, read by
/// `telemetry::report`; a run that cannot be read is skipped.
#[must_use]
pub fn read_runs(runs_dir: &Path) -> Vec<RunRecords> {
    let Ok(entries) = std::fs::read_dir(runs_dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs.iter()
        .filter_map(|dir| RunRecords::load(dir).ok())
        .collect()
}

/// Each loop's exposure and influence over the decision rows of `runs`
/// (S03 §4.4), by loop id. A route row belongs to its `loop_id`, L-route by
/// default; knowledge and playbook rows belong to L-know and L-play; M1's
/// `harness_policy` rows to L-M1; and a route row in which audit feedback
/// left a candidate out counts for L-M4 too (S03 §4.8). A row without S03's
/// fields counts as pre-instrumentation, and a pin (a task hint, an
/// override, a ladder rung outside L-M3's epochs) is no opportunity.
#[must_use]
pub fn measure(runs: &[RunRecords]) -> BTreeMap<String, MeasuredLoop> {
    measure_at(runs, &AuditParams::default())
        .into_iter()
        .map(|(loop_id, measurement)| (loop_id, measurement.measured))
        .collect()
}

/// What [`measure`] gives, with the estimates behind it for the state
/// machine (backlog 5126): their sequences at `params`' α/K, and judged
/// against its N_ε and ε_min.
#[must_use]
pub fn measure_at(runs: &[RunRecords], params: &AuditParams) -> BTreeMap<String, LoopMeasurement> {
    tallies(runs, params.loop_alpha())
        .into_iter()
        .map(|(loop_id, tally)| (loop_id, tally.measurement(params)))
        .collect()
}

/// Each loop's tally over the decision rows of `runs`, its sequences at
/// level `alpha`.
fn tallies(runs: &[RunRecords], alpha: f64) -> BTreeMap<String, Tally> {
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for run in runs {
        let executed: HashMap<&str, &ExecutedModel> = run
            .verdicts
            .iter()
            .map(|line| {
                (
                    line.record.identity.attempt_key.as_str(),
                    &line.record.executed,
                )
            })
            .collect();
        let predicted: HashSet<&str> = run
            .predictions
            .iter()
            .map(|line| line.record.identity.attempt_key.as_str())
            .collect();
        for line in &run.decisions {
            let row = &line.record;
            let ran = row
                .attempt_key
                .as_deref()
                .and_then(|key| executed.get(key).copied());
            if let Some(opportunity) = audit_feedback_opportunity(row, ran) {
                let feedback = tally_of(&mut tallies, AUDIT_FEEDBACK_LOOP, alpha);
                if feedback.count(&row.audit) {
                    feedback.push(&opportunity);
                }
            }
            let loop_id = row.audit.loop_id.as_deref().unwrap_or(ROUTE_LOOP);
            let loop_tally = tally_of(&mut tallies, loop_id, alpha);
            if !loop_tally.count(&row.audit) {
                continue;
            }
            let opportunity = if loop_id == SELF_MODEL_LOOP {
                let key = row.attempt_key.as_deref();
                let consumed = key.is_some_and(|key| predicted.contains(key));
                self_model_opportunity(row, ran, consumed)
            } else {
                route_opportunity(row, ran)
            };
            if let Some(opportunity) = opportunity {
                loop_tally.push(&opportunity);
            }
        }
        for line in &run.content_decisions {
            let row = &line.record;
            let loop_id = row.audit.loop_id.as_deref();
            let Some(loop_id) = loop_id.or_else(|| content_loop(row.decision_point)) else {
                continue;
            };
            let loop_tally = tally_of(&mut tallies, loop_id, alpha);
            if !loop_tally.count(&row.audit) {
                continue;
            }
            if let Some(opportunity) = content_opportunity(row) {
                loop_tally.push(&opportunity);
            }
        }
        let stamps: HashMap<&str, &HarnessStamp> = run
            .verdicts
            .iter()
            .filter_map(|line| {
                let stamp = line.record.harness.as_ref()?;
                Some((line.record.identity.attempt_key.as_str(), stamp))
            })
            .collect();
        for line in &run.harness_decisions {
            let row = &line.record;
            let loop_tally = tally_of(&mut tallies, HARNESS_LOOP, alpha);
            loop_tally.rows += 1;
            let stamp = stamps.get(row.identity.attempt_key.as_str()).copied();
            if let Some(opportunity) = harness_opportunity(row, stamp) {
                loop_tally.push(&opportunity);
            }
        }
    }
    tallies
}

/// `loop_id`'s tally in `tallies`, started at level `alpha` when new.
fn tally_of<'a>(
    tallies: &'a mut BTreeMap<String, Tally>,
    loop_id: &str,
    alpha: f64,
) -> &'a mut Tally {
    tallies
        .entry(loop_id.to_string())
        .or_insert_with(|| Tally::new(alpha))
}

/// The loop that reads a content decision point's outcome, for the points
/// whose rows carry proposals (backlog 5125).
const fn content_loop(point: ContentDecisionPoint) -> Option<&'static str> {
    match point {
        ContentDecisionPoint::Knowledge => Some("L-know"),
        ContentDecisionPoint::Playbooks => Some("L-play"),
        _ => None,
    }
}

/// The opportunity a route row records, if it is one: the router proposed a
/// pick (`proposals.learned`), and no pin decided. The receipt is the row's
/// own or, failing that, the provider's report on the attempt's verdict
/// (backlog 5124's option a), and the label is honest when that report
/// names the labelled model.
fn route_opportunity(
    row: &RoutingDecisionLog,
    ran: Option<&ExecutedModel>,
) -> Option<Opportunity<String>> {
    let pinned = matches!(
        row.source,
        Some(DecisionSource::TaskHint | DecisionSource::Override | DecisionSource::Ladder)
    );
    if pinned || !eligible(&row.audit) {
        return None;
    }
    let learned = row.proposals.learned.clone()?;
    let reported = ran.and_then(|ran| ran.model_reported.as_deref());
    let (honest, receipt) = match (&row.audit.receipt, reported) {
        (Some(receipt), _) => (receipt.ok, true),
        (None, Some(reported)) => (undated(reported) == undated(&row.selected_model), true),
        (None, None) => (true, false),
    };
    let (learned_arm, propensity) = arm_of(&row.audit, row.arm_set.as_ref());
    let default = row
        .proposals
        .default
        .clone()
        .or_else(|| row.default_model.clone());
    Some(Opportunity {
        learned_arm,
        propensity,
        learned,
        default: default.unwrap_or_default(),
        learned_again: row.proposals.aa.clone(),
        executed: row.selected_model.clone(),
        honest,
        read: read_status(row.state.as_ref()),
        receipt,
        cites: None,
    })
}

/// The opportunity a content row records, if it is one: the store offered
/// candidates. Its proposals are id sets, and its receipt, the rendered
/// sections found in the request, says whether the label is honest.
fn content_opportunity(row: &ContentDecisionRecord) -> Option<Opportunity<BTreeSet<String>>> {
    let offered = !row.candidates.is_empty();
    let eligible = row
        .audit
        .opportunity
        .as_ref()
        .map_or(offered, |opportunity| opportunity.eligible);
    if !eligible {
        return None;
    }
    let proposals = row.proposals.clone().unwrap_or_default();
    let set = |ids: Option<Vec<String>>| -> BTreeSet<String> {
        ids.unwrap_or_default().into_iter().collect()
    };
    let (learned_arm, propensity) = arm_of(&row.audit, row.arm_set.as_ref());
    let receipt = row.audit.receipt.as_ref();
    Some(Opportunity {
        learned_arm,
        propensity,
        learned: set(proposals.learned),
        default: set(proposals.default),
        learned_again: None,
        executed: row.chosen.iter().cloned().collect(),
        honest: receipt.is_none_or(|receipt| receipt.ok),
        read: read_status(row.state.as_ref()),
        receipt: receipt.is_some(),
        cites: None,
    })
}

/// The opportunity an L-M3 route row records (S03 §4.8), if it is one: the
/// self-model proposed the start rung and no pin decided. Its held-out
/// chains ran the ladder's own rung, π⁰, so a ladder source is its default
/// arm rather than a pin. The self-model read its state to propose, and the
/// receipt is the attempt's consumed prediction row.
fn self_model_opportunity(
    row: &RoutingDecisionLog,
    ran: Option<&ExecutedModel>,
    consumed: bool,
) -> Option<Opportunity<String>> {
    let pinned = matches!(
        row.source,
        Some(DecisionSource::TaskHint | DecisionSource::Override)
    );
    if pinned || !eligible(&row.audit) {
        return None;
    }
    let learned = row.proposals.learned.clone()?;
    let reported = ran.and_then(|ran| ran.model_reported.as_deref());
    let honest = reported.is_none_or(|reported| undated(reported) == undated(&row.selected_model));
    let (learned_arm, propensity) = arm_of(&row.audit, row.arm_set.as_ref());
    let default = row
        .proposals
        .default
        .clone()
        .or_else(|| row.default_model.clone());
    Some(Opportunity {
        learned_arm,
        propensity,
        learned,
        default: default.unwrap_or_default(),
        learned_again: row.proposals.aa.clone(),
        executed: row.selected_model.clone(),
        honest,
        read: ReadStatus::Loaded,
        receipt: consumed,
        cites: None,
    })
}

/// The opportunity a route row records for L-M4 (S03 §4.8), if it is one:
/// audit feedback left a candidate out (S05 DP4). The learned action is the
/// candidates feedback allowed; π⁰'s, every candidate; the executed one,
/// the allowed set, widened by an excluded model that ran all the same. The
/// receipt is the candidates' `audit_trust` reason, the penalty in a later
/// decision's learned state.
fn audit_feedback_opportunity(
    row: &RoutingDecisionLog,
    ran: Option<&ExecutedModel>,
) -> Option<Opportunity<BTreeSet<String>>> {
    let reason = Some(AUDIT_TRUST_REASON);
    let excluded: BTreeSet<String> = row
        .candidates
        .iter()
        .filter(|candidate| candidate.ineligible_reason.as_deref() == reason)
        .map(|candidate| candidate.model.clone())
        .collect();
    if excluded.is_empty() {
        return None;
    }
    let every: BTreeSet<String> = row
        .candidates
        .iter()
        .map(|candidate| candidate.model.clone())
        .collect();
    let learned: BTreeSet<String> = every.difference(&excluded).cloned().collect();
    let mut executed = learned.clone();
    if excluded.contains(&row.selected_model) {
        executed.insert(row.selected_model.clone());
    }
    let reported = ran.and_then(|ran| ran.model_reported.as_deref());
    let honest = reported.is_none_or(|reported| undated(reported) == undated(&row.selected_model));
    let draw = row
        .arm_set
        .as_ref()
        .and_then(|arms| arms.get(AUDIT_FEEDBACK_LAYER));
    let (learned_arm, propensity) = draw.map_or((true, 1.0), |draw| {
        (!takes_default(draw.arm), draw.propensity)
    });
    Some(Opportunity {
        learned_arm,
        propensity,
        learned,
        default: every,
        learned_again: None,
        executed,
        honest,
        read: read_status(row.state.as_ref()),
        receipt: true,
        cites: None,
    })
}

/// The opportunity M1's `harness_policy` row records (S03 §4.8), if it is
/// one: the controller's θ differs from θ₀ and no pin chose the model. The
/// actions are θs by digest: the controller's (learned), θ₀ (π⁰) and the θ
/// the attempt ran. The controller is read unless M1 is off; the receipt is
/// the row's params digest, honest when the verdict's stamp names the same
/// θ.
fn harness_opportunity(
    row: &HarnessPolicyDecisionRecord,
    stamp: Option<&HarnessStamp>,
) -> Option<Opportunity<String>> {
    if !row.differs || row.pinned {
        return None;
    }
    let read = if row.mode == HomeostasisMode::Off {
        ReadStatus::Missing
    } else {
        ReadStatus::Loaded
    };
    let honest = stamp.is_none_or(|stamp| stamp.params_digest == row.params_digest);
    Some(Opportunity {
        learned_arm: !takes_default(row.arm),
        propensity: row.assignment.propensity,
        learned: row.chosen.params_digest(),
        default: row.default.params_digest(),
        learned_again: None,
        executed: row.params_digest.clone(),
        honest,
        read,
        receipt: !row.params_digest.is_empty(),
        cites: None,
    })
}

/// Whether S03's fields leave the row an opportunity: they say so, or say
/// nothing.
fn eligible(audit: &AuditFields) -> bool {
    audit
        .opportunity
        .as_ref()
        .is_none_or(|opportunity| opportunity.eligible)
}

/// Whether the row's unit drew the learned arm, and that arm's logged
/// propensity: from S03's `assignment`, else from the chain's arm set on the
/// row's layer. A row with neither ran learned, at propensity 1.
fn arm_of(audit: &AuditFields, arm_set: Option<&ArmSet>) -> (bool, f64) {
    let draw = audit
        .assignment
        .as_ref()
        .map(|assignment| &assignment.draw)
        .or_else(|| arm_set?.get(audit.layer.as_deref()?));
    draw.map_or((true, 1.0), |draw| {
        (!takes_default(draw.arm), draw.propensity)
    })
}

/// What a decision's reader loaded: its state, an empty one, or none.
fn read_status(state: Option<&DecisionState>) -> ReadStatus {
    match state {
        Some(state) if state.read => ReadStatus::Loaded,
        Some(_) => ReadStatus::Empty,
        None => ReadStatus::Missing,
    }
}

/// Whether S03's fields say the row's arm was assigned at or after its
/// decision (S03 §4.6's ordering pre-check, `assigned_at ≥ decided_at`).
fn assigned_late(audit: &AuditFields) -> bool {
    let assigned_at = audit
        .assignment
        .as_ref()
        .map(|assignment| assignment.assigned_at);
    assigned_at
        .zip(audit.decided_at)
        .is_some_and(|(assigned_at, decided_at)| assigned_at >= decided_at)
}

/// One loop's opportunities, as the census folds them.
#[derive(Debug, Clone)]
struct Tally {
    exposure: ExposureEstimator,
    influence: InfluenceEstimator,
    n_learned: u64,
    n_default: u64,
    pre_instrumentation: u64,
    rows: u64,
    ordering_violations: u64,
}

impl Tally {
    fn new(alpha: f64) -> Self {
        Self {
            exposure: ExposureEstimator::new(alpha),
            influence: InfluenceEstimator::new(alpha),
            n_learned: 0,
            n_default: 0,
            pre_instrumentation: 0,
            rows: 0,
            ordering_violations: 0,
        }
    }

    /// Count a decision row by its S03 fields `audit`: a row that has them,
    /// and whether its arm was assigned late, or a row from before them.
    /// Returns whether it has them.
    fn count(&mut self, audit: &AuditFields) -> bool {
        if !audit.present() {
            self.pre_instrumentation += 1;
            return false;
        }
        self.rows += 1;
        self.ordering_violations += u64::from(assigned_late(audit));
        true
    }

    fn push<A: Action>(&mut self, opportunity: &Opportunity<A>) {
        self.exposure.push(opportunity);
        self.influence.push(opportunity);
        if opportunity.learned_arm {
            self.n_learned += 1;
        } else {
            self.n_default += 1;
        }
    }

    fn finish(&self, params: &AuditParams) -> MeasuredLoop {
        let exposure = self.exposure.estimate();
        let judged = exposure.opportunities >= params.n_eps;
        MeasuredLoop {
            n_opp: self.n_learned + self.n_default,
            n_learned: self.n_learned,
            n_default: self.n_default,
            pre_instrumentation: self.pre_instrumentation,
            eps: EpsilonFields {
                est: exposure.epsilon,
                ucb: exposure.interval.map_or(exposure.epsilon, |(_, high)| high),
                read: exposure.read,
                reach: exposure.reach,
                honest: exposure.honest,
                receipt: exposure.receipt,
            },
            iota_net: self.influence.estimate().iota_net,
            judged,
            reason: judged
                .then(|| exposure.dormant_reason(params.eps_min))
                .flatten(),
        }
    }

    fn measurement(&self, params: &AuditParams) -> LoopMeasurement {
        LoopMeasurement {
            measured: self.finish(params),
            exposure: self.exposure.estimate(),
            influence: self.influence.estimate(),
            rows: self.rows,
            ordering_violations: self.ordering_violations,
        }
    }
}

/// Whether two commit shas name the same commit: one is a prefix of the
/// other, at least 7 hex digits long.
fn same_commit(left: &str, right: &str) -> bool {
    let (left, right) = (left.trim(), right.trim());
    let shared = left.len().min(right.len());
    shared >= 7 && left[..shared].eq_ignore_ascii_case(&right[..shared])
}

/// What a loop's log rule found: a reason, a qualifier, or both.
struct LogVerdict {
    reason: Option<ReasonCode>,
    qualifier: Option<Qualifier>,
}

/// The counts the log rules read, gathered once per census.
#[derive(Debug, Default)]
struct Logs {
    router_observations: Option<u64>,
    linucb_observations: Option<u64>,
    bidder_posteriors: Vec<(f64, f64)>,
    section_outcomes: usize,
    sections_included: usize,
    retrievals: usize,
    episodes: usize,
    episodes_with_knowledge: usize,
    episodes_with_playbooks: usize,
    efficiency_rows: usize,
    efficiency_with_knowledge: usize,
    efficiency_with_playbooks: usize,
    playbooks: usize,
    holdout_costs: Option<(f64, f64)>,
    prompt_experiments: Option<usize>,
}

impl Logs {
    fn read(paths: &LearningPaths) -> Self {
        let mut logs = Self::default();
        if let Some(router) = read_json(&paths.cascade_router_json) {
            logs.router_observations = router["total_observations"].as_u64();
            logs.linucb_observations = router["linucb_state"]["observations"].as_u64();
        }
        if let Some(Value::Object(subsystems)) = read_json(&paths.root.join(ATTENTION_BIDDERS)) {
            for bidder in subsystems.values() {
                let Some(betas) = bidder["section_betas"].as_object() else {
                    continue;
                };
                for beta in betas.values() {
                    if let (Some(a), Some(b)) = (beta[0].as_f64(), beta[1].as_f64()) {
                        logs.bidder_posteriors.push((a, b));
                    }
                }
            }
        }
        for row in read_jsonl(&paths.root.join(SECTION_OUTCOMES)) {
            logs.section_outcomes += 1;
            logs.sections_included += usize::from(row["included"].as_bool() == Some(true));
        }
        logs.retrievals = read_jsonl(&paths.root.join(RETRIEVAL_OUTCOMES)).len();
        for episode in read_jsonl(&paths.episodes_jsonl) {
            logs.episodes += 1;
            let knowledge = has_ids(&episode["knowledge_ids_injected"]);
            logs.episodes_with_knowledge += usize::from(knowledge);
            let extra = &episode["extra"];
            let playbooks = has_ids(&extra["playbook_ids"]) || has_id(&extra["playbook_id"]);
            logs.episodes_with_playbooks += usize::from(playbooks);
        }
        for row in read_jsonl(&paths.efficiency_jsonl) {
            logs.efficiency_rows += 1;
            logs.efficiency_with_knowledge += usize::from(has_ids(&row["knowledge_ids"]));
            logs.efficiency_with_playbooks += usize::from(has_ids(&row["playbook_ids"]));
        }
        logs.playbooks = std::fs::read_dir(&paths.playbooks_dir).map_or(0, |entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
                .count()
        });
        if let Some(holdout) = read_json(&paths.root.join(HOLDOUT_STATE)) {
            let cost = |arm: &str| holdout[arm]["total_cost"].as_f64();
            if let (Some(train), Some(held)) = (cost("train_metrics"), cost("holdout_metrics")) {
                logs.holdout_costs = Some((train, held));
            }
        }
        if let Some(store) = read_json(&paths.experiments_json) {
            let experiments = store["experiments"].as_object();
            logs.prompt_experiments = experiments.map(|experiments| {
                experiments
                    .keys()
                    .filter(|id| id.as_str() != ExperimentStore::RETRIEVAL_STRATEGY_EXPERIMENT_ID)
                    .count()
            });
        }
        logs
    }

    /// The log rule of loop `id`, adding what it read to `facts`; `None`
    /// for a loop without one, or when its logs are missing.
    fn verdict(&self, id: &str, facts: &mut Vec<String>) -> Option<LogVerdict> {
        let reason = |code| {
            Some(LogVerdict {
                reason: Some(code),
                qualifier: None,
            })
        };
        match id {
            "L-linucb" => {
                let (router, linucb) = (self.router_observations?, self.linucb_observations?);
                facts.push(format!(
                    "LinUCB learned from {linucb} of {router} router observations"
                ));
                let starved = router >= MIN_ROUTER_OBSERVATIONS
                    && (linucb as f64) < LINUCB_LEARNING_RATIO * router as f64;
                if starved {
                    reason(ReasonCode::NoLearning)
                } else {
                    None
                }
            }
            "L-bid" => {
                let one_posterior = self
                    .bidder_posteriors
                    .first()
                    .filter(|first| self.bidder_posteriors.iter().all(|beta| beta == *first));
                if let Some((a, b)) = one_posterior {
                    facts.push(format!(
                        "all {} sections share the Beta({a}, {b}) posterior",
                        self.bidder_posteriors.len()
                    ));
                }
                let all_included =
                    self.section_outcomes > 0 && self.sections_included == self.section_outcomes;
                if all_included {
                    facts.push(format!(
                        "{0}/{0} section outcomes are included",
                        self.section_outcomes
                    ));
                }
                if one_posterior.is_some() || all_included {
                    reason(ReasonCode::Degenerate)
                } else {
                    None
                }
            }
            "L-know" => {
                let unlogged = self.retrievals > 0
                    && self.episodes_with_knowledge == 0
                    && self.efficiency_with_knowledge == 0;
                facts.push(format!(
                    "{} retrievals; {}/{} episodes and {}/{} efficiency rows carry knowledge ids",
                    self.retrievals,
                    self.episodes_with_knowledge,
                    self.episodes,
                    self.efficiency_with_knowledge,
                    self.efficiency_rows
                ));
                if unlogged {
                    reason(ReasonCode::Unlogged)
                } else {
                    None
                }
            }
            "L-play" => {
                let unlogged = self.playbooks > 0
                    && self.episodes_with_playbooks == 0
                    && self.efficiency_with_playbooks == 0;
                facts.push(format!(
                    "{} playbooks; {}/{} episodes and {}/{} efficiency rows carry playbook ids",
                    self.playbooks,
                    self.episodes_with_playbooks,
                    self.episodes,
                    self.efficiency_with_playbooks,
                    self.efficiency_rows
                ));
                if unlogged {
                    reason(ReasonCode::Unlogged)
                } else {
                    None
                }
            }
            "L-holdout" => {
                let (train, held) = self.holdout_costs?;
                facts.push(format!(
                    "the train arm cost {train} and the holdout arm {held}"
                ));
                (train == 0.0 && held == 0.0).then_some(LogVerdict {
                    reason: None,
                    qualifier: Some(Qualifier::Misspecified),
                })
            }
            "L-prompt-exp" => {
                let registered = self.prompt_experiments?;
                facts.push(format!("{registered} prompt experiments are registered"));
                if registered == 0 {
                    reason(ReasonCode::NoOpportunity)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Whether `value` is a non-empty array.
fn has_ids(value: &Value) -> bool {
    value.as_array().is_some_and(|ids| !ids.is_empty())
}

/// Whether `value` is a non-empty string.
fn has_id(value: &Value) -> bool {
    value.as_str().is_some_and(|id| !id.is_empty())
}

/// The JSON document at `path`; `None` when it is missing or unreadable.
fn read_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// The JSON rows of the JSONL file at `path`, skipping unreadable lines;
/// none when the file is missing.
fn read_jsonl(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::assign::{Arm, Assignment, AssignmentUnit};
    use crate::telemetry::records::{
        AttemptIdentity, AttemptKey, AttemptOutcome, AttemptVerdictRecord, ContentCandidate,
        ContentProposals, DECISION_SCHEMA, DecisionAssignment, DecisionReceipt, RunFile, Stamped,
        VERDICT_SCHEMA,
    };

    /// The 09-29 snapshot fixture (backlog 5105), laid out like `.roko`.
    fn fixture() -> LearningPaths {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/loop_census");
        LearningPaths::for_roko_dir(root)
    }

    /// S03 §7 A1 on the 09-29 fixture: every known-dormant loop is found,
    /// with the reason, qualifiers and evidence the logs and the registry
    /// give it, and no row is live. Retired loops keep their last reason.
    #[test]
    fn census_flags_known_dormant_loops() {
        use AuditState::Flagged;
        use Evidence::{Declared, Log};
        use Qualifier::Misspecified;
        use ReasonCode as R;

        let registry = Registry::embedded().expect("the embedded registry");
        let report = run_in(&fixture(), &registry, None);
        assert_eq!(report.rows.len(), registry.loops().len());
        let retired = |by: &str| Lifecycle::Retired { by: by.to_string() };
        let expected = [
            (
                "L-linucb",
                Lifecycle::Active,
                Some(Flagged),
                R::NoLearning,
                vec![],
                Log,
            ),
            ("L-bid", retired("4217"), None, R::Degenerate, vec![], Log),
            (
                "L-know",
                Lifecycle::Active,
                Some(Flagged),
                R::Unlogged,
                vec![],
                Log,
            ),
            (
                "L-play",
                Lifecycle::Active,
                Some(Flagged),
                R::Unlogged,
                vec![],
                Log,
            ),
            (
                "L-route",
                Lifecycle::Active,
                Some(Flagged),
                R::Mask,
                vec![],
                Declared,
            ),
            (
                "L-prompt-exp",
                Lifecycle::Active,
                Some(Flagged),
                R::NoOpportunity,
                vec![],
                Log,
            ),
            (
                "L-gate-thr",
                Lifecycle::ObserveOnly,
                Some(Flagged),
                R::NoOpportunity,
                vec![],
                Declared,
            ),
            (
                "L-holdout",
                retired("4101"),
                None,
                R::WriteOnly,
                vec![Misspecified],
                Declared,
            ),
            (
                "L-rag11",
                retired("4105"),
                None,
                R::LabelOnly,
                vec![],
                Declared,
            ),
        ];
        for (id, lifecycle, state, reason, qualifiers, evidence) in expected {
            let row = report.row(id).unwrap_or_else(|| panic!("{id} has no row"));
            let got = (
                &row.lifecycle,
                row.state,
                row.reason,
                &row.qualifiers,
                row.evidence,
            );
            let want = (&lifecycle, state, Some(reason), &qualifiers, Some(evidence));
            assert_eq!(got, want, "{id}: {:?}", row.facts);
        }
        assert!(
            report
                .rows
                .iter()
                .all(|row| row.state != Some(AuditState::Live)),
            "no loop is live"
        );
        let linucb = report.row("L-linucb").expect("L-linucb");
        assert_eq!(
            linucb.facts,
            ["LinUCB learned from 1 of 275 router observations"]
        );
    }

    /// A commit no finding was verified at.
    const OTHER_SHA: &str = "0000000000000000000000000000000000000000";

    /// A declared finding checked at another commit than the harness's is
    /// stale, and a row whose reason rests on it says so.
    #[test]
    fn census_qualifies_stale_declared_findings() {
        let registry = Registry::embedded().expect("the embedded registry");
        let report = run_in(&fixture(), &registry, Some(OTHER_SHA));
        let route = report.row("L-route").expect("L-route");
        assert!(route.findings.iter().all(|finding| finding.stale));
        assert!(route.qualifiers.contains(&Qualifier::DeclaredStale));
        let linucb = report.row("L-linucb").expect("L-linucb");
        assert!(
            !linucb.qualifiers.contains(&Qualifier::DeclaredStale),
            "log evidence"
        );

        let json = render_json(&report).expect("render");
        let parsed: Value = serde_json::from_str(&json).expect("JSON");
        assert_eq!(parsed["schema"], CENSUS_SCHEMA);
        assert_eq!(parsed["rows"][0]["loop"], "L-route");
        assert!(render_text(&report).contains("retired(4101)"));
    }

    /// `record` as one run-file line, in the writer's envelope.
    fn line<T: Serialize>(schema: &str, seq: u64, record: T) -> String {
        let stamped = Stamped {
            schema_version: schema.to_string(),
            record_id: format!("b3:{seq}"),
            seq,
            ts: "2026-10-03T09:00:00Z".to_string(),
            record,
        };
        serde_json::to_string(&stamped).expect("serialize a run line") + "\n"
    }

    /// A learned state that was read.
    fn read_state() -> DecisionState {
        DecisionState {
            read: true,
            version: "v1".to_string(),
            digest: "b3:state".to_string(),
            age_s: None,
            n_obs: 1,
        }
    }

    /// A draw on `layer` for `chain_key` that landed in `arm`, at h = 0.2.
    fn draw(layer: &str, chain_key: &str, arm: Arm) -> DecisionAssignment {
        let propensity = if arm == Arm::Learned { 0.8 } else { 0.2 };
        DecisionAssignment {
            draw: Assignment {
                unit: AssignmentUnit::Chain,
                layer: layer.to_string(),
                salt_id: format!("{layer}@2026-10-03"),
                u: 0.5,
                h: 0.2,
                g: 0.0,
                arm,
                propensity,
            },
            unit_key: chain_key.to_string(),
            audit_epoch: "2026-10-03".to_string(),
            global_off: false,
            assigned_at: 1,
        }
    }

    /// Attempt `key`'s content decision at `point` on `layer`, with S03's
    /// fields: the learned reader proposes `item`, which only the learned arm
    /// includes, and with `receipt` the rendered section was found in the
    /// request.
    fn content_row(
        key: &AttemptKey,
        point: ContentDecisionPoint,
        layer: &str,
        item: &str,
        arm: Arm,
        receipt: bool,
    ) -> ContentDecisionRecord {
        let included = if arm == Arm::Learned {
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
            chosen: included,
            chosen_propensity: Some(1.0),
            source: None,
            state: Some(read_state()),
            thresholds_digest: None,
            arm_set: None,
            proposals: Some(ContentProposals {
                learned: Some(vec![item.to_string()]),
                default: Some(Vec::new()),
            }),
            audit: AuditFields {
                layer: Some(layer.to_string()),
                assignment: Some(draw(layer, &key.chain_key(), arm)),
                decided_at: Some(2),
                receipt,
                ..AuditFields::default()
            },
        }
    }

    /// Attempt `key`'s route row: the router proposed `model-a`, the row
    /// labels `model-b`, and with `adec` it carries S03's fields.
    fn route_row(key: &AttemptKey, adec: bool) -> RoutingDecisionLog {
        let template = serde_json::json!({
            "task_id": key.task_id,
            "selected_model": "model-b",
            "candidates": [],
        });
        let mut row: RoutingDecisionLog = serde_json::from_value(template).expect("a route row");
        row.attempt_key = Some(key.attempt_key());
        row.source = Some(DecisionSource::Router);
        row.proposals.learned = Some("model-a".to_string());
        row.proposals.default = Some("model-c".to_string());
        row.state = Some(read_state());
        if adec {
            row.audit.layer = Some("route".to_string());
            row.audit.decided_at = Some(2);
        }
        row
    }

    /// S03 §7 A3 on a synthetic run: L-know's repaired rows measure ε > 0;
    /// L-route's masked rows (the router proposed one model, the provider
    /// served another) give `dormant:mask`; L-play's rows without receipts
    /// give `dormant:unlogged`; route rows from before S03's fields count as
    /// pre-instrumentation. A judged measurement decides the reason.
    #[test]
    fn census_measures_exposure_from_decision_rows() {
        let dir = tempfile::tempdir().expect("temp dir");
        let run_dir = dir.path().join(".roko").join(RUNS_DIR).join("gr-measured");
        std::fs::create_dir_all(&run_dir).expect("the run dir");
        let (mut decisions, mut verdicts, mut seq) = (String::new(), String::new(), 0_u64);
        let mut next = || {
            seq += 1;
            seq
        };
        for index in 0..40 {
            let key = AttemptKey::new("gr-measured", "plan", format!("t{index}"), 1);
            let arm = if index % 5 == 0 {
                Arm::Default
            } else {
                Arm::Learned
            };
            let know = ContentDecisionPoint::Knowledge;
            let row = content_row(&key, know, "knowledge", "kn-1", arm, true);
            decisions.push_str(&line(DECISION_SCHEMA, next(), row));
            let play = ContentDecisionPoint::Playbooks;
            let row = content_row(&key, play, "playbooks", "pb-1", Arm::Learned, false);
            decisions.push_str(&line(DECISION_SCHEMA, next(), row));
            decisions.push_str(&line(DECISION_SCHEMA, next(), route_row(&key, true)));
            let identity = AttemptIdentity::new(&key);
            let mut verdict = AttemptVerdictRecord::settle(identity, AttemptOutcome::Passed, true);
            verdict.executed.model_reported = Some("model-b".to_string());
            verdicts.push_str(&line(VERDICT_SCHEMA, next(), verdict));
        }
        for index in 0..3 {
            let key = AttemptKey::new("gr-measured", "plan", format!("old{index}"), 1);
            decisions.push_str(&line(DECISION_SCHEMA, next(), route_row(&key, false)));
        }
        std::fs::write(RunFile::Decisions.path_in(&run_dir), decisions).expect("decisions");
        std::fs::write(RunFile::Attempts.path_in(&run_dir), verdicts).expect("verdicts");

        let registry = Registry::embedded().expect("the embedded registry");
        let report = run(dir.path(), &registry, None);

        let know = report.row("L-know").expect("L-know");
        let measured = know.measured.as_ref().expect("L-know is measured");
        let counts = (measured.n_opp, measured.n_learned, measured.n_default);
        assert_eq!(counts, (40, 32, 8));
        assert!(measured.judged && measured.eps.est > 0.0, "{measured:?}");
        assert_eq!(measured.eps.est, 1.0);
        assert_eq!(measured.iota_net, 1.0);
        assert_eq!(know.reason, None);
        assert_eq!(know.state, Some(AuditState::Probation));
        assert_eq!(know.evidence, Some(Evidence::Measured));

        let route = report.row("L-route").expect("L-route");
        assert_eq!(route.reason, Some(ReasonCode::Mask), "{:?}", route.measured);
        assert_eq!(route.evidence, Some(Evidence::Measured));
        let measured = route.measured.as_ref().expect("L-route is measured");
        assert_eq!((measured.n_learned, measured.pre_instrumentation), (40, 3));
        assert_eq!((measured.eps.read, measured.eps.reach), (1.0, 0.0));

        let play = report.row("L-play").expect("L-play");
        let measured = play.measured.as_ref();
        assert_eq!(play.reason, Some(ReasonCode::Unlogged), "{measured:?}");
        assert_eq!(play.evidence, Some(Evidence::Measured));
        assert_eq!(measured.map(|measured| measured.eps.receipt), Some(0.0));

        // A loop no row belongs to keeps its unmeasured verdict.
        assert!(
            report
                .row("L-sec")
                .is_some_and(|row| row.measured.is_none())
        );
    }
    /// A chain's draw on the `harness_policy` layer that landed in `arm`, on
    /// S06's fixed 10% holdout.
    fn harness_draw(arm: Arm) -> Assignment {
        let propensity = if arm == Arm::Learned { 0.9 } else { 0.1 };
        Assignment {
            unit: AssignmentUnit::Chain,
            layer: "harness_policy".to_string(),
            salt_id: "harness_policy@2026-10-03".to_string(),
            u: 0.5,
            h: 0.1,
            g: 0.0,
            arm,
            propensity,
        }
    }

    /// S03 §4.8 (backlog 5135): the census measures the regulators from
    /// their own receipts, on synthetic rows in the shapes their writers
    /// write. L-M1: ten `harness_policy` rows, one on the default arm and
    /// one in shadow mode, so eight of nine learned-arm attempts ran the
    /// controller's θ (ε = 8/9). L-M3: ten route rows of its epochs, seven
    /// with the attempt's prediction row (ε = 0.7). L-M4: ten route rows in
    /// which audit trust left `model-x` out, one of which ran it anyway
    /// (ε = 0.9).
    #[test]
    fn census_measures_meta_loops_from_receipts() {
        use roko_core::config::harness_params::HarnessParams;

        use crate::routing_log::CandidateEntry;
        use crate::telemetry::records::{
            AttemptPredictionRecord, HARNESS_POLICY_DECISION_POINT, PREDICTION_SCHEMA,
            PredictionDecision, PredictionPredictor,
        };

        let dir = tempfile::tempdir().expect("temp dir");
        let run_dir = dir.path().join(".roko").join(RUNS_DIR).join("gr-meta");
        std::fs::create_dir_all(&run_dir).expect("the run dir");
        let (mut decisions, mut predictions, mut seq) = (String::new(), String::new(), 0_u64);
        let mut next = || {
            seq += 1;
            seq
        };

        let theta0 = HarnessParams::baseline(&roko_core::config::schema::RokoConfig::default());
        let theta = HarnessParams {
            retry_delta: 1,
            ..theta0.clone()
        };
        for index in 0..10 {
            let key = AttemptKey::new("gr-meta", "plan", format!("m1-{index}"), 1);
            let arm = if index == 0 {
                Arm::Default
            } else {
                Arm::Learned
            };
            let mode = if index == 1 {
                HomeostasisMode::Shadow
            } else {
                HomeostasisMode::On
            };
            let controls = arm == Arm::Learned && mode == HomeostasisMode::On;
            let ran = if controls { &theta } else { &theta0 };
            let row = HarnessPolicyDecisionRecord {
                identity: AttemptIdentity::new(&key),
                decision_point: HARNESS_POLICY_DECISION_POINT.to_string(),
                assignment: harness_draw(arm),
                arm,
                mode,
                policy_version: 2,
                params_digest: ran.params_digest(),
                chosen: theta.clone(),
                default: theta0.clone(),
                differs: true,
                source: DecisionSource::Control,
                pinned: false,
            };
            decisions.push_str(&line(DECISION_SCHEMA, next(), row));
        }

        for index in 0..10 {
            let key = AttemptKey::new("gr-meta", "plan", format!("m3-{index}"), 1);
            let mut row = route_row(&key, true);
            row.audit.loop_id = Some(SELF_MODEL_LOOP.to_string());
            row.source = Some(DecisionSource::SelfModel);
            row.selected_model = "model-a".to_string();
            decisions.push_str(&line(DECISION_SCHEMA, next(), row));
            if index < 7 {
                let predictor = PredictionPredictor {
                    version: "m3-v1".to_string(),
                    class: "m3-l1".to_string(),
                    mode: "active".to_string(),
                    trained_on_n: 40,
                    features_schema: 1,
                    features_hash: "b3:features".to_string(),
                };
                let decision = PredictionDecision {
                    would_choose: Some("model-a".to_string()),
                    default: Some("model-c".to_string()),
                    action: "dispatch".to_string(),
                };
                let identity = AttemptIdentity::new(&key);
                let prediction =
                    AttemptPredictionRecord::new(identity, predictor, Vec::new(), decision);
                predictions.push_str(&line(PREDICTION_SCHEMA, next(), prediction));
            }
        }

        for index in 0..10 {
            let key = AttemptKey::new("gr-meta", "plan", format!("m4-{index}"), 1);
            let mut row = route_row(&key, true);
            let trusted = CandidateEntry::new("model-a", "provider-a", 0.9, None);
            let reason = Some(AUDIT_TRUST_REASON.to_string());
            let distrusted = CandidateEntry::new("model-x", "provider-x", 0.8, reason);
            row.candidates = vec![trusted, distrusted];
            let selected = if index == 0 { "model-x" } else { "model-a" };
            row.selected_model = selected.to_string();
            decisions.push_str(&line(DECISION_SCHEMA, next(), row));
        }
        std::fs::write(RunFile::Decisions.path_in(&run_dir), decisions).expect("decisions");
        std::fs::write(RunFile::Predictions.path_in(&run_dir), predictions).expect("predictions");

        let registry = Registry::embedded().expect("the embedded registry");
        let report = run(dir.path(), &registry, None);
        let measured = |loop_id: &str| {
            let row = report.row(loop_id).expect("a registered loop");
            row.measured.clone().expect("measured from its receipts")
        };
        let m1 = measured("L-M1");
        assert_eq!((m1.n_opp, m1.n_learned, m1.n_default), (10, 9, 1));
        assert_eq!(m1.eps.est, 8.0 / 9.0, "{m1:?}");
        assert_eq!(m1.iota_net, 1.0, "the controller's θ differs from θ₀");
        let m3 = measured("L-M3");
        assert_eq!((m3.n_opp, m3.n_learned), (10, 10));
        assert_eq!((m3.eps.est, m3.eps.receipt), (0.7, 0.7), "{m3:?}");
        let m4 = measured("L-M4");
        assert_eq!(m4.n_opp, 10);
        assert_eq!(m4.eps.est, 0.9, "{m4:?}");
        assert_eq!(m4.eps.reach, 0.9, "the excluded model ran once");
    }
}
