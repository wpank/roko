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
//! pre-instrumentation. The audit tick (backlog 5126) folds the same rows,
//! its sequences at the auditor's α/K, into a [`CensusState`] it keeps in
//! the learn dir, so that a plan run's close reads only the rows no tick has
//! folded (gap-addf2a); [`measure_at`] folds every run at once.
//!
//! The regulators are measured from their own receipts (S03 §4.8; backlog
//! 5135): L-M1 from M1's `harness_policy` rows (`params_digest`: the θ the
//! attempt ran against the controller's θ, on S06's fixed holdout), L-M3
//! from the route rows its epochs write (`prediction_consumed`: the
//! attempt's prediction row), and L-M4 from later route rows in which
//! audit feedback left a candidate out (`audit_penalty_applied`: the
//! candidate's `audit_trust` reason, S05 DP4). L-M4's log rule checks those
//! rows against DP4's own count on each verdict, which tells an exclusion
//! lost before logging from none (gap-595e28).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use roko_core::audit_types::VerifyDepth;
use roko_core::config::homeostasis::HomeostasisMode;
use serde::{Deserialize, Serialize};
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
use crate::telemetry::RunProvenanceManifest;
use crate::telemetry::records::{
    AuditFields, ContentDecisionPoint, ContentDecisionRecord, DecisionSource, ExecutedModel,
    ExposureItemKind, HarnessPolicyDecisionRecord, HarnessStamp, RunFile, Stamped,
};
use crate::telemetry::report::{RunRecords, SrmFold, srm_fold, undated};

/// Where the audit tick keeps its [`CensusState`], in the learn dir.
pub const CENSUS_STATE_FILE: &str = "loop-census-state.json";

/// Schema of [`CENSUS_STATE_FILE`].
pub const CENSUS_STATE_SCHEMA: &str = "roko.loop_census_state/1";

/// How long an attempt without a verdict holds back the rows after it, in a
/// run that has not closed: one open longer than this died with its process.
const LIVE_ATTEMPT_MS: i64 = 24 * 60 * 60 * 1000;

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
/// Graph dispatch's retrieval outcomes, under the learn directory. Nothing
/// writes them since 4129, when exposure rows took over; L-know reads them
/// only for a workspace whose runs record no exposures.
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
    let logs = Logs::read(paths, runs);
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

/// What the audit tick keeps of the census between plan-run closes
/// (gap-addf2a): each loop's tally and each layer's SRM check over the rows
/// folded so far, the α its sequences run at, and how far it has read each
/// run. A tick reads the run that closed from where the last tick left it,
/// and the runs no tick has read; a run that had not ended at its last read
/// is read again once its files change, or once the attempt that held it
/// back stops counting as running. A run that ended is read again only at
/// its next close. So a close costs what its new rows cost, not what the
/// history under `.roko/runs/` does. The counts are those one read of every
/// run gives; the confidence sequences take the rows in the order the ticks
/// folded them. A run directory removed later keeps its rows in the tallies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CensusState {
    schema_version: String,
    alpha: f64,
    runs: BTreeMap<String, RunMark>,
    tallies: BTreeMap<String, Tally>,
    srm: BTreeMap<String, SrmFold>,
}

/// How far a [`CensusState`] has read one run.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RunMark {
    /// The first `seq` not folded yet.
    next: u64,
    /// Whether the run had ended at the last read: it was the run that
    /// closed, or its manifest says it closed.
    ended: bool,
    /// Its run files' total size at the last read.
    bytes: u64,
    /// When the attempt that held back the rows from `next` stops counting
    /// as running (unix ms).
    held_until: Option<i64>,
}

impl RunMark {
    /// Whether a tick at `now_ms` reads the run in `dir` again, though it
    /// is not the run that closed: it had not ended, and its files changed
    /// or the attempt that held it back stopped counting as running.
    fn due(&self, dir: &Path, now_ms: i64) -> bool {
        let released = self.held_until.is_some_and(|until| until <= now_ms);
        !self.ended && (released || run_bytes(dir) != self.bytes)
    }
}

