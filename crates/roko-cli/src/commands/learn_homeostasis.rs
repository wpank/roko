//! `roko learn homeostasis status|replay` (S06 §5; backlog 8133). Both only
//! read: `status` reports M1's saved state, and `replay` runs the controller
//! over a historical or synthetic stream of resolutions, writing its
//! `roko.controller/1` rows to `--out` or stdout. Neither calls a provider.
//!
//! `replay --evaluate` runs S06's replay evaluator instead (8117,
//! gap-1a8ee7): each `--arm` over the synthetic stream, once per seed, and
//! one `ArmReport` row per arm and disturbance, the table R-H6 reads
//! (`benchmarks/viabilitybench/analysis/replay_h6.py --table`).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow, bail};
use chrono::SecondsFormat;
use clap::Subcommand;
use roko_cli::exit_codes::EXIT_SUCCESS;
use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
use roko_fs::RokoLayout;
use roko_learn::guarded_commit::{COMMITS_DIR, GuardMode, GuardedStore};
use roko_learn::homeostasis::controller::{Controller, ControllerState, operator_mode};
use roko_learn::homeostasis::ledger::{ControllerRecord, Envelope, ledger_path, write_jsonl};
use roko_learn::homeostasis::lkg::STORE;
use roko_learn::homeostasis::replay::{Evaluator, ReplayArm};
use roko_learn::homeostasis::streams::{
    IN_CONTROL, StepKind, StreamSpec, SyntheticStream, replay_theta0,
};
use roko_learn::telemetry::Arm;
use serde::Serialize;
use serde_json::Value;

use crate::{Cli, resolve_workdir};

