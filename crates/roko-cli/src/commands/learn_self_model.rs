//! `roko learn self-model fit|replay` and `roko learn econ prices` (S04 T07 and T07b; backlog
//! 6124). Every command only reads: `fit` scores the self-model over the logs and saves nothing,
//! `replay` writes only under `--out`, and `econ prices` lists the price snapshot.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use clap::Subcommand;
use roko_cli::exit_codes::EXIT_SUCCESS;
use roko_core::config::routing::LadderConfig;
use roko_core::pricing_snapshot::PriceSnapshot;
use roko_fs::RokoLayout;
use roko_learn::self_model::Unit;
use roko_learn::self_model::baselines::{
    AttemptResult, FrugalCascade, Oracle, ProductionLadder, RouteTask, RouterPolicy, RoutingPolicy,
    StaticArm,
};
use roko_learn::self_model::features::TaskFeatures;
use roko_learn::self_model::gate::{
    CalibrationGate, CalibrationWindow, GateReport, WINDOW, WindowOutcome,
};
use roko_learn::self_model::ingest::{self, DataAudit};
use roko_learn::self_model::legacy;
use roko_learn::self_model::metrics::{
    ECE_BINS, Scored, auroc, base_rate, brier, brier_skill, calibration_in_the_large, ece,
};
use roko_learn::self_model::model::SelfModel;
use roko_learn::self_model::policy::LcbAciConfig;
use roko_learn::self_model::replay::{
    self, CascadePolicy, LcbAciPolicy, Matrix, ORDERINGS, ReplaySummary,
};
use serde::Serialize;

use crate::{Cli, resolve_workdir};

/// The policies `replay` runs when `--policies` names none.
const DEFAULT_POLICIES: [&str; 7] = [
    "H4-B0", "H4-B1", "H4-BL", "H4-B2", "H4-B3", "lcb_aci", "cascade",
];