impl CensusState {
    /// An empty census whose sequences run at level `alpha`.
    #[must_use]
    pub fn new(alpha: f64) -> Self {
        Self {
            schema_version: CENSUS_STATE_SCHEMA.to_string(),
            alpha,
            runs: BTreeMap::new(),
            tallies: BTreeMap::new(),
            srm: BTreeMap::new(),
        }
    }

    /// The census kept at `path`; an empty one at `alpha` when the file is
    /// missing, unreadable or damaged, or was folded under another schema
    /// or α, which its sequences' states depend on. The next tick then reads
    /// every run once.
    #[must_use]
    pub fn load(path: &Path, alpha: f64) -> Self {
        let kept = std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok());
        match kept {
            Some(state) if state.fits(alpha) => state,
            _ => Self::new(alpha),
        }
    }

    /// Whether a census read back can go on at `alpha`: its schema and α
    /// are the current ones, and its sequences' states are whole.
    fn fits(&self, alpha: f64) -> bool {
        self.schema_version == CENSUS_STATE_SCHEMA
            && self.alpha.to_bits() == alpha.to_bits()
            && self.tallies.values().all(Tally::is_consistent)
    }

    /// Write the census to `path`, atomically.
    ///
    /// # Errors
    ///
    /// Returns an error when it cannot be serialized or written.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec(self).map_err(std::io::Error::other)?;
        roko_fs::atomic_write_bytes(path, &bytes)
    }

    /// Fold the runs under `runs_dir` that a tick at `now` reads: `closing`,
    /// the run that just closed, and each run that ended, to its end; each
    /// other run that is due ([`CensusState`]), up to its first attempt
    /// still running, one opened in the last day with no verdict yet. A run
    /// that cannot be read is left for a later tick.
    pub fn fold_pending(&mut self, runs_dir: &Path, closing: &str, now: DateTime<Utc>) {
        let Ok(entries) = std::fs::read_dir(runs_dir) else {
            return;
        };
        let now_ms = now.timestamp_millis();
        let mut pending: Vec<(PathBuf, bool)> = entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let (dir, closes) = (entry.path(), name == closing);
                let mark = self.runs.get(&name);
                let due = closes || mark.is_none_or(|mark| mark.due(&dir, now_ms));
                (due && dir.is_dir()).then_some((dir, closes))
            })
            .collect();
        pending.sort();
        for (dir, closes) in pending {
            // Sized before the read: a row written in between is read at the
            // next tick.
            let bytes = run_bytes(&dir);
            let ended = closes || run_ended(&dir);
            let Ok(run) = RunRecords::load(&dir) else {
                continue;
            };
            let held = if ended {
                None
            } else {
                first_running(&run, now_ms)
            };
            self.fold(&run, held.map_or(u64::MAX, |(seq, _)| seq));
            let mark = self.runs.entry(run.run_id).or_default();
            mark.ended = ended;
            mark.bytes = bytes;
            mark.held_until = held.map(|(_, until)| until);
        }
    }

    /// Fold `run`'s decision rows from where the census left it up to
    /// `seq` `until` (exclusive): each loop's opportunities, and each
    /// layer's units first seen there.
    pub fn fold(&mut self, run: &RunRecords, until: u64) {
        let end = run
            .seqs
            .iter()
            .map(|(_, seq)| seq.saturating_add(1))
            .max()
            .unwrap_or(0);
        let mark = self.runs.entry(run.run_id.clone()).or_default();
        let from = mark.next;
        mark.next = until.min(end).max(from);
        fold_run(&mut self.tallies, run, from, until, self.alpha);
        srm_fold(&mut self.srm, run, from, until);
    }

    /// Each loop's measurement over the rows folded, as [`measure_at`]
    /// gives it, judged against `params`' N_ε and ε_min.
    #[must_use]
    pub fn measurements(&self, params: &AuditParams) -> BTreeMap<String, LoopMeasurement> {
        self.tallies
            .iter()
            .map(|(loop_id, tally)| (loop_id.clone(), tally.measurement(params)))
            .collect()
    }

    /// Whether some layer's units do not split as their logged propensities
    /// say (S02 SC3; S03 §4.5).
    #[must_use]
    pub fn srm_alarm(&self) -> bool {
        self.srm.values().any(SrmFold::mismatch)
    }

    /// `layer`'s SRM e-value; 1 for a layer with no units.
    #[must_use]
    pub fn srm_evalue(&self, layer: &str) -> f64 {
        self.srm.get(layer).map_or(1.0, SrmFold::e_value)
    }

    /// The first `seq` of `run_id` not folded yet; `None` for a run never
    /// read.
    #[must_use]
    pub fn next_seq(&self, run_id: &str) -> Option<u64> {
        self.runs.get(run_id).map(|mark| mark.next)
    }
}

