//! `roko learn self-model fit|replay` and `roko learn econ prices` (S04 T07 and T07b; backlog
//! 6124). Every command only reads: `fit` scores the self-model over the logs, joins the runs'
//! shadow forecasts to their verdicts and saves nothing, `replay` writes only under `--out`, and
//! `econ prices` lists the price snapshot.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use clap::Subcommand;
use roko_cli::exit_codes::EXIT_SUCCESS;
use roko_cli::learn_view;
use roko_cli::term_style::{self, Glyph, Role, Style};
use roko_core::config::routing::LadderConfig;
use roko_core::pricing_snapshot::PriceSnapshot;
use roko_fs::RokoLayout;
use roko_learn::self_model::Unit;
use roko_learn::self_model::baselines::{
    AttemptResult, FrugalCascade, Oracle, ProductionLadder, RouteTask, RouterPolicy, RoutingPolicy,
    StaticArm,
};
use roko_learn::self_model::features::TaskFeatures;
use roko_learn::self_model::gate::{CalibrationGate, CalibrationWindow, GateReport, WindowOutcome};
use roko_learn::self_model::ingest::{self, DataAudit, ForecastJoin};
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
    /// logs), and print the data audit, its prequential scores, and how the runs' shadow
    /// forecasts join their routed attempts and verdicts. Nothing is saved.
    Fit {
        /// Read the logs and dispatch nothing, the one mode there is, so it is the default; the
        /// flag stays accepted for scripts that pass it (gap-45f6bb).
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
    let snapshot = PriceSnapshot::for_workspace(&config.pricing, &workdir)
        .context("load the price snapshot")?;
    match cmd {
        SelfModelCmd::Fit { from, .. } => {
            let layout = from.map_or_else(|| RokoLayout::for_project(&workdir), RokoLayout::new);
            // The gate a plan run scores with: `[self_model] n_min` is its W (gap-4c531f).
            let gate = CalibrationGate::of(&config.self_model);
            let report = fit_offline(&layout, &snapshot, gate)?;
            if json {
                println!("{}", serde_json::to_string(&report)?);
            } else {
                print!("{}", render_fit(&report, Style::stdout()));
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
    let snapshot = PriceSnapshot::for_workspace(&config.pricing, &workdir)
        .context("load the price snapshot")?;
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
    /// W, the calibration gate's window: `[self_model] n_min` (gap-4c531f).
    pub(crate) window: usize,
    /// `insufficient_n` below the gate's window, else `eligible` or `not_eligible`.
    pub(crate) calibration: &'static str,
    /// The calibration gate's report on the last window, once there is one.
    pub(crate) gate: Option<GateReport>,
    /// How the runs' shadow forecasts join their routed attempts and verdicts (S04 §7.4).
    pub(crate) forecast_join: ForecastReport,
    /// Those forecasts scored against what happened, as the runs made them (gap-45f6bb).
    pub(crate) live_forecasts: ingest::LiveForecasts,
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
    /// Of those, the units the model learns: the labelled ones, a failover substitute's
    /// included (bug-a35286), though the prequential scores leave the substitutes out.
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

/// A fit's forecast join (gap-96547f): the share of routed attempts the self-model forecast,
/// and the share of its forecasts joined to a labelled verdict, with the counts behind both.
#[derive(Debug, Serialize)]
pub(crate) struct ForecastReport {
    /// `insufficient_n` when no run holds a forecast, else `measured`.
    pub(crate) status: &'static str,
    /// Routed attempts forecast, over the runs with a forecast.
    pub(crate) coverage: Option<f64>,
    /// Forecasts joined to a labelled verdict.
    pub(crate) join_share: Option<f64>,
    /// The counts behind the shares.
    #[serde(flatten)]
    pub(crate) counts: ForecastJoin,
}

impl ForecastReport {
    /// The report on `counts`.
    fn of(counts: ForecastJoin) -> Self {
        let status = if counts.forecasts == 0 {
            "insufficient_n"
        } else {
            "measured"
        };
        Self {
            status,
            coverage: counts.coverage(),
            join_share: counts.join_share(),
            counts,
        }
    }
}

/// Fit a fresh self-model over the logs under `layout`, scoring every training unit before the
/// model learns it and the last window of them with `gate`, and join the runs' shadow forecasts
/// to their verdicts. Reads only: the fitted state is not saved.
pub(crate) fn fit_offline(
    layout: &RokoLayout,
    snapshot: &PriceSnapshot,
    gate: CalibrationGate,
) -> Result<FitReport> {
    let runs = ingest::read_runs(&layout.runs_dir()).context("read the S01 runs")?;
    let forecasts =
        ingest::read_forecast_join(&layout.runs_dir()).context("read the S01 runs' forecasts")?;
    let live_forecasts = ingest::score_live_forecasts(&layout.runs_dir())
        .context("score the S01 runs' forecasts")?;
    let logs = legacy::read_files(&layout.efficiency_path(), &layout.episodes_path())
        .context("read the legacy logs")?;
    let held: BTreeSet<(&str, &str)> = runs.units.iter().map(plan_task).collect();
    let fresh: Vec<&Unit> = logs
        .units
        .iter()
        .filter(|unit| !held.contains(&plan_task(unit)))
        .collect();
    let mut model = SelfModel::new(snapshot);
    let mut window = CalibrationWindow::new(model.version.clone()).keeping(gate.keeps());
    let mut scored = Vec::new();
    let mut learned = 0;
    for unit in runs.units.iter().chain(fresh.iter().copied()) {
        let Some(passed) = unit.label.y_gate else {
            continue;
        };
        let task = TaskFeatures::from(unit);
        let w = unit.label.weight;
        // A failover substitute teaches the model what the model that ran did, but nobody
        // chose it: it is neither scored nor in the window, as in the live sink (bug-a35286).
        if unit.failover {
            model.observe_with(unit, &task, w);
            learned += 1;
            continue;
        }
        let forecasts = model.forecast(&task, std::slice::from_ref(&unit.arm));
        let Some(p) = forecasts.first().map(|forecast| forecast.p_gate) else {
            continue;
        };
        scored.push(Scored::weighted(p, passed, w));
        // The fit routes nothing, so it scores as a shadow window does: the routed pass rate
        // is unknown and fails no bound (bug-92cc56).
        let outcome = WindowOutcome {
            p,
            y: passed,
            w,
            routed: false,
        };
        window.push(&model.version, outcome);
        model.observe_with(unit, &task, w);
        learned += 1;
    }
    let report = (scored.len() >= gate.window).then(|| gate.evaluate(&window));
    let calibration = match &report {
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
            training_units: learned,
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
        window: gate.window,
        calibration,
        gate: report,
        forecast_join: ForecastReport::of(forecasts),
        live_forecasts,
    })
}

/// The (plan, task) a unit belongs to.
fn plan_task(unit: &Unit) -> (&str, &str) {
    (unit.plan_id.as_str(), unit.task_id.as_str())
}

/// The units the prequential scores cover: every unit the fit trains on but the failover
/// substitutes, which teach the model and are not scored (bug-a35286, gap-c63eb1).
fn prequential_scope(scored: usize, trained: usize) -> String {
    match trained.saturating_sub(scored) {
        0 => format!("all {scored} units"),
        1 => format!(
            "{scored} of the {trained} units (the failover substitute trains but isn't scored)"
        ),
        substitutes => format!(
            "{scored} of the {trained} units (the {substitutes} failover substitutes train but \
             aren't scored)"
        ),
    }
}

/// The width of the fit report's label column.
const FIT_LABEL_WIDTH: usize = 11;

/// The fit report as text, in roko's design system (gap-cb1a87): what the
/// fit is, then a label for each part of it: the run records and the older
/// logs it read, the units it trained on, its forecasts' scores, the
/// calibration gate, the forecasts the runs made and how many joined to a
/// verdict, and how those live forecasts scored.
pub(crate) fn render_fit(report: &FitReport, style: Style) -> String {
    let audit = &report.data_audit;
    let scores = &report.prequential;
    let number =
        |value: Option<f64>| value.map_or_else(|| "n/a".to_string(), |v| format!("{v:.3}"));
    let dot = format!(" {} ", style.glyph(Glyph::Dot));
    let context = format!(
        "how well roko forecasts a try's pass, refit offline over the run logs{dot}predictor \
         {}{dot}prices {}",
        report.predictor_version, report.price_snapshot_id
    );
    let mut out = term_style::header(style, "learn self-model fit", &context);
    let mut section = |label: &str, items: Vec<String>| {
        out.extend(learn_view::section(style, label, FIT_LABEL_WIDTH, &items));
    };
    let lines = |source: &DataAudit| source.to_string().lines().map(str::to_string).collect();
    section("Run records", lines(&audit.runs));
    section("Older logs", lines(&audit.legacy));
    section(
        "Units",
        vec![format!(
            "{} read, {} duplicates dropped, {} kept, {} to train on",
            audit.raw_units, audit.duplicates, audit.units, audit.training_units
        )],
    );
    section(
        "Forecasts",
        vec![format!(
            "scored before each unit taught the model, over {}: Brier {}, base rate {}, Brier \
             skill {}, ECE {}, calibration in the large {}, AUROC {}",
            prequential_scope(scores.n, audit.training_units),
            number(scores.brier),
            number(scores.base_rate),
            number(scores.brier_skill),
            number(scores.ece),
            number(scores.calibration_in_the_large),
            number(scores.auroc)
        )],
    );
    let (glyph, verdict) = match report.calibration {
        "eligible" => (
            style.paint(Role::Pass, style.glyph(Glyph::Pass)),
            "eligible",
        ),
        "not_eligible" => (
            style.paint(Role::Fail, style.glyph(Glyph::Fail)),
            "not eligible",
        ),
        _ => (
            style.paint(Role::Dim, style.glyph(Glyph::Waiting)),
            "not enough outcomes yet",
        ),
    };
    // The gate scores only its last W outcomes, a window of its own (gap-45f6bb).
    let calibration = match &report.gate {
        None => format!("{glyph} {verdict} (n = {} < {})", scores.n, report.window),
        Some(gate) if gate.reasons.is_empty() => format!(
            "{glyph} {verdict}, over the gate's last {} outcomes",
            gate.window
        ),
        Some(gate) => format!(
            "{glyph} {verdict}, over the gate's last {} outcomes ({})",
            gate.window,
            gate.reasons.join("; ")
        ),
    };
    section("Calibration", vec![calibration]);
    let join = &report.forecast_join;
    let counts = &join.counts;
    let joined = if counts.forecasts == 0 {
        vec![format!(
            "none yet: no predictions.jsonl row in {}",
            term_style::plural(counts.runs, "run", "runs")
        )]
    } else {
        vec![
            format!(
                "{} of {} runs: {} of {} routed attempts forecast (coverage {})",
                counts.runs_forecast,
                counts.runs,
                counts.routed_forecast,
                counts.routed,
                number(join.coverage)
            ),
            format!(
                "{} of {} joined to a labelled verdict (join share {}); unjoined: {} without a \
                 verdict, {} with an unlabelled verdict",
                counts.joined,
                counts.forecasts,
                number(join.join_share),
                counts.no_verdict,
                counts.unlabelled
            ),
        ]
    };
    section("Runs", joined);
    let live = &report.live_forecasts;
    let scored = if live.n == 0 {
        "none scored against a verdict".to_string()
    } else {
        format!(
            "as the runs made them: {} with a verdict, Brier {} against {} for always \
             forecasting the pass rate (Brier skill {})",
            live.n,
            number(live.brier),
            number(live.base_rate_brier),
            number(live.brier_skill)
        )
    };
    section("Live", vec![scored]);
    let mut text = out.join("\n");
    text.push('\n');
    text
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
    // An arm that ran several models is one arm per model (gap-b10978).
    let models = matrix.arm_models();
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
        policies.push(replay::replay(
            &matrix,
            &make,
            orderings,
            seed,
            &mut traces,
        )?);
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
    Ladder(Box<LadderConfig>, BTreeMap<String, String>),
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
                Box::new(ProductionLadder::new((**config).clone(), arms.clone()))
            }
            Self::Router(models) => {
                let models: Vec<(&str, &str, &str)> = models
                    .iter()
                    .map(|(model, provider, arm)| (model.as_str(), provider.as_str(), arm.as_str()))
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
            Spec::Ladder(Box::new(ladder.clone()), by_model)
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

/// H4-B0 over a whole matrix: each family's own static arm.
struct CrossFitStatic(BTreeMap<String, String>);

impl RoutingPolicy for CrossFitStatic {
    fn name(&self) -> &'static str {
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
    /// gate's window, and the fit says so. gap-4c531f: `[self_model] n_min = 9` lowers the bar
    /// to them, and the gate scores all nine.
    #[test]
    fn learn_self_model_fit_reports_insufficient_n() {
        use roko_core::config::self_model::SelfModelConfig;

        let workspace = tempfile::tempdir().expect("tempdir");
        let layout = RokoLayout::for_project(workspace.path());
        std::fs::create_dir_all(layout.learn_dir()).expect("create .roko/learn");
        let legacy = fixture("legacy");
        std::fs::copy(legacy.join("efficiency.jsonl"), layout.efficiency_path())
            .expect("copy the efficiency log");
        std::fs::copy(legacy.join("episodes.jsonl"), layout.episodes_path())
            .expect("copy the episode log");
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");

        let report = fit_offline(&layout, &snapshot, CalibrationGate::default()).expect("fit");
        assert_eq!(report.data_audit.runs.rows, 0);
        assert_eq!(report.data_audit.legacy.labelled, 9);
        assert_eq!(report.data_audit.duplicates, 0);
        assert_eq!(report.prequential.n, 9);
        assert_eq!(report.window, 100);
        assert_eq!(report.calibration, "insufficient_n");
        assert!(report.gate.is_none());
        let json = serde_json::to_string(&report).expect("serialize the report");
        assert!(json.contains(r#""calibration":"insufficient_n""#));
        let text = render_fit(&report, Style::plain());
        // In roko's design system (gap-cb1a87): the header, then a label for each part.
        assert!(
            text.starts_with(
                "◆ roko  learn self-model fit\n  how well roko forecasts a try's pass, refit \
                 offline over the run logs · predictor "
            ),
            "{text}"
        );
        assert!(text.contains("\n  Older logs   rows read: "), "{text}");
        assert!(text.contains("label sources: gate_passed 4, pre_gate_success 5"));
        assert!(
            text.contains("\n  Calibration  ○ not enough outcomes yet (n = 9 < 100)"),
            "{text}"
        );

        let settings = SelfModelConfig {
            n_min: 9,
            ..SelfModelConfig::default()
        };
        let report = fit_offline(&layout, &snapshot, CalibrationGate::of(&settings)).expect("fit");
        assert_eq!(report.window, 9);
        assert_ne!(report.calibration, "insufficient_n");
        let scored = report.gate.as_ref().map(|gate| (gate.window, gate.n));
        assert_eq!(scored, Some((9, 9)));
    }

    /// gap-96547f: the fit joins each run's shadow forecasts to its routed attempts and their
    /// verdicts. In a run with 4 routed attempts, of which the self-model forecast 3 and 2 of
    /// those settled with a label, it reports coverage 0.75 and join share 2/3: the third
    /// forecast's attempt has no verdict, and once it settles unverified, a verdict without a
    /// label. With no forecast the join is `insufficient_n`.
    #[test]
    fn fit_reports_the_forecast_join_share() {
        use roko_learn::routing_log::RoutingDecisionLog;
        use roko_learn::telemetry::records::{
            AttemptPredictionRecord, PredictionDecision, PredictionPredictor,
        };
        use roko_learn::telemetry::{
            AttemptIdentity, AttemptKey, AttemptOpenRecord, AttemptOutcome, AttemptVerdictRecord,
            TelemetryWriter, TelemetryWriterConfig,
        };

        let workspace = tempfile::tempdir().expect("tempdir");
        let layout = RokoLayout::for_project(workspace.path());
        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let empty = fit_offline(&layout, &snapshot, CalibrationGate::default()).expect("fit");
        assert_eq!(empty.forecast_join.status, "insufficient_n");
        assert_eq!(empty.forecast_join.join_share, None);
        let text = render_fit(&empty, Style::plain());
        assert!(
            text.contains("\n  Runs         none yet: no predictions.jsonl row in "),
            "{text}"
        );

        let identity =
            |task: &str| AttemptIdentity::new(&AttemptKey::new("run-f", "plan-f", task, 1));
        let route = |task: &str| -> RoutingDecisionLog {
            let row = serde_json::json!({
                "task_id": task,
                "selected_model": "gpt-oss-120b",
                "candidates": [],
                "attempt_key": identity(task).attempt_key,
            });
            serde_json::from_value(row).expect("a route decision")
        };
        let prediction = |task: &str| {
            let predictor = PredictionPredictor {
                version: "m3-l1-00000000".to_string(),
                class: "m3-l1".to_string(),
                mode: "shadow".to_string(),
                trained_on_n: 0,
                features_schema: 1,
                features_hash: "b3:0".to_string(),
            };
            let decision = PredictionDecision {
                would_choose: None,
                default: None,
                action: "dispatch".to_string(),
                start_rules: None,
            };
            AttemptPredictionRecord::new(identity(task), predictor, Vec::new(), decision)
        };
        let verdict = |task: &str, outcome| {
            let mut verdict = AttemptVerdictRecord::settle(identity(task), outcome, true);
            verdict.executed.provider = Some("cerebras".to_string());
            verdict.executed.model_requested = Some("gpt-oss-120b".to_string());
            verdict
        };
        // T1 to T4 route, and the self-model forecast T1 to T3. T1 passes and T2 fails, both
        // labelled; T3 has not settled; T4 passes.
        let run_dir = layout.run_dir("run-f");
        std::fs::create_dir_all(&run_dir).expect("the run's directory");
        let config = TelemetryWriterConfig::default();
        let writer = TelemetryWriter::spawn(&run_dir, config).expect("the run's writer");
        let attempts = [
            ("T1", true, Some(AttemptOutcome::Passed)),
            ("T2", true, Some(AttemptOutcome::GateFailed)),
            ("T3", true, None),
            ("T4", false, Some(AttemptOutcome::Passed)),
        ];
        for (task, forecast, outcome) in attempts {
            assert!(writer.submit(AttemptOpenRecord::new(identity(task), 0)));
            if forecast {
                assert!(writer.submit(prediction(task)));
            }
            assert!(writer.submit(route(task)));
            if let Some(outcome) = outcome {
                assert!(writer.submit(verdict(task, outcome)));
            }
        }
        writer.close();

        let report = fit_offline(&layout, &snapshot, CalibrationGate::default()).expect("fit");
        let join = &report.forecast_join;
        assert_eq!(join.status, "measured");
        assert_eq!(join.coverage, Some(0.75));
        assert_eq!(join.join_share, Some(2.0 / 3.0));
        let counts = &join.counts;
        let routed = (
            counts.runs,
            counts.runs_forecast,
            counts.routed,
            counts.routed_forecast,
        );
        assert_eq!(routed, (1, 1, 4, 3));
        let joined = (
            counts.forecasts,
            counts.joined,
            counts.no_verdict,
            counts.unlabelled,
        );
        assert_eq!(joined, (3, 2, 1, 0));
        let json = serde_json::to_value(&report).expect("serialize the report");
        assert_eq!(json["forecast_join"]["coverage"], 0.75);
        assert_eq!(json["forecast_join"]["no_verdict"], 1);
        let text = render_fit(&report, Style::plain());
        let coverage = "3 of 4 routed attempts forecast (coverage 0.750)";
        assert!(text.contains(coverage), "{text}");
        let share = "2 of 3 joined to a labelled verdict (join share 0.667)";
        assert!(text.contains(share), "{text}");

        // T3 settles unverified: a verdict without a label.
        let writer = TelemetryWriter::spawn(&run_dir, config).expect("the run's writer");
        assert!(writer.submit(verdict("T3", AttemptOutcome::Unverified)));
        writer.close();
        let counts = fit_offline(&layout, &snapshot, CalibrationGate::default())
            .expect("fit")
            .forecast_join
            .counts;
        assert_eq!(
            (counts.joined, counts.no_verdict, counts.unlabelled),
            (2, 0, 1)
        );
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
        let rows = snapshot.rows().len();
        assert_eq!(text.lines().count(), 1 + rows, "{text}");
        for row in snapshot.rows() {
            assert!(text.contains(&row.source_url));
        }
        let json = prices_json(&snapshot);
        assert_eq!(json["rows"].as_array().map(Vec::len), Some(rows));
    }

    /// gap-45f6bb: `roko learn self-model fit` runs without `--offline`: the fit reads the logs
    /// and dispatches nothing, the one mode there is.
    #[test]
    fn self_model_fit_runs_offline_by_default() {
        use clap::Parser as _;

        let workspace = tempfile::tempdir().expect("tempdir");
        let workdir = workspace.path().to_string_lossy().into_owned();
        let parse = || {
            Cli::try_parse_from(["roko", "learn", "self-model", "fit", "--workdir", &workdir])
                .expect("parse self-model fit")
        };
        let Some(Command::Learn {
            cmd: LearnCmd::SelfModel { cmd },
        }) = parse().command
        else {
            panic!("a self-model command");
        };
        assert_eq!(
            cmd_self_model(&parse(), cmd, true).expect("a fit without --offline"),
            0
        );
    }

    /// gap-45f6bb: the fit says what each score covers. The prequential scores every unit, the
    /// calibration gate its last W outcomes, and the live forecasts what the runs' shadow
    /// forecasts said at the time, against what happened.
    #[test]
    fn the_fit_names_its_windows_and_scores_the_live_forecasts() {
        use roko_learn::telemetry::records::{
            AttemptPredictionRecord, PredictionCandidate, PredictionDecision, PredictionPredictor,
        };
        use roko_learn::telemetry::{
            AttemptIdentity, AttemptKey, AttemptOpenRecord, AttemptOutcome, AttemptVerdictRecord,
            TelemetryWriter, TelemetryWriterConfig,
        };

        let workspace = tempfile::tempdir().expect("tempdir");

        // One run: the self-model forecast 0.8 for the arm that ran, and the try passed.
        let layout = RokoLayout::for_project(workspace.path());
        let identity = || AttemptIdentity::new(&AttemptKey::new("run-l", "plan-l", "T1", 1));
        let candidate: PredictionCandidate = serde_json::from_value(serde_json::json!({
            "arm": "roko/cerebras/gpt-oss-120b@default#V0",
            "p_gate": 0.8, "p_fg": 0.05, "p_vs": 0.76, "p_vs_lcb": 0.7,
            "cost_q50": 0.01, "cost_q90": 0.02, "lat_q50_s": 30.0, "lat_q90_s": 60.0,
        }))
        .expect("a candidate");
        let predictor = PredictionPredictor {
            version: "m3-l1-00000000".to_string(),
            class: "m3-l1".to_string(),
            mode: "shadow".to_string(),
            trained_on_n: 0,
            features_schema: 1,
            features_hash: "b3:0".to_string(),
        };
        let decision = PredictionDecision {
            would_choose: None,
            default: None,
            action: "dispatch".to_string(),
            start_rules: None,
        };
        let run_dir = layout.run_dir("run-l");
        std::fs::create_dir_all(&run_dir).expect("the run's directory");
        let writer =
            TelemetryWriter::spawn(&run_dir, TelemetryWriterConfig::default()).expect("a writer");
        assert!(writer.submit(AttemptOpenRecord::new(identity(), 0)));
        let forecast =
            AttemptPredictionRecord::new(identity(), predictor, vec![candidate], decision);
        assert!(writer.submit(forecast));
        let mut verdict = AttemptVerdictRecord::settle(identity(), AttemptOutcome::Passed, true);
        verdict.executed.provider = Some("cerebras".to_string());
        verdict.executed.model_dispatched = Some("gpt-oss-120b".to_string());
        assert!(writer.submit(verdict));
        writer.close();

        let snapshot = PriceSnapshot::builtin().expect("the built-in snapshot");
        let report = fit_offline(&layout, &snapshot, CalibrationGate::default()).expect("fit");
        let live = report.live_forecasts;
        assert_eq!((live.n, live.passes), (1, 1));
        let brier = live.brier.expect("a Brier score");
        assert!((brier - 0.04).abs() < 1e-9, "{brier}");
        let text = render_fit(&report, Style::plain());
        assert!(
            text.contains("scored before each unit taught the model, over all "),
            "{text}"
        );
        assert!(
            text.contains("\n  Live         as the runs made them: 1 with a verdict, Brier"),
            "{text}"
        );
    }

    /// gap-c63eb1: the prequential line counts the units it scores, and says why they are
    /// fewer than the units the fit trains on.
    #[test]
    fn the_prequential_line_counts_the_units_it_scores() {
        assert_eq!(prequential_scope(132, 132), "all 132 units");
        assert_eq!(
            prequential_scope(129, 139),
            "129 of the 139 units (the 10 failover substitutes train but aren't scored)"
        );
        assert_eq!(
            prequential_scope(5, 6),
            "5 of the 6 units (the failover substitute trains but isn't scored)"
        );
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
