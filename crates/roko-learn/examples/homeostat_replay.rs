//! Shadow replay of M1 over a stream of task resolutions: S06's first
//! slice, at $0.
//!
//! ```text
//! cargo run -p roko-learn --example homeostat_replay -- \
//!     --stream synthetic:model_swap@20 --mode shadow --seed 7
//! ```
//!
//! `--stream historical:<learn dir>` folds that directory's
//! `efficiency.jsonl` and `costs.jsonl`; `--stream synthetic:<kind>@<t>`
//! draws seeded resolutions that step after position `t` (`disturb.py`'s
//! kinds, `price_shock` and `provider_fault_all`). Every resolution goes to
//! the controller, and every row it causes prints as one `roko.controller/1`
//! JSON line on stdout. In shadow mode a move is a `param.change` with
//! `applied: false` and the SafetyBox's verdict, and a judgement is a
//! `param.evaluate` of `unevaluable_in_shadow`. A summary goes to stderr,
//! labelled `pre_instrumentation` for historical input. The replay reads
//! files only: it writes nothing and calls no provider.
//!
//! Other options: `--seed <n>` (7), `--length <n>` (synthetic only, `t` +
//! 80), `--policy <viability.toml>` (bounds calibrated on the stream), and
//! `--mode on`, which judges changes against outcomes that ignore θ: only
//! the replay evaluator's outcome table makes that meaningful.

use std::path::PathBuf;
use std::process::ExitCode;

use chrono::{DateTime, SecondsFormat, Utc};
use roko_core::config::homeostasis::{HomeostasisConfig, HomeostasisMode};
use roko_learn::homeostasis::controller::{Controller, ControllerEvent};
use roko_learn::homeostasis::ledger::{ControllerRecord, Envelope, write_jsonl};
use roko_learn::homeostasis::policy::ViabilityPolicy;
use roko_learn::homeostasis::streams::{StreamSpec, replay_theta0};
use roko_learn::telemetry::Arm;

/// What the command line asks for.
struct Args {
    stream: StreamSpec,
    mode: HomeostasisMode,
    seed: u64,
    length: Option<u64>,
    policy: Option<PathBuf>,
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(problem) => {
            eprintln!("homeostat_replay: {problem}");
            return ExitCode::from(2);
        }
    };
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(problem) => {
            eprintln!("homeostat_replay: {problem}");
            ExitCode::FAILURE
        }
    }
}

fn parse_args(mut words: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut stream = None;
    let mut mode = HomeostasisMode::Shadow;
    let mut seed = 7;
    let mut length = None;
    let mut policy = None;
    while let Some(flag) = words.next() {
        let value = words
            .next()
            .ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--stream" => stream = Some(StreamSpec::parse(&value)?),
            "--mode" => mode = parse_mode(&value)?,
            "--seed" => seed = value.parse().map_err(|error| format!("--seed: {error}"))?,
            "--length" => {
                let parsed = value
                    .parse()
                    .map_err(|error| format!("--length: {error}"))?;
                length = Some(parsed);
            }
            "--policy" => policy = Some(PathBuf::from(value)),
            other => return Err(format!("unknown option {other}")),
        }
    }
    let stream = stream.ok_or("--stream historical:<dir> | synthetic:<kind>@<t> is required")?;
    Ok(Args {
        stream,
        mode,
        seed,
        length,
        policy,
    })
}

fn parse_mode(text: &str) -> Result<HomeostasisMode, String> {
    match text {
        "shadow" => Ok(HomeostasisMode::Shadow),
        "on" => Ok(HomeostasisMode::On),
        other => Err(format!("--mode is shadow or on, not {other}")),
    }
}

fn run(args: &Args) -> Result<(), String> {
    let onset = match &args.stream {
        StreamSpec::Synthetic { onset, .. } => *onset,
        StreamSpec::Historical(_) => 0,
    };
    let length = args.length.unwrap_or(onset + 80);
    let loaded = args
        .stream
        .load(args.seed, length)
        .map_err(|error| format!("read the stream: {error}"))?;
    let policy = match &args.policy {
        Some(path) => ViabilityPolicy::load(path).map_err(|error| error.to_string())?,
        None => loaded.policy.clone(),
    };
    let settings = HomeostasisConfig {
        mode: args.mode,
        ..HomeostasisConfig::default()
    };
    let (theta0, ladders) = replay_theta0();
    let mut controller = Controller::new(
        &settings,
        policy.clone(),
        theta0,
        ladders,
        loaded.baseline,
        args.seed,
    );

    let mut stdout = std::io::stdout().lock();
    let (mut episodes, mut changes) = (0_usize, 0_usize);
    for (index, resolution) in loaded.resolutions.iter().enumerate() {
        let envelope = Envelope {
            ts: timestamp(resolution.resolved_at),
            run_id: None,
            policy_version: policy.policy_version,
            arm: Arm::Learned,
            seq: index as u64 + 1,
        };
        let events = controller.on_resolution(resolution);
        for event in &events {
            match event {
                ControllerEvent::EpisodeOpen { .. } => episodes += 1,
                ControllerEvent::Change(_) => changes += 1,
                _ => {}
            }
        }
        let records: Vec<ControllerRecord> = events
            .iter()
            .filter_map(|event| ControllerRecord::from_event(&envelope, event))
            .collect();
        write_jsonl(&mut stdout, &records).map_err(|error| format!("write: {error}"))?;
    }

    let resolutions = loaded.resolutions.len();
    let per_hundred = 100.0 * episodes as f64 / resolutions.max(1) as f64;
    let source = if loaded.pre_instrumentation {
        format!(
            "pre_instrumentation, replayed from logged outcomes ({}, n = {resolutions})",
            Utc::now().format("%Y-%m-%d")
        )
    } else {
        format!("replayed from synthetic outcomes, seed {}", args.seed)
    };
    eprintln!(
        "homeostat_replay: {}, mode {:?}: {resolutions} resolutions, {episodes} episodes \
         ({per_hundred:.1} per 100 resolutions), {changes} changes; {source}; $0 spent",
        args.stream.label(),
        args.mode
    );
    Ok(())
}

/// ISO-8601 UTC of unix ms; empty when unknown.
fn timestamp(ms: Option<i64>) -> String {
    ms.and_then(DateTime::<Utc>::from_timestamp_millis)
        .map(|at| at.to_rfc3339_opts(SecondsFormat::Millis, true))
        .unwrap_or_default()
}