/// `run`'s first attempt still running at `now_ms` (unix ms): no verdict
/// yet, and opened less than [`LIVE_ATTEMPT_MS`] before. Its rows, all after
/// its open line, may still wait for its verdict and prediction. Returns the
/// open line's `seq` and when the attempt stops counting as running.
fn first_running(run: &RunRecords, now_ms: i64) -> Option<(u64, i64)> {
    let settled: HashSet<&str> = run
        .verdicts
        .iter()
        .map(|line| line.record.identity.attempt_key.as_str())
        .collect();
    run.opens
        .iter()
        .filter_map(|line| {
            let open = &line.record;
            let until = open.attempt_started_at?.saturating_add(LIVE_ATTEMPT_MS);
            let key = open.identity.attempt_key.as_str();
            (until > now_ms && !settled.contains(key)).then_some((line.seq, until))
        })
        .min()
}

/// Whether the run in `dir` has ended: its manifest says it closed. A resume
/// reopens it; a run without a readable manifest has not ended.
fn run_ended(dir: &Path) -> bool {
    RunProvenanceManifest::load(dir)
        .is_ok_and(|loaded| loaded.is_some_and(|manifest| manifest.closed.is_some()))
}

/// The total size of the run files in `dir`, which every row adds to.
fn run_bytes(dir: &Path) -> u64 {
    RunFile::ALL
        .into_iter()
        .filter_map(|file| std::fs::metadata(file.path_in(dir)).ok())
        .map(|metadata| metadata.len())
        .sum()
}

/// Each loop's tally over the decision rows of `runs`, its sequences at
/// level `alpha`.
fn tallies(runs: &[RunRecords], alpha: f64) -> BTreeMap<String, Tally> {
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for run in runs {
        fold_run(&mut tallies, run, 0, u64::MAX, alpha);
    }
    tallies
}

