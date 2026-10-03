//! bench command handlers.

use crate::*;
use anyhow::Context as _;

#[derive(Debug, Subcommand)]
pub(crate) enum BenchCmd {
    /// Run a comparative benchmark: naive vs roko-optimized.
    #[command(after_help = "\
Examples:
  roko bench demo                     Run with simulated data
  roko bench demo --real              Run with real LLM dispatch")]
    Demo {
        /// Use real LLM dispatch instead of simulated results.
        #[arg(long)]
        real: bool,
        /// Working directory (default: cwd).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Run a native SWE-bench-style proxy batch.
    #[command(after_help = "\
Examples:
  roko bench swe --batch-size 2 --agent-mode gold      (control: checks the harness)
  roko bench swe --dataset ./swe-smoke.jsonl --predictions ./predictions.jsonl --agent-mode prediction-file
  roko bench swe --agent-mode command --agent-command './my-agent.sh'")]
    Swe {
        /// Local JSONL dataset. If omitted, a built-in two-task smoke dataset is generated.
        #[arg(long)]
        dataset: Option<PathBuf>,
        /// Number of instances to run.
        #[arg(long, default_value_t = 2)]
        batch_size: usize,
        /// Offset into the dataset.
        #[arg(long, default_value_t = 0)]
        offset: usize,
        /// Agent adapter to use (required). `gold` and `empty` are controls: they check the
        /// harness, not a model, and are never recorded as learning.
        #[arg(long, value_enum)]
        agent_mode: roko_cli::bench::SweAgentMode,
        /// Predictions JSONL path for --agent-mode prediction-file.
        #[arg(long)]
        predictions: Option<PathBuf>,
        /// Command for --agent-mode command. Receives instance JSON on stdin, prints a unified diff.
        #[arg(long)]
        agent_command: Option<String>,
        /// Scores JSONL output path.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Write SWE-bench-style predictions JSONL.
        #[arg(long)]
        export_predictions: Option<PathBuf>,
        /// Disable learning episode, efficiency, and C-factor writes.
        #[arg(long)]
        no_learning: bool,
        /// Keep per-instance benchmark workdirs for debugging.
        #[arg(long)]
        keep_workdirs: bool,
        /// Working directory (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
    },
    /// Run the ViabilityBench driver (benchmarks/viabilitybench/driver/vb.py) with the
    /// arguments after `--`, from the repository root, and exit with its exit code.
    #[command(after_help = "\
Examples:
  roko bench viability -- report --experiment PILOT-A
  roko bench viability -- estimate --stream pilot --arm cheap_direct --model gpt-oss-120b \
      --seeds 1-3

The driver runs under the benchmark's venv (benchmarks/viabilitybench/.venv/bin/python), or the
interpreter $VB_PYTHON names. benchmarks/viabilitybench/README.md describes its commands.")]
    Viability {
        /// Repository root that holds benchmarks/viabilitybench (default: cwd / --repo).
        #[arg(long)]
        workdir: Option<PathBuf>,
        /// Arguments passed to the driver unchanged.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

/// The ViabilityBench directory, relative to the repository root.
const VB_DIR: &str = "benchmarks/viabilitybench";
/// The driver, relative to [`VB_DIR`].
const VB_DRIVER: &str = "driver/vb.py";
/// The benchmark venv's interpreter, relative to [`VB_DIR`].
const VB_VENV_PYTHON: &str = ".venv/bin/python";

/// `roko bench viability`: run the ViabilityBench driver with `args` from the
/// repository root `root`, under `python` (`$VB_PYTHON`) or else the
/// benchmark's venv, and return the driver's exit code.
fn run_viability(
    root: &std::path::Path,
    args: &[String],
    python: Option<std::ffi::OsString>,
) -> Result<i32> {
    let driver = std::path::Path::new(VB_DIR).join(VB_DRIVER);
    anyhow::ensure!(
        root.join(&driver).is_file(),
        "no ViabilityBench driver at {}: run from the roko repository root, or pass --workdir",
        root.join(&driver).display()
    );
    let interpreter = match python {
        Some(python) => PathBuf::from(python),
        None => {
            let venv = root.join(VB_DIR).join(VB_VENV_PYTHON);
            anyhow::ensure!(
                venv.is_file(),
                "the ViabilityBench venv is missing ({}). Create it from the repository root with \
                 `python3 -m venv {VB_DIR}/.venv && {VB_DIR}/{VB_VENV_PYTHON} -m pip install \
                 --require-hashes -r {VB_DIR}/requirements.lock`, or set VB_PYTHON",
                venv.display()
            );
            venv
        }
    };
    let status = std::process::Command::new(&interpreter)
        .arg(&driver)
        .args(args)
        .current_dir(root)
        .status()
        .with_context(|| format!("run {} {}", interpreter.display(), driver.display()))?;
    Ok(status.code().unwrap_or(EXIT_FAILURE))
}

pub(crate) async fn cmd_bench(cli: &Cli, cmd: BenchCmd) -> Result<i32> {
    match cmd {
        BenchCmd::Demo { real, workdir } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            roko_cli::bench_demo::run_bench_demo(&workdir, real).await?;
            Ok(EXIT_SUCCESS)
        }
        BenchCmd::Swe {
            dataset,
            batch_size,
            offset,
            agent_mode,
            predictions,
            agent_command,
            report,
            export_predictions,
            no_learning,
            keep_workdirs,
            workdir,
        } => {
            let workdir = workdir.unwrap_or_else(|| resolve_workdir(cli));
            let report = roko_cli::bench::run_swe_bench(roko_cli::bench::SweBenchOptions {
                workdir,
                dataset,
                batch_size,
                offset,
                agent_mode,
                predictions,
                agent_command,
                report,
                export_predictions,
                record_learning: !no_learning,
                keep_workdirs,
            })
            .await?;

            if cli.json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", report.render_text());
                println!();
                println!(
                    "note: this is fast proxy scoring, not official SWE-bench Docker scoring."
                );
            }
            Ok(EXIT_SUCCESS)
        }
        BenchCmd::Viability { workdir, args } => {
            let root = workdir.unwrap_or_else(|| resolve_workdir(cli));
            run_viability(&root, &args, std::env::var_os("VB_PYTHON"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 3344: `roko bench viability -- <args>` hands the driver its arguments
    /// unchanged, from the repository root, and exits with the driver's code;
    /// a missing driver or venv is named.
    #[cfg(unix)]
    #[test]
    fn bench_viability_passes_arguments_through() {
        use std::os::unix::fs::PermissionsExt as _;

        let cli = Cli::try_parse_from([
            "roko",
            "bench",
            "viability",
            "--",
            "report",
            "--experiment",
            "PILOT-A",
        ])
        .expect("parse bench viability");
        let Some(Command::Bench {
            cmd: BenchCmd::Viability { workdir, args },
        }) = cli.command
        else {
            panic!("expected bench viability");
        };
        assert_eq!(workdir, None);
        assert_eq!(args, ["report", "--experiment", "PILOT-A"]);

        let root = tempfile::tempdir().expect("tempdir");
        let driver_dir = root.path().join(VB_DIR).join("driver");
        std::fs::create_dir_all(&driver_dir).expect("driver dir");
        std::fs::write(driver_dir.join("vb.py"), "").expect("driver");
        // A stub interpreter that records its directory and arguments.
        let log = root.path().join("argv.txt");
        let stub = root.path().join("stub-python");
        let script = format!(
            "#!/bin/sh\npwd -P > '{log}'\nprintf '%s\\n' \"$@\" >> '{log}'\nexit 7\n",
            log = log.display()
        );
        std::fs::write(&stub, script).expect("stub");
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");

        let code = run_viability(root.path(), &args, Some(stub.clone().into())).expect("ran");
        assert_eq!(code, 7, "the driver's exit code");
        let recorded = std::fs::read_to_string(&log).expect("stub log");
        let lines: Vec<&str> = recorded.lines().collect();
        let cwd = root.path().canonicalize().expect("canonical root");
        assert_eq!(lines[0], cwd.to_str().expect("utf-8 path"));
        let driver = format!("{VB_DIR}/{VB_DRIVER}");
        assert_eq!(
            lines[1..],
            [driver.as_str(), "report", "--experiment", "PILOT-A"]
        );

        let missing_venv = run_viability(root.path(), &args, None).expect_err("no venv");
        assert!(
            missing_venv.to_string().contains("venv is missing"),
            "{missing_venv}"
        );
        let elsewhere = tempfile::tempdir().expect("tempdir");
        let no_driver =
            run_viability(elsewhere.path(), &args, Some(stub.into())).expect_err("no driver");
        assert!(
            no_driver.to_string().contains("no ViabilityBench driver"),
            "{no_driver}"
        );
    }
}