/// `roko learn homeostasis`.
#[derive(Debug, Subcommand)]
pub(crate) enum HomeostasisCmd {
    /// M1's mode and phase, θ against θ₀ per knob, θ's committed versions, the open
    /// episode and the last value of each essential variable.
    Status {
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Run the controller over `historical:<learn dir>` or `synthetic:<kind>@<t>`, and
    /// write its rows to `--out`, else stdout. Spends nothing.
    Replay {
        /// The stream: `historical:<learn dir>` or `synthetic:<kind>@<t>`.
        #[arg(long)]
        stream: String,
        /// `shadow` or `on`.
        #[arg(long, default_value = "shadow")]
        mode: String,
        /// Seed of the synthetic draws and the controller's decisions.
        #[arg(long, default_value_t = 7)]
        seed: u64,
        /// Resolutions of a synthetic stream (default: `t` + 80).
        #[arg(long)]
        length: Option<u64>,
        /// File the rows go to.
        #[arg(long)]
        out: Option<PathBuf>,
        /// Run S06's replay evaluator over the synthetic stream instead, and write one
        /// `ArmReport` row per arm and disturbance; `synthetic:all@<t>` steps into each of
        /// disturb.py's six kinds, R-H6's grid.
        #[arg(long)]
        evaluate: bool,
        /// An arm to evaluate: A0 to A5, A3-gated or A3-mis (default: all eight).
        #[arg(long = "arm")]
        arms: Vec<String>,
        /// Seeds each evaluated stream is replayed over, from `--seed` on.
        #[arg(long, default_value_t = 20)]
        seeds: u64,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
}

/// Working directory of a `roko learn homeostasis` subcommand.
pub(crate) fn homeostasis_workdir(cli: &Cli, cmd: &HomeostasisCmd) -> PathBuf {
    match cmd {
        HomeostasisCmd::Status { workdir } | HomeostasisCmd::Replay { workdir, .. } => {
            workdir.clone().unwrap_or_else(|| resolve_workdir(cli))
        }
    }
}

/// Run `roko learn homeostasis`.
pub(crate) fn cmd_homeostasis(cli: &Cli, cmd: HomeostasisCmd, json: bool) -> Result<i32> {
    let workdir = homeostasis_workdir(cli, &cmd);
    match cmd {
        HomeostasisCmd::Status { .. } => {
            let config =
                roko_core::config::loader::load_config_unified(&workdir).unwrap_or_default();
            match status(&workdir, config.homeostasis.mode) {
                None if json => println!("{}", serde_json::json!({ "state": null })),
                None => println!("no controller state"),
                Some(report) if json => println!("{}", serde_json::to_string_pretty(&report)?),
                Some(report) => print!("{}", report.render()),
            }
        }
        HomeostasisCmd::Replay {
            stream,
            seed,
            length,
            out,
            evaluate: true,
            arms,
            seeds,
            ..
        } => {
            let seeds: Vec<u64> = (0..seeds.max(1)).map(|n| seed.wrapping_add(n)).collect();
            evaluate_arms(&stream, &arms, &seeds, length, out.as_deref())?;
        }
        HomeostasisCmd::Replay {
            stream,
            mode,
            seed,
            length,
            out,
            ..
        } => replay(&stream, &mode, seed, length, out.as_deref())?,
    }
    Ok(EXIT_SUCCESS)
}

/// What `status` reports.
#[derive(Debug, Serialize)]
pub(crate) struct Status {
    /// The mode the next run opens in: a person's, else the config's.
    mode: HomeostasisMode,
    /// `idle`, `search` or `hold`.
    phase: String,
    /// The knobs where θ differs from θ₀.
    changed: Vec<KnobRow>,
    /// The versions θ's guarded store keeps.
    lkg_versions: Vec<u64>,
    /// The open episode.
    episode: Option<Value>,
    /// The last `ev.*` row of each essential variable in the ledger.
    evs: Vec<Value>,
}

/// One knob where θ differs from θ₀.
#[derive(Debug, Serialize)]
struct KnobRow {
    knob: String,
    theta: Value,
    theta0: Value,
}

impl Status {
    /// The report as people read it.
    fn render(&self) -> String {
        let mut text = format!("M1: {}, phase {}\n", label(self.mode), self.phase);
        if self.changed.is_empty() {
            text.push_str("θ: θ₀\n");
        }
        for row in &self.changed {
            let _ = writeln!(text, "θ: {} = {} (θ₀ {})", row.knob, row.theta, row.theta0);
        }
        let versions: Vec<String> = self.lkg_versions.iter().map(u64::to_string).collect();
        let versions = if versions.is_empty() {
            "none".to_string()
        } else {
            versions.join(", ")
        };
        let _ = writeln!(text, "committed versions of θ: {versions}");
        let episode = self
            .episode
            .as_ref()
            .and_then(|episode| episode["id"].as_str())
            .unwrap_or("none");
        let _ = writeln!(text, "open episode: {episode}");
        for row in &self.evs {
            let _ = writeln!(text, "{} {}: {}", row["kind"], row["ev"], row["value"]);
        }
        text
    }
}

/// M1's saved state in `workdir`, as `status` reports it; `None` without
/// one.
pub(crate) fn status(workdir: &Path, config_mode: HomeostasisMode) -> Option<Status> {
    let roko = RokoLayout::for_project(workdir).root().to_path_buf();
    let text = std::fs::read_to_string(Controller::state_path(&roko)).ok()?;
    let state: ControllerState = serde_json::from_str(&text).ok()?;
    let changed = state
        .theta
        .changed_knobs(&state.theta0)
        .into_iter()
        .map(|knob| KnobRow {
            knob: knob.to_string(),
            theta: state.theta.value(knob).unwrap_or(Value::Null),
            theta0: state.theta0.value(knob).unwrap_or(Value::Null),
        })
        .collect();
    let episode = state
        .episode
        .as_ref()
        .and_then(|episode| serde_json::to_value(episode).ok());
    Some(Status {
        mode: operator_mode(&roko).unwrap_or(config_mode),
        phase: label(state.phase),
        changed,
        lkg_versions: lkg_versions(&roko),
        episode,
        evs: last_evs(&roko),
    })
}

/// The versions θ's guarded store keeps; none before its first commit.
fn lkg_versions(roko: &Path) -> Vec<u64> {
    let learn = roko.join("learn");
    if !learn.join(COMMITS_DIR).join(STORE).is_dir() {
        return Vec::new();
    }
    GuardedStore::open(&learn, STORE, GuardMode::Observe)
        .and_then(|store| store.versions())
        .unwrap_or_default()
}

/// The ledger's last `ev.*` row of each essential variable.
fn last_evs(roko: &Path) -> Vec<Value> {
    let mut last: BTreeMap<String, Value> = BTreeMap::new();
    let text = std::fs::read_to_string(ledger_path(roko)).unwrap_or_default();
    let rows = text
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok());
    for row in rows {
        if row["kind"].as_str().unwrap_or_default().starts_with("ev.") {
            last.insert(row["ev"].to_string(), row);
        }
    }
    last.into_values().collect()
}

/// Replay the controller over `stream` in `mode`, its rows to `out` or
/// stdout.
fn replay(
    stream: &str,
    mode: &str,
    seed: u64,
    length: Option<u64>,
    out: Option<&Path>,
) -> Result<()> {
    let spec = StreamSpec::parse(stream).map_err(|problem| anyhow!("--stream: {problem}"))?;
    let mode = match mode {
        "shadow" => HomeostasisMode::Shadow,
        "on" => HomeostasisMode::On,
        other => bail!("--mode is shadow or on, not {other}"),
    };
    let onset = match &spec {
        StreamSpec::Synthetic { onset, .. } => *onset,
        StreamSpec::Historical(_) => 0,
    };
    let loaded = spec
        .load(seed, length.unwrap_or(onset + 80))
        .context("read the stream")?;
    let settings = HomeostasisConfig {
        mode,
        ..HomeostasisConfig::default()
    };
    let (theta0, ladders) = replay_theta0();
    let policy = loaded.policy.clone();
    let mut controller = Controller::new(
        &settings,
        policy.clone(),
        theta0,
        ladders,
        loaded.baseline,
        seed,
    );
    let mut rows = Vec::new();
    for (index, resolution) in loaded.resolutions.iter().enumerate() {
        let ts = resolution
            .resolved_at
            .and_then(chrono::DateTime::from_timestamp_millis)
            .map(|at| at.to_rfc3339_opts(SecondsFormat::Millis, true))
            .unwrap_or_default();
        let envelope = Envelope {
            ts,
            run_id: None,
            policy_version: policy.policy_version,
            arm: Arm::Learned,
            seq: u64::try_from(index).unwrap_or(u64::MAX).saturating_add(1),
        };
        let events = controller.on_resolution(resolution);
        let records = events
            .iter()
            .filter_map(|event| ControllerRecord::from_event(&envelope, event));
        rows.extend(records);
    }
    match out {
        Some(path) => {
            let mut file = std::fs::File::create(path)
                .with_context(|| format!("create {}", path.display()))?;
            write_jsonl(&mut file, &rows)?;
        }
        None => write_jsonl(&mut std::io::stdout().lock(), &rows)?,
    }
    Ok(())
}

/// `replay --evaluate`: S06's replay evaluator over `stream`, each of `arms`
/// (every arm when empty) replayed once per seed, its `ArmReport` rows
/// written to `out` or stdout as JSON lines.
fn evaluate_arms(
    stream: &str,
    arms: &[String],
    seeds: &[u64],
    length: Option<u64>,
    out: Option<&Path>,
) -> Result<()> {
    let (steps, onset) = evaluated_steps(stream)?;
    let arms = if arms.is_empty() {
        ReplayArm::ALL.to_vec()
    } else {
        arms.iter()
            .map(|arm| parse_arm(arm))
            .collect::<Result<_>>()?
    };
    let (theta0, ladders) = replay_theta0();
    // A synthetic stream's bounds are calibrated in control, whatever it
    // steps into.
    let stream = SyntheticStream {
        step: steps[0],
        onset,
        seed: seeds[0],
    };
    let evaluator = Evaluator::new(stream.policy(), theta0, ladders, IN_CONTROL);
    let length = length.unwrap_or(onset + 80);
    let reports = evaluator.evaluate(&steps, &arms, onset, length, seeds);
    let mut text = String::new();
    for report in &reports {
        text.push_str(&serde_json::to_string(report)?);
        text.push('\n');
    }
    match out {
        Some(path) => {
            std::fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
        }
        None => print!("{text}"),
    }
    Ok(())
}

/// The steps and onset `--evaluate` replays: `synthetic:<kind>@<t>`, or
/// `synthetic:all@<t>` for disturb.py's six kinds. A historical stream has
/// no outcome table to replay under another θ until S09's Stage-A export.
fn evaluated_steps(stream: &str) -> Result<(Vec<StepKind>, u64)> {
    if let Some(onset) = stream.strip_prefix("synthetic:all@") {
        let onset = onset
            .parse()
            .with_context(|| format!("`{onset}` is not a position"))?;
        let steps = roko_learn::homeostasis::catalog::DisturbanceKind::DISTURB_PY
            .map(StepKind::of)
            .to_vec();
        return Ok((steps, onset));
    }
    match StreamSpec::parse(stream).map_err(|problem| anyhow!("--stream: {problem}"))? {
        StreamSpec::Synthetic { step, onset } => Ok((vec![step], onset)),
        StreamSpec::Historical(_) => {
            bail!("--evaluate needs a synthetic stream; historical ones have no outcome table yet")
        }
    }
}

/// The arm `label` names: `A0` to `A5`, `A3-gated` or `A3-mis`.
fn parse_arm(label: &str) -> Result<ReplayArm> {
    ReplayArm::ALL
        .into_iter()
        .find(|arm| arm.label() == label)
        .ok_or_else(|| anyhow!("--arm is A0 to A5, A3-gated or A3-mis, not {label}"))
}

/// `value`'s name as records write it.
fn label<T: Serialize>(value: T) -> String {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use roko_core::config::harness_params::{HarnessLadders, HarnessParams, Knob, Step};
    use roko_core::config::schema::RokoConfig;
    use roko_core::disturbance::DisturbanceKind;
    use roko_learn::homeostasis::detect::Baseline;
    use roko_learn::homeostasis::lkg::ThetaLkg;
    use roko_learn::homeostasis::policy::ViabilityPolicy;

    use super::*;

    const POLICY: &str = "policy_version = 1\n\
        ev.pass_rate = { lo = 0.70 }\nev.usd_per_verified_success = { hi = 0.12 }\n\
        ev.false_green = { hi = 0.10 }\nev.latency_p90_s = { hi = 900 }\n";

    /// S06 §5 (8133): `status` reads M1's saved state: its phase, the knob
    /// where θ left θ₀, and θ's two committed versions. A fresh workspace
    /// has none, and the command says so.
    #[test]
    fn learn_homeostasis_status_reads_controller_state() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(status(dir.path(), HomeostasisMode::Shadow).is_none());

        let roko = dir.path().join(".roko");
        let config = RokoConfig::default();
        let theta0 = HarnessParams::baseline(&config);
        let ladders = HarnessLadders::from_config(&config);
        let policy = ViabilityPolicy::parse(POLICY).expect("the policy parses");
        let baseline = Baseline {
            pass_rate: 0.80,
            usd_per_resolution: 0.05,
            wall_ms: 300_000.0,
        };
        let mut controller = Controller::new(
            &HomeostasisConfig::default(),
            policy.clone(),
            theta0.clone(),
            ladders.clone(),
            baseline,
            0,
        );
        let raised = theta0
            .step(Knob::RetryDelta, Step::Up, &ladders)
            .expect("one more retry");
        controller.adopt(raised.clone()).expect("θ is in the box");
        controller
            .save(&Controller::state_path(&roko))
            .expect("the state is saved");
        let mut lkg = ThetaLkg::open(&roko.join("learn"), theta0.clone(), ladders, policy)
            .expect("θ's guarded store");
        for theta in [&theta0, &raised] {
            lkg.commit(theta, "homeostat:test", true).expect("a commit");
        }

        let report = status(dir.path(), HomeostasisMode::Shadow).expect("a report");
        assert_eq!(report.mode, HomeostasisMode::Shadow);
        assert_eq!(report.phase, "idle");
        assert_eq!(report.lkg_versions, [1, 2]);
        let changed: Vec<(&str, &Value, &Value)> = report
            .changed
            .iter()
            .map(|row| (row.knob.as_str(), &row.theta, &row.theta0))
            .collect();
        assert_eq!(changed, [("retry_delta", &Value::from(1), &Value::from(0))]);
        assert!(report.episode.is_none());
        let text = report.render();
        assert!(text.contains("θ: retry_delta = 1 (θ₀ 0)"), "{text}");
        assert!(text.contains("committed versions of θ: 1, 2"), "{text}");
    }