/// Fold into `tallies`, at level `alpha`, the decision rows of `run` with a
/// `seq` in `from..until`. A row is scored against the run's verdicts and
/// predictions wherever they are, so a fold that stops at an attempt still
/// running leaves that attempt's rows for a later one (gap-addf2a).
fn fold_run(
    tallies: &mut BTreeMap<String, Tally>,
    run: &RunRecords,
    from: u64,
    until: u64,
    alpha: f64,
) {
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
    for line in window(&run.decisions, from, until) {
        let row = &line.record;
        let ran = row
            .attempt_key
            .as_deref()
            .and_then(|key| executed.get(key).copied());
        if let Some(opportunity) = audit_feedback_opportunity(row, ran) {
            let feedback = tally_of(tallies, AUDIT_FEEDBACK_LOOP, alpha);
            if feedback.count(&row.audit) {
                feedback.push(&opportunity);
            }
        }
        let loop_id = row.audit.loop_id.as_deref().unwrap_or(ROUTE_LOOP);
        let loop_tally = tally_of(tallies, loop_id, alpha);
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
    for line in window(&run.content_decisions, from, until) {
        let row = &line.record;
        let loop_id = row.audit.loop_id.as_deref();
        let Some(loop_id) = loop_id.or_else(|| content_loop(row.decision_point)) else {
            continue;
        };
        let loop_tally = tally_of(tallies, loop_id, alpha);
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
    for line in window(&run.harness_decisions, from, until) {
        let row = &line.record;
        let loop_tally = tally_of(tallies, HARNESS_LOOP, alpha);
        loop_tally.rows += 1;
        let stamp = stamps.get(row.identity.attempt_key.as_str()).copied();
        if let Some(opportunity) = harness_opportunity(row, stamp) {
            loop_tally.push(&opportunity);
        }
    }
}

/// The lines of `lines` with a `seq` in `from..until`.
fn window<T>(lines: &[Stamped<T>], from: u64, until: u64) -> impl Iterator<Item = &Stamped<T>> {
    lines
        .iter()
        .filter(move |line| (from..until).contains(&line.seq))
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Whether both sequences' states are whole.
    fn is_consistent(&self) -> bool {
        self.exposure.is_consistent() && self.influence.is_consistent()
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
    /// Exposure rows of every kind in the runs (S01 P0-9).
    exposures: usize,
    /// Knowledge entries the runs' prompts retrieved, by their exposure rows.
    knowledge_exposures: usize,
    /// Rows of the retired retrieval log, read only without exposures.
    legacy_retrievals: usize,
    episodes: usize,
    episodes_with_knowledge: usize,
    episodes_with_playbooks: usize,
    efficiency_rows: usize,
    efficiency_with_knowledge: usize,
    efficiency_with_playbooks: usize,
    playbooks: usize,
    holdout_costs: Option<(f64, f64)>,
    prompt_experiments: Option<usize>,
    /// Verdicts that carry DP4's own count of the exclusions their routing
    /// made (gap-595e28).
    trust_routed: usize,
    /// Those exclusions, summed.
    trust_counted: u64,
    /// Route-row candidates that name audit trust as why they were left
    /// out (`ineligible_reason audit_trust`).
    trust_logged: usize,
    /// Verdicts whose task type the strictness ladder held above V0 (DP3).
    ladder_deepened: usize,
}

impl Logs {
    fn read(paths: &LearningPaths, runs: &[RunRecords]) -> Self {
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
        for exposure in runs.iter().flat_map(|run| &run.exposures) {
            let row = &exposure.record;
            logs.exposures += 1;
            let knowledge = row.item_kind == ExposureItemKind::Knowledge && row.retrieved;
            logs.knowledge_exposures += usize::from(knowledge);
        }
        if logs.exposures == 0 {
            logs.legacy_retrievals = read_jsonl(&paths.root.join(RETRIEVAL_OUTCOMES)).len();
        }
        for verdict in runs.iter().flat_map(|run| &run.verdicts) {
            let verdict = &verdict.record;
            if let Some(excluded) = verdict.trust_exclusions {
                logs.trust_routed += 1;
                logs.trust_counted += excluded;
            }
            let depth = verdict.verify_depth.as_ref();
            let ladder = depth.and_then(|depth| depth.ladder);
            let deepened = ladder.is_some_and(|level| level > VerifyDepth::V0);
            logs.ladder_deepened += usize::from(deepened);
        }
        let audit_trust = Some(AUDIT_TRUST_REASON);
        for decision in runs.iter().flat_map(|run| &run.decisions) {
            let candidates = decision.record.candidates.iter();
            logs.trust_logged += candidates
                .filter(|candidate| candidate.ineligible_reason.as_deref() == audit_trust)
                .count();
        }
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
                // Exposure rows say what the prompts retrieved (4129); a
                // workspace whose runs record none keeps its retired log.
                let (retrievals, source) = if self.exposures > 0 {
                    (self.knowledge_exposures, "knowledge exposures")
                } else {
                    (self.legacy_retrievals, "retired retrieval-log rows")
                };
                let unlogged = retrievals > 0
                    && self.episodes_with_knowledge == 0
                    && self.efficiency_with_knowledge == 0;
                facts.push(format!(
                    "{retrievals} {source}; {}/{} episodes and {}/{} efficiency rows carry \
                     knowledge ids",
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
            "L-M4" => {
                // DP4 counts its exclusions on each verdict, apart from the
                // route rows (gap-595e28): some counted and none on a route
                // row were lost before logging; none counted and none logged
                // is no opportunity.
                if self.trust_routed == 0 {
                    return None;
                }
                facts.push(format!(
                    "DP4 counted {} exclusions on {} verdicts, and route rows name {}; verdicts \
                     with a strictness-ladder level above V0 (DP3): {}",
                    self.trust_counted, self.trust_routed, self.trust_logged, self.ladder_deepened
                ));
                match (self.trust_counted, self.trust_logged) {
                    (0, 0) => reason(ReasonCode::NoOpportunity),
                    (_, 0) => reason(ReasonCode::Unlogged),
                    _ => None,
                }
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
        ATTEMPT_OPEN_SCHEMA, AttemptIdentity, AttemptKey, AttemptOpenRecord, AttemptOutcome,
        AttemptVerdictRecord, ContentCandidate, ContentProposals, DECISION_SCHEMA,
        DecisionAssignment, DecisionReceipt, EXPOSURE_SCHEMA, ExposureRecord, RunClosed, RunFile,
        Stamped, VERDICT_SCHEMA,
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
        // gap-dadd56: the guards mask before the argmax and label a fallback
        // as one, so no mask is declared for L-route any more.
        let route = report.row("L-route").expect("L-route");
        assert_eq!(route.reason, None, "{:?}", route.facts);
        assert_eq!(route.state, Some(AuditState::Probation));
    }

    /// A commit no finding was verified at.
    const OTHER_SHA: &str = "0000000000000000000000000000000000000000";

    /// A declared finding checked at another commit than the harness's is
    /// stale, and a row whose reason rests on it says so.
    #[test]
    fn census_qualifies_stale_declared_findings() {
        let registry = Registry::embedded().expect("the embedded registry");
        let report = run_in(&fixture(), &registry, Some(OTHER_SHA));
        let thresholds = report.row("L-gate-thr").expect("L-gate-thr");
        assert!(thresholds.findings.iter().all(|finding| finding.stale));
        assert!(thresholds.qualifiers.contains(&Qualifier::DeclaredStale));
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
            section_draws: Vec::new(),
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

    /// gap-6c8965: L-know's log rule counts the knowledge items the runs'
    /// exposure rows say a prompt retrieved, since 4129 retired the
    /// retrieval log. A workspace whose runs record no exposures keeps its
    /// retired log's rows. Once a run records exposures the log is not read:
    /// section exposures alone leave L-know unflagged, and two knowledge
    /// exposures no episode names flag it unlogged, until an episode names
    /// one.
    #[test]
    fn census_l_know_counts_exposures_not_the_retired_retrieval_log() {
        let dir = tempfile::tempdir().expect("temp dir");
        let roko = dir.path().join(".roko");
        let learn = roko.join("learn");
        std::fs::create_dir_all(&learn).expect("the learn dir");
        let registry = Registry::embedded().expect("the embedded registry");
        let know = || {
            let report = run(dir.path(), &registry, None);
            let row = report.row("L-know").expect("L-know");
            (row.reason, row.facts.first().cloned().unwrap_or_default())
        };
        let retired = "{\"query\":\"q\"}\n".repeat(5);
        std::fs::write(learn.join(RETRIEVAL_OUTCOMES), retired).expect("the retired log");
        let (reason, fact) = know();
        assert_eq!(reason, Some(ReasonCode::Unlogged), "{fact}");
        assert!(fact.starts_with("5 retired retrieval-log rows;"), "{fact}");

        let run_dir = roko.join(RUNS_DIR).join("gr-exposed");
        std::fs::create_dir_all(&run_dir).expect("the run dir");
        let identity = AttemptIdentity::new(&AttemptKey::new("gr-exposed", "plan", "t1", 1));
        let exposures = |items: &[(ExposureItemKind, &str)]| {
            let rows: String = items
                .iter()
                .enumerate()
                .map(|(index, &(kind, id))| {
                    let row = ExposureRecord::new(identity.clone(), kind, id);
                    line(EXPOSURE_SCHEMA, index as u64 + 1, row)
                })
                .collect();
            std::fs::write(RunFile::Exposures.path_in(&run_dir), rows).expect("exposures");
        };
        exposures(&[(ExposureItemKind::Section, "conventions")]);
        let (reason, fact) = know();
        assert_eq!(reason, None, "{fact}");
        assert!(fact.starts_with("0 knowledge exposures;"), "{fact}");

        let knowledge = ExposureItemKind::Knowledge;
        exposures(&[
            (knowledge, "kn-1"),
            (knowledge, "kn-2"),
            (ExposureItemKind::Section, "conventions"),
        ]);
        let (reason, fact) = know();
        assert_eq!(reason, Some(ReasonCode::Unlogged), "{fact}");
        assert!(fact.starts_with("2 knowledge exposures;"), "{fact}");

        let episode = "{\"knowledge_ids_injected\":[\"kn-1\"]}\n";
        std::fs::write(roko.join("episodes.jsonl"), episode).expect("an episode");
        let (reason, fact) = know();
        assert_eq!(reason, None, "{fact}");
    }

    /// Append `lines` to the file at `path`.
    fn append(path: &Path, lines: &str) {
        use std::io::Write;

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open a run file");
        file.write_all(lines.as_bytes())
            .expect("append to a run file");
    }

    /// Append to run `run_id` under `runs_dir` the chains `chains`, one
    /// attempt each, numbered from `seq`: its open line, started at
    /// `started` (unix ms), its L-know row, every fifth on the default arm,
    /// and its verdict, except for chain `running`. Returns the next `seq`.
    fn append_chains(
        runs_dir: &Path,
        run_id: &str,
        chains: std::ops::Range<usize>,
        running: Option<usize>,
        started: i64,
        mut seq: u64,
    ) -> u64 {
        let run_dir = runs_dir.join(run_id);
        std::fs::create_dir_all(&run_dir).expect("the run dir");
        let (mut attempts, mut decisions) = (String::new(), String::new());
        for index in chains {
            let key = AttemptKey::new(run_id, "plan", format!("t{index}"), 1);
            let open = AttemptOpenRecord::new(AttemptIdentity::new(&key), started);
            attempts.push_str(&line(ATTEMPT_OPEN_SCHEMA, seq, open));
            let arm = if index % 5 == 0 {
                Arm::Default
            } else {
                Arm::Learned
            };
            let know = ContentDecisionPoint::Knowledge;
            let row = content_row(&key, know, "knowledge", "kn-1", arm, true);
            decisions.push_str(&line(DECISION_SCHEMA, seq + 1, row));
            seq += 2;
            if running != Some(index) {
                let identity = AttemptIdentity::new(&key);
                let verdict = AttemptVerdictRecord::settle(identity, AttemptOutcome::Passed, true);
                attempts.push_str(&line(VERDICT_SCHEMA, seq, verdict));
                seq += 1;
            }
        }
        append(&RunFile::Attempts.path_in(&run_dir), &attempts);
        append(&RunFile::Decisions.path_in(&run_dir), &decisions);
        seq
    }

    /// gap-addf2a: the census the audit tick keeps reads only the rows no
    /// tick has folded. A run that ended is not opened again until it closes
    /// once more. A run still going is read up to its attempt still running,
    /// and again once its files change or at its own close; an attempt open
    /// for days died with its process, and one in a run whose manifest says
    /// it closed was abandoned, so neither holds rows back. The census comes
    /// back from its file, and its counts end where one read of every run
    /// ends.
    #[test]
    fn audit_census_reads_only_rows_no_tick_has_folded() {
        use chrono::TimeZone;

        let dir = tempfile::tempdir().expect("temp dir");
        let runs_dir = dir.path().join(RUNS_DIR);
        let kept = dir.path().join(CENSUS_STATE_FILE);
        let params = AuditParams::default();
        let alpha = params.loop_alpha();
        let now = Utc
            .with_ymd_and_hms(2026, 10, 3, 10, 0, 0)
            .single()
            .expect("a valid time");
        let started = now.timestamp_millis() - 60_000;
        let know_opps = |census: &CensusState| {
            let measured = census.measurements(&params);
            measured.get("L-know").map(|know| know.measured.n_opp)
        };

        // gr-a closes with 20 chains, and the tick keeps its census.
        let next_a = append_chains(&runs_dir, "gr-a", 0..20, None, started, 1);
        let mut census = CensusState::load(&kept, alpha);
        census.fold_pending(&runs_dir, "gr-a", now);
        census.save(&kept).expect("keep the census");
        assert_eq!(know_opps(&census), Some(20));

        // gr-c's tick reads the census back. gr-a's later rows wait for its
        // next close; gr-b is read up to its chain 4, still running, at seq
        // 13; gr-d's chain 1, open for two days, and gr-e's chain 0, in a run
        // whose manifest closed, hold nothing back.
        let mut census = CensusState::load(&kept, alpha);
        assert_eq!(census.next_seq("gr-a"), Some(next_a));
        assert_eq!(know_opps(&census), Some(20));
        append_chains(&runs_dir, "gr-a", 20..25, None, started, next_a);
        append_chains(&runs_dir, "gr-b", 0..10, Some(4), started, 1);
        append_chains(&runs_dir, "gr-c", 0..5, None, started, 1);
        let two_days = 2 * LIVE_ATTEMPT_MS;
        let end_d = append_chains(&runs_dir, "gr-d", 0..3, Some(1), started - two_days, 1);
        append_chains(&runs_dir, "gr-e", 0..2, Some(0), started, 1);
        let mut manifest = RunProvenanceManifest::new("gr-e", "plan_run");
        manifest.closed = Some(RunClosed::default());
        manifest
            .store(&runs_dir.join("gr-e"))
            .expect("close gr-e's manifest");
        census.fold_pending(&runs_dir, "gr-c", now);
        assert_eq!(know_opps(&census), Some(20 + 4 + 5 + 3 + 2));
        assert_eq!(census.next_seq("gr-b"), Some(13));
        assert_eq!(census.next_seq("gr-d"), Some(end_d));

        // gr-b's close reads the rest of it, and gr-d, which has not ended,
        // is read again once it grows; gr-a's next close reads its new rows.
        // No row counts twice.
        append_chains(&runs_dir, "gr-d", 3..5, None, started, end_d);
        census.fold_pending(&runs_dir, "gr-b", now);
        assert_eq!(know_opps(&census), Some(34 + 6 + 2));
        census.fold_pending(&runs_dir, "gr-a", now);
        assert_eq!(know_opps(&census), Some(47));
        census.fold_pending(&runs_dir, "gr-a", now);
        assert_eq!(know_opps(&census), Some(47));

        let counts = |measured: &BTreeMap<String, LoopMeasurement>| -> Vec<_> {
            measured
                .iter()
                .map(|(loop_id, measurement)| {
                    let counted = &measurement.measured;
                    let opps = (counted.n_opp, counted.n_learned, counted.n_default);
                    (loop_id.clone(), opps, measurement.rows, counted.eps.est)
                })
                .collect()
        };
        let whole = measure_at(&read_runs(&runs_dir), &params);
        assert_eq!(counts(&census.measurements(&params)), counts(&whole));
    }

    /// gap-595e28: L-M4's census tells a lost exclusion from none. DP4
    /// counts the exclusions each attempt's routing made on its verdict,
    /// apart from the route row. Verdicts that count none, with no route row
    /// naming audit trust, are no opportunity; verdicts that count some with
    /// no such route row lost them before logging (`dormant:unlogged`); once
    /// route rows name them, the log rule has nothing to add. The facts count
    /// the verdicts whose verify depth the strictness ladder raised (DP3).
    #[test]
    fn l_m4_census_distinguishes_unlogged_from_no_opportunity() {
        use crate::routing_log::CandidateEntry;
        use crate::telemetry::records::VerifyDepthRecord;

        // L-M4's census row over one run of four routed attempts: each
        // verdict counts `excluded` DP4 exclusions, the route rows name them
        // when `logged`, and the strictness ladder held the first attempt's
        // task type at V2.
        let l_m4 = |excluded: u64, logged: bool| {
            let dir = tempfile::tempdir().expect("temp dir");
            let run_dir = dir.path().join(".roko").join(RUNS_DIR).join("gr-dp4");
            std::fs::create_dir_all(&run_dir).expect("the run dir");
            let (mut decisions, mut verdicts) = (String::new(), String::new());
            for index in 0..4_u64 {
                let key = AttemptKey::new("gr-dp4", "plan", format!("t{index}"), 1);
                let mut row = route_row(&key, true);
                if logged {
                    let reason = Some(AUDIT_TRUST_REASON.to_string());
                    row.candidates = vec![CandidateEntry::new("model-x", "", 0.0, reason)];
                }
                decisions.push_str(&line(DECISION_SCHEMA, 2 * index + 1, row));
                let (identity, passed) = (AttemptIdentity::new(&key), AttemptOutcome::Passed);
                let mut verdict = AttemptVerdictRecord::settle(identity, passed, true);
                verdict.trust_exclusions = Some(excluded);
                verdict.verify_depth = (index == 0).then(|| VerifyDepthRecord {
                    task_type: "code".to_string(),
                    depth: VerifyDepth::V2,
                    ladder: Some(VerifyDepth::V2),
                    floor: VerifyDepth::V0,
                });
                verdicts.push_str(&line(VERDICT_SCHEMA, 2 * index + 2, verdict));
            }
            std::fs::write(RunFile::Decisions.path_in(&run_dir), decisions).expect("decisions");
            std::fs::write(RunFile::Attempts.path_in(&run_dir), verdicts).expect("verdicts");
            let registry = Registry::embedded().expect("the embedded registry");
            let report = run(dir.path(), &registry, None);
            report.row("L-M4").cloned().expect("L-M4's row")
        };

        let none = l_m4(0, false);
        let no_opportunity = Some(ReasonCode::NoOpportunity);
        assert_eq!(none.reason, no_opportunity, "{:?}", none.facts);
        assert_eq!(none.evidence, Some(Evidence::Log));

        let lost = l_m4(2, false);
        assert_eq!(lost.reason, Some(ReasonCode::Unlogged), "{:?}", lost.facts);
        assert_eq!(lost.evidence, Some(Evidence::Log));
        let counted = "DP4 counted 8 exclusions on 4 verdicts, and route rows name 0; verdicts \
                       with a strictness-ladder level above V0 (DP3): 1";
        assert!(
            lost.facts.iter().any(|fact| fact == counted),
            "{:?}",
            lost.facts
        );

        let logged = l_m4(2, true);
        assert_eq!(logged.reason, None, "{:?}", logged.facts);
    }
}
