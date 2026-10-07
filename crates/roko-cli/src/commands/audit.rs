//! `roko audit`: what M4's audits found, read-only (S05 §7.3, §7.6; 7136).
//!
//! - `status [--strata]`: the latest closed window's estimate per stratum,
//!   with its interval and n_eff; the strictness ladder's levels; routing
//!   trust and the pairs DP4 leaves out; gate-gaming alerts; audit spend and
//!   budget stops; the selected units still to audit; and the judge's
//!   calibration against audit labels (7126). `--strata` also lists every
//!   verdict stratum the runs counted, `forced_accept` included.
//! - `replay --runs N`: N lotteries re-drawn over the latest window's
//!   audited units, a sensitivity check of its interval.
//! - `reveal <run_id>`: every draw of the run recomputed from its revealed
//!   key, the key checked against its commitment, and the ledger's hash
//!   chain walked; the exit status is non-zero on any mismatch.
//! - `incidents`: the incidents, their status and fix proposals, and the
//!   isolation proposals that wait for an operator.
//!
//! Nothing here writes an audit record. No key is printed, revealed or not,
//! and no hidden test.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow};
use clap::Subcommand;
use roko_core::audit_home::AuditVault;
use roko_core::audit_types::VerifyDepth;
use roko_core::config::audit::AuditConfig;
use roko_gate::audit::estimate::estimate;
use roko_gate::audit::feedback::{Ladder, TrustBook, ladder_path, trust_path};
use roko_gate::audit::incident::{Incident, IncidentStore};
use roko_gate::audit::ledger::{AuditEvent, LedgerRecord, records, verify_chain};
use roko_gate::audit::policy::{RunKey, Selection, draw, is_selected, verify_reveal};
use roko_gate::audit::window::{GroupEstimate, WINDOW_PREFIX, WindowUnit, units, window_span};
use roko_gate::judge_calibration::{compute_calibration, load_calibration_log};
use roko_learn::cascade_router::{TRUST_EXCLUDE_PROBABILITY, trust_exceedance};
use serde::Serialize;
use serde_json::Value;

use crate::{Cli, EXIT_FAILURE, EXIT_SUCCESS, resolve_workdir};

/// The share of a window's audited units each replayed lottery draws.
const REPLAY_RATE: f64 = 0.5;

/// The secret replayed lotteries derive their keys from; they draw nothing
/// that counts.
const REPLAY_SECRET: &[u8; 32] = b"roko-audit-replay-lottery-key/v1";