/// `roko learn self-model`.
#[derive(Debug, Subcommand)]
pub(crate) enum SelfModelCmd {
    /// Fit the self-model over the logs (the S01 runs and the legacy efficiency and episode
    /// logs), and print the data audit and its prequential scores. Nothing is saved.
    Fit {
        /// Read the logs and dispatch nothing: the one mode there is.
        #[arg(long)]
        offline: bool,
        /// The `.roko` directory to read (default: the working directory's).
        #[arg(long)]
        from: Option<PathBuf>,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Replay routing policies over a matrix of `vb.run_record/1` rows (S09 R-H4), writing
    /// `traces.jsonl` and `summary.json` under `--out`.
    Replay {
        /// Directory of run-record JSONL files.
        #[arg(long)]
        matrix: PathBuf,
        /// Policies, comma-separated (default: H4-B0, H4-B1, H4-BL, H4-B2, H4-B3, lcb_aci and
        /// cascade). `lcb_aci@0.85` runs lcb_aci at π* = 0.85, one point of the π* sweep.
        #[arg(long, value_delimiter = ',')]
        policies: Vec<String>,
        /// Task orderings per policy.
        #[arg(long, default_value_t = ORDERINGS)]
        orderings: u32,
        /// Seed of the orderings.
        #[arg(long, default_value_t = 7)]
        seed: u64,
        /// Directory the traces and the summary go to.
        #[arg(long)]
        out: PathBuf,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

/// `roko learn econ`.
#[derive(Debug, Subcommand)]
pub(crate) enum EconCmd {
    /// List the price snapshot's rows with their source URLs (S04 §4.8).
    Prices {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

/// Working directory of a `roko learn self-model` subcommand.
pub(crate) fn self_model_workdir(cli: &Cli, cmd: &SelfModelCmd) -> PathBuf {
    match cmd {
        SelfModelCmd::Fit { workdir, .. } | SelfModelCmd::Replay { workdir, .. } => {
            workdir.clone().unwrap_or_else(|| resolve_workdir(cli))
        }
    }
}

/// Working directory of a `roko learn econ` subcommand.
pub(crate) fn econ_workdir(cli: &Cli, cmd: &EconCmd) -> PathBuf {
    match cmd {
        EconCmd::Prices { workdir } => workdir.clone().unwrap_or_else(|| resolve_workdir(cli)),
    }
}

/// `roko learn self-model fit|replay`.
pub(crate) fn cmd_self_model(cli: &Cli, cmd: SelfModelCmd, json: bool) -> Result<i32> {
    let workdir = self_model_workdir(cli, &cmd);
    let config = roko_core::config::loader::load_config_unified(&workdir).unwrap_or_default();
    let snapshot =
        PriceSnapshot::for_workspace(&config.pricing, &workdir).context("load the price snapshot")?;
    match cmd {
        SelfModelCmd::Fit { offline, from, .. } => {
            if !offline {
                bail!("only `--offline` fits exist: the fit reads the logs and dispatches nothing");
            }
            let layout = from.map_or_else(|| RokoLayout::for_project(&workdir), RokoLayout::new);
            let report = fit_offline(&layout, &snapshot)?;
            if json {
                println!("{}", serde_json::to_string(&report)?);
            } else {
                print!("{}", render_fit(&report));
            }
        }
        SelfModelCmd::Replay {
            matrix,
            policies,
            orderings,
            seed,
            out,
            ..
        } => {
            let ladder = &config.routing.ladder;
            let report =
                replay_matrix(&matrix, &policies, orderings, seed, &out, &snapshot, ladder)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", render_replay(&report, &out));
            }
        }
    }
    Ok(EXIT_SUCCESS)
}

/// `roko learn econ prices`.
pub(crate) fn cmd_econ(cli: &Cli, cmd: EconCmd, json: bool) -> Result<i32> {
    let workdir = econ_workdir(cli, &cmd);
    let config = roko_core::config::loader::load_config_unified(&workdir).unwrap_or_default();
    let snapshot =
        PriceSnapshot::for_workspace(&config.pricing, &workdir).context("load the price snapshot")?;
    match cmd {
        EconCmd::Prices { .. } => {
            if json {
                println!("{}", serde_json::to_string_pretty(&prices_json(&snapshot))?);
            } else {
                print!("{}", render_prices(&snapshot));
            }
        }
    }
    Ok(EXIT_SUCCESS)
}

// -----------------------------------------------------------------------
// fit
// -----------------------------------------------------------------------

/// What `roko learn self-model fit --offline` reports.
#[derive(Debug, Serialize)]
pub(crate) struct FitReport {
    /// The counts behind the units, so local data is never mistaken for evidence (S04 §0).
    pub(crate) data_audit: FitAudit,
    /// The version the scores belong to.
    pub(crate) predictor_version: String,
    /// The price snapshot the cost model prices from.
    pub(crate) price_snapshot_id: String,
    /// Every training unit, forecast before the model learned it.
    pub(crate) prequential: Prequential,
    /// `insufficient_n` below the gate's window, else `eligible` or `not_eligible`.
    pub(crate) calibration: &'static str,
    /// The calibration gate's report on the last window, once there is one.
    pub(crate) gate: Option<GateReport>,
}

/// A fit's data audit: each source's counts, and the units left once the legacy logs' copies
/// of S01 tasks are dropped.
#[derive(Debug, Serialize)]
pub(crate) struct FitAudit {
    /// The S01 run records under `.roko/runs`.
    pub(crate) runs: DataAudit,
    /// The legacy efficiency and episode logs.
    pub(crate) legacy: DataAudit,
    /// Units read from both sources.
    pub(crate) raw_units: usize,
    /// Legacy units whose plan and task the S01 runs hold, dropped: S01 records each attempt.
    pub(crate) duplicates: usize,
    /// Units left.
    pub(crate) units: usize,
    /// Of those, the units the model trains on: labelled, and not a failover substitute.
    pub(crate) training_units: usize,
}

/// Prequential scores of the gate forecast, each unit scored before it is learned.
#[derive(Debug, Serialize)]
pub(crate) struct Prequential {
    /// Units scored.
    pub(crate) n: usize,
    /// Brier score.
    pub(crate) brier: Option<f64>,
    /// The weighted pass rate.
    pub(crate) base_rate: Option<f64>,
    /// Brier skill against the base rate.
    pub(crate) brier_skill: Option<f64>,
    /// ECE over equal-mass bins.
    pub(crate) ece: Option<f64>,
    /// Mean forecast minus the base rate.
    pub(crate) calibration_in_the_large: Option<f64>,
    /// AUROC.
    pub(crate) auroc: Option<f64>,
}

/// Fit a fresh self-model over the logs under `layout`, scoring every training unit before the
/// model learns it. Reads only: the fitted state is not saved.
pub(crate) fn fit_offline(layout: &RokoLayout, snapshot: &PriceSnapshot) -> Result<FitReport> {
    let runs = ingest::read_runs(&layout.runs_dir()).context("read the S01 runs")?;
    let logs = legacy::read_files(&layout.efficiency_path(), &layout.episodes_path())
        .context("read the legacy logs")?;
    let held: BTreeSet<(&str, &str)> = runs.units.iter().map(plan_task).collect();
    let fresh: Vec<&Unit> = logs
        .units
        .iter()
        .filter(|unit| !held.contains(&plan_task(unit)))
        .collect();
    let mut model = SelfModel::new(snapshot);
    let mut window = CalibrationWindow::new(model.version.clone());
    let mut scored = Vec::new();
    for unit in runs.units.iter().chain(fresh.iter().copied()) {
        let Some(passed) = unit.label.y_gate.filter(|_| !unit.failover) else {
            continue;
        };
        let task = TaskFeatures::from(unit);
        let forecasts = model.forecast(&task, std::slice::from_ref(&unit.arm));
        let Some(p) = forecasts.first().map(|forecast| forecast.p_gate) else {
            continue;
        };
        let w = unit.label.weight;
        scored.push(Scored::weighted(p, passed, w));
        let outcome = WindowOutcome {
            p,
            y: passed,
            w,
            routed: false,
        };
        window.push(&model.version, outcome);
        model.observe_with(unit, &task, w);
    }
    let gate = (scored.len() >= WINDOW).then(|| CalibrationGate::default().evaluate(&window));
    let calibration = match &gate {
        None => "insufficient_n",
        Some(report) if report.eligible => "eligible",
        Some(_) => "not_eligible",
    };
    let raw_units = runs.units.len() + logs.units.len();
    let units = runs.units.len() + fresh.len();
    Ok(FitReport {
        data_audit: FitAudit {
            runs: runs.audit.clone(),
            legacy: logs.audit.clone(),
            raw_units,
            duplicates: raw_units - units,
            units,
            training_units: scored.len(),
        },
        predictor_version: model.version.to_string(),
        price_snapshot_id: snapshot.id().to_string(),
        prequential: Prequential {
            n: scored.len(),
            brier: brier(&scored),
            base_rate: base_rate(&scored),
            brier_skill: brier_skill(&scored),
            ece: ece(&scored, ECE_BINS),
            calibration_in_the_large: calibration_in_the_large(&scored),
            auroc: auroc(&scored),
        },
        calibration,
        gate,
    })
}

/// The (plan, task) a unit belongs to.
fn plan_task(unit: &Unit) -> (&str, &str) {
    (unit.plan_id.as_str(), unit.task_id.as_str())
}

/// The fit report as text.
pub(crate) fn render_fit(report: &FitReport) -> String {
    let audit = &report.data_audit;
    let scores = &report.prequential;
    let number =
        |value: Option<f64>| value.map_or_else(|| "n/a".to_string(), |v| format!("{v:.3}"));
    let mut out = format!(
        "self-model fit (offline): predictor {}, prices {}\n",
        report.predictor_version, report.price_snapshot_id
    );
    for (name, source) in [("S01 runs", &audit.runs), ("legacy logs", &audit.legacy)] {
        let _ = writeln!(out, "{name}:");
        for line in source.to_string().lines() {
            let _ = writeln!(out, "  {line}");
        }
    }
    let _ = writeln!(
        out,
        "units: {} read, {} duplicates dropped, {} kept, {} to train on",
        audit.raw_units, audit.duplicates, audit.units, audit.training_units
    );
    let _ = writeln!(
        out,
        "prequential (n = {}): Brier {}, base rate {}, Brier skill {}, ECE {}, calibration in \
         the large {}, AUROC {}",
        scores.n,
        number(scores.brier),
        number(scores.base_rate),
        number(scores.brier_skill),
        number(scores.ece),
        number(scores.calibration_in_the_large),
        number(scores.auroc)
    );
    match &report.gate {
        None => {
            let _ = writeln!(
                out,
                "calibration: {} (n = {} < {WINDOW})",
                report.calibration, scores.n
            );
        }
        Some(gate) if gate.reasons.is_empty() => {
            let _ = writeln!(out, "calibration: {}", report.calibration);
        }
        Some(gate) => {
            let reasons = gate.reasons.join("; ");
            let _ = writeln!(out, "calibration: {} ({reasons})", report.calibration);
        }
    }
    out
}

// -----------------------------------------------------------------------
// replay
// -----------------------------------------------------------------------

/// What `roko learn self-model replay` writes to `summary.json`.
#[derive(Debug, Serialize)]
pub(crate) struct ReplayReport {
    /// The matrix replayed.
    pub(crate) matrix: MatrixSummary,
    /// Orderings per policy.
    pub(crate) orderings: u32,
    /// Seed of the orderings.
    pub(crate) seed: u64,
    /// The price snapshot the self-model policies forecast costs with.
    pub(crate) price_snapshot_id: String,
    /// One summary per policy replayed, in the order asked for.
    pub(crate) policies: Vec<ReplaySummary>,
    /// Policies that cannot run on this matrix, with why.
    pub(crate) skipped: Vec<Skipped>,
}

/// The matrix a replay read.
#[derive(Debug, Serialize)]
pub(crate) struct MatrixSummary {
    /// Tasks, by instance id.
    pub(crate) tasks: usize,
    /// Arms by mean cell cost, cheapest first: the rung order.
    pub(crate) arms: Vec<String>,
    /// Each arm's model, as its runs' first attempts asked for it.
    pub(crate) models: BTreeMap<String, String>,
    /// Records left out for an unknown cost.
    pub(crate) excluded_unknown_cost: usize,
    /// Lines that are not run records or carry no VS label.
    pub(crate) unreadable: usize,
    /// The matrix oracle's cost per resolved task.
    pub(crate) oracle_cpr: Option<f64>,
}

/// A policy a replay could not run.
#[derive(Debug, Serialize)]
pub(crate) struct Skipped {
    /// The policy.
    pub(crate) policy: String,
    /// Why it cannot run.
    pub(crate) why: String,
}

/// Replay `names` (default [`DEFAULT_POLICIES`]) over the run records in `dir`, writing every
/// trace line to `out/traces.jsonl` and the report to `out/summary.json`.
pub(crate) fn replay_matrix(
    dir: &Path,
    names: &[String],
    orderings: u32,
    seed: u64,
    out: &Path,
    snapshot: &PriceSnapshot,
    ladder: &LadderConfig,
) -> Result<ReplayReport> {
    let matrix =
        Matrix::read_dir(dir).with_context(|| format!("read the matrix in {}", dir.display()))?;
    let arms = rungs_by_cost(&matrix);
    let mut models = arm_models(dir)?;
    models.retain(|arm, _| arms.contains(arm));
    let names: Vec<&str> = if names.is_empty() {
        DEFAULT_POLICIES.to_vec()
    } else {
        names.iter().map(String::as_str).collect()
    };
    let mut specs = Vec::new();
    let mut skipped = Vec::new();
    for name in names {
        match policy_spec(name, &matrix, &arms, &models, snapshot, ladder)? {
            Ok(spec) => specs.push(spec),
            Err(why) => skipped.push(Skipped {
                policy: name.to_string(),
                why,
            }),
        }
    }
    std::fs::create_dir_all(out).with_context(|| format!("create {}", out.display()))?;
    let file = std::fs::File::create(out.join("traces.jsonl")).context("create traces.jsonl")?;
    let mut traces = std::io::BufWriter::new(file);
    let mut policies = Vec::new();
    for spec in &specs {
        let make = || spec.build(snapshot);
        policies.push(replay::replay(&matrix, &make, orderings, seed, &mut traces)?);
    }
    traces.flush()?;
    let report = ReplayReport {
        matrix: MatrixSummary {
            tasks: matrix.tasks().len(),
            arms,
            models,
            excluded_unknown_cost: matrix.excluded_unknown_cost,
            unreadable: matrix.unreadable,
            oracle_cpr: matrix.oracle_cpr(),
        },
        orderings,
        seed,
        price_snapshot_id: snapshot.id().to_string(),
        policies,
        skipped,
    };
    let summary = serde_json::to_string_pretty(&report)? + "\n";
    std::fs::write(out.join("summary.json"), summary).context("write summary.json")?;
    Ok(report)
}

/// The replay report as text.
fn render_replay(report: &ReplayReport, out: &Path) -> String {
    let matrix = &report.matrix;
    let mut text = format!(
        "replay: {} tasks, arms {}, {} orderings, seed {}\n",
        matrix.tasks,
        matrix.arms.join(", "),
        report.orderings,
        report.seed
    );
    for summary in &report.policies {
        let cpr = summary
            .cpr
            .map_or_else(|| "n/a".to_string(), |cpr| format!("${cpr:.4}"));
        let _ = writeln!(
            text,
            "  {}: {} of {} task runs resolved, {} attempts, cost per resolved task {cpr}",
            summary.policy, summary.resolved, summary.runs, summary.attempts
        );
    }
    for skipped in &report.skipped {
        let _ = writeln!(text, "  skipped {}: {}", skipped.policy, skipped.why);
    }
    let _ = writeln!(text, "wrote {}", out.join("traces.jsonl").display());
    text
}

/// A policy `replay` builds afresh for every ordering.
enum Spec {
    /// H4-B0, cross-fit: each family's static arm, chosen on the other families (D34).
    Static(BTreeMap<String, String>),
    /// H4-B1 on the rungs.
    Frugal(Vec<String>),
    /// H4-BL: the ladder, and each rung model's arm.
    Ladder(LadderConfig, BTreeMap<String, String>),
    /// H4-B2 over (model, provider, arm) triples.
    Router(Vec<(String, String, String)>),
    /// H4-B3 over every cell.
    Oracle(BTreeMap<(String, String), AttemptResult>),
    /// Policy (a) under a name, over the arms, at π*.
    LcbAci(String, Vec<String>, f64),
    /// Policy (b) on the rungs.
    Cascade(Vec<String>),
}

impl Spec {
    fn build(&self, snapshot: &PriceSnapshot) -> Box<dyn RoutingPolicy> {
        match self {
            Self::Static(arms) => Box::new(CrossFitStatic(arms.clone())),
            Self::Frugal(rungs) => Box::new(FrugalCascade::new(rungs.clone())),
            Self::Ladder(config, arms) => {
                Box::new(ProductionLadder::new(config.clone(), arms.clone()))
            }
            Self::Router(models) => {
                let models: Vec<(&str, &str, &str)> = models
                    .iter()
                    .map(|(model, provider, arm)| {
                        (model.as_str(), provider.as_str(), arm.as_str())
                    })
                    .collect();
                match RouterPolicy::new(&models) {
                    Some(policy) => Box::new(policy),
                    None => Box::new(Abstain("H4-B2")),
                }
            }
            Self::Oracle(outcomes) => Box::new(Oracle::new(outcomes)),
            Self::LcbAci(name, arms, target) => {
                let config = LcbAciConfig {
                    target: *target,
                    ..LcbAciConfig::default()
                };
                let inner = Box::new(LcbAciPolicy::new(arms.clone(), config, snapshot));
                Box::new(Named {
                    name: name.clone(),
                    inner,
                })
            }
            Self::Cascade(rungs) => Box::new(CascadePolicy::new(rungs.clone(), snapshot)),
        }
    }
}

/// The policy `name` on this matrix, `Ok(Err(why))` when it cannot run here, or an error for
/// a name no policy has.
fn policy_spec(
    name: &str,
    matrix: &Matrix,
    arms: &[String],
    models: &BTreeMap<String, String>,
    snapshot: &PriceSnapshot,
    ladder: &LadderConfig,
) -> Result<std::result::Result<Spec, String>> {
    // A model several arms run goes to the cheapest of them.
    let mut by_model: BTreeMap<String, String> = BTreeMap::new();
    for arm in arms {
        if let Some(model) = models.get(arm) {
            by_model.entry(model.clone()).or_insert(arm.clone());
        }
    }
    let spec = match name {
        "H4-B0" => Spec::Static(cross_fit_static(matrix, arms)),
        "H4-B1" => Spec::Frugal(arms.to_vec()),
        "H4-BL" => {
            let rungs: Vec<&str> = ladder
                .rungs
                .iter()
                .map(|rung| rung.model.as_str())
                .collect();
            if !rungs.iter().any(|model| by_model.contains_key(*model)) {
                let why = format!("no arm runs a ladder rung's model ({})", rungs.join(", "));
                return Ok(Err(why));
            }
            Spec::Ladder(ladder.clone(), by_model)
        }
        "H4-B2" => {
            if by_model.is_empty() {
                return Ok(Err("no arm records the model it ran".to_string()));
            }
            let triples = by_model
                .into_iter()
                .map(|(model, arm)| {
                    let provider = snapshot
                        .row(&model)
                        .map_or_else(|| "replay".to_string(), |row| row.provider.clone());
                    (model, provider, arm)
                })
                .collect();
            Spec::Router(triples)
        }
        "H4-B3" => Spec::Oracle(matrix.outcomes()),
        "cascade" => Spec::Cascade(arms.to_vec()),
        "lcb_aci" => Spec::LcbAci(
            name.to_string(),
            arms.to_vec(),
            LcbAciConfig::default().target,
        ),
        _ => {
            let target = name
                .strip_prefix("lcb_aci@")
                .and_then(|value| value.parse::<f64>().ok())
                .filter(|target| *target > 0.0 && *target < 1.0);
            let Some(target) = target else {
                bail!(
                    "unknown policy {name}: expected one of {} or lcb_aci@<π* in (0, 1)>",
                    DEFAULT_POLICIES.join(", ")
                );
            };
            Spec::LcbAci(name.to_string(), arms.to_vec(), target)
        }
    };
    Ok(Ok(spec))
}

/// Each family's best static arm by cost per resolved task, chosen on the other families only
/// (D34), so no task's own outcome picks its arm.
fn cross_fit_static(matrix: &Matrix, arms: &[String]) -> BTreeMap<String, String> {
    let mut families = BTreeSet::new();
    for task in matrix.tasks() {
        families.insert(task.family);
    }
    families
        .into_iter()
        .filter_map(|family| {
            let best = replay::cross_fit(matrix, &family, arms, |arm, training| {
                let make = || -> Box<dyn RoutingPolicy> { Box::new(StaticArm::new(arm.clone())) };
                replay::replay(training, &make, 1, 0, &mut std::io::sink())
                    .ok()
                    .and_then(|summary| summary.cpr)
                    .unwrap_or(f64::INFINITY)
            })?;
            Some((family, best))
        })
        .collect()
}

/// The matrix's arms by mean cell cost, cheapest first (ties by id): the rung order.
fn rungs_by_cost(matrix: &Matrix) -> Vec<String> {
    let mut totals: BTreeMap<String, (f64, u32)> = BTreeMap::new();
    for ((_, arm), result) in matrix.outcomes() {
        let total = totals.entry(arm).or_insert((0.0, 0));
        total.0 += result.cost_usd;
        total.1 += 1;
    }
    let mut rungs: Vec<(f64, String)> = totals
        .into_iter()
        .map(|(arm, (cost, cells))| (cost / f64::from(cells), arm))
        .collect();
    rungs.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    rungs.into_iter().map(|(_, arm)| arm).collect()
}

/// Each arm's model, as its runs' first attempts asked for it: the most frequent, ties by
/// name. Reads every `*.jsonl` file in `dir`, as [`Matrix::read_dir`] does.
fn arm_models(dir: &Path) -> Result<BTreeMap<String, String>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
        })
        .collect();
    paths.sort();
    let mut counts: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for path in paths {
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        for line in text.lines() {
            let Ok(record) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            let arm = record["arm"].as_str();
            let model = record["execution"]["attempts"][0]["model_requested"].as_str();
            if let (Some(arm), Some(model)) = (arm, model) {
                *counts
                    .entry(arm.to_string())
                    .or_default()
                    .entry(model.to_string())
                    .or_default() += 1;
            }
        }
    }
    Ok(counts
        .into_iter()
        .filter_map(|(arm, models)| {
            let model = models
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0)))?;
            Some((arm, model.0))
        })
        .collect())
}