    /// The JSON rows of the file at `path`.
    fn read_rows(path: &Path) -> Vec<Value> {
        let text = std::fs::read_to_string(path).expect("the table");
        text.lines()
            .map(|line| serde_json::from_str(line).expect("a row"))
            .collect()
    }

    /// gap-1a8ee7: `replay --evaluate` writes the evaluator's verdict, one
    /// `ArmReport` row per arm and disturbance in the shape R-H6's `--table`
    /// reads: an IAE over the seeds where the arm ran, a note where it could
    /// not, and every disturb.py kind for `synthetic:all@<t>`.
    #[test]
    fn homeostasis_replay_reports_the_evaluator_s_verdict() {
        let temp = tempfile::tempdir().expect("tempdir");
        let out = temp.path().join("h6-table.jsonl");
        let arms = ["A0", "A3", "A3-gated"].map(String::from);
        evaluate_arms(
            "synthetic:model_swap@20",
            &arms,
            &[1, 2, 3],
            Some(100),
            Some(&out),
        )
        .expect("evaluated");
        let rows = read_rows(&out);
        let label = |row: &Value| {
            let arm = row["arm"].as_str().unwrap_or_default();
            let kind = row["disturbance"].as_str().unwrap_or_default();
            format!("{arm} {kind}")
        };
        let labels: Vec<String> = rows.iter().map(label).collect();
        assert_eq!(
            labels,
            ["A0 model_swap", "A3 model_swap", "A3-gated model_swap"]
        );
        for row in &rows[..2] {
            let iae = &row["iae"];
            assert_eq!(iae["n"], 3, "{row}");
            let at = |key: &str| iae[key].as_f64().expect("a number");
            assert!(at("low") <= at("mean"), "{row}");
            assert!(at("mean") <= at("high"), "{row}");
            assert!(row["note"].is_null(), "{row}");
        }
        assert!(rows[2]["iae"].is_null());
        assert_eq!(rows[2]["note"], "no predictions");

        // `synthetic:all@<t>`: one row per disturb.py kind, R-H6's six.
        let a0 = ["A0".to_string()];
        evaluate_arms("synthetic:all@20", &a0, &[1], Some(60), Some(&out)).expect("evaluated");
        let kinds: Vec<String> = read_rows(&out)
            .iter()
            .map(|row| row["disturbance"].as_str().unwrap_or_default().to_string())
            .collect();
        let six = DisturbanceKind::DISTURB_PY.map(DisturbanceKind::name);
        assert_eq!(kinds, six);

        // A historical stream has no outcome table, and an unknown arm is refused.
        assert!(evaluate_arms("historical:/nowhere", &[], &[1], None, None).is_err());
        assert!(parse_arm("A9").is_err());
    }
}