/// M4's audits, read-only.
#[derive(Debug, Subcommand)]
pub(crate) enum AuditCmd {
    /// The latest window's estimates, the ladder, routing trust, alerts,
    /// spend, queued units and the judge's calibration.
    Status {
        /// Also list every verdict stratum, `forced_accept` included.
        #[arg(long)]
        strata: bool,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Re-draw lotteries over the latest window's audited units, a
    /// sensitivity check of its interval.
    Replay {
        /// Lotteries to draw.
        #[arg(long, default_value_t = 200)]
        runs: u32,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Recompute every draw of a run from its revealed key; exits non-zero
    /// on any mismatch.
    Reveal {
        /// The run.
        run_id: String,
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// The incidents, with their status and fix proposals.
    Incidents {
        /// Workspace root (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

pub(crate) fn cmd_audit(cli: &Cli, cmd: AuditCmd) -> Result<i32> {
    let workdir = match &cmd {
        AuditCmd::Status { workdir, .. }
        | AuditCmd::Replay { workdir, .. }
        | AuditCmd::Reveal { workdir, .. }
        | AuditCmd::Incidents { workdir } => workdir.clone(),
    };
    let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
    let config = roko_core::config::loader::load_config_unified(&workdir)
        .map_err(|error| anyhow!("{error}"))
        .with_context(|| format!("load the config of {}", workdir.display()))?;
    let vault = config
        .audit
        .vault(&workdir)
        .map_err(|error| anyhow!("{error}"))?;
    match cmd {
        AuditCmd::Status { strata, .. } => {
            let report = status(&vault, &workdir, &config.audit, strata)?;
            print(cli, &report, print_status)
        }
        AuditCmd::Replay { runs, .. } => {
            let report = replay(&vault, runs)?;
            print(cli, &report, print_replay)
        }
        AuditCmd::Reveal { run_id, .. } => {
            let report = reveal(&vault.ledger_dir(), &run_id)?;
            print(cli, &report, print_reveal)?;
            Ok(if report.holds() {
                EXIT_SUCCESS
            } else {
                EXIT_FAILURE
            })
        }
        AuditCmd::Incidents { .. } => {
            let report = incidents(&vault)?;
            print(cli, &report, print_incidents)
        }
    }
}

/// Print `report` as JSON with `--json`, else as `text` renders it.
fn print<T: Serialize>(cli: &Cli, report: &T, text: fn(&T)) -> Result<i32> {
    if cli.json {
        println!("{}", serde_json::to_string_pretty(report)?);
    } else {
        text(report);
    }
    Ok(EXIT_SUCCESS)
}

/// One stratum's estimate in a window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct StratumLine {
    /// `task type / model / arm / verdict`.
    pub stratum: String,
    /// Its green units.
    pub n_green: u64,
    /// Those the lottery selected.
    pub n_selected: u64,
    /// θ̂, the false-green rate; `None` without a known label.
    pub theta: Option<f64>,
    /// Its 95% Wilson interval at n_eff.
    pub ci: (f64, f64),
    /// Kish's n_eff.
    pub n_eff: f64,
}

/// One (model, harness) pair's routing trust.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct TrustLine {
    /// The model.
    pub model: String,
    /// The harness.
    pub harness: String,
    /// The posterior mean false-green rate.
    pub mean: f64,
    /// P(θ > 2·θ_max).
    pub p_exceed: f64,
    /// Whether DP4 leaves the pair out of standard-band routing.
    pub excluded: bool,
}

/// The judge's verdicts against audit labels, joined on the attempt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct JudgeLine {
    /// Verdicts with an audit label.
    pub joined: usize,
    /// Those the audit agreed with.
    pub correct: usize,
    /// `correct / joined`.
    pub accuracy: f64,
}

/// `roko audit status`.
#[derive(Debug, Default, Serialize)]
pub(crate) struct StatusReport {
    /// The latest closed window.
    pub window: Option<String>,
    /// Each of its strata's estimate.
    pub estimates: Vec<StratumLine>,
    /// Each task type's verify depth.
    pub ladder: BTreeMap<String, VerifyDepth>,
    /// Each pair's routing trust.
    pub trust: Vec<TrustLine>,
    /// Models a gate-gaming alert was logged for.
    pub gaming_alerts: Vec<String>,
    /// What audits spent, in USD.
    pub spent_usd: f64,
    /// Audits that reported.
    pub audits: usize,
    /// Times the audit budget stopped audits.
    pub budget_stops: usize,
    /// `[audit] budget_frac`, the share of a run's spend audits may take.
    pub budget_frac: f64,
    /// Selected units still to audit.
    pub queued: usize,
    /// The judge's calibration against audit labels, when any verdict joins.
    pub judge: Option<JudgeLine>,
    /// With `--strata`: every verdict stratum's green units, over all runs.
    pub strata: Option<BTreeMap<String, u64>>,
}

/// `roko audit status` for the workspace at `workdir`, with `vault` its
/// vault and `audit` its `[audit]`.
pub(crate) fn status(
    vault: &AuditVault,
    workdir: &Path,
    audit: &AuditConfig,
    strata: bool,
) -> Result<StatusReport> {
    let all = records(&vault.ledger_dir())?;
    let mut report = StatusReport {
        budget_frac: audit.budget_frac,
        ..StatusReport::default()
    };
    if let Some(window) = latest_window(&all) {
        report.estimates = window_estimates(&all, &window);
        report.window = Some(window);
    }
    let ladder = Ladder::load(&ladder_path(vault))?;
    report.ladder = ladder
        .task_types
        .into_iter()
        .map(|(task_type, rung)| (task_type, rung.level))
        .collect();
    let bound = 2.0 * audit.theta_max;
    report.trust = TrustBook::load(&trust_path(vault))?
        .estimates
        .into_iter()
        .map(|estimate| {
            let p_exceed = trust_exceedance(&estimate, bound);
            TrustLine {
                mean: estimate.mean(),
                p_exceed,
                excluded: p_exceed > TRUST_EXCLUDE_PROBABILITY,
                model: estimate.model,
                harness: estimate.harness,
            }
        })
        .collect();
    let mut labels: BTreeMap<&str, bool> = BTreeMap::new();
    for record in &all {
        match &record.event {
            AuditEvent::Result {
                attempt_key,
                labels: found,
                cost_usd,
                ..
            } => {
                report.audits += 1;
                report.spent_usd += cost_usd.unwrap_or(0.0);
                if let Some(y) = found.y {
                    labels.insert(attempt_key, y);
                }
            }
            AuditEvent::BudgetExhausted { .. } => report.budget_stops += 1,
            AuditEvent::PolicyChange { knob, to, .. } if knob == "gaming_alert" => {
                let model = to["model_slug"].as_str().unwrap_or("?").to_string();
                report.gaming_alerts.push(model);
            }
            _ => {}
        }
    }
    report.queued = units(&all).iter().filter(|unit| !unit.reported()).count();
    report.judge = judge_against(workdir, &labels);
    if strata {
        report.strata = Some(verdict_strata(&all));
    }
    Ok(report)
}

/// The id of the latest window whose summary the ledger holds.
fn latest_window(all: &[LedgerRecord]) -> Option<String> {
    all.iter().rev().find_map(|record| match &record.event {
        AuditEvent::Estimate {
            window,
            stratum: None,
            ..
        } if window.starts_with(WINDOW_PREFIX) => Some(window.clone()),
        _ => None,
    })
}

/// Each stratum's estimate in `window`, the latest record of each.
fn window_estimates(all: &[LedgerRecord], window: &str) -> Vec<StratumLine> {
    let mut lines: BTreeMap<String, StratumLine> = BTreeMap::new();
    for record in all {
        let AuditEvent::Estimate {
            window: id,
            stratum: Some(stratum),
            estimate,
        } = &record.event
        else {
            continue;
        };
        let Ok(group) = serde_json::from_value::<GroupEstimate>(estimate.clone()) else {
            continue;
        };
        if id != window {
            continue;
        }
        let name = [
            stratum.task_type.as_str(),
            stratum.model.as_str(),
            stratum.arm.as_str(),
            stratum.verdict.as_str(),
        ]
        .join(" / ");
        let theta = group.theta.estimate;
        let line = StratumLine {
            stratum: name.clone(),
            n_green: group.n_green,
            n_selected: group.n_selected,
            theta: theta.theta_hajek,
            ci: theta.ci,
            n_eff: theta.n_eff,
        };
        lines.insert(name, line);
    }
    lines.into_values().collect()
}

/// Every verdict stratum's green units over the runs the ledger summarises
/// (`audit.estimate` of `run:<run_id>`), `forced_accept` included.
fn verdict_strata(all: &[LedgerRecord]) -> BTreeMap<String, u64> {
    let mut strata = BTreeMap::new();
    for record in all {
        let AuditEvent::Estimate {
            window,
            stratum: None,
            estimate,
        } = &record.event
        else {
            continue;
        };
        if !window.starts_with("run:") {
            continue;
        }
        let counts = estimate["green_units"].as_object().into_iter().flatten();
        for (verdict, count) in counts {
            *strata.entry(verdict.clone()).or_default() += count.as_u64().unwrap_or(0);
        }
    }
    strata
}

/// The judge's verdicts (`.roko/learn/judge-calibration.jsonl`) against the
/// audit labels by attempt: a pass is right when the audit found no false
/// green. `None` when no verdict joins a label.
fn judge_against(workdir: &Path, labels: &BTreeMap<&str, bool>) -> Option<JudgeLine> {
    let log = roko_fs::RokoLayout::for_project(workdir)
        .learn_dir()
        .join("judge-calibration.jsonl");
    let pairs: Vec<(bool, bool)> = load_calibration_log(&log)
        .ok()?
        .iter()
        .filter_map(|row| {
            let false_green = labels.get(row.attempt_key.as_deref()?)?;
            Some((row.verdict, !false_green))
        })
        .collect();
    if pairs.is_empty() {
        return None;
    }
    let calibration = compute_calibration(&pairs);
    Some(JudgeLine {
        joined: calibration.total_examples,
        correct: calibration.correct,
        accuracy: calibration.accuracy,
    })
}

fn print_status(report: &StatusReport) {
    match &report.window {
        Some(window) => println!("window {window}"),
        None => println!("no window has closed yet"),
    }
    for line in &report.estimates {
        let theta = line
            .theta
            .map_or_else(|| "-".to_string(), |theta| format!("{theta:.3}"));
        println!(
            "  {}  green {}  selected {}  θ̂ {theta}  95% CI [{:.3}, {:.3}]  n_eff {:.1}",
            line.stratum, line.n_green, line.n_selected, line.ci.0, line.ci.1, line.n_eff
        );
    }
    for (task_type, level) in &report.ladder {
        println!("ladder  {task_type}  {level:?}");
    }
    for line in &report.trust {
        let out = if line.excluded { "  (left out)" } else { "" };
        println!(
            "trust   {} / {}  mean {:.3}  P(θ > 2·θ_max) {:.3}{out}",
            line.model, line.harness, line.mean, line.p_exceed
        );
    }
    if !report.gaming_alerts.is_empty() {
        println!("gate-gaming alerts: {}", report.gaming_alerts.join(", "));
    }
    println!(
        "spend   ${:.4} over {} audits (budget {:.0}% of run spend), {} budget stops",
        report.spent_usd,
        report.audits,
        report.budget_frac * 100.0,
        report.budget_stops
    );
    println!("queued  {} selected units to audit", report.queued);
    if let Some(judge) = &report.judge {
        println!(
            "judge   {} of {} verdicts agree with the audit ({:.3})",
            judge.correct, judge.joined, judge.accuracy
        );
    }
    if let Some(strata) = &report.strata {
        for (verdict, green) in strata {
            println!("stratum {verdict}  {green} green units");
        }
    }
}

/// `roko audit replay`.
#[derive(Debug, Default, Serialize)]
pub(crate) struct ReplayReport {
    /// The window replayed.
    pub window: Option<String>,
    /// Lotteries drawn.
    pub runs: u32,
    /// The window's own θ̂.
    pub theta: Option<f64>,
    /// Its 95% interval.
    pub ci: (f64, f64),
    /// The replays' θ̂ at 2.5%, 50% and 97.5%.
    pub quantiles: Option<(f64, f64, f64)>,
    /// The share of replays whose interval holds the window's θ̂.
    pub coverage: Option<f64>,
}

/// `runs` lotteries re-drawn over the latest window's audited units: each
/// keeps a unit at [`REPLAY_RATE`], by a keyed draw, and estimates θ from
/// those it keeps.
pub(crate) fn replay(vault: &AuditVault, runs: u32) -> Result<ReplayReport> {
    let all = records(&vault.ledger_dir())?;
    let mut report = ReplayReport {
        runs,
        ci: (0.0, 1.0),
        ..ReplayReport::default()
    };
    let Some(window) = latest_window(&all) else {
        return Ok(report);
    };
    let (first, last) = window_span(&window).unwrap_or((0, 0));
    let inside: Vec<WindowUnit> = units(&all)
        .into_iter()
        .filter(|unit| (first..=last).contains(&unit.seq))
        .collect();
    let n_green = inside.len() as u64;
    let sample: Vec<(f64, u8)> = inside
        .iter()
        .filter_map(WindowUnit::sampled)
        .filter_map(|(pi, labels)| Some((pi, u8::from(labels.y?))))
        .collect();
    report.window = Some(window);
    if n_green == 0 {
        return Ok(report);
    }
    let full = estimate(&sample, n_green, 0.05)?;
    report.theta = full.theta_hajek;
    report.ci = full.ci;
    let mut thetas = Vec::new();
    let mut covered = 0_u32;
    for run in 0..runs {
        let key = RunKey::derive(REPLAY_SECRET, &format!("replay-{run}"))?;
        let kept: Vec<(f64, u8)> = sample
            .iter()
            .enumerate()
            .filter(|(index, _)| {
                let x = draw(&key, "replay", &run.to_string(), &index.to_string(), "-");
                is_selected(x, REPLAY_RATE)
            })
            .map(|(_, &(pi, y))| (pi * REPLAY_RATE, y))
            .collect();
        let drawn = estimate(&kept, n_green, 0.05)?;
        let Some(theta) = drawn.theta_hajek else {
            continue;
        };
        thetas.push(theta);
        if full
            .theta_hajek
            .is_some_and(|value| (drawn.ci.0..=drawn.ci.1).contains(&value))
        {
            covered += 1;
        }
    }
    if !thetas.is_empty() {
        thetas.sort_by(f64::total_cmp);
        let at = |q: f64| thetas[((thetas.len() - 1) as f64 * q).round() as usize];
        report.quantiles = Some((at(0.025), at(0.5), at(0.975)));
        report.coverage = Some(f64::from(covered) / thetas.len() as f64);
    }
    Ok(report)
}

fn print_replay(report: &ReplayReport) {
    let Some(window) = &report.window else {
        println!("no window has closed yet");
        return;
    };
    let theta = report
        .theta
        .map_or_else(|| "-".to_string(), |theta| format!("{theta:.3}"));
    println!(
        "window {window}: θ̂ {theta}, 95% CI [{:.3}, {:.3}]",
        report.ci.0, report.ci.1
    );
    match (report.quantiles, report.coverage) {
        (Some((low, middle, high)), Some(coverage)) => println!(
            "{} lotteries at {REPLAY_RATE}: θ̂ {middle:.3} [{low:.3}, {high:.3}], \
             their intervals hold the window's θ̂ in {:.1}%",
            report.runs,
            coverage * 100.0
        ),
        _ => println!("no lottery kept a labelled unit"),
    }
}

/// `roko audit reveal`.
#[derive(Debug, Serialize)]
pub(crate) struct RevealReport {
    /// The run.
    pub run_id: String,
    /// Draws recomputed.
    pub draws: usize,
    /// Records in the ledger's chain, or where it breaks.
    pub chain: std::result::Result<u64, String>,
    /// Every mismatch: the commitment, each draw.
    pub mismatches: Vec<String>,
}

impl RevealReport {
    /// Whether the run's key opens its commitment and reproduces every draw,
    /// and the chain holds.
    pub(crate) fn holds(&self) -> bool {
        self.chain.is_ok() && self.mismatches.is_empty()
    }
}

/// Recompute every draw of `run_id` in the ledger at `ledger_dir` from its
/// revealed key (S05 §7.3), check the key against its commitment and walk
/// the chain.
pub(crate) fn reveal(ledger_dir: &Path, run_id: &str) -> Result<RevealReport> {
    let all = records(ledger_dir)?;
    let mut committed = None;
    let mut revealed = None;
    let mut selections = Vec::new();
    for record in &all {
        match &record.event {
            AuditEvent::KeyCommit {
                run_id: run,
                commitment,
            } if run == run_id => committed = Some(commitment.clone()),
            AuditEvent::KeyReveal {
                run_id: run,
                key_hex,
            } if run == run_id => revealed = Some(RunKey::from_hex(key_hex)),
            AuditEvent::Selection {
                run_id: run,
                task_id,
                attempt_key,
                pi,
                prf_u,
                selected,
                result_tree,
                ..
            } if run == run_id => selections.push(Selection {
                run_id: run.clone(),
                task_id: task_id.clone(),
                attempt_id: attempt_key.clone(),
                accepted_commit: result_tree.clone().unwrap_or_else(|| "-".to_string()),
                pi: *pi,
                prf_u: prf_u.clone(),
                selected: *selected,
            }),
            _ => {}
        }
    }
    let mismatches = match (committed, revealed) {
        (None, _) => vec![format!("no audit.key_commit for run {run_id}")],
        (Some(_), None) => vec![format!("run {run_id}'s key is not revealed yet")],
        (Some(_), Some(None)) => vec![format!("run {run_id}'s revealed key does not parse")],
        (Some(committed), Some(Some(key))) => verify_reveal(&key, run_id, &committed, &selections),
    };
    Ok(RevealReport {
        run_id: run_id.to_string(),
        draws: selections.len(),
        chain: verify_chain(ledger_dir).map_err(|error| error.to_string()),
        mismatches,
    })
}

fn print_reveal(report: &RevealReport) {
    let run = &report.run_id;
    match &report.chain {
        Ok(count) => println!("the ledger's chain holds ({count} records)"),
        Err(error) => println!("the ledger's chain breaks: {error}"),
    }
    println!(
        "run {run}: {} draws recomputed, {} mismatches",
        report.draws,
        report.mismatches.len()
    );
    for mismatch in &report.mismatches {
        println!("  {mismatch}");
    }
}

/// `roko audit incidents`.
#[derive(Debug, Default, Serialize)]
pub(crate) struct IncidentsReport {
    /// The incidents, oldest first.
    pub incidents: Vec<Incident>,
    /// The isolation proposals that wait for an operator.
    pub isolation_proposals: Vec<Value>,
}

/// The vault's incidents and isolation proposals.
pub(crate) fn incidents(vault: &AuditVault) -> Result<IncidentsReport> {
    let store = IncidentStore::open(vault)?;
    let mut report = IncidentsReport {
        incidents: store.list()?,
        ..IncidentsReport::default()
    };
    for entry in std::fs::read_dir(store.dir())? {
        let path = entry?.path();
        let name = path.file_name().and_then(|name| name.to_str());
        if !name.is_some_and(|name| name.starts_with("isolate-") && name.ends_with(".json")) {
            continue;
        }
        let text = std::fs::read_to_string(&path)?;
        report
            .isolation_proposals
            .extend(serde_json::from_str(&text).ok());
    }
    Ok(report)
}

fn print_incidents(report: &IncidentsReport) {
    if report.incidents.is_empty() {
        println!("no incidents");
    }
    for incident in &report.incidents {
        let fix = incident.fix_proposal.as_deref().unwrap_or("-");
        println!(
            "{}  {}  {}  model {}  task {}  fix proposal {fix}",
            incident.incident_id,
            incident.kind.label(),
            incident.status().label(),
            incident.model,
            incident.task_id
        );
    }
    for proposal in &report.isolation_proposals {
        println!(
            "isolation proposal  {} / {}  confirmed {}",
            proposal["model"].as_str().unwrap_or("?"),
            proposal["harness"].as_str().unwrap_or("?"),
            proposal["confirmed"]
        );
    }
}

#[cfg(test)]
mod tests {
    use roko_core::audit_types::Stratum;
    use roko_gate::audit::ledger::AuditLedger;
    use roko_gate::audit::policy::select;
    use serde_json::json;

    use super::*;

    fn vault(temp: &tempfile::TempDir) -> (PathBuf, AuditVault) {
        let workspace = temp.path().join("repo");
        std::fs::create_dir_all(&workspace).expect("mkdir");
        let home = temp.path().join("vault");
        let vault = AuditVault::resolve_with(&workspace, Some(&home), None).expect("a vault");
        (workspace, vault)
    }

    /// The selection a draw of `run` at `pi` logs, for task `t<n>`.
    fn selection(key: &RunKey, run: &str, n: u32, pi: f64, flip: bool) -> AuditEvent {
        let task = format!("t{n}");
        let attempt = format!("{run}:plan:{task}:1");
        let tree = format!("tree-{n}");
        let drawn = select(key, run, &task, &attempt, &tree, pi).expect("a draw");
        AuditEvent::Selection {
            sel_id: format!("sel-{n}"),
            attempt_key: attempt,
            run_id: run.to_string(),
            task_id: task,
            stratum: Stratum::default(),
            pi: drawn.pi,
            prf_u: drawn.prf_u,
            selected: drawn.selected != flip,
            base_tree: None,
            result_tree: Some(tree),
            risk_r: None,
            prediction_id: None,
        }
    }

    /// S05 §7.3: once a run reveals its key, `reveal` recomputes every draw
    /// with no mismatch; before it nothing can be checked, and a forged draw
    /// or an edited ledger line is caught.
    #[test]
    fn audit_reveal_recomputes_every_draw() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (_, vault) = vault(&temp);
        let mut ledger = AuditLedger::open(&vault).expect("a ledger");
        let key = RunKey::derive(&[7_u8; 32], "run-1").expect("a key");
        ledger.commit_key("run-1", &key).expect("the commitment");
        for n in 0..12 {
            let drawn = selection(&key, "run-1", n, 0.3, false);
            ledger.append(drawn).expect("a selection");
        }
        let early = reveal(&vault.ledger_dir(), "run-1").expect("a report");
        assert!(!early.holds(), "nothing is checked before the reveal");

        ledger.reveal_key("run-1", &key).expect("the reveal");
        let report = reveal(&vault.ledger_dir(), "run-1").expect("a report");
        assert_eq!(report.draws, 12);
        assert!(report.holds(), "{report:?}");
        assert_eq!(report.chain, Ok(14), "a commit, 12 draws and the reveal");

        // A draw logged against its key is a mismatch.
        let forged = selection(&key, "run-1", 12, 0.3, true);
        ledger.append(forged).expect("a forged selection");
        let report = reveal(&vault.ledger_dir(), "run-1").expect("a report");
        assert_eq!(report.mismatches.len(), 1, "{report:?}");
        assert!(!report.holds());

        // An edited line breaks the chain.
        let day = std::fs::read_dir(vault.ledger_dir())
            .expect("the ledger")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "jsonl")
            })
            .expect("a day file");
        let text = std::fs::read_to_string(&day).expect("the day file");
        std::fs::write(&day, text.replacen("\"sel-3\"", "\"sel-x\"", 1)).expect("an edit");
        let report = reveal(&vault.ledger_dir(), "run-1").expect("a report");
        assert!(report.chain.is_err(), "{report:?}");
    }

    /// S05 §7.6: `status --strata` lists every verdict stratum the runs
    /// counted, and the Graph path's `forced_accept` stratum holds none.
    #[test]
    fn status_strata_shows_an_empty_forced_accept_stratum() {
        let temp = tempfile::tempdir().expect("tempdir");
        let (workspace, vault) = vault(&temp);
        let mut ledger = AuditLedger::open(&vault).expect("a ledger");
        // Two runs' closes, as DP1 logs them.
        for (run, passed) in [("run-1", 3), ("run-2", 2)] {
            let green_units = json!({
                "passed": passed, "passed_with_preexisting_failures": 0, "already_satisfied": 1,
                "unverified": 1, "forced_accept": 0
            });
            ledger
                .append(AuditEvent::Estimate {
                    window: format!("run:{run}"),
                    stratum: None,
                    estimate: json!({ "green_units": green_units }),
                })
                .expect("a run summary");
        }
        let audit = AuditConfig::default();
        let report = status(&vault, &workspace, &audit, true).expect("the status");
        let strata = report.strata.expect("--strata lists the strata");
        assert_eq!(strata.get("forced_accept"), Some(&0));
        assert_eq!(strata.get("passed"), Some(&5));
        assert_eq!(strata.len(), 5);
        assert_eq!(report.window, None, "no window has closed");
        let plain = status(&vault, &workspace, &audit, false).expect("the status");
        assert!(plain.strata.is_none());
    }
}
