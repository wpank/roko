//! Report-only loop census over the logs that exist today (S03 §0, §7 A1; backlog 5106).
//!
//! [`run`] reads a workspace's learning logs and the loop registry and gives
//! every registered loop one row: `(state, reason, qualifiers, evidence)` in
//! S03 §4.6's closed enum. Log rules read the files below; the registry's
//! declared findings add code-review facts, each printed with its pointer and
//! `verified_at`. The census spends nothing, makes no transitions and writes
//! nothing: [`render_json`] (`roko.loop_census/1`) and [`render_text`] print
//! it. A retired loop keeps its last reason, for its history, and no state.
//! The measured mode, over per-run decision and exposure rows, is backlog
//! 5123.

use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use super::spec::{AuditState, Lifecycle, LoopSpec, Qualifier, ReasonCode, Registry};
use crate::prompt_experiment::ExperimentStore;
use crate::runtime_feedback::LearningPaths;

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
/// The dream cycle's routing advice, under the learn directory.
const DREAM_ROUTING_ADVICE: &str = "dream-routing-advice.json";

/// Where a row's reason comes from (S03 §9.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Evidence {
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

/// The census of the logs at `paths` against `registry`.
#[must_use]
pub fn run_in(
    paths: &LearningPaths,
    registry: &Registry,
    harness_sha: Option<&str>,
) -> CensusReport {
    let logs = Logs::read(paths);
    let rows = registry
        .loops()
        .iter()
        .map(|spec| census_row(spec, &logs, harness_sha))
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

/// One loop's row: its log rules and declared findings, resolved by S03
/// §4.6's precedence (log evidence first on a tie).
fn census_row(spec: &LoopSpec, logs: &Logs, harness_sha: Option<&str>) -> CensusRow {
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
    let winner = candidates.first().copied();
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
        evidence: winner.map(|(_, evidence, _)| evidence),
        facts,
        findings,
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
    dream_recommendations: Option<usize>,
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
        if let Some(advice) = read_json(&paths.root.join(DREAM_ROUTING_ADVICE)) {
            logs.dream_recommendations = advice["recommendations"].as_array().map(Vec::len);
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
            "L-dream-bias" => {
                let written = self.dream_recommendations?;
                facts.push(format!(
                    "the dream cycle wrote {written} routing recommendations"
                ));
                None
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
            (
                "L-bid",
                retired("4217"),
                None,
                R::Degenerate,
                vec![],
                Log,
            ),
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
                "L-dream-bias",
                Lifecycle::Active,
                Some(Flagged),
                R::WriteOnly,
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
}