/// H4-B0 over a whole matrix: each family's own static arm.
struct CrossFitStatic(BTreeMap<String, String>);

impl RoutingPolicy for CrossFitStatic {
    fn name(&self) -> &str {
        "H4-B0"
    }

    fn start(&mut self, task: &RouteTask) -> Option<String> {
        self.0.get(&task.family).cloned()
    }

    fn next(&mut self, _task: &RouteTask, _arm: &str, _result: &AttemptResult) -> Option<String> {
        None
    }
}

/// A policy under another name: one point of a sweep (`lcb_aci@0.85`).
struct Named {
    name: String,
    inner: Box<dyn RoutingPolicy>,
}

impl RoutingPolicy for Named {
    fn name(&self) -> &str {
        &self.name
    }

    fn start(&mut self, task: &RouteTask) -> Option<String> {
        self.inner.start(task)
    }

    fn next(&mut self, task: &RouteTask, arm: &str, result: &AttemptResult) -> Option<String> {
        self.inner.next(task, arm, result)
    }
}

/// A policy that dispatches nothing.
struct Abstain(&'static str);

impl RoutingPolicy for Abstain {
    fn name(&self) -> &str {
        self.0
    }

    fn start(&mut self, _task: &RouteTask) -> Option<String> {
        None
    }

    fn next(&mut self, _task: &RouteTask, _arm: &str, _result: &AttemptResult) -> Option<String> {
        None
    }
}

// -----------------------------------------------------------------------
// econ prices
// -----------------------------------------------------------------------

/// The snapshot as JSON: its id, when its rates were fetched, and every row.
fn prices_json(snapshot: &PriceSnapshot) -> serde_json::Value {
    serde_json::json!({
        "price_snapshot_id": snapshot.id(),
        "fetched_at": snapshot.fetched_at(),
        "rows": snapshot.rows(),
    })
}

/// One line per snapshot row: the model, its provider, its rates in USD per 1M tokens, and
/// the page the rates come from.
pub(crate) fn render_prices(snapshot: &PriceSnapshot) -> String {
    let mut out = format!(
        "price snapshot {} (fetched {}), USD per 1M tokens\n",
        snapshot.id(),
        snapshot.fetched_at()
    );
    for row in snapshot.rows() {
        let _ = writeln!(
            out,
            "  {} ({}): input {}, cache read {}, cache write {} (5m) {} (1h), output {}; \
             verified {}; {}",
            row.slug,
            row.provider,
            row.input,
            row.cache_read,
            row.cache_write_5m,
            row.cache_write_1h,
            row.output,
            row.verified,
            row.source_url
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Command;
    use crate::commands::learn::LearnCmd;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../roko-learn/tests/fixtures/self_model")
            .join(name)
    }

    /// 6124: the legacy fixture alone holds nine training units, too few for the calibration
    /// gate's window, and the fit says so.
    #[test]
    fn learn_self_model_fit_reports_insufficient_n() {
        let workspace = tempfile::tempdir().expect("tempdir");
        let layout = RokoLayout::for_project(workspace.path());
        std::fs::create_dir_all(layout.learn_dir()).expect("create .roko/learn");
        let legacy = fixture("legacy");
        std::fs::copy(legacy.join("efficiency.jsonl"), layout.efficiency_path())
            .expect("copy the efficiency log");
        std::fs::copy(legacy.join("episodes.jsonl"), layout.episodes_path())
            .expect("copy the episode log");
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");

        let report = fit_offline(&layout, &snapshot).expect("fit");
        assert_eq!(report.data_audit.runs.rows, 0);
        assert_eq!(report.data_audit.legacy.labelled, 9);
        assert_eq!(report.data_audit.duplicates, 0);
        assert_eq!(report.prequential.n, 9);
        assert_eq!(report.calibration, "insufficient_n");
        assert!(report.gate.is_none());
        let json = serde_json::to_string(&report).expect("serialize the report");
        assert!(json.contains(r#""calibration":"insufficient_n""#));
        let text = render_fit(&report);
        assert!(text.contains("label sources: gate_passed 4, pre_gate_success 5"));
        assert!(text.contains("calibration: insufficient_n (n = 9 < 100)"));
    }

    /// 6124: two replays of the matrix fixture with seed 7 write identical bytes.
    #[test]
    fn learn_self_model_replay_twice_writes_identical_bytes() {
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let ladder = LadderConfig::default();
        let matrix = fixture("matrix");
        let out = tempfile::tempdir().expect("tempdir");
        let mut names: Vec<String> = DEFAULT_POLICIES.map(str::to_string).to_vec();
        names.push("lcb_aci@0.9".to_string());
        let mut written = Vec::new();
        for run in ["a", "b"] {
            let dir = out.path().join(run);
            let report =
                replay_matrix(&matrix, &names, 2, 7, &dir, &snapshot, &ladder).expect("replay");
            assert!(report.skipped.is_empty(), "{:?}", report.skipped);
            assert_eq!(report.policies.len(), 8);
            assert_eq!(
                report.matrix.arms,
                ["cheap_direct", "mid_direct", "top_direct"]
            );
            let traces = std::fs::read(dir.join("traces.jsonl")).expect("traces.jsonl");
            let summary = std::fs::read(dir.join("summary.json")).expect("summary.json");
            written.push((traces, summary));
        }
        assert_eq!(written[0], written[1]);
        let traces = String::from_utf8(written[0].0.clone()).expect("utf-8");
        assert_eq!(traces.lines().count(), 8 * 2 * 12);
        assert!(traces.contains(r#""policy":"lcb_aci@0.9""#));

        let unknown = ["H4-B9".to_string()];
        let dir = out.path().join("c");
        assert!(replay_matrix(&matrix, &unknown, 1, 7, &dir, &snapshot, &ladder).is_err());
    }

    /// 6124: `roko learn econ prices` lists every row of the snapshot, each with its source URL.
    #[test]
    fn learn_econ_prices_lists_every_snapshot_row() {
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let text = render_prices(&snapshot);
        assert_eq!(text.lines().count(), 1 + 10, "{text}");
        for row in snapshot.rows() {
            assert!(text.contains(&row.source_url));
        }
        let json = prices_json(&snapshot);
        assert_eq!(json["rows"].as_array().map(Vec::len), Some(10));
    }

    /// 6124: the three commands parse.
    #[test]
    fn learn_self_model_commands_parse() {
        use clap::Parser as _;

        let cli = Cli::try_parse_from(["roko", "learn", "self-model", "fit", "--offline"])
            .expect("parse self-model fit");
        assert!(matches!(
            cli.command,
            Some(Command::Learn {
                cmd: LearnCmd::SelfModel {
                    cmd: SelfModelCmd::Fit {
                        offline: true,
                        from: None,
                        workdir: None,
                    },
                },
            })
        ));
        let cli = Cli::try_parse_from([
            "roko",
            "learn",
            "self-model",
            "replay",
            "--matrix",
            "matrix",
            "--policies",
            "H4-B0,lcb_aci@0.9",
            "--out",
            "out",
        ])
        .expect("parse self-model replay");
        assert!(matches!(
            cli.command,
            Some(Command::Learn {
                cmd: LearnCmd::SelfModel {
                    cmd: SelfModelCmd::Replay {
                        ref policies,
                        orderings: ORDERINGS,
                        seed: 7,
                        ..
                    },
                },
            }) if *policies == ["H4-B0", "lcb_aci@0.9"]
        ));
        let cli =
            Cli::try_parse_from(["roko", "learn", "econ", "prices"]).expect("parse econ prices");
        assert!(matches!(
            cli.command,
            Some(Command::Learn {
                cmd: LearnCmd::Econ {
                    cmd: EconCmd::Prices { workdir: None },
                },
            })
        ));
    }
}
